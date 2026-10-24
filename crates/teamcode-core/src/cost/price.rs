//! Pricing a usage record with LiveSpec's pinned table (`livespec_core::usage::price`), cache tokens at 1.25× (write)
//! and 0.1× (read) of the input rate.

use livespec_core::usage::price;

use super::transcript::UsageRecord;

pub const CACHE_WRITE_RATE: f64 = 1.25;
pub const CACHE_READ_RATE: f64 = 0.10;

/// List-price dollars for one record; `None` for a model the table lacks.
pub fn price_usage(r: &UsageRecord) -> Option<f64> {
    let p = price(&r.model)?;
    let input = r.input as f64 + CACHE_WRITE_RATE * r.cache_write as f64 + CACHE_READ_RATE * r.cache_read as f64;
    Some((input * p.input_per_mtok + r.output as f64 * p.output_per_mtok) / 1_000_000.0)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn rec(model: &str, input: u64, cache_write: u64, cache_read: u64, output: u64) -> UsageRecord {
        UsageRecord {
            at: Utc::now(),
            model: model.into(),
            input,
            cache_write,
            cache_read,
            output,
            agent_id: None,
        }
    }

    #[test]
    fn price_uses_the_table_with_cache_rates() {
        let r = rec("claude-sonnet-5-5", 1000, 1000, 10000, 500);
        assert!((price_usage(&r).unwrap() - 0.0115).abs() < 1e-9); // (1000 + 1250 + 1000)·2 + 500·10, per million
    }

    #[test]
    fn unknown_models_are_unpriced() {
        assert_eq!(price_usage(&rec("mystery-9", 1, 0, 0, 1)), None);
    }
}
