use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionConfig {
    pub enabled: bool,
    pub keep_recent_messages: u32,
    pub min_context_usage_ratio: f32,
    pub max_summary_chars: u32,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            keep_recent_messages: 8,
            min_context_usage_ratio: 0.7,
            max_summary_chars: 8_000,
        }
    }
}
