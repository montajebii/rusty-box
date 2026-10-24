//! The JSON file store: load, atomic save, and every mutation the app can
//! perform. This is v1's localStorage logic (`load`/`save`, quarantine,
//! `addIdea`, `commitEdit`, delete, `deleteBucketFlow`, `visibleIdeas`, `dip`)
//! expressed over a file instead of a storage key.

use std::io;
use std::path::{Path, PathBuf};

use rand::Rng;

use crate::buckets::{clean_bucket_name, in_bucket};
use crate::model::{new_idea, normalize_idea, now_ms, validate_text, Idea, CAPTURE_MAX, EDIT_MAX};

/// Which ideas a view (the glass, the list, a dip) looks at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    /// Exact bucket name, or `None` for "every bucket" (v1's `activeBucket`).
    pub bucket: Option<String>,
    /// Case-insensitive substring over idea text and bucket names
    /// (v1's drawer search).
    pub q: Option<String>,
}

impl Filter {
    pub fn all() -> Self {
        Filter::default()
    }

    pub fn with_bucket(name: impl Into<String>) -> Self {
        Filter {
            bucket: Some(name.into()),
            q: None,
        }
    }
}

/// What `Store::edit` actually changed, so the UI can say "Idea updated" vs
/// "Buckets updated" exactly like v1's `commitEdit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditReport {
    pub text_changed: bool,
    pub buckets_changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    NotFound,
    Text(crate::model::TextError),
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditError::NotFound => write!(f, "Idea not found"),
            EditError::Text(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for EditError {}

/// The whole app state: a flat array of ideas (newest first, like v1's
/// `unshift`) backed by one JSON file.
pub struct Store {
    pub ideas: Vec<Idea>,
    path: PathBuf,
}

impl Store {
    /// An empty store at `path` (no file is touched until `save`).
    pub fn empty(path: impl Into<PathBuf>) -> Self {
        Store {
            ideas: Vec::new(),
            path: path.into(),
        }
    }

    /// Load the store from disk.
    ///
    /// Missing file → empty store (first run). A payload that is not valid
    /// JSON, or not an array, is **quarantined** — renamed to
    /// `<file>.corrupt-backup` — before starting fresh, so a truncated write
    /// or manual edit can never silently destroy the user's data. That is the
    /// port of v1's `quarantineStorage` (whose absence of a backup was the
    /// first bug the jsdom suite found and fixed).
    ///
    /// Individual entries that are not ideas (missing `text`, wrong types) are
    /// dropped, like v1's `data.filter(i => i && typeof i.text === 'string')`;
    /// surviving entries are normalized.
    pub fn load(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Store::empty(path)),
            Err(e) => return Err(e),
        };

        if raw.trim().is_empty() {
            return Ok(Store::empty(path));
        }

        let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&raw);
        let values = match parsed {
            Ok(values) => values,
            Err(_) => {
                quarantine(&path)?;
                return Ok(Store::empty(path));
            }
        };

        let mut ideas = Vec::with_capacity(values.len());
        for value in values {
            if let Ok(mut idea) = serde_json::from_value::<Idea>(value) {
                normalize_idea(&mut idea);
                ideas.push(idea);
            }
        }
        Ok(Store { ideas, path })
    }

    /// Atomically write the store: serialize, write to `<file>.tmp` in the
    /// same directory, fsync, then rename over the target. A crash mid-save
    /// leaves the previous file intact, never a half-written one.
    pub fn save(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let body = serde_json::to_string_pretty(&self.ideas)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let tmp = tmp_path(&self.path);
        {
            use std::io::Write;
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(body.as_bytes())?;
            f.write_all(b"\n")?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Where this store persists (`IDEABUCKET_DATA`).
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get(&self, id: &str) -> Option<&Idea> {
        self.ideas.iter().find(|i| i.id == id)
    }

    /// Capture an idea (v1's `addIdea`): validate, build, unshift to the
    /// front. Bucket names are cleaned; an empty-name-only list stays empty
    /// (the idea lives in the inbox).
    pub fn add(
        &mut self,
        text: &str,
        buckets: &[String],
        source: &str,
    ) -> Result<&Idea, crate::model::TextError> {
        let text = validate_text(text, CAPTURE_MAX)?;
        let mut cleaned: Vec<String> = Vec::with_capacity(buckets.len());
        for b in buckets {
            let name = clean_bucket_name(b);
            if !name.is_empty() && !cleaned.contains(&name) {
                cleaned.push(name);
            }
        }
        let idea = new_idea(text, cleaned, source);
        self.ideas.insert(0, idea);
        Ok(&self.ideas[0])
    }

    /// Edit an idea (v1's `commitEdit`): new text (validated) and new bucket
    /// list. When nothing actually changed, the idea is left untouched —
    /// v1 exits without marking `edited` — and the report says so.
    pub fn edit(
        &mut self,
        id: &str,
        text: &str,
        buckets: &[String],
    ) -> Result<EditReport, EditError> {
        let idea = self
            .ideas
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(EditError::NotFound)?;
        let text = validate_text(text, EDIT_MAX).map_err(EditError::Text)?;

        let mut kept: Vec<String> = Vec::with_capacity(buckets.len());
        for b in buckets {
            let name = clean_bucket_name(b);
            if !name.is_empty() && !kept.contains(&name) {
                kept.push(name);
            }
        }

        let text_changed = text != idea.text;
        let buckets_changed = kept != idea.buckets;
        if !text_changed && !buckets_changed {
            return Ok(EditReport {
                text_changed: false,
                buckets_changed: false,
            });
        }

        let now = now_ms();
        idea.text = text;
        idea.buckets = kept;
        idea.edited = Some(now);
        idea.last_viewed = now;
        Ok(EditReport {
            text_changed,
            buckets_changed,
        })
    }

    /// Delete an idea by id; returns whether it existed.
    pub fn delete(&mut self, id: &str) -> bool {
        let before = self.ideas.len();
        self.ideas.retain(|i| i.id != id);
        self.ideas.len() != before
    }

    /// Delete a bucket: every idea stops being filed under `name`, none is
    /// ever deleted with it (v1's `deleteBucketFlow` guarantee). Returns how
    /// many ideas were members.
    pub fn delete_bucket(&mut self, name: &str) -> usize {
        let mut members = 0;
        for idea in &mut self.ideas {
            if in_bucket(idea, name) {
                members += 1;
                idea.buckets.retain(|b| b != name);
            }
        }
        members
    }

    /// The ideas a view sees, in list order (newest first).
    pub fn visible(&self, filter: &Filter) -> Vec<&Idea> {
        let q = filter.q.as_deref().map(str::trim).filter(|q| !q.is_empty());
        self.ideas
            .iter()
            .filter(|i| {
                if let Some(bucket) = &filter.bucket {
                    if !in_bucket(i, bucket) {
                        return false;
                    }
                }
                match q {
                    None => true,
                    Some(q) => {
                        let q = q.to_lowercase();
                        i.text.to_lowercase().contains(&q)
                            || i.buckets.iter().any(|b| b.to_lowercase().contains(&q))
                    }
                }
            })
            .collect()
    }

    /// Dip for inspiration (v1's `dip`): a random idea from what you are
    /// looking at, with `lastViewed` refreshed. Uses the injectable RNG so
    /// tests can seed it.
    pub fn dip(&mut self, filter: &Filter, rng: &mut impl Rng) -> Option<&Idea> {
        let pool = self.visible(filter);
        if pool.is_empty() {
            return None;
        }
        let pick = rng.random_range(0..pool.len());
        let id = pool[pick].id.clone();
        let idx = self.ideas.iter().position(|i| i.id == id)?;
        self.ideas[idx].last_viewed = now_ms();
        self.ideas.get(idx)
    }

    /// Record that an idea page was opened (v1's `openIdeaPage` touches
    /// `lastViewed`). Returns false for an unknown id.
    pub fn touch_viewed(&mut self, id: &str) -> bool {
        match self.ideas.iter_mut().find(|i| i.id == id) {
            Some(idea) => {
                idea.last_viewed = now_ms();
                true
            }
            None => false,
        }
    }
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(".tmp");
    PathBuf::from(s)
}

/// Rename a corrupt payload out of the way before starting fresh.
fn quarantine(path: &Path) -> io::Result<()> {
    let mut backup = path.as_os_str().to_os_string();
    backup.push(".corrupt-backup");
    let backup = PathBuf::from(backup);
    // v1's setItem overwrites an old backup; mirror that.
    if backup.exists() {
        std::fs::remove_file(&backup)?;
    }
    std::fs::rename(path, backup)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn store_with(ideas: &[(&str, &[&str])]) -> Store {
        let mut store = Store::empty("/tmp/does-not-matter.json");
        for (text, buckets) in ideas {
            let b: Vec<String> = buckets.iter().map(|s| s.to_string()).collect();
            store.add(text, &b, "").unwrap();
        }
        store
    }

    #[test]
    fn add_validates_and_unshifts_newest_first() {
        let mut store = store_with(&[("first", &[])]);
        store.add("second", &[], "").unwrap();
        assert_eq!(store.ideas[0].text, "second");
        assert_eq!(store.ideas[1].text, "first");
        assert!(matches!(
            store.add("   ", &[], ""),
            Err(crate::model::TextError::Empty)
        ));
    }

    #[test]
    fn add_cleans_bucket_names_and_dedups_exactly_like_v1() {
        // v1's normalizeIdea dedups case-SENSITIVELY (kept.includes(name));
        // case-insensitive reuse happens in the UI naming flow, not here.
        let mut store = Store::empty("/tmp/x.json");
        let idea = store
            .add("t", &[" A  b ".into(), "a b".into(), "  ".into()], "")
            .unwrap();
        assert_eq!(idea.buckets, vec!["A b", "a b"]);
        let idea = store.add("t2", &["A b".into(), "A b".into()], "").unwrap();
        assert_eq!(idea.buckets, vec!["A b"]);
    }

    #[test]
    fn edit_reports_changes_and_marks_edited() {
        let mut store = store_with(&[("hello", &["Work"])]);
        let id = store.ideas[0].id.clone();

        let no_change = store.edit(&id, "hello", &["Work".into()]).unwrap();
        assert!(!no_change.text_changed && !no_change.buckets_changed);
        assert!(
            store.ideas[0].edited.is_none(),
            "no-op edit must not mark edited"
        );

        let report = store
            .edit(&id, "hi", &["Work".into(), "Side".into()])
            .unwrap();
        assert!(report.text_changed && report.buckets_changed);
        assert_eq!(store.ideas[0].text, "hi");
        assert_eq!(store.ideas[0].buckets, vec!["Work", "Side"]);
        assert!(store.ideas[0].edited.is_some());

        assert_eq!(store.edit("nope", "x", &[]), Err(EditError::NotFound));
        assert!(matches!(
            store.edit(&id, "  ", &[]),
            Err(EditError::Text(crate::model::TextError::Empty))
        ));
    }

    #[test]
    fn edit_allows_filing_without_touching_text_like_the_close_guard() {
        // v1's closeWithGuard saves bucket-only changes keeping the old text.
        let mut store = store_with(&[("keep me", &[])]);
        let id = store.ideas[0].id.clone();
        let report = store.edit(&id, "keep me", &["Inbox".into()]).unwrap();
        assert!(!report.text_changed && report.buckets_changed);
        assert_eq!(store.ideas[0].text, "keep me");
        assert_eq!(store.ideas[0].buckets, vec!["Inbox"]);
    }

    #[test]
    fn delete_removes_only_that_idea() {
        let mut store = store_with(&[("a", &[]), ("b", &[])]);
        let id = store.ideas[0].id.clone();
        assert!(store.delete(&id));
        assert_eq!(store.ideas.len(), 1);
        assert!(!store.delete(&id));
    }

    #[test]
    fn delete_bucket_unfiles_but_never_deletes() {
        let mut store = store_with(&[("a", &["Work", "Side"]), ("b", &["Work"]), ("c", &[])]);
        let members = store.delete_bucket("Work");
        assert_eq!(members, 2);
        assert_eq!(store.ideas.len(), 3, "ideas must survive the bucket");
        // newest first: c, b, a — the two former Work members are b and a
        assert_eq!(store.ideas[0].buckets, Vec::<String>::new());
        assert_eq!(store.ideas[1].buckets, Vec::<String>::new());
        assert_eq!(store.ideas[2].buckets, vec!["Side"]);
        assert_eq!(store.delete_bucket("Work"), 0);
    }

    #[test]
    fn visible_filters_by_bucket_and_query() {
        let store = store_with(&[
            ("Rust pattern", &["Work"]),
            ("coffee thought", &["Side"]),
            ("rust & coffee", &[]),
        ]);
        let all = store.visible(&Filter::all());
        assert_eq!(all.len(), 3);

        let work = store.visible(&Filter::with_bucket("Work"));
        assert_eq!(work.len(), 1);
        assert_eq!(work[0].text, "Rust pattern");

        let q = store.visible(&Filter {
            bucket: None,
            q: Some("RUST".into()),
        });
        assert_eq!(q.len(), 2, "matches text case-insensitively");

        // a query can also find ideas by bucket name (v1 searches buckets too)
        let by_bucket_name = store.visible(&Filter {
            bucket: None,
            q: Some("side".into()),
        });
        assert_eq!(by_bucket_name.len(), 1);
        assert_eq!(by_bucket_name[0].text, "coffee thought");

        let both = store.visible(&Filter {
            bucket: Some("Work".into()),
            q: Some("coffee".into()),
        });
        assert!(both.is_empty());
    }

    #[test]
    fn dip_picks_from_the_visible_pool_and_touches_viewed() {
        let mut store = store_with(&[("a", &[]), ("b", &[]), ("c", &[])]);
        let mut rng = StdRng::seed_from_u64(42);
        let seen: Vec<String> = (0..20)
            .map(|_| {
                let idea = store.dip(&Filter::all(), &mut rng).unwrap();
                idea.id.clone()
            })
            .collect();
        // a seeded PRNG over 3 ideas will not land on the same one 20 times
        assert!(seen.iter().any(|id| *id != seen[0]));
        // dip out of a filtered empty pool fails
        assert!(store.dip(&Filter::with_bucket("Nope"), &mut rng).is_none());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("idea-bucket.json");
        let mut store = Store::empty(&path);
        store.add("round trip", &["Work".into()], "src").unwrap();
        let first_id = store.ideas[0].id.clone();
        store.edit(&first_id, "edited!", &["Work".into()]).unwrap();
        store.save().unwrap();

        let loaded = Store::load(&path).unwrap();
        assert_eq!(loaded.ideas.len(), 1);
        assert_eq!(loaded.ideas[0].text, "edited!");
        assert_eq!(loaded.ideas[0].buckets, vec!["Work"]);
        assert_eq!(loaded.ideas[0].source, "src");
    }

    #[test]
    fn load_missing_file_starts_empty() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::load(dir.path().join("nope.json")).unwrap();
        assert!(store.ideas.is_empty());
    }

    #[test]
    fn load_quarantines_corrupt_payload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("idea-bucket.json");
        std::fs::write(&path, "{not json at all").unwrap();

        let store = Store::load(&path).unwrap();
        assert!(store.ideas.is_empty());
        let backup = dir.path().join("idea-bucket.json.corrupt-backup");
        assert_eq!(
            std::fs::read_to_string(&backup).unwrap(),
            "{not json at all"
        );
        assert!(!path.exists(), "the corrupt file must be out of the way");
    }

    #[test]
    fn load_quarantines_non_array_payload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("idea-bucket.json");
        std::fs::write(&path, r#"{"id": "an object, not an array"}"#).unwrap();
        let store = Store::load(&path).unwrap();
        assert!(store.ideas.is_empty());
        assert!(dir.path().join("idea-bucket.json.corrupt-backup").exists());
    }

    #[test]
    fn load_drops_junk_entries_but_keeps_good_ones() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("idea-bucket.json");
        std::fs::write(
            &path,
            r#"[
                {"text": "keep me", "id": "x", "created": 1700000000000},
                {"nope": true},
                null,
                42,
                {"text": 5}
            ]"#,
        )
        .unwrap();
        let store = Store::load(&path).unwrap();
        assert_eq!(store.ideas.len(), 1);
        assert_eq!(store.ideas[0].text, "keep me");
        assert_eq!(store.ideas[0].id, "x");
        assert!(
            store.ideas[0].last_viewed > 0,
            "normalized dates are filled"
        );
    }

    #[test]
    fn save_is_atomic_no_tmp_left_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/idea-bucket.json");
        let mut store = Store::empty(&path);
        store.add("x", &[], "").unwrap();
        store.save().unwrap();
        store.save().unwrap(); // second save replaces the first atomically
        assert!(!path.with_extension("tmp").exists());
        assert!(!dir.path().join("nested/idea-bucket.json.tmp").exists());
        assert_eq!(Store::load(&path).unwrap().ideas.len(), 1);
    }

    #[test]
    fn touch_viewed_updates_only_that_idea() {
        let mut store = store_with(&[("a", &[]), ("b", &[])]);
        let id = store.ideas[1].id.clone();
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert!(store.touch_viewed(&id));
        assert!(store.ideas[1].last_viewed >= store.ideas[0].last_viewed);
        assert!(!store.touch_viewed("ghost"));
    }
}
