//! Bucket model — the direct port of v1's bucket rules from
//! `legacy/index.html` (the "buckets (model)" section).
//!
//! A bucket is just a name, and every idea carries its own list of them, so an
//! idea can sit in several buckets at once while storage stays a flat array of
//! ideas. There is no bucket registry: a bucket exists for exactly as long as
//! some idea mentions it, which is why deleting one only unfiles its ideas.

use serde::Serialize;

use crate::model::{Idea, PALETTE};

/// v1's `cleanBucketName`: collapse whitespace runs to single spaces, trim,
/// cap the length at [`crate::model::BUCKET_MAX`] characters. (JS slices
/// UTF-16 units; we slice chars, which is the right behavior for text.)
pub fn clean_bucket_name(raw: &str) -> String {
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    crate::model::take_chars(&collapsed, crate::model::BUCKET_MAX)
}

/// v1's `bucketColor`: a stable color per bucket name, derived with the same
/// hash (`h = h*31 + code, mod 9973`) and the same palette, so the same bucket
/// keeps the same color in v1 and v2 alike.
pub fn bucket_color(name: &str) -> &'static str {
    let mut h: u64 = 0;
    for c in name.chars() {
        h = (h * 31 + c as u64) % 9973;
    }
    PALETTE[(h % PALETTE.len() as u64) as usize]
}

/// One entry of the bucket index: the name and how many ideas are in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BucketEntry {
    pub name: String,
    pub count: usize,
}

/// All bucket names this idea is filed under (already normalized on load).
pub fn idea_buckets(idea: &Idea) -> &[String] {
    &idea.buckets
}

/// Whether `idea` is filed under exactly `name` (v1's `inBucket`).
pub fn in_bucket(idea: &Idea, name: &str) -> bool {
    idea.buckets.iter().any(|b| b == name)
}

/// v1's `bucketIndex`: every bucket with its member count, most members
/// first, ties broken alphabetically (case-insensitive like `localeCompare`
/// for the ASCII case, then by raw name for stability).
pub fn bucket_index(ideas: &[Idea]) -> Vec<BucketEntry> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for idea in ideas {
        for name in idea_buckets(idea) {
            if let Some(entry) = counts.iter_mut().find(|(n, _)| n == name) {
                entry.1 += 1;
            } else {
                counts.push((name.clone(), 1));
            }
        }
    }
    counts.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
            .then_with(|| a.0.cmp(&b.0))
    });
    counts
        .into_iter()
        .map(|(name, count)| BucketEntry { name, count })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idea(buckets: &[&str]) -> Idea {
        let mut i = crate::model::new_idea(
            "t".into(),
            buckets.iter().map(|s| s.to_string()).collect(),
            "",
        );
        crate::model::normalize_idea(&mut i);
        i
    }

    #[test]
    fn clean_collapses_whitespace_trims_and_caps_length() {
        assert_eq!(clean_bucket_name("  a   b  "), "a b");
        let long = "x".repeat(40);
        assert_eq!(clean_bucket_name(&long), "x".repeat(24));
    }

    #[test]
    fn clean_keeps_multibyte_names_on_char_boundaries() {
        let name = "ایده".repeat(20); // 40 chars, would slice mid-char as bytes
        assert_eq!(clean_bucket_name(&name).chars().count(), 24);
    }

    #[test]
    fn color_hash_matches_v1_vectors() {
        // h = (h*31 + code) % 9973, palette index h % 6
        // "work": 119 -> 3800 -> 8211 -> 5323;  5323 % 6 == 1
        assert_eq!(bucket_color("work"), "#7ec8a9");
        // "Work": 87 -> 2808 -> 7378 -> 9419;  9419 % 6 == 5
        assert_eq!(bucket_color("Work"), "#6bc7d6");
        assert_eq!(bucket_color(""), "#f0b429"); // h stays 0
                                                 // stable: same name, same color
        assert_eq!(bucket_color("reading list"), bucket_color("reading list"));
    }

    #[test]
    fn index_counts_and_orders_like_v1() {
        let ideas = vec![
            idea(&["Work", "Side"]),
            idea(&["work"]), // distinct bucket: matching is case-sensitive, like v1
            idea(&["Side"]),
            idea(&[]),
        ];
        let idx = bucket_index(&ideas);
        assert_eq!(
            idx,
            vec![
                BucketEntry {
                    name: "Side".into(),
                    count: 2
                },
                BucketEntry {
                    name: "Work".into(),
                    count: 1
                },
                BucketEntry {
                    name: "work".into(),
                    count: 1
                },
            ]
        );
        // Work before work: count tie broken alphabetically, case-insensitively
    }

    #[test]
    fn in_bucket_is_exact_match() {
        let i = idea(&["Work"]);
        assert!(in_bucket(&i, "Work"));
        assert!(!in_bucket(&i, "work"));
        assert!(!in_bucket(&i, ""));
    }
}
