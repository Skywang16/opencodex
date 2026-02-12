//! Prompt configuration - simplified version
//!
//! Prompts are now all stored in prompts/*.md files,
//! this module only retains necessary configuration types.

use serde::{Deserialize, Serialize};

/// Agent type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentType {
    Coder,
    Plan,
    Explore,
    General,
    Research,
    Custom(String),
}

impl AgentType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Coder => "coder",
            Self::Plan => "plan",
            Self::Explore => "explore",
            Self::General => "general",
            Self::Research => "research",
            Self::Custom(name) => name.as_str(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "coder" => Self::Coder,
            "plan" => Self::Plan,
            "explore" => Self::Explore,
            "general" => Self::General,
            "research" => Self::Research,
            other => Self::Custom(other.to_string()),
        }
    }
}

/// Reminder type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ReminderType {
    PlanMode,
    CoderWithPlan,
    NoWorkspace,
    LoopWarning,
    DuplicateTools,
}

impl ReminderType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::PlanMode => "plan_mode",
            Self::CoderWithPlan => "coder_with_plan",
            Self::NoWorkspace => "no_workspace",
            Self::LoopWarning => "loop_warning",
            Self::DuplicateTools => "duplicate_tools",
        }
    }
}
