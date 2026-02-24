/*!
 * Shortcut action execution system
 *
 * Responsibilities:
 * - Action registration and management
 * - Action execution scheduling
 * - Context passing
 */

use super::types::{ActionContext, OperationResult, ShortcutEvent, ShortcutEventType};
use crate::config::error::{ShortcutsActionError, ShortcutsActionResult, ShortcutsResult};
use crate::config::types::ShortcutAction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, warn};

pub type ActionHandler =
    Box<dyn Fn(&ActionContext) -> ShortcutsActionResult<serde_json::Value> + Send + Sync>;

pub type ShortcutEventListener = Arc<dyn Fn(&ShortcutEvent) + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMetadata {
    pub name: String,
    pub description: String,
    pub requires_terminal: bool,
    pub is_system_action: bool,
    pub supported_platforms: Vec<String>,
}

pub struct ActionRegistry {
    handlers: Arc<RwLock<HashMap<String, ActionHandler>>>,
    metadata: Arc<RwLock<HashMap<String, ActionMetadata>>>,
    event_listeners: Arc<RwLock<Vec<ShortcutEventListener>>>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            event_listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn init_defaults(&mut self) {
        self.register_default_actions().await;
    }

    pub async fn register_action<F>(
        &mut self,
        metadata: ActionMetadata,
        handler: F,
    ) -> ShortcutsResult<()>
    where
        F: Fn(&ActionContext) -> ShortcutsActionResult<serde_json::Value> + Send + Sync + 'static,
    {
        let action_name = metadata.name.clone();

        {
            let handlers = self.handlers.read().await;
            if handlers.contains_key(&action_name) {
                return Err(ShortcutsActionError::AlreadyRegistered {
                    action: action_name,
                }
                .into());
            }
        }

        {
            let mut meta_map = self.metadata.write().await;
            meta_map.insert(action_name.clone(), metadata);
        }

        {
            let mut handler_map = self.handlers.write().await;
            handler_map.insert(action_name, Box::new(handler));
        }
        Ok(())
    }

    pub async fn execute_action(
        &self,
        action: &ShortcutAction,
        context: &ActionContext,
    ) -> OperationResult<serde_json::Value> {
        let action_name = self.extract_action_name(action);

        self.emit_event(ShortcutEvent {
            event_type: ShortcutEventType::KeyPressed,
            key_combination: Some(context.key_combination.clone()),
            action: Some(action_name.clone()),
            data: HashMap::new(),
            timestamp: chrono::Utc::now(),
        })
        .await;

        let handler_exists = {
            let handlers = self.handlers.read().await;
            handlers.contains_key(&action_name)
        };

        if !handler_exists {
            let error_msg = format!("Action not registered: {action_name}");
            warn!("{}", error_msg);

            self.emit_event(ShortcutEvent {
                event_type: ShortcutEventType::ActionFailed,
                key_combination: Some(context.key_combination.clone()),
                action: Some(action_name),
                data: HashMap::from([(
                    "error".to_string(),
                    serde_json::Value::String(error_msg.clone()),
                )]),
                timestamp: chrono::Utc::now(),
            })
            .await;

            return OperationResult::failure(error_msg);
        }

        let result = {
            let handlers = self.handlers.read().await;
            match handlers.get(&action_name) {
                Some(handler) => handler(context),
                None => Err(ShortcutsActionError::NotRegistered {
                    action: action_name.clone(),
                }),
            }
        };

        match result {
            Ok(value) => {
                self.emit_event(ShortcutEvent {
                    event_type: ShortcutEventType::ActionExecuted,
                    key_combination: Some(context.key_combination.clone()),
                    action: Some(action_name),
                    data: HashMap::from([("result".to_string(), value.clone())]),
                    timestamp: chrono::Utc::now(),
                })
                .await;

                OperationResult::success(value)
            }
            Err(err) => {
                let error_msg = format!("Action execution failed: {err}");
                error!("{}", error_msg);

                self.emit_event(ShortcutEvent {
                    event_type: ShortcutEventType::ActionFailed,
                    key_combination: Some(context.key_combination.clone()),
                    action: Some(action_name),
                    data: HashMap::from([(
                        "error".to_string(),
                        serde_json::Value::String(error_msg.clone()),
                    )]),
                    timestamp: chrono::Utc::now(),
                })
                .await;

                OperationResult::failure(error_msg)
            }
        }
    }

    pub async fn is_action_registered(&self, action_name: &str) -> bool {
        let handlers = self.handlers.read().await;
        handlers.contains_key(action_name)
    }

    async fn emit_event(&self, event: ShortcutEvent) {
        let listeners = self.event_listeners.read().await.clone();
        for listener in &listeners {
            listener(&event);
        }
    }

    fn extract_action_name(&self, action: &ShortcutAction) -> String {
        match action {
            ShortcutAction::Simple(name) => name.clone(),
            ShortcutAction::Complex { action_type, .. } => action_type.clone(),
        }
    }

    async fn register_default_actions(&mut self) {
        self.register_global_actions().await;
        self.register_terminal_actions().await;
        self.register_system_actions().await;
    }

    async fn register_global_actions(&mut self) {
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "copy_to_clipboard".to_string(),
                    description: "Copy selected content to clipboard".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 Copy function triggered!".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "paste_from_clipboard".to_string(),
                    description: "Paste content from clipboard".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 Paste function triggered!".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "terminal_search".to_string(),
                    description: "Terminal search".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 Search function triggered!".to_string(),
                    ))
                },
            )
            .await;
    }

    async fn register_terminal_actions(&mut self) {
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "new_terminal".to_string(),
                    description: "New terminal".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 New terminal function triggered!".to_string(),
                    ))
                },
            )
            .await;
    }

    async fn register_system_actions(&mut self) {
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "clear_terminal".to_string(),
                    description: "Clear terminal".to_string(),

                    requires_terminal: true,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 Clear terminal function triggered!".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "open_settings".to_string(),
                    description: "Open settings".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 Open settings function triggered!".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "command_palette".to_string(),
                    description: "Toggle command palette".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "Command palette toggled".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "toggle_terminal_panel".to_string(),
                    description: "Toggle terminal panel".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "Terminal panel toggled".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "toggle_window_pin".to_string(),
                    description: "Pin/unpin window".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| Ok(serde_json::Value::String("Window pin toggled".to_string())),
            )
            .await;
    }
}

impl Clone for ActionRegistry {
    fn clone(&self) -> Self {
        Self {
            handlers: Arc::clone(&self.handlers),
            metadata: Arc::clone(&self.metadata),
            event_listeners: Arc::clone(&self.event_listeners),
        }
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::shortcuts::KeyCombination;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_action_registration() {
        let mut registry = ActionRegistry::new();

        let metadata = ActionMetadata {
            name: "test_action".to_string(),
            description: "Test action".to_string(),

            requires_terminal: false,
            is_system_action: false,
            supported_platforms: vec!["test".to_string()],
        };

        let result = registry
            .register_action(metadata, |_| {
                Ok(serde_json::Value::String("test".to_string()))
            })
            .await;

        assert!(result.is_ok());
        assert!(registry.is_action_registered("test_action").await);
    }

    #[tokio::test]
    async fn test_action_execution() {
        let mut registry = ActionRegistry::new();

        let metadata = ActionMetadata {
            name: "test_action".to_string(),
            description: "Test action".to_string(),

            requires_terminal: false,
            is_system_action: false,
            supported_platforms: vec!["test".to_string()],
        };

        registry
            .register_action(metadata, |_| {
                Ok(serde_json::Value::String("executed".to_string()))
            })
            .await
            .unwrap();

        let context = ActionContext {
            key_combination: KeyCombination::new("t".to_string(), vec!["cmd".to_string()]),
            active_terminal_id: None,
            metadata: HashMap::new(),
        };

        let action = ShortcutAction::Simple("test_action".to_string());
        let result = registry.execute_action(&action, &context).await;

        assert!(result.success);
        assert_eq!(
            result.data,
            Some(serde_json::Value::String("executed".to_string()))
        );
    }
}
