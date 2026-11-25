//! `time` attribute → milliseconds conversion with round-half-up.

use crate::config::TimeUnit;

/// Convert a time value in seconds to milliseconds, rounding half-up.
pub(super) fn time_to_ms(value: f64, time_unit: TimeUnit) -> u32 {
    let ms = match time_unit {
        TimeUnit::Seconds => value * 1000.0,
        TimeUnit::Milliseconds => value,
    };
    round_half_up(ms)
}

/// Round-half-up to the nearest u32.
fn round_half_up(value: f64) -> u32 {
    let rounded = value.round();
    // Handle the tie-breaking: .round() in Rust uses round-half-to-even (bankers' rounding),
    // which is NOT what we want. We need round-half-up. Check if the fractional part is
    // exactly 0.5 and the integer part is even.
    let truncated = value.trunc();
    let fractional = (value - truncated).abs();
    if (fractional - 0.5).abs() < f64::EPSILON && truncated as i64 % 2 == 0 {
        // Banker's rounding would round down; we need to round up
        (truncated + 1.0).max(0.0) as u32
    } else {
        rounded.max(0.0) as u32
    }
}
