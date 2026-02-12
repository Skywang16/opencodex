//! High-performance ring output buffer

/// Ring output buffer
///
/// Fixed size to avoid unlimited memory growth
pub struct OutputRingBuffer {
    /// Buffer data
    buffer: Vec<u8>,
    /// Maximum capacity
    capacity: usize,
    /// Whether overflowed (data was overwritten)
    overflowed: bool,
    /// Total bytes written
    total_written: u64,
}

impl OutputRingBuffer {
    /// Create a new ring buffer
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            overflowed: false,
            total_written: 0,
        }
    }

    /// Write data
    pub fn write(&mut self, data: &[u8]) {
        self.total_written += data.len() as u64;

        if data.len() >= self.capacity {
            // Data is larger than buffer, keep only the last capacity bytes
            self.buffer.clear();
            self.buffer
                .extend_from_slice(&data[data.len() - self.capacity..]);
            self.overflowed = true;
        } else if self.buffer.len() + data.len() > self.capacity {
            // Need to discard old data
            let overflow = self.buffer.len() + data.len() - self.capacity;
            self.buffer.drain(0..overflow);
            self.buffer.extend_from_slice(data);
            self.overflowed = true;
        } else {
            // Direct append
            self.buffer.extend_from_slice(data);
        }
    }

    /// Write string
    pub fn write_str(&mut self, s: &str) {
        self.write(s.as_bytes());
    }

    /// Get current content
    pub fn content(&self) -> &[u8] {
        &self.buffer
    }

    /// Get current content as string
    pub fn content_string(&self) -> String {
        String::from_utf8_lossy(&self.buffer).to_string()
    }

    /// Whether overflowed
    pub fn is_overflowed(&self) -> bool {
        self.overflowed
    }

    /// Total bytes written
    pub fn total_written(&self) -> u64 {
        self.total_written
    }

    /// Current buffer size
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Whether buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Clear buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.overflowed = false;
        self.total_written = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_write() {
        let mut buf = OutputRingBuffer::new(100);
        buf.write_str("hello");
        assert_eq!(buf.content_string(), "hello");
        assert!(!buf.is_overflowed());
    }

    #[test]
    fn test_overflow() {
        let mut buf = OutputRingBuffer::new(10);
        buf.write_str("hello world!"); // 12 bytes > 10
        assert_eq!(buf.len(), 10);
        assert!(buf.is_overflowed());
        assert_eq!(buf.content_string(), "llo world!"); // Keep last 10 bytes
    }

    #[test]
    fn test_incremental_overflow() {
        let mut buf = OutputRingBuffer::new(10);
        buf.write_str("hello"); // 5 bytes
        buf.write_str("world!"); // 6 bytes, total 11 > 10
        assert!(buf.is_overflowed());
        assert_eq!(buf.len(), 10);
        assert_eq!(buf.content_string(), "elloworld!"); // Discard first byte 'h'
        assert_eq!(buf.total_written(), 11);
    }

    #[test]
    fn test_clear() {
        let mut buf = OutputRingBuffer::new(10);
        buf.write_str("hello world!"); // Trigger overflow
        assert!(buf.is_overflowed());

        buf.clear();
        assert!(!buf.is_overflowed());
        assert!(buf.is_empty());
        assert_eq!(buf.total_written(), 0);
    }

    #[test]
    fn test_exact_capacity() {
        let mut buf = OutputRingBuffer::new(5);
        buf.write_str("hello"); // Exactly 5 bytes
        assert_eq!(buf.len(), 5);
        // Note: when single write >= capacity, implementation marks as overflow
        // This is because we cannot distinguish if old data was overwritten
        assert!(buf.is_overflowed());
        assert_eq!(buf.content_string(), "hello");
    }

    #[test]
    fn test_under_capacity() {
        let mut buf = OutputRingBuffer::new(10);
        buf.write_str("hello"); // 5 bytes < 10
        assert_eq!(buf.len(), 5);
        assert!(!buf.is_overflowed());
        assert_eq!(buf.content_string(), "hello");
    }
}
