//! Terminal event handler

use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::sync::broadcast;
use tracing::{error, warn};

use crate::completion::output_analyzer::OutputAnalyzer;
use crate::events::{ShellEvent, TerminalContextEvent};
use crate::mux::{MuxNotification, PaneId, SubscriberCallback, TerminalMux};
use crate::terminal::error::EventHandlerResult;
use crate::terminal::TerminalScrollback;

/// Unified terminal event handler
///
/// Responsible for integrating terminal events from different sources and sending them uniformly to frontend
///
/// Subscribes to three layers of events:
/// 1. Mux layer - process lifecycle events (crossbeam channel)
/// 2. Shell layer - OSC parsing events (broadcast channel)
/// 3. Context layer - context change events (broadcast channel)
pub struct TerminalEventHandler {
    mux: Arc<TerminalMux>,
    mux_subscriber_id: usize,
    /// Context event handling task handle (aborted on Drop)
    context_task_handle: std::sync::Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    /// Shell event handling task handle (aborted on Drop)
    shell_task_handle: std::sync::Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

impl TerminalEventHandler {
    /// Start event handler (runs immediately upon creation)
    pub fn start<R: Runtime>(
        app_handle: AppHandle<R>,
        mux: Arc<TerminalMux>,
        context_event_receiver: broadcast::Receiver<TerminalContextEvent>,
        shell_event_receiver: broadcast::Receiver<(crate::mux::PaneId, crate::shell::ShellEvent)>,
    ) -> EventHandlerResult<Self> {
        // Subscribe to TerminalMux events (PaneOutput uses buffered throttling, other events sent immediately)
        let app_handle_for_mux = app_handle.clone();
        let mux_subscriber: SubscriberCallback = Box::new(move |notification| match notification {
            MuxNotification::PaneOutput { pane_id, data } => {
                TerminalScrollback::global().append(pane_id.as_u32(), data.as_ref());

                let state = app_handle_for_mux
                    .state::<crate::terminal::channel_state::TerminalChannelState>();
                state.manager.send_data(pane_id.as_u32(), data.as_ref());

                // Synchronously feed to OutputAnalyzer for history cache
                let text = String::from_utf8_lossy(data);
                if let Err(e) = OutputAnalyzer::global().analyze_output(pane_id.as_u32(), &text) {
                    warn!(
                        "OutputAnalyzer analyze_output failed: pane_id={}, err={}",
                        pane_id.as_u32(),
                        e
                    );
                }
                true
            }
            MuxNotification::PaneRemoved(pane_id) => {
                TerminalScrollback::global().remove(pane_id.as_u32());

                // Notify Channel is closed
                let state = app_handle_for_mux
                    .state::<crate::terminal::channel_state::TerminalChannelState>();
                state.manager.close(pane_id.as_u32());
                let (event_name, payload) = Self::mux_notification_to_tauri_event(notification);
                if let Err(e) = app_handle_for_mux.emit(event_name, payload.clone()) {
                    error!(
                        "Failed to send Mux event: {}, error: {}, payload: {}",
                        event_name, e, payload
                    );
                }
                true
            }
            _ => {
                let (event_name, payload) = Self::mux_notification_to_tauri_event(notification);
                if let Err(e) = app_handle_for_mux.emit(event_name, payload.clone()) {
                    error!(
                        "Failed to send Mux event: {}, error: {}, payload: {}",
                        event_name, e, payload
                    );
                }
                true
            }
        });
        let subscriber_id = mux.subscribe(mux_subscriber);
        let app_handle_for_context = app_handle.clone();
        let mut context_receiver = context_event_receiver;
        let context_task_handle = tauri::async_runtime::spawn(async move {
            loop {
                match context_receiver.recv().await {
                    Ok(event) => {
                        Self::handle_context_event(&app_handle_for_context, event);
                    }
                    Err(e) => {
                        if matches!(e, broadcast::error::RecvError::Closed) {
                            break;
                        }
                        warn!("Context event receive lag: {:?}", e);
                    }
                }
            }
        });

        let app_handle_for_shell = app_handle.clone();
        let mut shell_receiver = shell_event_receiver;
        let shell_task_handle = tauri::async_runtime::spawn(async move {
            loop {
                match shell_receiver.recv().await {
                    Ok((pane_id, event)) => {
                        Self::handle_shell_event(&app_handle_for_shell, pane_id, event);
                    }
                    Err(e) => {
                        if matches!(e, broadcast::error::RecvError::Closed) {
                            break;
                        }
                        warn!("Shell event receive lag: {:?}", e);
                    }
                }
            }
        });

        Ok(Self {
            mux,
            mux_subscriber_id: subscriber_id,
            context_task_handle: std::sync::Mutex::new(Some(context_task_handle)),
            shell_task_handle: std::sync::Mutex::new(Some(shell_task_handle)),
        })
    }

    /// Handle Shell events
    fn handle_shell_event<R: Runtime>(
        app_handle: &AppHandle<R>,
        pane_id: PaneId,
        event: ShellEvent,
    ) {
        // Feed command events from Shell Integration to completion context system:
        // This is the fundamental data source for "predict next command" hit rate.
        if let ShellEvent::CommandEvent { command } = &event {
            if let Err(e) =
                OutputAnalyzer::global().on_shell_command_event(pane_id.as_u32(), command)
            {
                warn!(
                    "OutputAnalyzer on_shell_command_event failed: pane_id={}, err={}",
                    pane_id.as_u32(),
                    e
                );
            }

            // Also feed to offline learning model (SQLite) for "next command" prediction and ranking.
            if command.is_finished() {
                use crate::completion::learning::{CommandFinishedEvent, CompletionLearningState};
                use std::time::{SystemTime, UNIX_EPOCH};

                if let Some(command_line) = command.command_line.clone() {
                    let finished_ts = command
                        .end_time_wallclock
                        .unwrap_or_else(SystemTime::now)
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();

                    let learning = app_handle.state::<CompletionLearningState>();
                    learning.record_finished(CommandFinishedEvent {
                        pane_id: pane_id.as_u32(),
                        command_line,
                        cwd: command.working_directory.clone(),
                        exit_code: command.exit_code,
                        finished_ts,
                    });
                }
            }
        }

        let (event_name, payload) = Self::shell_event_to_tauri_event(pane_id, &event);

        if let Err(e) = app_handle.emit(event_name, payload) {
            error!("Failed to send Shell event: {}, error: {}", event_name, e);
        }
    }

    /// Handle terminal context events
    fn handle_context_event<R: Runtime>(app_handle: &AppHandle<R>, event: TerminalContextEvent) {
        // Avoid duplication with Mux events: no longer forward context-level pane_cwd_changed to frontend
        if let TerminalContextEvent::PaneCwdChanged { .. } = &event {
            return;
        }
        let (event_name, payload) = Self::context_event_to_tauri_event(&event);

        if let Err(e) = app_handle.emit(event_name, payload) {
            error!("Failed to send context event: {}, error: {}", event_name, e);
        }
    }

    /// Convert MuxNotification to Tauri event
    pub fn mux_notification_to_tauri_event(
        notification: &MuxNotification,
    ) -> (&'static str, serde_json::Value) {
        match notification {
            MuxNotification::PaneOutput { pane_id, data } => (
                "terminal_output",
                json!({
                    "paneId": pane_id.as_u32(),
                    "data": String::from_utf8_lossy(data)
                }),
            ),
            MuxNotification::PaneAdded(pane_id) => (
                "terminal_created",
                json!({
                    "paneId": pane_id.as_u32()
                }),
            ),
            MuxNotification::PaneRemoved(pane_id) => (
                "terminal_closed",
                json!({
                    "paneId": pane_id.as_u32()
                }),
            ),
            MuxNotification::PaneResized { pane_id, size } => (
                "terminal_resized",
                json!({
                    "paneId": pane_id.as_u32(),
                    "rows": size.rows,
                    "cols": size.cols
                }),
            ),
            MuxNotification::PaneExited { pane_id, exit_code } => (
                "terminal_exit",
                json!({
                    "paneId": pane_id.as_u32(),
                    "exitCode": exit_code
                }),
            ),
        }
    }

    /// Convert ShellEvent to Tauri event
    pub fn shell_event_to_tauri_event(
        pane_id: PaneId,
        event: &ShellEvent,
    ) -> (&'static str, serde_json::Value) {
        match event {
            ShellEvent::CwdChanged { new_cwd } => (
                "pane_cwd_changed",
                json!({
                    "paneId": pane_id.as_u32(),
                    "cwd": new_cwd
                }),
            ),
            ShellEvent::NodeVersionChanged { version } => (
                "node_version_changed",
                json!({
                    "paneId": pane_id.as_u32(),
                    "version": version
                }),
            ),
            ShellEvent::TitleChanged { new_title } => (
                "pane_title_changed",
                json!({
                    "paneId": pane_id.as_u32(),
                    "title": new_title
                }),
            ),
            ShellEvent::CommandEvent { command } => (
                "pane_command_event",
                json!({
                    "paneId": pane_id.as_u32(),
                    "command": command
                }),
            ),
        }
    }

    /// Convert TerminalContextEvent to Tauri event
    pub fn context_event_to_tauri_event(
        event: &TerminalContextEvent,
    ) -> (&'static str, serde_json::Value) {
        match event {
            TerminalContextEvent::ActivePaneChanged {
                old_pane_id,
                new_pane_id,
            } => (
                "active_pane_changed",
                json!({
                    "oldPaneId": old_pane_id.map(|id| id.as_u32()),
                    "newPaneId": new_pane_id.map(|id| id.as_u32())
                }),
            ),
            TerminalContextEvent::PaneContextUpdated { pane_id, context } => (
                "pane_context_updated",
                json!({
                    "paneId": pane_id.as_u32(),
                    "context": context
                }),
            ),
            TerminalContextEvent::PaneShellIntegrationChanged { pane_id, enabled } => (
                "pane_shell_integration_changed",
                json!({
                    "paneId": pane_id.as_u32(),
                    "enabled": enabled
                }),
            ),
            // Note: PaneCwdChanged events should not be sent from Context layer to frontend, Mux is the single source
            TerminalContextEvent::PaneCwdChanged { .. } => unreachable!(
                "PaneCwdChanged should never be serialized from context_event_to_tauri_event; Mux is the single source"
            ),
        }
    }
}

impl Drop for TerminalEventHandler {
    fn drop(&mut self) {
        if !self.mux.unsubscribe(self.mux_subscriber_id) {
            warn!(
                "Unable to unsubscribe Mux subscriber {}",
                self.mux_subscriber_id
            );
        }

        if let Ok(mut handle) = self.context_task_handle.lock() {
            if let Some(handle) = handle.take() {
                handle.abort();
            }
        }

        if let Ok(mut handle) = self.shell_task_handle.lock() {
            if let Some(handle) = handle.take() {
                handle.abort();
            }
        }
    }
}

/// Convenience function: create and start terminal event handler
pub fn create_terminal_event_handler<R: Runtime>(
    app_handle: AppHandle<R>,
    mux: Arc<TerminalMux>,
    context_event_receiver: broadcast::Receiver<TerminalContextEvent>,
    shell_event_receiver: broadcast::Receiver<(PaneId, ShellEvent)>,
) -> EventHandlerResult<TerminalEventHandler> {
    TerminalEventHandler::start(
        app_handle,
        mux,
        context_event_receiver,
        shell_event_receiver,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mux::PaneId;

    #[test]
    fn test_mux_notification_to_tauri_event() {
        let pane_id = PaneId::new(1);
        let notification = MuxNotification::PaneAdded(pane_id);

        let (event_name, payload) =
            TerminalEventHandler::mux_notification_to_tauri_event(&notification);

        assert_eq!(event_name, "terminal_created");
        assert_eq!(payload["paneId"], 1);
    }

    #[test]
    fn test_context_event_to_tauri_event() {
        let pane_id = PaneId::new(1);
        let event = TerminalContextEvent::ActivePaneChanged {
            old_pane_id: None,
            new_pane_id: Some(pane_id),
        };

        let (event_name, payload) = TerminalEventHandler::context_event_to_tauri_event(&event);

        assert_eq!(event_name, "active_pane_changed");
        assert_eq!(payload["oldPaneId"], serde_json::Value::Null);
        assert_eq!(payload["newPaneId"], 1);
    }

    #[test]
    fn test_cwd_changed_event_conversion() {
        let pane_id = PaneId::new(1);
        let event = TerminalContextEvent::PaneCwdChanged {
            pane_id,
            old_cwd: Some("/old/path".to_string()),
            new_cwd: "/new/path".to_string(),
        };
        // No longer allow serializing PaneCwdChanged events from Context layer, should be unreachable
        let result = std::panic::catch_unwind(|| {
            let _ = TerminalEventHandler::context_event_to_tauri_event(&event);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_shell_integration_changed_event_conversion() {
        let pane_id = PaneId::new(1);
        let event = TerminalContextEvent::PaneShellIntegrationChanged {
            pane_id,
            enabled: true,
        };

        let (event_name, payload) = TerminalEventHandler::context_event_to_tauri_event(&event);

        assert_eq!(event_name, "pane_shell_integration_changed");
        assert_eq!(payload["paneId"], 1);
        assert_eq!(payload["enabled"], true);
    }

    // Note: Event handler status test requires a real Tauri app context
    // which is not easily mockable in unit tests. Integration tests would be more appropriate.
}
