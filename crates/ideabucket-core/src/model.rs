//! The `Idea` record and its normalization rules, ported from v1's
//! `normalizeIdea` / `addIdea` in `legacy/index.html`.
//!
//! Field names stay camelCase (`lastViewed`, not `last_viewed`) so that a JSON
//! export from v1 — or the browser's `idea-bucket-v1` localStorage payload —
//! can be imported by v2 unchanged, and v2 exports can be pasted back into v1.

use serde::{Deserialize, Serialize};

use crate::buckets::{bucket_color, clean_bucket_name};

/// v1's blob/chip palette, in the exact order it defines it.
pub const PALETTE: [&str; 6] = [
    "#f0b429", "#7ec8a9", "#8ea8ff", "#f28c6b", "#c78fff", "#6bc7d6",
];

/// v1's `BUCKET_MAX`: a bucket name collapses to at most this many characters.
pub const BUCKET_MAX: usize = 24;

/// v1's composer `maxlength` on capture.
pub const CAPTURE_MAX: usize = 280;

/// v1's edit-area `maxlength`.
pub const EDIT_MAX: usize = 2000;

/// A single idea. Stored as a flat array, exactly like v1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Idea {
    /// Missing on legacy entries; `normalize_idea` fills a fresh UUID.
    #[serde(default)]
    pub id: String,
    pub text: String,
    /// Legacy or hand-edited entries can lack a color entirely; `normalize_idea`
    /// fills a stable one, so this field defaults like v1's lenient loader.
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub created: i64,
    #[serde(default)]
    pub last_viewed: i64,
    /// Set the first time the idea is edited; absent otherwise, like v1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited: Option<i64>,
    #[serde(default)]
    pub buckets: Vec<String>,
    /// v1's "from: …" capture provenance line; empty when captured without one.
    #[serde(default)]
    pub source: String,
}

/// Current wall-clock time in epoch milliseconds (v1 uses `Date.now()`).
pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// A fresh UUID v4, matching v1's `crypto.randomUUID()` ids.
pub fn new_idea_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Validation failures for idea text, mirroring v1's guards
/// ("An idea can't be empty" / the 280 and 2000 char maxlengths).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextError {
    Empty,
    TooLong { len: usize, max: usize },
}

impl std::fmt::Display for TextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextError::Empty => write!(f, "An idea can't be empty"),
            TextError::TooLong { len, max } => {
                write!(f, "Idea is {len} characters — the limit is {max}")
            }
        }
    }
}

impl std::error::Error for TextError {}

/// Validate idea text the way v1 does at capture (`addIdea`) and at edit
/// (`commitEdit`): trim, reject empty, enforce the length limit.
pub fn validate_text(raw: &str, max: usize) -> Result<String, TextError> {
    let text = raw.trim();
    if text.is_empty() {
        return Err(TextError::Empty);
    }
    let len = text.chars().count();
    if len > max {
        return Err(TextError::TooLong { len, max });
    }
    Ok(text.to_string())
}

/// Truncate to at most `n` characters (v1 slices the source line to 280 and
/// appends an ellipsis; slicing must respect char boundaries, unlike JS UTF-16
/// `slice`).
pub fn take_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// Build a new idea the way v1's `addIdea` does: fresh UUID, a random palette
/// color, now timestamps, the given buckets (cleaned) and source line.
pub fn new_idea(text: String, buckets: Vec<String>, source: &str) -> Idea {
    let now = now_ms();
    let mut rng = rand::rng();
    let color = PALETTE[rand::Rng::random_range(&mut rng, 0..PALETTE.len())].to_string();
    // v1's source line is trimmed selection text, truncated to 280 chars with
    // an ellipsis when longer.
    let src = source.trim();
    let source = if src.chars().count() > CAPTURE_MAX {
        format!("{}…", take_chars(src, CAPTURE_MAX))
    } else {
        src.to_string()
    };
    Idea {
        id: new_idea_id(),
        text,
        color,
        created: now,
        last_viewed: now,
        edited: None,
        buckets: buckets.iter().map(|b| clean_bucket_name(b)).collect(),
        source,
    }
}

/// Fill in what legacy or hand-edited entries are missing, so ids stay
/// deletable, dates render, and colors never read "undefined" — the direct
/// port of v1's `normalizeIdea` (the jsdom suite caught v1 skipping this for
/// legacy entries; v2 enforces it at load and import time).
pub fn normalize_idea(idea: &mut Idea) {
    // buckets: drop empties, clean names, dedupe preserving order
    let mut kept: Vec<String> = Vec::with_capacity(idea.buckets.len());
    for raw in &idea.buckets {
        let name = clean_bucket_name(raw);
        if !name.is_empty() && !kept.contains(&name) {
            kept.push(name);
        }
    }
    idea.buckets = kept;

    if idea.id.is_empty() {
        idea.id = new_idea_id();
    }
    let valid_color = idea.color.len() >= 2
        && idea.color.starts_with('#')
        && idea.color.len() <= 9
        && idea.color[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !valid_color {
        idea.color = bucket_color(&idea.id).to_string();
    }
    if idea.created <= 0 {
        idea.created = now_ms();
    }
    if idea.last_viewed <= 0 {
        idea.last_viewed = idea.created;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_text_rejects_empty_and_whitespace() {
        assert_eq!(validate_text("   ", EDIT_MAX), Err(TextError::Empty));
        assert_eq!(validate_text("", CAPTURE_MAX), Err(TextError::Empty));
    }

    #[test]
    fn validate_text_trims_and_enforces_limit() {
        assert_eq!(validate_text("  hi  ", CAPTURE_MAX), Ok("hi".into()));
        let long = "x".repeat(281);
        assert_eq!(
            validate_text(&long, CAPTURE_MAX),
            Err(TextError::TooLong { len: 281, max: 280 })
        );
        let ok = "x".repeat(280);
        assert_eq!(validate_text(&ok, CAPTURE_MAX), Ok(ok));
    }

    #[test]
    fn new_idea_sets_all_fields_like_v1() {
        let idea = new_idea("hello".into(), vec![" Work ".into()], " https://x ");
        assert_eq!(idea.text, "hello");
        assert_eq!(idea.buckets, vec!["Work"]);
        assert_eq!(idea.source, "https://x");
        assert!(PALETTE.contains(&idea.color.as_str()));
        assert_eq!(idea.created, idea.last_viewed);
        assert!(idea.edited.is_none());
        assert_eq!(idea.id.len(), 36); // uuid v4
    }

    #[test]
    fn new_idea_truncates_long_source_with_ellipsis() {
        let src = "a".repeat(300);
        let idea = new_idea("t".into(), vec![], &src);
        assert_eq!(idea.source.chars().count(), CAPTURE_MAX + 1);
        assert!(idea.source.ends_with('…'));
    }

    #[test]
    fn normalize_fills_missing_fields() {
        let mut idea = Idea {
            id: String::new(),
            text: "legacy".into(),
            color: "nope".into(),
            created: 0,
            last_viewed: 0,
            edited: None,
            buckets: vec!["  a  ".into(), "a".into(), "".into(), "b".into()],
            source: String::new(),
        };
        normalize_idea(&mut idea);
        assert!(!idea.id.is_empty());
        assert!(PALETTE.contains(&idea.color.as_str()));
        assert!(idea.created > 0);
        assert_eq!(idea.last_viewed, idea.created);
        assert_eq!(idea.buckets, vec!["a", "b"]);
    }

    #[test]
    fn normalize_keeps_valid_color_and_dates() {
        let mut idea = Idea {
            id: "abc".into(),
            text: "t".into(),
            color: "#f0b429".into(),
            created: 1_700_000_000_000,
            last_viewed: 1_700_000_000_000,
            edited: None,
            buckets: vec![],
            source: String::new(),
        };
        let before = idea.clone();
        normalize_idea(&mut idea);
        assert_eq!(idea, before);
    }

    #[test]
    fn idea_serializes_with_v1_camel_case_names() {
        let mut idea = new_idea("t".into(), vec!["B".into()], "");
        idea.edited = Some(123);
        let v: serde_json::Value = serde_json::to_value(&idea).unwrap();
        for key in [
            "id",
            "text",
            "color",
            "created",
            "lastViewed",
            "buckets",
            "source",
        ] {
            assert!(v.get(key).is_some(), "missing {key}");
        }
        assert!(v.get("last_viewed").is_none());
        assert_eq!(v["edited"], 123);
    }

    #[test]
    fn unedited_idea_omits_edited_key_like_v1() {
        let idea = new_idea("t".into(), vec![], "");
        let v: serde_json::Value = serde_json::to_value(&idea).unwrap();
        assert!(v.get("edited").is_none());
    }
}
