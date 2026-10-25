# Rusty Box

Drop a thought in. Reach back in for old ones.

A quiet place for your ideas: type a thought, drop it in the box, file it into
tins, and reach back in for old ones later. Ideas can sit in several tins at
once, can be searched, edited, renamed, exported as JSON or Markdown, and
imported again. One idea, one JSON file, one binary — nothing ever leaves the
machine it runs on.

Since **v2.0.0** the entire application is **pure Rust** — a Cargo workspace
with one domain library and one web server binary. The UI is fully
server-rendered and ships **zero JavaScript**; every interaction is a link or
a form. **v2.1.0** is the rebrand: The Idea Bucket became Rusty Box, and the
interface was redesigned around a real design-token system with two designed
themes. The v1 single-file web app and its Capacitor Android wrapper are
frozen under [`legacy/`](legacy/) and still build from there.

## Version

Current release: **v2.1.0** — see [`RELEASES.md`](RELEASES.md) and
[`docs/release-notes-v2.1.0.md`](docs/release-notes-v2.1.0.md).

The version is tracked in the workspace `Cargo.toml`
(`[workspace.package] version = "2.1.0"`) and both crates inherit it.

## Architecture

```
crates/
  rustybox-core        domain: Idea model (v1-compatible JSON shape), tin
                       rules (incl. rename), JSON file store (atomic saves,
                       corrupt-file quarantine), search, dip, JSON/Markdown
                       export, import
  rustybox-server      axum web server: server-rendered askama templates,
                       token-driven CSS, two themes, flash toasts, deep
                       links, exports/imports, in-binary static assets
docs/
  ui-redesign.md       the v2.1.0 design system: tokens, contrast tables,
                       motion spec, CSS-only techniques, component inventory
  release-notes-v2.1.0.md  the published release notes
  rust-rewrite-plan.md the v2.0 rewrite plan: goals, parity table, decisions
legacy/                frozen v1.2.0: index.html, Capacitor/Android wrapper,
                       jsdom test suite, build scripts
```

Design highlights:

- **No JavaScript anywhere.** Every interaction is a link or a form; CSS-only
  interactivity uses `<details>`, `:checked` and `:has()`. Filters travel in
  the query string, so any view — including search results and a chosen
  theme — is a bookmarkable URL. The zero-JS rule is enforced by tests.
- **Drop-in data compatibility.** The store file and both export formats keep
  v1's exact payload shape (`id/text/color/created/lastViewed/buckets/source`,
  camelCase). A v1 `idea-bucket.json` export pastes straight into the import
  box, and a Rusty Box export pastes back into the v1 browser app.
- **Two designed themes.** "Daylight on the workbench" and "lamp-lit cellar"
  are token sets, not filter inversions — the dark theme cools the accent to
  embers and keeps one v1 keepsake: a lamp of old gold. Contrast is verified
  against WCAG ratios (body ≥ 7:1, secondary ≥ 4.5:1, interactive ≥ 3:1);
  see `docs/ui-redesign.md`.

## Quick start

```bash
cargo run -p rustybox-server
# → http://127.0.0.1:8088
```

Configuration (all optional, via environment). The `RUSTYBOX_*` names are
primary; the pre-rename `IDEABUCKET_*` names are still honored so existing
service files keep working. The boot log states exactly which name supplied
each value.

| variable          | default                         | meaning                       |
|-------------------|---------------------------------|-------------------------------|
| `RUSTYBOX_HOST`   | `127.0.0.1`                     | bind address (local by default — there is no auth) |
| `RUSTYBOX_PORT`   | `8088`                          | bind port                     |
| `RUSTYBOX_DATA`   | `data/rusty-box.json`           | the JSON store file           |

Notes on the data path: on a fresh install the store is created as
`data/rusty-box.json` on first save. If a v2.0 store already exists at
`data/idea-bucket.json`, it is picked up as the default so the rename never
orphans anyone's ideas. `IDEABUCKET_HOST/PORT/DATA` work exactly like their
`RUSTYBOX_*` counterparts (the Rusty Box name wins when both are set).

The store is created on first save. A corrupt data file is renamed to
`<file>.corrupt-backup` before a fresh one starts — the same quarantine the
browser app applies to a damaged localStorage payload — and the front page
announces it, with the backup path, until you move the file back.

## Tests

Both crates carry a full suite, run on every push in the `rust` job of the
Build workflow:

```bash
cargo test --workspace   # 106 tests
```

| Suite | What it covers |
|---|---|
| `rustybox-core` unit tests (43) | text validation and limits, legacy-entry normalization, tin naming/dedup/color-hash vectors, position-preserving tin rename (dedup, counted no-ops), index ordering, search over text and tins, seeded dip, atomic save round-trip, corrupt-payload quarantine with the reported backup path, junk-entry filtering, exact JSON/Markdown export shapes, import replace/merge/dedup and error cases |
| `rustybox-server` unit tests (27) | glass→box geometry, date/age lines, URL encoding, flash kind+text resolution, theme-query/cookie resolution and the cookie contract, config resolution matrix (defaults, primary/legacy precedence, blank-as-unset, bad-port variable naming, data-path fallback), poisoned-mutex recovery, quarantine notice plumbing |
| `rustybox-server/tests/integration.rs` (36) | every route's happy and failure paths driven through the real router in-process: capture (empty, too-long, filing, new tins, active-tin fallback), XSS escaping, search and tin filters, tin rename and delete (unfiles but never deletes), idea page + edit flows, delete, dip (pool-aware), exports as attachments, v1-export import, the two-step destructive replace (guard page, then confirm), theme override/cookie, styled 404, static SVG assets, quarantine notice, and a zero-`<script>` sweep over every served page |

Quality gates in CI: `cargo fmt --check` and `cargo clippy -D warnings`
(`unsafe` is forbidden workspace-wide) alongside the test run. The legacy v1
suite (`npm test` in `legacy/`) and the APK pipeline keep running in their own
jobs, unchanged.

## Migrating from the browser app (v1)

1. Open the v1 app, press **⬇ JSON**, and save `idea-bucket.json`.
2. Start the server, expand **Take ideas out / bring ideas in** on the main page.
3. Paste the file, choose **merge** (keep existing) or **replace**, import.
   Replace asks twice before it empties the box.

Ideas come across with ids, colors, dates, tins and source lines intact;
entries that are incomplete (bare `{"text": …}` from old exports) are
normalized on the way in. Nothing else changes: old URLs still work
(`/?q=…`, `/?bucket=…`, `/idea/<id>`, `/export/json`, `/export/md`,
`POST /import`), and the env vars under their old `IDEABUCKET_*` names are
still honored.

## Legacy v1 (web + Android APK)

`legacy/` contains the frozen v1.2.0 app exactly as it shipped: the
single-file `index.html`, the Capacitor Android project, the jsdom suite
(80 tests) and the build/check scripts. `legacy/README.md` documents the APK
pipeline; CI still builds the APK on every push. The v1 release pipeline
fires on `v1.*` tags only — Rust releases are ordinary tags on this
repository.

## Repository layout

- `Cargo.toml` — workspace: shared dependencies, lints (`unsafe` forbidden), release profile (thin LTO, stripped)
- `crates/rustybox-core/` — domain library, no framework dependencies
- `crates/rustybox-server/` — the web binary (`rustybox-server`), templates and token CSS
- `docs/ui-redesign.md` — the v2.1.0 design system document
- `docs/rust-rewrite-plan.md` — the rewrite plan and feature-parity table
- `legacy/` — frozen v1 web/Android app
- `.github/workflows/build.yml` — Rust + legacy web + APK jobs on every push
- `.github/workflows/release.yml` — legacy APK releases for `v1.*` tags

## Tech

- Rust (stable), edition 2021, `unsafe` forbidden workspace-wide
- Server: axum 0.8, tokio, askama (compile-time templates with auto-escaping)
- Core: serde/serde_json, uuid, rand, chrono — nothing else
- Fonts: system stacks only (`Iowan Old Style`/`Palatino` for display,
  system-ui for body, `ui-monospace` for code) — nothing to fetch, nothing
  to license
