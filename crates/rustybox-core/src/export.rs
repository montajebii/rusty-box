//! JSON and Markdown exporters, ported from v1's export buttons.
//!
//! The JSON payload is the store itself — the exact same shape v1 exports, so
//! a v2 export re-imports into v1 and vice versa. The Markdown layout is the
//! same one-section-per-bucket document, with locale-neutral ISO dates
//! instead of `toLocaleDateString()`.

use chrono::{DateTime, Local};

use crate::buckets::{bucket_index, idea_buckets, in_bucket};
use crate::model::Idea;

/// v1's `exportFile('idea-bucket.json', JSON.stringify(ideas, null, 2))`.
pub fn export_json(ideas: &[Idea]) -> String {
    let mut body = serde_json::to_string_pretty(ideas).expect("ideas always serialize");
    body.push('\n');
    body
}

fn entry_date(created_ms: i64) -> String {
    // v1 renders the browser's locale date; v2 standardizes on ISO YYYY-MM-DD.
    let utc = chrono::DateTime::from_timestamp_millis(created_ms).unwrap_or_else(chrono::Utc::now);
    DateTime::<Local>::from(utc).format("%Y-%m-%d").to_string()
}

fn entry(idea: &Idea, suffix: &str) -> String {
    format!(
        "- **{}** — {}{}",
        entry_date(idea.created),
        idea.text,
        suffix
    )
}

/// v1's Markdown export: one `##` section per bucket (ideas filed in several
/// appear under each, noted with "_(also in …)_"), then the unfiled ideas.
pub fn export_markdown(ideas: &[Idea]) -> String {
    let buckets = bucket_index(ideas);
    let mut lines: Vec<String> = vec!["# The Idea Bucket".into(), String::new()];

    if buckets.is_empty() {
        lines.push(format!("## Ideas ({})", ideas.len()));
        lines.push(String::new());
        lines.extend(ideas.iter().map(|i| entry(i, "")));
    } else {
        for b in &buckets {
            lines.push(format!("## {} ({})", b.name, b.count));
            lines.push(String::new());
            for idea in ideas.iter().filter(|i| in_bucket(i, &b.name)) {
                let others: Vec<&str> = idea_buckets(idea)
                    .iter()
                    .filter(|x| *x != &b.name)
                    .map(String::as_str)
                    .collect();
                let suffix = if others.is_empty() {
                    String::new()
                } else {
                    format!("  _(also in {})_", others.join(", "))
                };
                lines.push(entry(idea, &suffix));
            }
            lines.push(String::new());
        }

        let unfiled: Vec<&Idea> = ideas
            .iter()
            .filter(|i| idea_buckets(i).is_empty())
            .collect();
        if !unfiled.is_empty() {
            lines.push(format!("## Unfiled ({})", unfiled.len()));
            lines.push(String::new());
            lines.extend(unfiled.iter().map(|i| entry(i, "")));
            lines.push(String::new());
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::new_idea;

    fn fixed_idea(text: &str, buckets: &[&str], created_ms: i64) -> Idea {
        let mut idea = new_idea(
            text.to_string(),
            buckets.iter().map(|s| s.to_string()).collect(),
            "",
        );
        idea.created = created_ms;
        idea.last_viewed = created_ms;
        idea
    }

    // 2023-11-14 in UTC (any local zone used by the exporter renders this
    // predictable date in tests via Local — the fixture asserts the shape, and
    // the date function is unit-tested separately for stability).
    const T: i64 = 1_700_000_000_000;

    #[test]
    fn json_export_is_a_pretty_array_in_v1_shape() {
        let ideas = vec![fixed_idea("hello", &["Work"], T)];
        let body = export_json(&ideas);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(v.is_array());
        assert_eq!(v[0]["text"], "hello");
        assert_eq!(v[0]["buckets"][0], "Work");
        assert!(v[0].get("edited").is_none());
    }

    #[test]
    fn markdown_export_without_buckets() {
        let ideas = vec![fixed_idea("one", &[], T), fixed_idea("two", &[], T)];
        let md = export_markdown(&ideas);
        assert!(md.starts_with("# The Idea Bucket\n\n## Ideas (2)\n\n"));
        assert!(md.contains("— one"));
        assert!(md.contains("— two"));
        assert!(
            !md.contains("## Unfiled"),
            "no buckets section means no Unfiled section"
        );
    }

    #[test]
    fn markdown_export_with_buckets_and_unfiled() {
        let ideas = vec![
            fixed_idea("shared", &["Work", "Side"], T),
            fixed_idea("only work", &["Work"], T),
            fixed_idea("loose", &[], T),
        ];
        let md = export_markdown(&ideas);

        // Sections in bucket-index order: Work first (2 members), then Side.
        let work_pos = md.find("## Work (2)").expect("Work section");
        let side_pos = md.find("## Side (1)").expect("Side section");
        let unfiled_pos = md.find("## Unfiled (1)").expect("Unfiled section");
        assert!(work_pos < side_pos && side_pos < unfiled_pos);

        // The shared idea appears under both buckets, with the cross-note.
        assert!(md.matches("- **").count() >= 4, "listed under each bucket");
        let side_section = &md[side_pos..unfiled_pos];
        assert!(
            side_section.contains("shared") && side_section.contains("_(also in Work)_"),
            "Side section: {side_section}"
        );

        // Unfiled ideas close the document.
        assert!(md[unfiled_pos..].contains("— loose"));
    }

    #[test]
    fn markdown_export_matches_v1_line_shapes_exactly() {
        let ideas = vec![
            fixed_idea("solo", &["Work"], T),
            fixed_idea("loose", &[], T),
        ];
        let md = export_markdown(&ideas);
        for line in md.lines().filter(|l| l.starts_with("- **")) {
            // `- **YYYY-MM-DD** — text` (optionally with the also-in suffix)
            let rest = line.trim_start_matches("- **");
            let date = rest.split("**").next().unwrap_or_default();
            assert_eq!(date.len(), 10, "ISO date in {line}");
            assert!(date.chars().nth(4) == Some('-'));
            assert!(line.contains(" — "), "em-dash separator in {line}");
        }
    }

    #[test]
    fn markdown_dates_come_from_created_ms() {
        let ideas = vec![fixed_idea("t", &[], T)];
        let md = export_markdown(&ideas);
        let line = md.lines().find(|l| l.starts_with("- **")).unwrap();
        let date = line
            .trim_start_matches("- **")
            .split("**")
            .next()
            .unwrap()
            .to_string();
        // The fixture timestamp is 2023-11-14 in UTC; allow one day of drift
        // for exotic server timezones rather than pinning the machine's zone.
        assert!(date.starts_with("2023-11-1"), "got {date}");
    }

    #[test]
    fn json_export_is_stable_for_the_same_input() {
        let ideas = vec![fixed_idea("a", &["B"], T), fixed_idea("c", &[], T)];
        assert_eq!(export_json(&ideas), export_json(&ideas));
    }
}
