/// Command sequence prediction engine
///
use super::command_pairs::get_suggested_commands;
use crate::completion::smart_extractor::SmartExtractor;
use crate::completion::types::{CompletionItem, CompletionType};
use std::path::PathBuf;

/// Prediction result
#[derive(Debug, Clone)]
pub struct PredictionResult {
    /// Predicted command
    pub command: String,
    /// Auto-injected arguments
    pub arguments: Vec<String>,
    /// Confidence score
    pub confidence: f64,
    /// Source description
    pub source: String,
}

impl PredictionResult {
    /// Generate full command string
    pub fn full_command(&self) -> String {
        if self.arguments.is_empty() {
            self.command.clone()
        } else {
            format!("{} {}", self.command, self.arguments.join(" "))
        }
    }

    /// Convert to completion item
    pub fn to_completion_item(&self) -> CompletionItem {
        let score = (90.0 + (self.confidence / 10.0)).min(100.0);
        CompletionItem::new(self.full_command(), CompletionType::Command)
            .with_score(score)
            .with_source(self.source.clone())
            .with_description(format!(
                "Predicted next command (confidence: {:.0}%)",
                self.confidence
            ))
    }
}

/// Command sequence predictor
pub struct CommandPredictor {
    /// Entity extractor
    extractor: &'static SmartExtractor,
}

impl CommandPredictor {
    /// Create new predictor
    pub fn new(_current_dir: PathBuf) -> Self {
        Self {
            extractor: SmartExtractor::global(),
        }
    }

    /// Predict next command
    pub fn predict_next_commands(
        &self,
        last_command: &str,
        last_output: Option<&str>,
        input_prefix: &str,
    ) -> Vec<PredictionResult> {
        let mut predictions = Vec::new();

        // Step 1: Find related commands
        if let Some(suggested) = get_suggested_commands(last_command) {
            for cmd in suggested {
                // Filter: only keep commands matching input prefix
                if cmd.starts_with(input_prefix) || input_prefix.is_empty() {
                    // Step 2: Extract entities and inject arguments
                    let prediction = self.build_prediction(&cmd, last_command, last_output);
                    predictions.push(prediction);
                }
            }
        }

        // Step 3: Sort by confidence
        predictions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        predictions
    }

    pub fn build_prediction_for_suggestion(
        &self,
        suggested_cmd: &str,
        last_command: &str,
        last_output: Option<&str>,
    ) -> PredictionResult {
        self.build_prediction(suggested_cmd, last_command, last_output)
    }

    /// Build prediction result (including argument injection)
    fn build_prediction(
        &self,
        suggested_cmd: &str,
        last_command: &str,
        last_output: Option<&str>,
    ) -> PredictionResult {
        let mut arguments = Vec::new();
        let mut confidence = 50.0;

        // Try to inject arguments based on command type
        if let Some(output) = last_output {
            match suggested_cmd {
                cmd if cmd.starts_with("kill") => {
                    // Extract PID from lsof/ps output
                    if let Some(pid) = self.extract_first_pid(last_command, output) {
                        arguments.push(pid);
                        confidence = 85.0;
                    }
                }
                cmd if cmd.starts_with("docker stop") || cmd.starts_with("docker logs") => {
                    // Extract container ID from docker ps output
                    if let Some(container_id) = self.extract_container_id(output) {
                        arguments.push(container_id);
                        confidence = 85.0;
                    }
                }
                cmd if cmd.starts_with("git add") => {
                    // Extract modified file from git status output
                    if let Some(file) = self.extract_modified_file(output) {
                        arguments.push(file);
                        confidence = 80.0;
                    }
                }
                _ => {
                    // Default confidence
                    confidence = 50.0;
                }
            }
        }

        PredictionResult {
            command: suggested_cmd.to_string(),
            arguments,
            confidence,
            source: format!(
                "Prediction based on '{}'",
                last_command.split_whitespace().next().unwrap_or("")
            ),
        }
    }

    /// Extract first PID
    fn extract_first_pid(&self, last_command: &str, output: &str) -> Option<String> {
        // Use SmartExtractor to extract PID
        match self.extractor.extract_entities(last_command, output) {
            Ok(results) => {
                // Find first PID
                results
                    .into_iter()
                    .find(|r| r.entity_type == "pid")
                    .map(|r| r.value)
            }
            Err(_) => None,
        }
    }

    /// Extract container ID
    fn extract_container_id(&self, output: &str) -> Option<String> {
        // Simple pattern matching for docker ps output
        // First column is container ID
        output
            .lines()
            .nth(1)
            .and_then(|line| line.split_whitespace().next())
            .map(|id| id.to_string())
    }

    /// Extract modified file
    fn extract_modified_file(&self, output: &str) -> Option<String> {
        // Match filename in git status output
        // Example: "modified:   src/main.rs"
        for line in output.lines() {
            if line.contains("modified:") || line.contains("new file:") {
                if let Some(file) = line.split(':').nth(1) {
                    return Some(file.trim().to_string());
                }
            }
            // Also support short format: " M src/main.rs"
            if line.starts_with(" M ") || line.starts_with("?? ") {
                if let Some(file) = line.split_whitespace().nth(1) {
                    return Some(file.to_string());
                }
            }
        }
        None
    }

    // Context-based scoring has been removed: learning models are responsible for dynamic weights
    // based on "project/user behavior". Static heuristics only create edge cases and unexplainable rankings.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_lsof_kill_prediction() {
        let predictor = CommandPredictor::new(env::current_dir().unwrap());

        let last_cmd = "lsof -i :8080";
        let last_output = Some("COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME\nnode    12345 user   23u  IPv4 0x1234      0t0  TCP *:8080 (LISTEN)");

        let predictions = predictor.predict_next_commands(last_cmd, last_output, "");

        assert!(!predictions.is_empty());
        assert!(predictions.iter().any(|p| p.command.starts_with("kill")));

        // Should automatically extract PID
        let kill_pred = predictions
            .iter()
            .find(|p| p.command.starts_with("kill"))
            .unwrap();
        assert!(kill_pred.arguments.contains(&"12345".to_string()));
    }

    #[test]
    fn test_docker_workflow() {
        let predictor = CommandPredictor::new(env::current_dir().unwrap());

        let last_cmd = "docker ps";
        let last_output = Some("CONTAINER ID   IMAGE     COMMAND                  CREATED         STATUS         PORTS                    NAMES\nabc123def456   nginx     \"/docker-entrypoint.…\"   2 hours ago     Up 2 hours     0.0.0.0:8080->80/tcp     my-nginx");

        let predictions = predictor.predict_next_commands(last_cmd, last_output, "");

        assert!(!predictions.is_empty());
        let stop_pred = predictions
            .iter()
            .find(|p| p.command.starts_with("docker stop"));
        assert!(stop_pred.is_some());

        // Should automatically extract container ID
        assert!(stop_pred
            .unwrap()
            .arguments
            .contains(&"abc123def456".to_string()));
    }

    #[test]
    fn test_git_workflow() {
        let predictor = CommandPredictor::new(env::current_dir().unwrap());

        let last_cmd = "git status";
        let last_output =
            Some("On branch main\nChanges not staged for commit:\n  modified:   src/main.rs");

        let predictions = predictor.predict_next_commands(last_cmd, last_output, "");

        assert!(!predictions.is_empty());
        let add_pred = predictions
            .iter()
            .find(|p| p.command.starts_with("git add"));
        assert!(add_pred.is_some());

        // Should automatically extract filename
        assert!(add_pred
            .unwrap()
            .arguments
            .contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn test_input_prefix_filter() {
        let predictor = CommandPredictor::new(env::current_dir().unwrap());

        let last_cmd = "git status";

        // Only return predictions starting with "git a"
        let predictions = predictor.predict_next_commands(last_cmd, None, "git a");

        assert!(predictions.iter().all(|p| p.command.starts_with("git a")));
    }
}
