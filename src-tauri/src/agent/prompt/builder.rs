//! Prompt builder - assembles complete system prompt

use chrono::Utc;
use std::collections::HashMap;

use super::loader::{BuiltinPrompts, PromptLoader};

/// Parts of the system prompt
///
/// `agent_prompt` and `model_profile` are
/// **mutually exclusive** — if the agent defines a custom prompt it takes
/// precedence; otherwise the model-family profile is used as the primary
/// system prompt.  Environment, project instructions, and reminders are
/// appended after the primary prompt.
#[derive(Debug, Clone, Default)]
pub struct SystemPromptParts {
    /// Agent-specific prompt — if set, **replaces** model_profile as the
    /// primary system prompt (loaded from agents/*.md, frontmatter removed).
    pub agent_prompt: Option<String>,
    /// Model-family system prompt (loaded from prompts/models/*.md).
    /// Complete behavioral instructions: rules, methodology, tool usage,
    /// tone, conventions — everything the model needs to operate.
    /// Used as the primary prompt when agent_prompt is None.
    pub model_profile: Option<String>,
    /// Environment information (cwd, platform, date, file listing)
    pub env_info: Option<String>,
    /// Runtime reminder (plan mode, max steps, loop warnings)
    pub reminder: Option<String>,
    /// User custom instructions (AGENTS.md / CLAUDE.md / project rules)
    pub custom_instructions: Option<String>,
    /// Optional per-user system message (passed through from the user)
    pub user_system: Option<String>,
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
    /// ```text
    /// primary = agent_prompt || model_profile
    /// system  = [primary, env_info, custom_instructions, user_system, reminder]
    /// ```
    ///
    /// `agent_prompt` and `model_profile` are **mutually exclusive**.
    /// If the agent defines a custom prompt it is used as the sole primary
    /// system prompt; otherwise the model-family profile takes that role.
    pub async fn build_system_prompt(&mut self, parts: SystemPromptParts) -> String {
        let mut sections: Vec<String> = Vec::new();

        // Primary system prompt: agent_prompt wins over model_profile
        let primary = if let Some(agent_prompt) = parts.agent_prompt {
            let body = strip_frontmatter(&agent_prompt);
            if body.trim().is_empty() {
                None
            } else {
                Some(body)
            }
        } else {
            None
        };

        let primary = primary.or_else(|| {
            parts
                .model_profile
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
        });

        if let Some(p) = primary {
            sections.push(p);
        }

        // Environment information
        if let Some(env_info) = parts.env_info {
            sections.push(env_info);
        }

        // Project instructions (AGENTS.md / CLAUDE.md / user custom rules)
        if let Some(custom) = parts.custom_instructions {
            if !custom.trim().is_empty() {
                sections.push(format!("# Project Instructions\n\n{}", custom.trim()));
            }
        }

        // Per-user system message
        if let Some(user_sys) = parts.user_system {
            if !user_sys.trim().is_empty() {
                sections.push(user_sys.trim().to_string());
            }
        }

        // Runtime reminder (placed last, highest priority override)
        if let Some(reminder) = parts.reminder {
            let trimmed = reminder.trim();
            if trimmed.starts_with("<system-reminder>") {
                sections.push(trimmed.to_string());
            } else {
                sections.push(format!("<system-reminder>\n{trimmed}\n</system-reminder>"));
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
            "Here is useful information about the environment you are running in:\n\n<env>\nWorking directory: {wd}\nPlatform: {platform}\nToday's date: {date}"
        );

        if let Some(git) = git_info {
            env.push('\n');
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

    /// Get model profile prompt
    pub async fn get_model_profile_prompt(&mut self, profile_key: &str) -> Option<String> {
        self.loader.load("models", profile_key).await
    }

    /// Get reminder
    pub async fn get_reminder(&mut self, name: &str) -> Option<String> {
        self.loader.load("reminders", name).await
    }

    /// Render template variables
    pub fn render_template(template: &str, vars: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in vars {
            result = result.replace(&format!("{{{{{key}}}}}"), value);
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
