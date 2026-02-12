/*!
 * Terminal context management Tauri command interface
 *
 * Provides terminal context management commands for frontend calls, including:
 * - Active terminal management
 * - Terminal context queries
 * - Parameter validation and error handling
 */

use crate::terminal::{ActiveTerminalContextRegistry, TerminalContextService};
use std::sync::Arc;

/// Terminal context management state
///
/// Contains shared state for active terminal registry and terminal context service
pub struct TerminalContextState {
    /// Active terminal context registry
    pub registry: Arc<ActiveTerminalContextRegistry>,
    /// Terminal context service
    pub context_service: Arc<TerminalContextService>,
}

impl TerminalContextState {
    /// Create new terminal context state
    ///
    /// # Arguments
    /// * `registry` - Active terminal context registry
    /// * `context_service` - Terminal context service
    ///
    /// # Returns
    /// * `TerminalContextState` - New state instance
    pub fn new(
        registry: Arc<ActiveTerminalContextRegistry>,
        context_service: Arc<TerminalContextService>,
    ) -> Self {
        Self {
            registry,
            context_service,
        }
    }

    /// Get reference to active terminal registry
    pub fn registry(&self) -> &Arc<ActiveTerminalContextRegistry> {
        &self.registry
    }

    /// Get reference to terminal context service
    pub fn context_service(&self) -> &Arc<TerminalContextService> {
        &self.context_service
    }
}

pub mod context;
pub mod pane;
pub mod stream;

pub use context::{terminal_context_get, terminal_context_get_active};
pub use pane::{terminal_context_get_active_pane, terminal_context_set_active_pane};
pub use stream::{terminal_subscribe_output, terminal_subscribe_output_cancel};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mux::TerminalMux;
    use crate::shell::ShellIntegrationManager;
    use crate::storage::cache::UnifiedCache;
    use std::sync::Arc;

    /// Create test terminal context state
    pub(crate) fn create_test_state() -> TerminalContextState {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new());
        let terminal_mux = TerminalMux::new_shared();
        let cache = Arc::new(UnifiedCache::new());
        let context_service = Arc::new(TerminalContextService::new(
            registry.clone(),
            shell_integration,
            terminal_mux,
            cache,
        ));

        TerminalContextState::new(registry, context_service)
    }

    #[tokio::test]
    async fn test_state_creation_and_access() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new());
        let terminal_mux = TerminalMux::new_shared();
        let cache = Arc::new(UnifiedCache::new());
        let context_service = Arc::new(TerminalContextService::new(
            registry.clone(),
            shell_integration,
            terminal_mux,
            cache,
        ));

        let state = TerminalContextState::new(registry.clone(), context_service.clone());

        // Verify state access methods
        assert!(Arc::ptr_eq(state.registry(), &registry));
        assert!(Arc::ptr_eq(state.context_service(), &context_service));
    }
}
