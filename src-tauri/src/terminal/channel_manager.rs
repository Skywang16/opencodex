use std::collections::HashMap;
use std::sync::RwLock;
use tauri::ipc::Channel;
use tracing::warn;

use super::types::TerminalChannelMessage;
use crate::terminal::TerminalScrollback;

#[derive(Default)]
pub struct TerminalChannelManager {
    channels: RwLock<HashMap<u32, Channel<TerminalChannelMessage>>>,
}

impl TerminalChannelManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, pane_id: u32, channel: Channel<TerminalChannelMessage>) {
        match self.channels.write() {
            Ok(mut map) => {
                map.insert(pane_id, channel);
            }
            Err(err) => {
                warn!("failed to acquire terminal channel write lock: {}", err);
                return;
            }
        }

        // Always replay scrollback on subscribe. The UI terminal is frequently mounted/unmounted
        // when switching tabs, so it needs a durable source of history.
        let replay = TerminalScrollback::global().get_bytes(pane_id);
        if replay.is_empty() {
            return;
        }

        match self.channels.read() {
            Ok(map) => {
                if let Some(ch) = map.get(&pane_id) {
                    if let Err(err) = ch.send(TerminalChannelMessage::Data {
                        pane_id,
                        data: replay,
                    }) {
                        warn!(
                            "failed to replay terminal scrollback for pane {}: {}",
                            pane_id, err
                        );
                    }
                }
            }
            Err(err) => {
                warn!("failed to acquire terminal channel read lock: {}", err);
            }
        }
    }

    pub fn remove(&self, pane_id: u32) {
        match self.channels.write() {
            Ok(mut map) => {
                map.remove(&pane_id);
            }
            Err(err) => {
                warn!("failed to acquire terminal channel write lock: {}", err);
            }
        }
    }

    pub fn send_data(&self, pane_id: u32, data: &[u8]) {
        let mut should_remove = false;

        match self.channels.read() {
            Ok(map) => {
                if let Some(ch) = map.get(&pane_id) {
                    let payload = TerminalChannelMessage::Data {
                        pane_id,
                        data: data.to_vec(),
                    };
                    if ch.send(payload).is_err() {
                        should_remove = true;
                    }
                }
            }
            Err(err) => {
                warn!("failed to acquire terminal channel read lock: {}", err);
            }
        }

        if should_remove {
            match self.channels.write() {
                Ok(mut map) => {
                    map.remove(&pane_id);
                }
                Err(err) => {
                    warn!("failed to acquire terminal channel write lock: {}", err);
                }
            }
        }
    }

    pub fn close(&self, pane_id: u32) {
        match self.channels.read() {
            Ok(map) => {
                if let Some(ch) = map.get(&pane_id) {
                    if let Err(err) = ch.send(TerminalChannelMessage::Close { pane_id }) {
                        warn!(
                            "failed to send terminal close event for pane {}: {}",
                            pane_id, err
                        );
                    }
                }
            }
            Err(err) => {
                warn!("failed to acquire terminal channel read lock: {}", err);
            }
        }
        self.remove(pane_id);
    }
}
