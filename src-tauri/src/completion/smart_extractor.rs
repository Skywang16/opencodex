//! Smart entity extractor

use crate::completion::error::{SmartExtractorError, SmartExtractorResult};
use crate::completion::CompletionRuntime;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Smart entity extractor
pub struct SmartExtractor {
    /// Extraction rules
    rules: Vec<ExtractionRule>,
    /// Common patterns
    patterns: HashMap<String, Regex>,
}

/// Extraction rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionRule {
    /// Rule name
    pub name: String,

    /// Applicable command patterns
    pub command_patterns: Vec<String>,

    /// Output signatures (used to identify command output types)
    pub output_signatures: Vec<String>,

    /// Entity extraction patterns
    pub entity_patterns: Vec<EntityPattern>,

    /// Rule priority
    pub priority: i32,

    /// Whether enabled
    pub enabled: bool,
}

/// Entity pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityPattern {
    /// Entity type
    pub entity_type: String,

    /// Regular expression pattern
    pub pattern: String,

    /// Capture group index (defaults to 1)
    pub capture_group: Option<usize>,

    /// Minimum confidence
    pub min_confidence: f64,

    /// Context requirements (optional)
    pub context_requirements: Option<Vec<String>>,
}

/// Extraction result
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// Entity type
    pub entity_type: String,

    /// Entity value
    pub value: String,

    /// Confidence
    pub confidence: f64,

    /// Context information
    pub context: HashMap<String, String>,
}

impl SmartExtractor {
    /// Create new smart extractor
    pub fn new() -> Self {
        let mut extractor = Self {
            rules: Vec::new(),
            patterns: HashMap::new(),
        };

        // Load default rules
        extractor.load_default_rules();
        extractor.compile_patterns();

        extractor
    }

    /// Get global instance
    pub fn global() -> &'static SmartExtractor {
        CompletionRuntime::global().extractor()
    }

    /// Load default rules
    fn load_default_rules(&mut self) {
        // Process-related rules
        self.rules.push(ExtractionRule {
            name: "process_list".to_string(),
            command_patterns: vec!["lsof.*".to_string(), "ps.*".to_string()],
            output_signatures: vec![
                "COMMAND.*PID.*USER".to_string(),
                "PID.*TTY.*TIME.*CMD".to_string(),
            ],
            entity_patterns: vec![
                EntityPattern {
                    entity_type: "pid".to_string(),
                    pattern: r"\b(\d{1,6})\b".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.8,
                    context_requirements: None,
                },
                EntityPattern {
                    entity_type: "process_name".to_string(),
                    pattern: r"^(\S+)\s+\d+".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.7,
                    context_requirements: None,
                },
            ],
            priority: 10,
            enabled: true,
        });

        // Network-related rules
        self.rules.push(ExtractionRule {
            name: "network_info".to_string(),
            command_patterns: vec![
                "netstat.*".to_string(),
                "ss.*".to_string(),
                "lsof.*-i.*".to_string(),
            ],
            output_signatures: vec![
                "Proto.*Local Address.*Foreign Address".to_string(),
                "Netid.*State.*Recv-Q".to_string(),
                "COMMAND.*PID.*USER.*FD.*TYPE".to_string(),
            ],
            entity_patterns: vec![
                EntityPattern {
                    entity_type: "port".to_string(),
                    pattern: r":(\d{1,5})\b".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.9,
                    context_requirements: None,
                },
                EntityPattern {
                    entity_type: "ip_address".to_string(),
                    pattern: r"\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\b".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.8,
                    context_requirements: None,
                },
                EntityPattern {
                    entity_type: "pid".to_string(),
                    pattern: r"\b(\d{1,6})\b".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.7,
                    context_requirements: Some(vec!["COMMAND".to_string()]),
                },
            ],
            priority: 10,
            enabled: true,
        });

        // Filesystem-related rules
        self.rules.push(ExtractionRule {
            name: "filesystem".to_string(),
            command_patterns: vec!["ls.*".to_string(), "find.*".to_string()],
            output_signatures: vec![
                r"^[drwx-]{10}".to_string(), // ls -l output
                r"^\./".to_string(),         // find output
            ],
            entity_patterns: vec![
                EntityPattern {
                    entity_type: "file_path".to_string(),
                    pattern: r"([^\s]+\.[a-zA-Z0-9]+)$".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.6,
                    context_requirements: None,
                },
                EntityPattern {
                    entity_type: "directory_path".to_string(),
                    pattern: r"^d[rwx-]{9}\s+\d+\s+\S+\s+\S+\s+\d+\s+\S+\s+\d+\s+[\d:]+\s+(.+)$"
                        .to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.7,
                    context_requirements: None,
                },
            ],
            priority: 5,
            enabled: true,
        });

        // Git-related rules
        self.rules.push(ExtractionRule {
            name: "git_info".to_string(),
            command_patterns: vec!["git.*".to_string()],
            output_signatures: vec![
                "commit [a-f0-9]{40}".to_string(),
                r"\* \w+".to_string(), // git branch output
            ],
            entity_patterns: vec![
                EntityPattern {
                    entity_type: "git_commit".to_string(),
                    pattern: r"commit ([a-f0-9]{7,40})".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.9,
                    context_requirements: None,
                },
                EntityPattern {
                    entity_type: "git_branch".to_string(),
                    pattern: r"^\*?\s*([^\s]+)$".to_string(),
                    capture_group: Some(1),
                    min_confidence: 0.8,
                    context_requirements: None,
                },
            ],
            priority: 8,
            enabled: true,
        });

        // Generic number pattern (as fallback)
        self.rules.push(ExtractionRule {
            name: "generic_numbers".to_string(),
            command_patterns: vec![".*".to_string()],
            output_signatures: vec![],
            entity_patterns: vec![EntityPattern {
                entity_type: "number".to_string(),
                pattern: r"\b(\d+)\b".to_string(),
                capture_group: Some(1),
                min_confidence: 0.3,
                context_requirements: None,
            }],
            priority: 1,
            enabled: true,
        });
    }

    /// Compile regular expression patterns
    fn compile_patterns(&mut self) {
        for rule in &self.rules {
            for pattern in &rule.entity_patterns {
                if let Ok(regex) = Regex::new(&pattern.pattern) {
                    let key = format!("{}_{}", rule.name, pattern.entity_type);
                    self.patterns.insert(key, regex);
                }
            }

            // Compile command patterns
            for (i, cmd_pattern) in rule.command_patterns.iter().enumerate() {
                if let Ok(regex) = Regex::new(cmd_pattern) {
                    let key = format!("{}_cmd_{}", rule.name, i);
                    self.patterns.insert(key, regex);
                }
            }

            // Compile output signature patterns
            for (i, sig_pattern) in rule.output_signatures.iter().enumerate() {
                if let Ok(regex) = Regex::new(sig_pattern) {
                    let key = format!("{}_sig_{}", rule.name, i);
                    self.patterns.insert(key, regex);
                }
            }
        }
    }

    /// Extract entities
    pub fn extract_entities(
        &self,
        command: &str,
        output: &str,
    ) -> SmartExtractorResult<Vec<ExtractionResult>> {
        let mut results = Vec::new();

        // Find applicable rules
        let applicable_rules = self.find_applicable_rules(command, output);

        for rule in applicable_rules {
            for pattern in &rule.entity_patterns {
                if let Some(entities) = self.extract_with_pattern(pattern, output, rule)? {
                    results.extend(entities);
                }
            }
        }

        // Sort by confidence and deduplicate
        results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        self.deduplicate_results(results)
    }

    /// Find applicable rules
    fn find_applicable_rules(&self, command: &str, output: &str) -> Vec<&ExtractionRule> {
        let mut applicable = Vec::new();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            let command_matches = rule.command_patterns.iter().any(|pattern| {
                if let Some(regex) = self.patterns.get(&format!(
                    "{}_cmd_{}",
                    rule.name,
                    rule.command_patterns
                        .iter()
                        .position(|p| p == pattern)
                        .unwrap()
                )) {
                    regex.is_match(command)
                } else {
                    false
                }
            });

            let output_matches = rule.output_signatures.is_empty()
                || rule.output_signatures.iter().any(|signature| {
                    if let Some(regex) = self.patterns.get(&format!(
                        "{}_sig_{}",
                        rule.name,
                        rule.output_signatures
                            .iter()
                            .position(|s| s == signature)
                            .unwrap()
                    )) {
                        regex.is_match(output)
                    } else {
                        false
                    }
                });

            if command_matches && output_matches {
                applicable.push(rule);
            }
        }

        // Sort by priority
        applicable.sort_by_key(|rule| std::cmp::Reverse(rule.priority));
        applicable
    }

    /// Extract entities using pattern
    fn extract_with_pattern(
        &self,
        pattern: &EntityPattern,
        output: &str,
        rule: &ExtractionRule,
    ) -> SmartExtractorResult<Option<Vec<ExtractionResult>>> {
        let pattern_key = format!("{}_{}", rule.name, pattern.entity_type);
        let regex = self.patterns.get(&pattern_key).ok_or_else(|| {
            SmartExtractorError::MissingCompiledPattern {
                pattern_key: pattern_key.clone(),
            }
        })?;

        let mut results = Vec::new();
        let capture_group = pattern.capture_group.unwrap_or(1);

        for captures in regex.captures_iter(output) {
            if let Some(matched) = captures.get(capture_group) {
                let value = matched.as_str().to_string();

                if let Some(requirements) = &pattern.context_requirements {
                    if !self.check_context_requirements(requirements, output, matched.start()) {
                        continue;
                    }
                }

                let confidence = self.calculate_confidence(pattern, &value);

                if confidence >= pattern.min_confidence {
                    results.push(ExtractionResult {
                        entity_type: pattern.entity_type.clone(),
                        value,
                        confidence,
                        context: HashMap::new(),
                    });
                }
            }
        }

        Ok(if results.is_empty() {
            None
        } else {
            Some(results)
        })
    }

    /// Check context requirements
    fn check_context_requirements(
        &self,
        requirements: &[String],
        output: &str,
        position: usize,
    ) -> bool {
        let line_start = output[..position]
            .rfind('\n')
            .map(|pos| pos + 1)
            .unwrap_or(0);
        let line_end = output[position..]
            .find('\n')
            .map(|pos| position + pos)
            .unwrap_or(output.len());
        let line = &output[line_start..line_end];

        requirements.iter().any(|req| line.contains(req))
    }

    /// Calculate confidence
    fn calculate_confidence(&self, pattern: &EntityPattern, value: &str) -> f64 {
        let mut confidence = pattern.min_confidence;

        // Adjust confidence based on entity type
        match pattern.entity_type.as_str() {
            "pid" => {
                if let Ok(pid) = value.parse::<u32>() {
                    if pid > 0 && pid < 65536 {
                        confidence += 0.1;
                    }
                }
            }
            "port" => {
                if let Ok(port) = value.parse::<u16>() {
                    if port > 0 {
                        confidence += 0.1;
                    }
                }
            }
            "ip_address" => {
                // Simple IP address validation
                let parts: Vec<&str> = value.split('.').collect();
                if parts.len() == 4 && parts.iter().all(|part| part.parse::<u8>().is_ok()) {
                    confidence += 0.1;
                }
            }
            _ => {}
        }

        confidence.min(1.0)
    }

    /// Deduplicate results
    fn deduplicate_results(
        &self,
        mut results: Vec<ExtractionResult>,
    ) -> SmartExtractorResult<Vec<ExtractionResult>> {
        let mut seen = std::collections::HashSet::new();
        results.retain(|result| {
            let key = format!("{}:{}", result.entity_type, result.value);
            seen.insert(key)
        });

        Ok(results)
    }

    /// Get rule statistics
    pub fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();

        stats.insert(
            "total_rules".to_string(),
            serde_json::Value::Number(serde_json::Number::from(self.rules.len())),
        );

        stats.insert(
            "enabled_rules".to_string(),
            serde_json::Value::Number(serde_json::Number::from(
                self.rules.iter().filter(|r| r.enabled).count(),
            )),
        );

        stats.insert(
            "total_patterns".to_string(),
            serde_json::Value::Number(serde_json::Number::from(self.patterns.len())),
        );

        let entity_types: std::collections::HashSet<String> = self
            .rules
            .iter()
            .flat_map(|r| r.entity_patterns.iter().map(|p| p.entity_type.clone()))
            .collect();

        stats.insert(
            "supported_entity_types".to_string(),
            serde_json::Value::Array(
                entity_types
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );

        stats
    }
}

impl Default for SmartExtractor {
    fn default() -> Self {
        Self::new()
    }
}
