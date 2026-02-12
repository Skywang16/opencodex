/*!
 * Active terminal context registry implementation
 *
 * Provides thread-safe active terminal state management and event sending mechanism
 */

use crate::events::TerminalContextEvent;
use crate::mux::PaneId;
use crate::terminal::error::{ContextRegistryError, ContextRegistryResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tracing::warn;

/// Window ID type (reserved for future multi-window support)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WindowId(pub u32);

impl WindowId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl From<u32> for WindowId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

/// Active terminal context registry
///
/// Responsible for maintaining the state of current active terminals, providing thread-safe query and update operations
#[derive(Debug)]
pub struct ActiveTerminalContextRegistry {
    /// Global active terminal
    global_active_pane: Arc<RwLock<Option<PaneId>>>,
    /// Active terminals grouped by window (future extension)
    window_active_panes: Arc<RwLock<HashMap<WindowId, PaneId>>>,
    /// Event broadcast sender
    event_sender: broadcast::Sender<TerminalContextEvent>,
}

impl ActiveTerminalContextRegistry {
    /// Create new active terminal context registry
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(1000);

        Self {
            global_active_pane: Arc::new(RwLock::new(None)),
            window_active_panes: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
        }
    }

    /// Set global active terminal
    ///
    /// # Arguments
    /// * `pane_id` - Pane ID to set as active
    ///
    /// # Returns
    /// * `Ok(())` - Set successful
    /// * `Err(ContextError)` - Set failed
    pub fn terminal_context_set_active_pane(&self, pane_id: PaneId) -> ContextRegistryResult<()> {
        let old_pane_id = {
            let mut active_pane = self.global_active_pane.write().map_err(|err| {
                ContextRegistryError::from_write_poison("global_active_pane", err)
            })?;

            let old_id = *active_pane;

            if old_id == Some(pane_id) {
                return Ok(());
            }

            *active_pane = Some(pane_id);
            old_id
        };

        // Send event
        let event = TerminalContextEvent::ActivePaneChanged {
            old_pane_id,
            new_pane_id: Some(pane_id),
        };

        if let Err(e) = self.event_sender.send(event) {
            warn!("Failed to send active terminal change event: {}", e);
        }

        Ok(())
    }

    /// Get current global active terminal
    ///
    /// # Returns
    /// * `Some(PaneId)` - Current active pane ID
    /// * `None` - No active terminal
    pub fn terminal_context_get_active_pane(&self) -> Option<PaneId> {
        match self.global_active_pane.read() {
            Ok(active_pane) => *active_pane,
            Err(e) => {
                warn!("Failed to acquire read lock for active terminal: {}", e);
                None
            }
        }
    }

    /// Clear global active terminal
    ///
    /// # Returns
    /// * `Ok(())` - Clear successful
    /// * `Err(ContextError)` - Clear failed
    pub fn terminal_context_clear_active_pane(&self) -> ContextRegistryResult<()> {
        let old_pane_id = {
            let mut active_pane = self.global_active_pane.write().map_err(|err| {
                ContextRegistryError::from_write_poison("global_active_pane", err)
            })?;

            let old_id = *active_pane;
            *active_pane = None;
            old_id
        };

        if old_pane_id.is_some() {
            // Send event
            let event = TerminalContextEvent::ActivePaneChanged {
                old_pane_id,
                new_pane_id: None,
            };

            if let Err(e) = self.event_sender.send(event) {
                warn!("Failed to send active terminal clear event: {}", e);
            }
        }

        Ok(())
    }

    /// Check if specified pane is active terminal
    ///
    /// # Arguments
    /// * `pane_id` - Pane ID to check
    ///
    /// # Returns
    /// * `true` - This pane is the active terminal
    /// * `false` - This pane is not the active terminal or failed to get state
    pub fn terminal_context_is_pane_active(&self, pane_id: PaneId) -> bool {
        match self.global_active_pane.read() {
            Ok(active_pane) => *active_pane == Some(pane_id),
            Err(e) => {
                warn!(
                    "Failed to acquire read lock when checking pane active status: {}",
                    e
                );
                false
            }
        }
    }

    /// Set active terminal for specified window (future extension feature)
    ///
    /// # Arguments
    /// * `window_id` - Window ID
    /// * `pane_id` - Pane ID to set as active
    ///
    /// # Returns
    /// * `Ok(())` - Set successful
    /// * `Err(ContextError)` - Set failed
    pub fn set_window_active_pane(
        &self,
        window_id: WindowId,
        pane_id: PaneId,
    ) -> ContextRegistryResult<()> {
        let _old_pane_id = {
            let mut window_panes = self.window_active_panes.write().map_err(|err| {
                ContextRegistryError::from_write_poison("window_active_panes", err)
            })?;

            window_panes.insert(window_id, pane_id)
        };

        if window_id.as_u32() == 0 || self.terminal_context_get_active_pane().is_none() {
            self.terminal_context_set_active_pane(pane_id)?;
        }

        Ok(())
    }

    /// Get active terminal for specified window (future extension feature)
    ///
    /// # Arguments
    /// * `window_id` - Window ID
    ///
    /// # Returns
    /// * `Some(PaneId)` - Current active pane ID for this window
    /// * `None` - This window has no active terminal
    pub fn get_window_active_pane(&self, window_id: WindowId) -> Option<PaneId> {
        match self.window_active_panes.read() {
            Ok(window_panes) => window_panes.get(&window_id).copied(),
            Err(e) => {
                warn!(
                    "Failed to acquire read lock for window active terminal: {}",
                    e
                );
                None
            }
        }
    }

    /// Remove active terminal record for specified window (future extension feature)
    ///
    /// # Arguments
    /// * `window_id` - Window ID
    ///
    /// # Returns
    /// * `Ok(Option<PaneId>)` - Remove successful, returns previous active pane ID
    /// * `Err(ContextError)` - Remove failed
    pub fn remove_window_active_pane(
        &self,
        window_id: WindowId,
    ) -> ContextRegistryResult<Option<PaneId>> {
        let removed_pane = {
            let mut window_panes = self.window_active_panes.write().map_err(|err| {
                ContextRegistryError::from_write_poison("window_active_panes", err)
            })?;

            window_panes.remove(&window_id)
        };

        if let Some(pane_id) = removed_pane {
            if self.terminal_context_get_active_pane() == Some(pane_id) {
                self.terminal_context_clear_active_pane()?;
            }
        }

        Ok(removed_pane)
    }

    /// Get event receiver
    ///
    /// Used to subscribe to terminal context events
    ///
    /// # Returns
    /// * `broadcast::Receiver<TerminalContextEvent>` - Event receiver
    pub fn subscribe_events(&self) -> broadcast::Receiver<TerminalContextEvent> {
        self.event_sender.subscribe()
    }

    /// Send custom event
    ///
    /// # Arguments
    /// * `event` - Event to send
    ///
    /// # Returns
    /// * `Ok(usize)` - Send successful, returns receiver count
    /// * `Err(ContextError)` - Send failed
    pub fn send_event(&self, event: TerminalContextEvent) -> ContextRegistryResult<usize> {
        self.event_sender
            .send(event)
            .map_err(|err| ContextRegistryError::EventSend(err.to_string()))
    }

    /// Get registry statistics
    ///
    /// # Returns
    /// * `RegistryStats` - Registry statistics
    pub fn get_stats(&self) -> RegistryStats {
        let global_active = self.terminal_context_get_active_pane();
        let window_count = self
            .window_active_panes
            .read()
            .map(|panes| panes.len())
            .unwrap_or(0);
        let subscriber_count = self.event_sender.receiver_count();

        RegistryStats {
            global_active_pane: global_active,
            window_active_pane_count: window_count,
            event_subscriber_count: subscriber_count,
        }
    }
}

impl Default for ActiveTerminalContextRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry statistics
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryStats {
    pub global_active_pane: Option<PaneId>,
    pub window_active_pane_count: usize,
    pub event_subscriber_count: usize,
}

// Implement thread-safe clone
impl Clone for ActiveTerminalContextRegistry {
    fn clone(&self) -> Self {
        Self {
            global_active_pane: Arc::clone(&self.global_active_pane),
            window_active_panes: Arc::clone(&self.window_active_panes),
            event_sender: self.event_sender.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_active_pane_management() {
        let registry = ActiveTerminalContextRegistry::new();
        let pane_id = PaneId::new(1);

        // Initial state should have no active terminal
        assert_eq!(registry.terminal_context_get_active_pane(), None);
        assert!(!registry.terminal_context_is_pane_active(pane_id));

        registry.terminal_context_set_active_pane(pane_id).unwrap();
        assert_eq!(registry.terminal_context_get_active_pane(), Some(pane_id));
        assert!(registry.terminal_context_is_pane_active(pane_id));

        // Clear active terminal
        registry.terminal_context_clear_active_pane().unwrap();
        assert_eq!(registry.terminal_context_get_active_pane(), None);
        assert!(!registry.terminal_context_is_pane_active(pane_id));
    }

    #[tokio::test]
    async fn test_window_active_pane_management() {
        let registry = ActiveTerminalContextRegistry::new();
        let window_id = WindowId::new(1);
        let pane_id = PaneId::new(1);

        // Initial state should have no window active terminal
        assert_eq!(registry.get_window_active_pane(window_id), None);

        registry.set_window_active_pane(window_id, pane_id).unwrap();
        assert_eq!(registry.get_window_active_pane(window_id), Some(pane_id));

        // Remove window active terminal
        let removed = registry.remove_window_active_pane(window_id).unwrap();
        assert_eq!(removed, Some(pane_id));
        assert_eq!(registry.get_window_active_pane(window_id), None);
    }

    #[tokio::test]
    async fn test_event_broadcasting() {
        let registry = ActiveTerminalContextRegistry::new();
        let mut receiver = registry.subscribe_events();
        let pane_id = PaneId::new(1);

        registry.terminal_context_set_active_pane(pane_id).unwrap();

        // Receive event
        let event = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("Should receive event before timeout")
            .expect("Should successfully receive event");

        match event {
            TerminalContextEvent::ActivePaneChanged {
                old_pane_id,
                new_pane_id,
            } => {
                assert_eq!(old_pane_id, None);
                assert_eq!(new_pane_id, Some(pane_id));
            }
            _ => panic!("Received wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let mut handles = Vec::new();

        // Start multiple concurrent tasks
        for i in 0..10 {
            let registry_clone = Arc::clone(&registry);
            let handle = tokio::spawn(async move {
                let pane_id = PaneId::new(i);

                registry_clone
                    .terminal_context_set_active_pane(pane_id)
                    .unwrap();

                let active = registry_clone.terminal_context_get_active_pane();
                let is_active = registry_clone.terminal_context_is_pane_active(pane_id);

                // Clear active terminal
                registry_clone.terminal_context_clear_active_pane().unwrap();

                (active, is_active)
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            let (_active, _was_active) = handle.await.unwrap();
            // Due to concurrent execution, we can only verify basic consistency
            // In concurrent environments, state may change rapidly, so we mainly verify operations don't crash
        }

        // Final state should have no active terminal
        assert_eq!(registry.terminal_context_get_active_pane(), None);
    }

    #[test]
    fn test_registry_stats() {
        let registry = ActiveTerminalContextRegistry::new();
        let pane_id = PaneId::new(1);
        let window_id = WindowId::new(1);

        // Initial statistics
        let stats = registry.get_stats();
        assert_eq!(stats.global_active_pane, None);
        assert_eq!(stats.window_active_pane_count, 0);

        registry.terminal_context_set_active_pane(pane_id).unwrap();
        registry.set_window_active_pane(window_id, pane_id).unwrap();

        let stats = registry.get_stats();
        assert_eq!(stats.global_active_pane, Some(pane_id));
        assert_eq!(stats.window_active_pane_count, 1);
    }
}
