//! The scrollback ring: the last `max_lines` raw output lines of a session, replayed to a pane on attach.
//! A line is the bytes up to and including `\n`; an unterminated tail is kept as `partial` and comes last in
//! `snapshot()`, so a snapshot never cuts a line (or the multi-byte sequence inside it). Bytes are kept verbatim,
//! escape sequences included.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// What `sessions_attach` and `sessions_bring` return. The bytes themselves are replayed first on the channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Scrollback {
    pub lines: u32,
    pub dropped: u64,
    pub bytes: u64,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone)]
pub struct ScrollbackBuffer {
    max_lines: usize,
    lines: VecDeque<Vec<u8>>,
    partial: Vec<u8>,
    dropped: u64,
    bytes: usize,
}

impl ScrollbackBuffer {
    /// The `scrollback_lines` default (§13).
    pub const DEFAULT_LINES: usize = 10_000;

    pub fn new(max_lines: usize) -> Self {
        Self {
            max_lines: max_lines.max(1),
            lines: VecDeque::new(),
            partial: Vec::new(),
            dropped: 0,
            bytes: 0,
        }
    }

    pub fn push(&mut self, bytes: &[u8]) {
        let mut rest = bytes;
        while let Some(i) = rest.iter().position(|b| *b == b'\n') {
            let mut line = std::mem::take(&mut self.partial);
            line.extend_from_slice(&rest[..=i]);
            self.bytes += line.len();
            self.lines.push_back(line);
            rest = &rest[i + 1..];
            while self.lines.len() > self.max_lines {
                if let Some(old) = self.lines.pop_front() {
                    self.bytes -= old.len();
                    self.dropped += 1;
                }
            }
        }
        self.partial.extend_from_slice(rest);
    }

    /// Every kept line, then the partial tail.
    pub fn snapshot(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.bytes + self.partial.len());
        for line in &self.lines {
            out.extend_from_slice(line);
        }
        out.extend_from_slice(&self.partial);
        out
    }

    /// Completed lines kept.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Completed lines dropped off the front.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// The size of `snapshot()`.
    pub fn byte_len(&self) -> usize {
        self.bytes + self.partial.len()
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.partial.clear();
        self.bytes = 0;
    }

    /// The IPC description of the current snapshot at a terminal size.
    pub fn describe(&self, cols: u16, rows: u16) -> Scrollback {
        Scrollback {
            lines: self.lines.len() as u32,
            dropped: self.dropped,
            bytes: self.byte_len() as u64,
            cols,
            rows,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_last_max_lines() {
        let mut b = ScrollbackBuffer::new(3);
        for i in 0..5 {
            b.push(format!("l{i}\n").as_bytes());
        }
        assert_eq!(b.snapshot(), b"l2\nl3\nl4\n");
        assert_eq!(b.line_count(), 3);
        assert_eq!(b.dropped(), 2);
    }

    #[test]
    fn partial_line_survives_across_pushes_and_is_last_in_snapshot() {
        let mut b = ScrollbackBuffer::new(10);
        b.push(b"abc");
        b.push(b"def\ngh");
        assert_eq!(b.snapshot(), b"abcdef\ngh");
        assert_eq!(b.line_count(), 1);
    }

    #[test]
    fn default_is_ten_thousand_lines() {
        assert_eq!(ScrollbackBuffer::DEFAULT_LINES, 10_000);
    }

    #[test]
    fn raw_escape_bytes_are_kept_verbatim() {
        let mut b = ScrollbackBuffer::new(2);
        b.push(b"\x1b[2J\x1b[Hhi\n");
        assert_eq!(b.snapshot(), b"\x1b[2J\x1b[Hhi\n");
    }
}
