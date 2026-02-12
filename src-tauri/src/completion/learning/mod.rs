//! Offline completion learning (SQLite-backed).
//!
//! Design goals:
//! - Truly "get smarter": Each command execution updates transition probabilities and usage frequency
//! - Small size: Keep model size around ~10MB through TTL + TopK + Key count limits
//! - Don't interfere with shell: Only affects OpenCodex suggestions/ranking, Tab completion still handled by shell

use crate::completion::command_line::extract_command_key;
use crate::storage::database::DatabaseManager;
use crate::storage::repositories::CompletionModelRepo;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::mpsc;
use tracing::warn;

#[derive(Debug, Clone, Copy)]
pub struct CompletionLearningConfig {
    pub ttl_days: u64,
    pub max_command_keys: i64,
    pub transitions_per_prev: i64,
    pub maintenance_every: u64,
}

impl Default for CompletionLearningConfig {
    fn default() -> Self {
        Self {
            ttl_days: 30,
            max_command_keys: 10_000,
            transitions_per_prev: 12,
            maintenance_every: 64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandFinishedEvent {
    pub pane_id: u32,
    pub command_line: String,
    pub cwd: Option<String>,
    pub exit_code: Option<i32>,
    pub finished_ts: u64,
}

#[derive(Clone)]
pub struct CompletionLearningState {
    database: Arc<DatabaseManager>,
    config: CompletionLearningConfig,
    sender: OnceLock<mpsc::UnboundedSender<CommandFinishedEvent>>,
}

impl CompletionLearningState {
    pub fn new(database: Arc<DatabaseManager>, config: CompletionLearningConfig) -> Self {
        Self {
            database,
            config,
            sender: OnceLock::new(),
        }
    }

    pub fn record_finished(&self, event: CommandFinishedEvent) {
        let sender = self.ensure_started();
        let _ = sender.send(event);
    }

    fn ensure_started(&self) -> &mpsc::UnboundedSender<CommandFinishedEvent> {
        self.sender.get_or_init(|| {
            let (sender, mut receiver) = mpsc::unbounded_channel::<CommandFinishedEvent>();
            let database = Arc::clone(&self.database);
            let config = self.config;

            // Note: Cannot spawn here during app setup phase (no tokio reactor).
            // Use lazy initialization: Start background task only when first command event is received, at which point we're already in Tauri runtime.
            tauri::async_runtime::spawn(async move {
                let mut per_pane_prev: HashMap<u32, i64> = HashMap::new();
                let mut seen = 0_u64;

                while let Some(event) = receiver.recv().await {
                    if let Err(err) = apply_finished_event(
                        &database,
                        config,
                        &mut per_pane_prev,
                        &event,
                        &mut seen,
                    )
                    .await
                    {
                        warn!(error = %err, "completion.learning.apply_failed");
                    }
                }
            });

            sender
        })
    }
}

async fn apply_finished_event(
    database: &DatabaseManager,
    config: CompletionLearningConfig,
    per_pane_prev: &mut HashMap<u32, i64>,
    event: &CommandFinishedEvent,
    seen: &mut u64,
) -> crate::storage::error::RepositoryResult<()> {
    let Some(key) = extract_command_key(&event.command_line) else {
        return Ok(());
    };

    let success = event.exit_code.map(|code| code == 0);
    let repo = CompletionModelRepo::new(database);
    let current_id = repo
        .upsert_command_key(
            &key.key,
            &key.root,
            key.sub.as_deref(),
            event.finished_ts,
            success,
        )
        .await?;

    if let Some(prev_id) = per_pane_prev.get(&event.pane_id).copied() {
        if prev_id != current_id {
            repo.upsert_transition(prev_id, current_id, event.finished_ts, success)
                .await?;
            repo.enforce_transition_top_k_per_prev(prev_id, config.transitions_per_prev)
                .await?;
        }
    }

    per_pane_prev.insert(event.pane_id, current_id);

    *seen = seen.saturating_add(1);
    if *seen % config.maintenance_every == 0 {
        let cutoff_ts = event
            .finished_ts
            .saturating_sub(config.ttl_days.saturating_mul(24 * 60 * 60));
        repo.prune_older_than(cutoff_ts).await?;
        repo.enforce_command_key_limit(config.max_command_keys)
            .await?;
    }

    Ok(())
}
