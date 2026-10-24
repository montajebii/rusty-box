//! JSON import — new in v2, and the migration path from the browser app:
//! paste the JSON that v1 exported (or its `idea-bucket-v1` localStorage
//! payload) and the store fills up with the same ideas.

use serde_json::Value;

use crate::model::{normalize_idea, Idea};

/// Replace wipes the store and installs the file's ideas as-is; Merge keeps
/// existing ideas and appends the file's new ones (deduplicated by id).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportMode {
    Replace,
    Merge,
}

/// How many ideas came in and how many entries were skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportReport {
    pub imported: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    /// The payload is not valid JSON at all.
    NotJson,
    /// Valid JSON, but not the array v1 exports (an object, a string, …).
    NotAnArray,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::NotJson => write!(f, "That doesn't look like JSON"),
            ImportError::NotAnArray => write!(f, "Expected a JSON array of ideas"),
        }
    }
}

impl std::error::Error for ImportError {}

/// Import ideas from a JSON payload (v1's export shape) into `ideas`.
///
/// Junk entries (missing/typed-wrong fields) are skipped and counted, like
/// v1's loader drops them; surviving entries are normalized, so an idea that
/// only has `text` arrives with an id, a color, dates and clean buckets.
pub fn import(
    ideas: &mut Vec<Idea>,
    raw: &str,
    mode: ImportMode,
) -> Result<ImportReport, ImportError> {
    let value: Value = serde_json::from_str(raw.trim()).map_err(|_| ImportError::NotJson)?;
    let values: Vec<Value> = match value {
        Value::Array(items) => items,
        _ => return Err(ImportError::NotAnArray),
    };

    let mut incoming: Vec<Idea> = Vec::with_capacity(values.len());
    let mut skipped = 0usize;
    for value in values {
        match serde_json::from_value::<Idea>(value) {
            Ok(mut idea) => {
                normalize_idea(&mut idea);
                incoming.push(idea);
            }
            Err(_) => skipped += 1,
        }
    }

    match mode {
        ImportMode::Replace => {
            let imported = incoming.len();
            *ideas = incoming;
            Ok(ImportReport { imported, skipped })
        }
        ImportMode::Merge => {
            let mut known: std::collections::HashSet<String> =
                ideas.iter().map(|i| i.id.clone()).collect();
            let mut imported = 0usize;
            for idea in incoming {
                if known.insert(idea.id.clone()) {
                    ideas.push(idea);
                    imported += 1;
                }
            }
            Ok(ImportReport { imported, skipped })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1_EXPORT: &str = r##"[
      {
        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        "text": "from the browser",
        "color": "#f0b429",
        "created": 1700000000000,
        "lastViewed": 1700000000000,
        "buckets": ["Work", "Work", ""],
        "source": ""
      },
      { "text": "legacy bare entry" },
      42
    ]"##;

    fn fresh() -> Vec<Idea> {
        Vec::new()
    }

    #[test]
    fn replace_installs_file_order_and_normalizes() {
        let mut ideas = fresh();
        let report = import(&mut ideas, V1_EXPORT, ImportMode::Replace).unwrap();
        assert_eq!(report.imported, 2);
        assert_eq!(report.skipped, 1, "the bare 42 is junk");
        assert_eq!(ideas[0].text, "from the browser");
        assert_eq!(ideas[0].buckets, vec!["Work"], "cleaned and deduped");
        assert_eq!(ideas[1].text, "legacy bare entry");
        assert!(!ideas[1].id.is_empty(), "normalize fills the id");
        assert!(ideas[1].created > 0, "normalize fills the date");
        assert!(crate::model::PALETTE.contains(&ideas[1].color.as_str()));
    }

    #[test]
    fn merge_appends_only_new_ids() {
        let mut ideas = fresh();
        import(&mut ideas, V1_EXPORT, ImportMode::Replace).unwrap();
        let before = ideas.len();

        let payload = format!(
            r##"[{{ "id": "{}", "text": "duplicate id", "color": "#f0b429", "created": 1, "lastViewed": 1, "buckets": [], "source": "" }},
               {{ "text": "genuinely new" }}]"##,
            ideas[0].id
        );
        let report = import(&mut ideas, &payload, ImportMode::Merge).unwrap();
        assert_eq!(report.imported, 1, "only the genuinely new idea is merged");
        assert_eq!(ideas.len(), before + 1);
        assert_eq!(ideas.last().unwrap().text, "genuinely new");
        assert_eq!(ideas[0].text, "from the browser", "existing ideas stay first");
    }

    #[test]
    fn merge_into_populated_store_keeps_existing_order() {
        let mut ideas = fresh();
        import(&mut ideas, r#"[{"text": "existing"}]"#, ImportMode::Replace).unwrap();
        import(&mut ideas, r#"[{"text": "new-1"}, {"text": "new-2"}]"#, ImportMode::Merge).unwrap();
        let texts: Vec<&str> = ideas.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(texts, vec!["existing", "new-1", "new-2"]);
    }

    #[test]
    fn rejects_non_json_and_non_array_payloads() {
        let mut ideas = fresh();
        assert_eq!(import(&mut ideas, "not json", ImportMode::Replace), Err(ImportError::NotJson));
        assert_eq!(
            import(&mut ideas, r#"{"just": "an object"}"#, ImportMode::Replace),
            Err(ImportError::NotAnArray)
        );
        assert!(ideas.is_empty(), "failed imports change nothing");
    }

    #[test]
    fn import_then_export_round_trips() {
        let mut ideas = fresh();
        import(&mut ideas, V1_EXPORT, ImportMode::Replace).unwrap();
        let out = crate::export::export_json(&ideas);
        let mut again = fresh();
        import(&mut again, &out, ImportMode::Replace).unwrap();
        assert_eq!(ideas, again);
    }
}
