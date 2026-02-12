use crate::{
    mux::{
        error::{IoHandlerError, IoHandlerResult},
        MuxNotification, Pane, PaneId,
    },
    shell::ShellIntegrationManager,
};
use bytes::Bytes;
use crossbeam_channel::Sender;
use std::{
    collections::HashMap,
    io::{self, Read},
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};
use tracing::{error, warn};

pub struct IoHandler {
    buffer_size: usize,
    notification_sender: Sender<MuxNotification>,
    shell_integration: Arc<ShellIntegrationManager>,
    /// Store read thread handles for each pane
    reader_threads: Arc<RwLock<HashMap<PaneId, thread::JoinHandle<()>>>>,
}

impl IoHandler {
    pub fn new(
        notification_sender: Sender<MuxNotification>,
        shell_integration: Arc<ShellIntegrationManager>,
    ) -> Self {
        Self {
            buffer_size: 8192,
            notification_sender,
            shell_integration,
            reader_threads: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_buffer_size(
        notification_sender: Sender<MuxNotification>,
        shell_integration: Arc<ShellIntegrationManager>,
        buffer_size: usize,
    ) -> Self {
        Self {
            buffer_size,
            notification_sender,
            shell_integration,
            reader_threads: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    pub fn spawn_io_threads(&self, pane: Arc<dyn Pane>) -> IoHandlerResult<()> {
        let pane_id = pane.pane_id();
        let reader = pane.reader().map_err(|err| IoHandlerError::PaneReader {
            reason: format!("Failed to acquire reader for {pane_id:?}: {err}"),
        })?;

        let handle = self.spawn_reader_thread(pane_id, reader, pane);

        // Store thread handle
        if let Ok(mut threads) = self.reader_threads.write() {
            threads.insert(pane_id, handle);
        } else {
            warn!("Unable to store thread handle for pane {:?}", pane_id);
        }

        Ok(())
    }

    pub fn stop_pane_io(&self, pane_id: PaneId) -> IoHandlerResult<()> {
        if let Ok(mut threads) = self.reader_threads.write() {
            if let Some(handle) = threads.remove(&pane_id) {
                // Use thread::spawn to join in background, avoid blocking
                thread::spawn(move || {
                    if let Err(e) = handle.join() {
                        warn!(
                            "Error occurred when I/O thread for pane {:?} ended: {:?}",
                            pane_id, e
                        );
                    }
                });
            }
        }
        Ok(())
    }

    pub fn shutdown(&self) -> IoHandlerResult<()> {
        if let Ok(mut threads) = self.reader_threads.write() {
            if threads.is_empty() {
                return Ok(());
            }

            // Use background thread to batch join, set timeout and record results
            let handles: Vec<_> = threads.drain().collect();
            let (tx, rx) = std::sync::mpsc::channel();

            thread::spawn(move || {
                let start = std::time::Instant::now();
                let mut joined = 0;
                let mut panicked = 0;

                for (pane_id, handle) in handles {
                    if start.elapsed() > Duration::from_secs(2) {
                        warn!(
                            "I/O thread shutdown timeout, giving up waiting for remaining threads"
                        );
                        break;
                    }

                    match handle.join() {
                        Ok(_) => joined += 1,
                        Err(e) => {
                            warn!("I/O thread for pane {:?} panicked: {:?}", pane_id, e);
                            panicked += 1;
                        }
                    }
                }

                // Send result statistics
                let _ = tx.send((joined, panicked));
            });

            // Record results non-blockingly
            thread::spawn(move || {
                if rx.recv_timeout(Duration::from_secs(3)).is_err() {
                    warn!("Timeout waiting for I/O thread shutdown result");
                }
            });
        }

        Ok(())
    }

    fn spawn_reader_thread(
        &self,
        pane_id: PaneId,
        mut reader: Box<dyn Read + Send>,
        pane: Arc<dyn Pane>,
    ) -> thread::JoinHandle<()> {
        let mut buffer = vec![0u8; self.buffer_size];
        let sender = self.notification_sender.clone();
        let integration = self.shell_integration.clone();

        thread::spawn(move || {
            let mut pending = Vec::new();

            loop {
                // Check if pane is dead
                if pane.is_dead() {
                    break;
                }

                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(len) => {
                        for chunk in decode_utf8_stream(&mut pending, &buffer[..len]) {
                            // Shell events are now sent via broadcast channel, no longer returned
                            integration.process_output(pane_id, &chunk);

                            let cleaned = integration.strip_osc_sequences(&chunk);

                            if cleaned.is_empty() {
                                continue;
                            }

                            let notification = MuxNotification::PaneOutput {
                                pane_id,
                                data: Bytes::from(cleaned.into_bytes()),
                            };

                            if sender.send(notification).is_err() {
                                return;
                            }
                        }
                    }
                    Err(err) => {
                        if err.kind() == io::ErrorKind::Interrupted {
                            continue;
                        }
                        warn!("Read thread error for pane {:?}: {}", pane_id, err);
                        break;
                    }
                }
            }

            for chunk in decode_utf8_stream(&mut pending, &[]) {
                integration.process_output(pane_id, &chunk);
                let cleaned = integration.strip_osc_sequences(&chunk);

                if cleaned.is_empty() {
                    continue;
                }

                let notification = MuxNotification::PaneOutput {
                    pane_id,
                    data: Bytes::from(cleaned.into_bytes()),
                };

                if sender.send(notification).is_err() {
                    return;
                }
            }

            let exit_notification = MuxNotification::PaneExited {
                pane_id,
                exit_code: None,
            };

            if let Err(err) = sender.send(exit_notification) {
                error!(
                    "Failed to send exit notification for pane {:?} (may be closed): {}",
                    pane_id, err
                );
            }
        })
    }
}

/// Optimized UTF-8 stream decoding function
///
/// Use more efficient method to handle byte stream to string conversion:
/// - Reduce Vec operations, use split_off instead of drain
/// - Pre-allocate string capacity
/// - Reduce intermediate allocations
fn decode_utf8_stream(pending: &mut Vec<u8>, input: &[u8]) -> Vec<String> {
    if input.is_empty() && pending.is_empty() {
        return Vec::new();
    }

    pending.extend_from_slice(input);

    // Pre-allocate result vector (usually only 1-2 fragments)
    let mut frames = Vec::with_capacity(2);

    loop {
        if pending.is_empty() {
            break;
        }

        match std::str::from_utf8(pending) {
            Ok(valid) => {
                // Entire buffer is valid UTF-8
                if !valid.is_empty() {
                    frames.push(valid.to_string());
                }
                pending.clear();
                break;
            }
            Err(err) => {
                let valid_up_to = err.valid_up_to();

                if valid_up_to > 0 {
                    // Has partially valid UTF-8 data
                    let valid = unsafe { std::str::from_utf8_unchecked(&pending[..valid_up_to]) };
                    if !valid.is_empty() {
                        frames.push(valid.to_string());
                    }

                    // Use split_off instead of drain, more efficient
                    *pending = pending.split_off(valid_up_to);
                    continue;
                }

                // Handle invalid bytes
                if let Some(error_len) = err.error_len() {
                    // Skip invalid bytes
                    let drop_len = error_len.max(1).min(pending.len());
                    *pending = pending.split_off(drop_len);
                    continue;
                }

                // Incomplete UTF-8 sequence, keep in buffer waiting for more data
                break;
            }
        }
    }

    frames
}
