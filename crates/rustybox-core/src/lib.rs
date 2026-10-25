//! # rustybox-core
//!
//! Domain model and logic for **The Idea Bucket** (راستی) — the pure-Rust v2
//! rewrite of the single-file web app frozen under `legacy/index.html`.
//!
//! This crate owns everything that makes the app *the app*: the `Idea` record
//! (shaped exactly like v1's `idea-bucket-v1` localStorage payload so exports
//! stay interchangeable), bucket naming and coloring rules, the JSON file
//! store with atomic saves and corrupt-payload quarantine, search/filtering,
//! the random "dip", and the JSON/Markdown exporters.
//!
//! It deliberately depends on nothing but `serde`, `serde_json`, `uuid`,
//! `rand` and `chrono`, so any future UI (server, desktop, mobile shell) can
//! reuse it unchanged.

pub mod buckets;
pub mod export;
pub mod import;
pub mod model;
pub mod store;

pub use buckets::{bucket_color, bucket_index, clean_bucket_name, BucketEntry};
pub use export::{export_json, export_markdown};
pub use import::{import, ImportError, ImportMode, ImportReport};
pub use model::{
    new_idea, normalize_idea, take_chars, validate_text, Idea, TextError, CAPTURE_MAX, EDIT_MAX,
};
pub use store::{EditError, EditReport, Filter, Store};
