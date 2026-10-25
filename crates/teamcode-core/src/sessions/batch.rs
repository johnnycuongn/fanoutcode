//! Output batching: coalesces PTY reads into frames of at most `MAX_BYTES`, flushed one `INTERVAL` (about 60 Hz)
//! after the first byte of a frame, so a burst costs one IPC message per frame, not one per read.

use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Batcher {
    interval: Duration,
    max_bytes: usize,
    buf: Vec<u8>,
    opened_at: Option<Instant>,
}

impl Batcher {
    /// One frame at about 60 Hz.
    pub const INTERVAL: Duration = Duration::from_millis(16);
    /// A frame is sent at once when it reaches this size.
    pub const MAX_BYTES: usize = 65_536;

    pub fn new(interval: Duration, max_bytes: usize) -> Self {
        Self {
            interval,
            max_bytes: max_bytes.max(1),
            buf: Vec::new(),
            opened_at: None,
        }
    }

    /// Adds `bytes` to the open frame; returns the frame when it has reached `max_bytes`.
    pub fn push(&mut self, bytes: &[u8], now: Instant) -> Option<Vec<u8>> {
        if bytes.is_empty() {
            return None;
        }
        if self.opened_at.is_none() {
            self.opened_at = Some(now);
        }
        self.buf.extend_from_slice(bytes);
        (self.buf.len() >= self.max_bytes).then(|| self.take())
    }

    /// The open frame, once one interval has passed since its first byte.
    pub fn flush_if_due(&mut self, now: Instant) -> Option<Vec<u8>> {
        let opened = self.opened_at?;
        (now.saturating_duration_since(opened) >= self.interval).then(|| self.take())
    }

    /// The open frame, now.
    pub fn flush(&mut self) -> Option<Vec<u8>> {
        (!self.buf.is_empty()).then(|| self.take())
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// When the open frame got its first byte; `None` while empty.
    pub fn opened_at(&self) -> Option<Instant> {
        self.opened_at
    }

    /// The instant the open frame falls due.
    pub fn due_at(&self) -> Option<Instant> {
        self.opened_at.map(|t| t + self.interval)
    }

    fn take(&mut self) -> Vec<u8> {
        self.opened_at = None;
        std::mem::take(&mut self.buf)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn push_returns_a_frame_only_when_max_bytes_reached() {
        let t0 = Instant::now();
        let mut b = Batcher::new(Batcher::INTERVAL, 8);
        assert_eq!(b.push(b"12345", t0), None);
        assert_eq!(b.push(b"6789", t0), Some(b"123456789".to_vec()));
        assert!(b.is_empty());
    }

    #[test]
    fn flush_if_due_waits_one_interval_from_first_byte() {
        let t0 = Instant::now();
        let mut b = Batcher::new(Duration::from_millis(16), 1 << 20);
        b.push(b"a", t0);
        assert_eq!(b.flush_if_due(t0 + Duration::from_millis(15)), None);
        assert_eq!(b.flush_if_due(t0 + Duration::from_millis(16)), Some(b"a".to_vec()));
        assert_eq!(b.opened_at(), None);
    }
}
