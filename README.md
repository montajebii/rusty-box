# The Idea Bucket

Drop a thought in. Dip for old ones.

A minimal idea-capture app: type an idea, drop it into a bucket, and dip back
into old ones later. Ideas live in buckets, can be in several at once, and can
be searched, edited, exported as JSON or Markdown, and imported again.

Since **v2.0.0** the entire application is **pure Rust** — a Cargo workspace
with one domain library and one web server binary. The UI is fully
server-rendered and ships **zero JavaScript**; every interaction is a link or
a form. The v1 single-file web app and its Capacitor Android wrapper are
frozen under [`legacy/`](legacy/) and still build from there.

## Version

Current release: **v2.0.0** — see [`RELEASES.md`](RELEASES.md).

The version is tracked in the workspace `Cargo.toml`
(`[workspace.package] version = "2.0.0"`) and both crates inherit it.

## Architecture

```
crates/
  ideabucket-core      domain: Idea model (v1-compatible JSON shape), bucket
                       rules, JSON file store (atomic saves, corrupt-file
                       quarantine), search, dip, JSON/Markdown export, import
  ideabucket-server    axum web server: server-rendered askama templates,
                       flash toasts, deep links, exports/imports
docs/
  rust-rewrite-plan.md the full rewrite plan: goals, parity table, decisions
legacy/                frozen v1.2.0: index.html, Capacitor/Android wrapper,
                       jsdom test suite, build scripts
```

Design highlights:

- **No JavaScript anywhere.** v1's composer, drawer, idea page and toasts all
  became plain HTML forms and links. Filters travel in the query string, so
  any view — including search results — is a bookmarkable URL.
- **Drop-in data compatibility.** The store file is the same flat JSON array
  v1 kept in localStorage and exported, so you can migrate from the browser
  app by pasting an existing `idea-bucket.json` export into the import form.
- **Ported algorithms.** Bucket colors, the glass liquid level, blob layout
  and export formats are exact ports of v1 (see `docs/rust-rewrite-plan.md`).

## Quick start

```bash
cargo run -p ideabucket-server
# → http://127.0.0.1:8088
```

Configuration (all optional, via environment):

| variable          | default                 | meaning                       |
|-------------------|-------------------------|-------------------------------|
| `IDEABUCKET_HOST` | `127.0.0.1`             | bind address (local by default — there is no auth) |
| `IDEABUCKET_PORT` | `8088`                  | bind port                     |
| `IDEABUCKET_DATA` | `data/idea-bucket.json` | the JSON store file           |

The store is created on first save. A corrupt data file is renamed to
`<file>.corrupt-backup` before a fresh one starts — the same quarantine the
browser app applies to a damaged localStorage payload.

## Tests

Both crates carry a full suite, run on every push in the `rust` job of the
Build workflow:

```bash
cargo test --workspace
```

| Suite | What it covers |
|---|---|
| `ideabucket-core` unit tests (39) | text validation and limits, legacy-entry normalization, bucket naming/dedup/color-hash vectors, index ordering, search over text and buckets, seeded dip, atomic save round-trip, corrupt-payload quarantine (JSON and non-array), junk-entry filtering, exact JSON/Markdown export shapes, import replace/merge/dedup and error cases |
| `ideabucket-server` unit tests (13) | glass geometry (`levelY`, surface, blob formulas), date/age lines, URL encoding, flash-code resolution, poisoned-mutex recovery |
| `ideabucket-server/tests/integration.rs` (25) | every route's happy and failure paths driven through the real router in-process: capture (empty, too-long, filing, active-bucket fallback), XSS escaping, search and bucket filters, idea page + edit flows, delete, bucket delete (unfiles but never deletes), dip (pool-aware), export attachments, v1-export import (merge/replace/failures), flash codes, corrupt-store boot |

Quality gates in CI: `cargo fmt --check` and `cargo clippy -D warnings`
(`unsafe` is forbidden workspace-wide) alongside the test run. The legacy v1
suite (`npm test` in `legacy/`) and the APK pipeline keep running in their own
jobs, unchanged.

## Migrating from the browser app (v1)

1. Open the v1 app, press **⬇ JSON**, and save `idea-bucket.json`.
2. Start the server, expand **⬆ Import a JSON export** on the main page.
3. Paste the file, choose **merge** (keep existing) or **replace**, import.

Ideas come across with ids, colors, dates, buckets and source lines intact;
entries that are incomplete (bare `{"text": …}` from old exports) are
normalized on the way in.

## Legacy v1 (web + Android APK)

`legacy/` contains the frozen v1.2.0 app exactly as it shipped: the
single-file `index.html`, the Capacitor Android project, the jsdom suite
(80 tests) and the build/check scripts. `legacy/README.md` (see the v1
sections below in git history) documents the APK pipeline; CI still builds
the APK on every push. The v1 release pipeline fires on `v1.*` tags only —
Rust releases are ordinary tags on this repository.

## Repository layout

- `Cargo.toml` — workspace: shared dependencies, lints (`unsafe` forbidden), release profile (thin LTO, stripped)
- `crates/ideabucket-core/` — domain library, no framework dependencies
- `crates/ideabucket-server/` — the web binary (`ideabucket-server`)
- `docs/rust-rewrite-plan.md` — the rewrite plan and feature-parity table
- `legacy/` — frozen v1 web/Android app
- `.github/workflows/build.yml` — Rust + legacy web + APK jobs on every push
- `.github/workflows/release.yml` — legacy APK releases for `v1.*` tags

## Tech

- Rust (stable), edition 2021, `unsafe` forbidden workspace-wide
- Server: axum 0.8, tokio, askama (compile-time templates with auto-escaping)
- Core: serde/serde_json, uuid, rand, chrono — nothing else
- Storage: one JSON file, atomic writes (temp + rename), quarantine on corruption
