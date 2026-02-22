//! Prompt Orchestrator - Build task prompts

use std::path::Path;
use std::sync::Arc;

use crate::agent::agents::AgentConfigLoader;
use crate::agent::context::ProjectContextLoader;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::prompt::{BuiltinPrompts, PromptBuilder, SystemPromptParts};
use crate::agent::tools::ToolRegistry;
use crate::settings::SettingsManager;
use crate::storage::repositories::AppPreferences;
use crate::storage::{DatabaseManager, UnifiedCache};

pub struct PromptOrchestrator {
    cache: Arc<UnifiedCache>,
    database: Arc<DatabaseManager>,
    settings_manager: Arc<SettingsManager>,
}

impl PromptOrchestrator {
    pub fn new(
        cache: Arc<UnifiedCache>,
        database: Arc<DatabaseManager>,
        settings_manager: Arc<SettingsManager>,
    ) -> Self {
        Self {
            cache,
            database,
            settings_manager,
        }
    }

    async fn load_rules(
        &self,
        workspace_path: &str,
    ) -> TaskExecutorResult<(Option<String>, Option<String>)> {
        let effective = self
            .settings_manager
            .get_effective_settings(Some(std::path::PathBuf::from(workspace_path)))
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let global_rules = {
            let rules = effective.rules_content.trim();
            if rules.is_empty() {
                None
            } else {
                Some(rules.to_string())
            }
        };

        let prefs = AppPreferences::new(&self.database);
        let project_rules = prefs
            .get("workspace.project_rules")
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let _ = self.cache.set_global_rules(global_rules.clone()).await;
        let _ = self.cache.set_project_rules(project_rules.clone()).await;

        Ok((global_rules, project_rules))
    }

    async fn has_agent_messages(
        &self,
        session_id: i64,
        agent_type: &str,
    ) -> TaskExecutorResult<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(1) FROM messages WHERE session_id = ? AND agent_type = ? LIMIT 1",
        )
        .bind(session_id)
        .bind(agent_type)
        .fetch_one(self.database.pool())
        .await
        .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        Ok(count > 0)
    }

    fn get_reminder(&self, agent_type: &str, has_plan_history: bool) -> Option<String> {
        if agent_type == "plan" {
            return Some(BuiltinPrompts::reminder_plan_mode().to_string());
        }

        if agent_type == "coder" && has_plan_history {
            return Some(BuiltinPrompts::reminder_coder_with_plan().to_string());
        }

        None
    }

    fn get_directory_listing(workspace_path: &str) -> Option<String> {
        let path = Path::new(workspace_path);
        let mut entries: Vec<String> = std::fs::read_dir(path)
            .ok()?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || name == "~" {
                    return None;
                }
                Some(if entry.file_type().ok()?.is_dir() {
                    format!("{name}/")
                } else {
                    name
                })
            })
            .collect();

        entries.sort();
        (!entries.is_empty()).then(|| {
            format!(
                "Directory listing (top-level):\n```\n{}\n```",
                entries.join("\n")
            )
        })
    }

    fn get_git_info(workspace_path: &str) -> Option<String> {
        use std::process::Command;

        let path = Path::new(workspace_path);
        if !path.join(".git").exists() {
            return None;
        }

        let mut info_parts = Vec::new();

        // Get current branch
        if let Ok(output) = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(workspace_path)
            .output()
        {
            if output.status.success() {
                let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                info_parts.push(format!("Git branch: {branch}"));
            }
        }

        // Get short status (dirty/clean)
        if let Ok(output) = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(workspace_path)
            .output()
        {
            if output.status.success() {
                let status = String::from_utf8_lossy(&output.stdout);
                let changed_count = status.lines().count();
                if changed_count > 0 {
                    info_parts.push(format!("Git status: {changed_count} file(s) changed"));
                } else {
                    info_parts.push("Git status: clean".to_string());
                }
            }
        }

        if info_parts.is_empty() {
            None
        } else {
            Some(info_parts.join("\n"))
        }
    }

    pub async fn build_task_prompts(
        &self,
        session_id: i64,
        _task_id: String,
        user_prompt: &str,
        agent_type: &str,
        workspace_path: &str,
        _tool_registry: &ToolRegistry,
        model_id: Option<&str>,
    ) -> TaskExecutorResult<(String, String)> {
        let cwd = workspace_path;

        // Load agent configuration
        let agent_configs = AgentConfigLoader::load_for_workspace(&std::path::PathBuf::from(cwd))
            .await
            .unwrap_or_default();

        let agent_cfg = agent_configs
            .get(agent_type)
            .or_else(|| agent_configs.get("coder"));

        // Load rules
        let (global_rules, project_rules) = self.load_rules(workspace_path).await?;

        // Load project context
        let loader = ProjectContextLoader::new(cwd);
        let project_context = loader.load_with_preference(project_rules.as_deref()).await;

        // Build custom instructions
        let mut custom_parts = Vec::new();
        if let Some(ctx) = project_context {
            custom_parts.push(ctx.format_for_prompt());
        }
        if let Some(rules) = global_rules {
            custom_parts.push(rules);
        }

        let custom_instructions = if custom_parts.is_empty() {
            None
        } else {
            Some(custom_parts.join("\n\n"))
        };

        // Get reminder
        let has_plan_history = self
            .has_agent_messages(session_id, "plan")
            .await
            .unwrap_or(false);
        let reminder = self.get_reminder(agent_type, has_plan_history);

        // Build environment info with directory listing and git status
        let mut prompt_builder = PromptBuilder::new(Some(workspace_path.to_string()));
        let dir_listing = Self::get_directory_listing(cwd);
        let git_info = Self::get_git_info(cwd);
        let env_info =
            prompt_builder.build_env_info(Some(cwd), dir_listing.as_deref(), git_info.as_deref());

        let agent_prompt = agent_cfg.map(|cfg| cfg.system_prompt.clone());

        // Model-specific prompt profile (workspace override > builtin > generic fallback)
        let model_profile = if let Some(mid) = model_id {
            let family = super::model_harness::ModelFamily::detect(mid);
            let profile_key = family.profile_key();
            tracing::debug!(
                "Model harness: model_id={}, family={}, profile={}",
                mid,
                family.name(),
                profile_key
            );

            match prompt_builder.get_model_profile_prompt(profile_key).await {
                Some(profile) => Some(profile),
                None if profile_key != "generic" => {
                    tracing::warn!(
                        "Missing model profile '{}' for model '{}', fallback to generic",
                        profile_key,
                        mid
                    );
                    prompt_builder.get_model_profile_prompt("generic").await
                }
                None => None,
            }
        } else {
            tracing::debug!("Model harness: no model_id, using generic profile");
            prompt_builder.get_model_profile_prompt("generic").await
        };

        let system_prompt = prompt_builder
            .build_system_prompt(SystemPromptParts {
                agent_prompt,
                model_profile,
                env_info: Some(env_info),
                reminder,
                custom_instructions,
                user_system: None,
            })
            .await;

        Ok((system_prompt, user_prompt.to_string()))
    }
}
