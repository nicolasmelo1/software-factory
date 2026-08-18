//! Today, as an ISO date, without pulling in a date library.

use std::time::{SystemTime, UNIX_EPOCH};

pub fn today() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0) as i64;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

pub fn plus_months(iso: &str, months: i64) -> String {
    let mut parts = iso.split('-');
    let year: i64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1970);
    let month: i64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1);
    let day: i64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1);
    let total = (year * 12 + month - 1) + months;
    let (y, m) = (total / 12, total % 12 + 1);
    // Clamp rather than roll over: a review date is a reminder, not an instant.
    let d = day.min(days_in_month(y, m));
    format!("{y:04}-{m:02}-{d:02}")
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        _ => 28,
    }
}

/// Howard Hinnant's days-from-civil, inverted.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}
