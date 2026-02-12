//! Prompt builder - assembles complete system prompt

use chrono::Utc;
use std::collections::HashMap;

use super::loader::{BuiltinPrompts, PromptLoader};

/// Parts of the system prompt
#[derive(Debug, Clone, Default)]
pub struct SystemPromptParts {
    /// Agent-specific prompts (loaded from agents/*.md, frontmatter removed)
    pub agent_prompt: Option<String>,
    /// Basic rules
    pub rules: Option<String>,
    /// Work methodology
    pub methodology: Option<String>,
    /// Environment information
    pub env_info: Option<String>,
    /// Runtime reminder
    pub reminder: Option<String>,
    /// User custom instructions (CLAUDE.md / project rules)
    pub custom_instructions: Option<String>,
    /// Model-specific hints (from model_harness module)
    pub model_hints: Option<String>,
}

/// Prompt builder
pub struct PromptBuilder {
    loader: PromptLoader,
}

impl PromptBuilder {
    pub fn new(workspace_path: Option<String>) -> Self {
        Self {
            loader: PromptLoader::new(workspace_path),
        }
    }

    /// Build complete system prompt
    ///
    /// Assembly order (each section builds on the previous):
    /// 1. Role — identity, personality, output format, progress updates
    /// 2. Agent-specific prompt — mode-specific behavior (coder, plan, explore, etc.)
    /// 3. Rules — autonomous execution, tone, tool usage, proactiveness
    /// 4. Methodology — conventions, code style, task workflow, git safety, validation
    /// 5. Environment — working directory, platform, date, project structure
    /// 6. Project instructions — AGENTS.md, CLAUDE.md, user custom rules
    /// 7. Reminder — runtime context (plan mode, max steps, loop warnings)
    pub async fn build_system_prompt(&mut self, parts: SystemPromptParts) -> String {
        let mut sections = Vec::new();

        // 1. Role — identity and core behavior (always included)
        sections.push(BuiltinPrompts::role().to_string());

        // 2. Agent-specific prompts (coder, plan, explore, etc.)
        if let Some(agent_prompt) = parts.agent_prompt {
            let body = strip_frontmatter(&agent_prompt);
            if !body.trim().is_empty() {
                sections.push(body);
            }
        }

        // 3. Rules — autonomous execution, tone, proactiveness
        if let Some(rules) = parts.rules {
            sections.push(rules);
        } else {
            sections.push(BuiltinPrompts::rules().to_string());
        }

        // 4. Methodology — conventions, workflow, git safety, validation
        if let Some(methodology) = parts.methodology {
            sections.push(methodology);
        } else {
            sections.push(BuiltinPrompts::methodology().to_string());
        }

        // 5. Environment information
        if let Some(env_info) = parts.env_info {
            sections.push(env_info);
        }

        // 6. Project instructions (AGENTS.md / CLAUDE.md / user custom rules)
        if let Some(custom) = parts.custom_instructions {
            if !custom.trim().is_empty() {
                sections.push(format!("# Project Instructions\n\n{}", custom.trim()));
            }
        }

        // 6.5. Model-specific hints (after project instructions, before reminder)
        if let Some(hints) = parts.model_hints {
            if !hints.trim().is_empty() {
                sections.push(hints.trim().to_string());
            }
        }

        // 7. Runtime reminder (placed last, highest priority override)
        if let Some(reminder) = parts.reminder {
            // Check if reminder already contains system-reminder tags
            let trimmed = reminder.trim();
            if trimmed.starts_with("<system-reminder>") {
                sections.push(trimmed.to_string());
            } else {
                sections.push(format!(
                    "<system-reminder>\n{}\n</system-reminder>",
                    trimmed
                ));
            }
        }

        sections.join("\n\n").trim().to_string()
    }

    /// Build environment information
    pub fn build_env_info(
        &self,
        working_directory: Option<&str>,
        file_list_preview: Option<&str>,
        git_info: Option<&str>,
    ) -> String {
        let wd = working_directory.unwrap_or("(none)");
        let platform = std::env::consts::OS;
        let date = Utc::now().format("%Y-%m-%d").to_string();

        let mut env = format!(
            "Here is useful information about the environment you are running in:\n\n<env>\nWorking directory: {}\nPlatform: {}\nToday's date: {}",
            wd, platform, date
        );

        if let Some(git) = git_info {
            env.push_str("\n");
            env.push_str(git);
        }

        env.push_str("\n</env>");

        if let Some(files) = file_list_preview {
            if !files.trim().is_empty() {
                env.push_str("\n\n");
                env.push_str(files);
            }
        }

        env
    }

    /// Get agent prompt
    pub async fn get_agent_prompt(&mut self, agent_type: &str) -> Option<String> {
        self.loader.load("agents", agent_type).await
    }

    /// Get reminder
    pub async fn get_reminder(&mut self, name: &str) -> Option<String> {
        self.loader.load("reminders", name).await
    }

    /// Render template variables
    pub fn render_template(template: &str, vars: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in vars {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }
        result
    }

    /// Get loop warning reminder and fill variables
    pub fn get_loop_warning(&self, count: usize, tools: &str) -> String {
        let template = BuiltinPrompts::reminder_loop_warning();
        let mut vars = HashMap::new();
        vars.insert("count".to_string(), count.to_string());
        vars.insert("tools".to_string(), tools.to_string());
        Self::render_template(template, &vars)
    }

    /// Get duplicate tools reminder and fill variables
    pub fn get_duplicate_tools_warning(&self, count: usize) -> String {
        let template = BuiltinPrompts::reminder_duplicate_tools();
        let mut vars = HashMap::new();
        vars.insert("count".to_string(), count.to_string());
        Self::render_template(template, &vars)
    }
}

/// Remove frontmatter, return only body content
fn strip_frontmatter(content: &str) -> String {
    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        return content.to_string();
    }

    // Find the second ---
    if let Some(end_idx) = trimmed[3..].find("---") {
        let body_start = 3 + end_idx + 3;
        return trimmed[body_start..].trim().to_string();
    }

    content.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_frontmatter() {
        let content = r#"---
name: test
description: Test agent
---

This is the body."#;

        let body = strip_frontmatter(content);
        assert_eq!(body, "This is the body.");
    }

    #[test]
    fn test_strip_frontmatter_no_frontmatter() {
        let content = "Just plain content";
        let body = strip_frontmatter(content);
        assert_eq!(body, "Just plain content");
    }

    #[test]
    fn test_render_template() {
        let template = "Hello {{name}}, you have {{count}} messages.";
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        vars.insert("count".to_string(), "5".to_string());

        let result = PromptBuilder::render_template(template, &vars);
        assert_eq!(result, "Hello Alice, you have 5 messages.");
    }
}
