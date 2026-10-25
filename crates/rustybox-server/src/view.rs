//! Pure view math ported from v1's rendering code: the glass liquid level,
//! the resting blob layout, dates, and truncation. No HTTP, no I/O — every
//! function is unit-testable on its own.

use rustybox_core::take_chars;

/// v1's glass geometry: liquid surface Y between `TOP_Y` (20+ ideas) and
/// `BOTTOM_Y` (empty), scaled by `min(n, 20) / 20`.
pub const TOP_Y: f64 = 150.0;
pub const BOTTOM_Y: f64 = 325.0;

pub fn level_y(n: usize) -> f64 {
    if n == 0 {
        BOTTOM_Y
    } else {
        BOTTOM_Y - (n.min(20) as f64 / 20.0) * (BOTTOM_Y - TOP_Y)
    }
}

/// v1's `renderBucket`: the elliptic surface narrows as the bucket fills.
pub fn surface_rx(n: usize) -> f64 {
    (90.0 - n as f64 * 1.2).max(60.0)
}

/// One resting blob under the liquid surface (v1's `renderBlobs` formulas).
#[derive(Debug, Clone, PartialEq)]
pub struct Blob {
    pub cx: i64,
    pub cy: i64,
    pub r: i64,
}

/// At most 14 blobs are drawn; the template shows a "+N more…" note beyond.
pub const MAX_BLOBS: usize = 14;

pub fn blob_layout(n: usize) -> Vec<Blob> {
    let show = n.min(MAX_BLOBS);
    let surface = level_y(n);
    (0..show)
        .map(|i| Blob {
            cx: 145 + ((i * 37 + 13) % 110) as i64,
            cy: (surface + 8.0 + (i % 3) as f64 * 14.0) as i64,
            r: (5 + (i % 3) * 2) as i64,
        })
        .collect()
}

pub fn more_count(n: usize) -> Option<usize> {
    n.checked_sub(MAX_BLOBS).filter(|m| *m > 0)
}

/// v1's `truncate(s, 140)` for idea cards: cut to `max - 1` chars + ellipsis.
pub fn truncate_card(text: &str) -> String {
    const MAX: usize = 140;
    if text.chars().count() > MAX {
        format!("{}…", take_chars(text, MAX - 1))
    } else {
        text.to_string()
    }
}

/// `2024-05-01` — v2 renders ISO dates (v1 used the browser's locale).
pub fn fmt_date(ms: i64) -> String {
    let dt = chrono::DateTime::from_timestamp_millis(ms).unwrap_or_else(chrono::Utc::now);
    chrono::DateTime::<chrono::Local>::from(dt)
        .format("%Y-%m-%d")
        .to_string()
}

/// `2024-05-01 14:32` for the idea page's created stamp.
pub fn fmt_datetime(ms: i64) -> String {
    let dt = chrono::DateTime::from_timestamp_millis(ms).unwrap_or_else(chrono::Utc::now);
    chrono::DateTime::<chrono::Local>::from(dt)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

/// Compact age for card meta: "today", "1d", "3d" — the full date sits next
/// to it, so the card answers "is this fresh?" at a glance.
pub fn age_short(created_ms: i64, now_ms: i64) -> String {
    let days = (now_ms - created_ms).max(0) / 86_400_000;
    match days {
        0 => "today".to_string(),
        1 => "1d".to_string(),
        n => format!("{n}d"),
    }
}

/// v1's age line: "Created today" / "Created yesterday" / "Created N days ago".
pub fn age_line(created_ms: i64, now_ms: i64) -> String {
    let days = (now_ms - created_ms).max(0) / 86_400_000;
    match days {
        0 => "Created today".to_string(),
        1 => "Created yesterday".to_string(),
        n => format!("Created {n} days ago"),
    }
}

/// Percent-encode a query-string component (RFC 3986 unreserved characters
/// pass through; everything else becomes %XX). Used for bucket filter links.
pub fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glass_levels_match_v1_geometry() {
        assert_eq!(level_y(0), BOTTOM_Y);
        assert_eq!(level_y(1), 325.0 - 8.75); // 1/20 of the 175px range
        assert_eq!(level_y(20), TOP_Y);
        assert_eq!(level_y(100), TOP_Y, "clamped at 20");
    }

    #[test]
    fn surface_narrows_until_clamped() {
        assert_eq!(surface_rx(0), 90.0);
        assert_eq!(surface_rx(10), 78.0);
        assert_eq!(surface_rx(25), 60.0);
    }

    #[test]
    fn blob_layout_uses_v1_formulas() {
        let blobs = blob_layout(3);
        assert_eq!(blobs.len(), 3);
        // level_y(3) = 325 - 3/20*175 = 298.75; first blob sits 8px under it
        assert_eq!(
            blobs[0],
            Blob {
                cx: 158,
                cy: 306,
                r: 5
            }
        );
        assert_eq!(blob_layout(100).len(), MAX_BLOBS);
        assert_eq!(blob_layout(0).len(), 0);
    }

    #[test]
    fn more_count_only_beyond_fourteen() {
        assert_eq!(more_count(14), None);
        assert_eq!(more_count(15), Some(1));
        assert_eq!(more_count(0), None);
    }

    #[test]
    fn card_truncation_matches_v1() {
        assert_eq!(truncate_card("short"), "short");
        let long = "x".repeat(141);
        let cut = truncate_card(&long);
        assert_eq!(cut.chars().count(), 140);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn age_short_compacts_to_days() {
        let now = 1_700_000_000_000;
        assert_eq!(age_short(now, now), "today");
        assert_eq!(age_short(now - 86_400_000, now), "1d");
        assert_eq!(age_short(now - 3 * 86_400_000, now), "3d");
        assert_eq!(age_short(now - 30 * 86_400_000, now), "30d");
    }

    #[test]
    fn age_short_never_goes_negative() {
        // a clock skew must not produce "-1d"
        let now = 1_700_000_000_000;
        assert_eq!(age_short(now + 5 * 86_400_000, now), "today");
    }

    #[test]
    fn age_lines_like_v1() {
        let now = 1_700_000_000_000;
        assert_eq!(age_line(now, now), "Created today");
        assert_eq!(age_line(now - 86_400_000, now), "Created yesterday");
        assert_eq!(age_line(now - 3 * 86_400_000, now), "Created 3 days ago");
    }

    #[test]
    fn url_encode_keeps_unreserved_only() {
        assert_eq!(url_encode("work"), "work");
        assert_eq!(url_encode("side projects"), "side%20projects");
        assert_eq!(url_encode("a&b=c"), "a%26b%3Dc");
        assert_eq!(url_encode("راستی"), "%D8%B1%D8%A7%D8%B3%D8%AA%DB%8C");
    }
}
