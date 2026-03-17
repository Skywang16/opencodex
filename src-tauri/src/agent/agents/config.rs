use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::agent::permissions::{matches_simple_glob, PermissionDecision, ToolFilter};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    Primary,
    TaskProfile,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPermissionRule {
    pub pattern: String,
    pub decision: PermissionDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    pub name: String,
    pub description: Option<String>,
    pub mode: AgentMode,
    pub system_prompt: String,
    /// Tool filter for agent capability boundaries (whitelist/blacklist).
    /// This is separate from Settings permissions (allow/deny/ask).
    pub tool_filter: ToolFilter,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub task_permissions: Vec<TaskPermissionRule>,
    pub max_steps: Option<u32>,
    pub model_id: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub color: Option<String>,
    pub hidden: bool,
    pub source_path: Option<String>,
    pub is_builtin: bool,
}

impl AgentConfig {
    pub fn task_permission_for(&self, target_agent: &str) -> PermissionDecision {
        let mut best: Option<(usize, PermissionDecision)> = None;

        for rule in &self.task_permissions {
            if !matches_simple_glob(&rule.pattern, target_agent) {
                continue;
            }

            let specificity = rule
                .pattern
                .chars()
                .filter(|ch| *ch != '*' && *ch != '?')
                .count();

            match best {
                None => best = Some((specificity, rule.decision)),
                Some((best_specificity, best_decision)) => {
                    if specificity > best_specificity
                        || (specificity == best_specificity
                            && decision_rank(rule.decision) > decision_rank(best_decision))
                    {
                        best = Some((specificity, rule.decision));
                    }
                }
            }
        }

        best.map(|(_, decision)| decision)
            .unwrap_or(PermissionDecision::Allow)
    }

    pub fn can_delegate_to(&self, target_agent: &str) -> bool {
        self.task_permission_for(target_agent) != PermissionDecision::Deny
    }
}

fn decision_rank(decision: PermissionDecision) -> u8 {
    match decision {
        PermissionDecision::Allow => 0,
        PermissionDecision::Ask => 1,
        PermissionDecision::Deny => 2,
    }
}

pub fn visible_task_profiles(
    caller: &AgentConfig,
    configs: &HashMap<String, AgentConfig>,
) -> Vec<String> {
    let mut targets = configs
        .values()
        .filter(|cfg| cfg.mode == AgentMode::TaskProfile && !cfg.hidden)
        .filter(|cfg| caller.can_delegate_to(&cfg.name))
        .map(|cfg| cfg.name.clone())
        .collect::<Vec<_>>();
    targets.sort();
    targets.dedup();
    targets
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(name: &str, mode: AgentMode, hidden: bool) -> AgentConfig {
        AgentConfig {
            name: name.to_string(),
            description: None,
            mode,
            system_prompt: String::new(),
            tool_filter: ToolFilter::allow_all(),
            skills: Vec::new(),
            task_permissions: Vec::new(),
            max_steps: None,
            model_id: None,
            temperature: None,
            top_p: None,
            color: None,
            hidden,
            source_path: None,
            is_builtin: false,
        }
    }

    #[test]
    fn task_permission_priority_is_deny_then_ask_then_allow() {
        let mut cfg = make_config("coder", AgentMode::Primary, false);
        cfg.task_permissions = vec![
            TaskPermissionRule {
                pattern: "*".to_string(),
                decision: PermissionDecision::Allow,
            },
            TaskPermissionRule {
                pattern: "research".to_string(),
                decision: PermissionDecision::Ask,
            },
            TaskPermissionRule {
                pattern: "research".to_string(),
                decision: PermissionDecision::Deny,
            },
        ];

        assert_eq!(
            cfg.task_permission_for("explore"),
            PermissionDecision::Allow
        );
        assert_eq!(
            cfg.task_permission_for("research"),
            PermissionDecision::Deny
        );
    }

    #[test]
    fn visible_targets_respect_hidden_and_permissions() {
        let mut caller = make_config("coder", AgentMode::Primary, false);
        caller.task_permissions = vec![
            TaskPermissionRule {
                pattern: "*".to_string(),
                decision: PermissionDecision::Deny,
            },
            TaskPermissionRule {
                pattern: "explore".to_string(),
                decision: PermissionDecision::Allow,
            },
        ];

        let explore = make_config("explore", AgentMode::TaskProfile, false);
        let research = make_config("research", AgentMode::TaskProfile, false);
        let hidden = make_config("internal-research", AgentMode::TaskProfile, true);

        let configs = HashMap::from([
            (caller.name.clone(), caller.clone()),
            (explore.name.clone(), explore),
            (research.name.clone(), research),
            (hidden.name.clone(), hidden),
        ]);

        assert_eq!(visible_task_profiles(&caller, &configs), vec!["explore"]);
    }
}
