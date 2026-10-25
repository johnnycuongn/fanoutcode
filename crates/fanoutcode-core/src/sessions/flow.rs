//! Flow control: the reader pauses when `HIGH_WATER` bytes are sent to the pane and not yet acknowledged, and
//! resumes once the unacknowledged bytes fall below `LOW_WATER`. Nothing is dropped; the PTY simply blocks the child.

#[derive(Debug, Clone)]
pub struct FlowControl {
    high: usize,
    low: usize,
    in_flight: usize,
    paused: bool,
}

impl Default for FlowControl {
    fn default() -> Self {
        Self::new(Self::HIGH_WATER, Self::LOW_WATER)
    }
}

impl FlowControl {
    /// 1 MiB unacknowledged pauses the reader.
    pub const HIGH_WATER: usize = 1_048_576;
    /// Below 256 KiB unacknowledged it resumes.
    pub const LOW_WATER: usize = 262_144;

    pub fn new(high: usize, low: usize) -> Self {
        Self {
            high,
            low: low.min(high),
            in_flight: 0,
            paused: false,
        }
    }

    /// Counts `n` bytes sent; true when this send just paused the reader.
    pub fn on_sent(&mut self, n: usize) -> bool {
        self.in_flight = self.in_flight.saturating_add(n);
        if !self.paused && self.in_flight >= self.high {
            self.paused = true;
            return true;
        }
        false
    }

    /// Counts `n` bytes acknowledged (saturating: an over-ack is a client bug, not a panic); true when this ack just
    /// resumed the reader.
    pub fn on_ack(&mut self, n: usize) -> bool {
        self.in_flight = self.in_flight.saturating_sub(n);
        if self.paused && self.in_flight < self.low {
            self.paused = false;
            return true;
        }
        false
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn in_flight(&self) -> usize {
        self.in_flight
    }

    pub fn reset(&mut self) {
        self.in_flight = 0;
        self.paused = false;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::sessions::batch::Batcher;

    #[test]
    fn pauses_at_high_water_and_resumes_below_low_water() {
        let mut f = FlowControl::new(1000, 200);
        assert!(!f.on_sent(999));
        assert!(f.on_sent(1)); // 1000 in flight → paused
        assert!(f.paused());
        assert!(!f.on_ack(700)); // 300 left, still paused
        assert!(f.on_ack(101)); // 199 < 200 → resumed
        assert!(!f.paused());
    }

    #[test]
    fn reset_clears_in_flight_and_pause() {
        let mut f = FlowControl::new(10, 5);
        f.on_sent(10);
        f.reset();
        assert_eq!(f.in_flight(), 0);
        assert!(!f.paused());
    }

    #[test]
    fn defaults_match_spec_budget() {
        assert_eq!(FlowControl::HIGH_WATER, 1 << 20);
        assert_eq!(FlowControl::LOW_WATER, 256 * 1024);
        assert_eq!(Batcher::INTERVAL, Duration::from_millis(16));
    }
}
