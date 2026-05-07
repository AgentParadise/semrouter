//! Timestamp helpers built on `std::time::SystemTime`.
//!
//! Replaces our prior `chrono` dependency. Two formats:
//! - `iso8601_now()` → "2026-05-07T17:42:05Z"
//! - `compact_now()` → "20260507_174205" (filename-safe)

use std::time::{SystemTime, UNIX_EPOCH};

/// Current UTC time as RFC 3339 / ISO 8601 ("YYYY-MM-DDTHH:MM:SSZ").
pub fn iso8601_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_iso8601(secs)
}

/// Current UTC time as a compact filename-safe string ("YYYYMMDD_HHMMSS").
pub fn compact_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_compact(secs)
}

/// Civil date/time from unix seconds. Algorithm: Howard Hinnant's date.h
/// "civil_from_days" — exact, no allocation, no leap-second weirdness.
fn civil(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let secs_of_day = (secs % 86_400) as u32;
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;

    let z = days + 719_468;
    let era = if z >= 0 { z / 146_097 } else { (z - 146_096) / 146_097 };
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = (y + if m <= 2 { 1 } else { 0 }) as i32;

    (y, m, d, hour, minute, second)
}

fn format_iso8601(secs: u64) -> String {
    let (y, mo, d, h, mi, s) = civil(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn format_compact(secs: u64) -> String {
    let (y, mo, d, h, mi, s) = civil(secs);
    format!("{y:04}{mo:02}{d:02}_{h:02}{mi:02}{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso8601_format_for_unix_epoch() {
        assert_eq!(format_iso8601(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn iso8601_format_for_known_2026_timestamp() {
        // 2026-05-08T17:42:05Z = 1778262125 secs since unix epoch
        assert_eq!(format_iso8601(1_778_262_125), "2026-05-08T17:42:05Z");
    }

    #[test]
    fn compact_format_for_known_2026_timestamp() {
        assert_eq!(format_compact(1_778_262_125), "20260508_174205");
    }

    #[test]
    fn iso8601_now_has_correct_shape() {
        let s = iso8601_now();
        assert_eq!(s.len(), 20, "got {s:?}");
        assert!(s.ends_with('Z'));
        assert_eq!(&s[4..5], "-");
        assert_eq!(&s[7..8], "-");
        assert_eq!(&s[10..11], "T");
    }

    #[test]
    fn compact_now_has_correct_shape() {
        let s = compact_now();
        assert_eq!(s.len(), 15, "got {s:?}");
        assert_eq!(&s[8..9], "_");
        // First 4 chars should be year between 2020-2099 for sanity
        let year: u32 = s[0..4].parse().unwrap();
        assert!((2020..=2099).contains(&year), "year out of range: {year}");
    }
}
