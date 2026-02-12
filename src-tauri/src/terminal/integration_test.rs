/*!
 * Terminal event system integration tests
 *
 * Tests integration between unified event handler and terminal context system
 */

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::timeout;

    use crate::mux::PaneId;
    use crate::terminal::{ActiveTerminalContextRegistry, TerminalContextEvent};

    #[tokio::test]
    async fn test_event_integration_flow() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());

        // Subscribe to events
        let mut event_receiver = registry.subscribe_events();

        let pane_id = PaneId::new(1);
        registry.terminal_context_set_active_pane(pane_id).unwrap();

        // Verify event was sent
        let event = timeout(Duration::from_millis(100), event_receiver.recv())
            .await
            .expect("Should receive event")
            .expect("Event receive should not fail");

        match event {
            TerminalContextEvent::ActivePaneChanged {
                old_pane_id,
                new_pane_id,
            } => {
                assert_eq!(old_pane_id, None);
                assert_eq!(new_pane_id, Some(pane_id));
            }
            _ => panic!("Should receive ActivePaneChanged event"),
        }
    }

    #[tokio::test]
    async fn test_context_service_event_integration() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());

        // Subscribe to events
        let mut event_receiver = registry.subscribe_events();

        let pane_id = PaneId::new(1);
        registry.terminal_context_set_active_pane(pane_id).unwrap();

        // Verify active pane change event
        let event = timeout(Duration::from_millis(100), event_receiver.recv())
            .await
            .expect("Should receive event")
            .expect("Event receive should not fail");

        assert!(matches!(
            event,
            TerminalContextEvent::ActivePaneChanged { .. }
        ));

        // Verify context service can get active pane
        assert_eq!(registry.terminal_context_get_active_pane(), Some(pane_id));
    }

    #[tokio::test]
    async fn test_multiple_event_subscribers() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());

        let mut receiver1 = registry.subscribe_events();
        let mut receiver2 = registry.subscribe_events();

        let pane_id = PaneId::new(1);
        registry.terminal_context_set_active_pane(pane_id).unwrap();

        // Verify all subscribers receive events
        let event1 = timeout(Duration::from_millis(100), receiver1.recv())
            .await
            .expect("Subscriber 1 should receive event")
            .expect("Event receive should not fail");

        let event2 = timeout(Duration::from_millis(100), receiver2.recv())
            .await
            .expect("Subscriber 2 should receive event")
            .expect("Event receive should not fail");

        // Verify event contents are the same
        assert!(matches!(
            event1,
            TerminalContextEvent::ActivePaneChanged { .. }
        ));
        assert!(matches!(
            event2,
            TerminalContextEvent::ActivePaneChanged { .. }
        ));
    }

    #[tokio::test]
    async fn test_event_handler_conversion_functions() {
        use crate::mux::MuxNotification;
        use crate::terminal::event_handler::TerminalEventHandler;

        // Test Mux notification conversion
        let pane_id = PaneId::new(1);
        let notification = MuxNotification::PaneAdded(pane_id);

        let (event_name, payload) =
            TerminalEventHandler::mux_notification_to_tauri_event(&notification);

        assert_eq!(event_name, "terminal_created");
        assert_eq!(payload["paneId"], 1);

        // Test context event conversion
        let context_event = TerminalContextEvent::ActivePaneChanged {
            old_pane_id: None,
            new_pane_id: Some(pane_id),
        };

        let (event_name, payload) =
            TerminalEventHandler::context_event_to_tauri_event(&context_event);

        assert_eq!(event_name, "active_pane_changed");
        assert_eq!(payload["oldPaneId"], serde_json::Value::Null);
        assert_eq!(payload["newPaneId"], 1);
    }

    #[tokio::test]
    async fn test_event_deduplication() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());

        // Subscribe to events
        let mut event_receiver = registry.subscribe_events();

        let pane_id = PaneId::new(1);

        registry.terminal_context_set_active_pane(pane_id).unwrap();
        registry.terminal_context_set_active_pane(pane_id).unwrap();

        // Should only receive one event (first setting)
        let event = timeout(Duration::from_millis(100), event_receiver.recv())
            .await
            .expect("Should receive first event")
            .expect("Event receive should not fail");

        assert!(matches!(
            event,
            TerminalContextEvent::ActivePaneChanged { .. }
        ));

        // Second event should timeout (no change)
        let result = timeout(Duration::from_millis(50), event_receiver.recv()).await;
        assert!(result.is_err(), "Should not receive duplicate event");
    }
}
