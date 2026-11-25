//! Purpose: wall-clock and calendar helpers for history timestamps.
//! Responsibilities: unix-seconds/ISO-8601 conversion via civil-date
//! algorithms, `now` helpers, run-ID minting; no I/O.
//! Rationale: split out of src/history.rs (wave-4 ratchet,
//! pretender-5zo.15.4) to keep files under gate thresholds; bodies moved
//! verbatim. The ISO-8601 calendar helpers stay private, so their unit tests
//! remain in-file instead of the integration target.

use std::time::{SystemTime, UNIX_EPOCH};

pub fn make_run_id() -> String {
    format!("{}-{}", now_unix_secs(), std::process::id())
}

pub fn now_iso8601() -> String {
    unix_to_iso8601(now_unix_secs())
}

pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn unix_to_iso8601(secs: u64) -> String {
    let days = secs / 86400;
    let time = secs % 86400;
    let h = time / 3600;
    let m = (time % 3600) / 60;
    let s = time % 60;
    let (year, month, day) = days_to_ymd(days as i64);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, h, m, s
    )
}

pub fn parse_timestamp_secs(ts: &str) -> Option<u64> {
    iso8601_to_unix(ts)
}

fn iso8601_to_unix(s: &str) -> Option<u64> {
    if s.len() < 19 || !s.is_ascii() {
        return None;
    }
    let year: i64 = s[0..4].parse().ok()?;
    let month: u64 = s[5..7].parse().ok()?;
    let day: u64 = s[8..10].parse().ok()?;
    let hour: u64 = s[11..13].parse().ok()?;
    let min: u64 = s[14..16].parse().ok()?;
    let sec: u64 = s[17..19].parse().ok()?;
    let days = ymd_to_days(year, month, day)?;
    Some(days * 86400 + hour * 3600 + min * 60 + sec)
}

// Civil date from days since 1970-01-01.
// Algorithm: https://howardhinnant.github.io/date_algorithms.html
fn days_to_ymd(days: i64) -> (u32, u32, u32) {
    let z = days + 719468;
    let era: i64 = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m as u32, d as u32)
}

fn ymd_to_days(year: i64, month: u64, day: u64) -> Option<u64> {
    let y = if month <= 2 { year - 1 } else { year };
    let era: i64 = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let result = era * 146097 + doe as i64 - 719468;
    if result < 0 {
        None
    } else {
        Some(result as u64)
    }
}

#[cfg(test)]
mod tests {
    // The calendar helpers are private, so their tests stay in-file. No
    // unwrap/expect per gate rules — sanctioned panic-on-failure forms only.
    use super::*;

    #[test]
    fn test_iso8601_roundtrip() {
        let ts = "2026-06-10T14:30:00Z";
        let secs = iso8601_to_unix(ts).unwrap_or_else(|| panic!("parse failed: {ts}"));
        assert_eq!(unix_to_iso8601(secs), ts);
    }

    #[test]
    fn test_iso8601_boundary_dates() {
        // Unix epoch
        assert_eq!(unix_to_iso8601(0), "1970-01-01T00:00:00Z");
        // Y2K leap day
        let ts = "2000-02-29T12:00:00Z";
        let secs = iso8601_to_unix(ts).unwrap_or_else(|| panic!("parse failed: {ts}"));
        assert_eq!(unix_to_iso8601(secs), ts);
        // Century non-leap (2100 is not a leap year)
        let ts = "2100-02-28T00:00:00Z";
        let secs = iso8601_to_unix(ts).unwrap_or_else(|| panic!("parse failed: {ts}"));
        assert_eq!(unix_to_iso8601(secs), ts);
        // Near Unix i32 max
        let ts = "2038-01-19T03:14:07Z";
        let secs = iso8601_to_unix(ts).unwrap_or_else(|| panic!("parse failed: {ts}"));
        assert_eq!(unix_to_iso8601(secs), ts);
    }

    #[test]
    fn test_iso8601_rejects_non_ascii() {
        assert!(iso8601_to_unix("2026-06-10T14:30:0\u{00e9}Z").is_none());
    }
}
