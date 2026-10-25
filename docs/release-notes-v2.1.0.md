# Rusty Box v2.1.0 — the rebrand and UI redesign

**Rusty Box** is the new name of The Idea Bucket — same app, same data, same
quiet promise: drop a thought in, reach back in for old ones, and nothing
ever leaves your machine. This release rebrands the product and rebuilds its
interface on a real design system, with two designed themes.

## Breaking changes: none — data and URLs are fully compatible

- Your store file works as-is. If you ran v2.0.0, your `data/idea-bucket.json`
  remains the default store; fresh installs create `data/rusty-box.json`.
  No migration, no format change.
- Every v1/v2 route and query parameter still works: `/?q=…`, `/?bucket=…`,
  `/idea/<id>`, `/idea/<id>?edit=1`, `/drop`, `/buckets/delete`, `/dip`,
  `/export/json`, `/export/md`, `POST /import`, `/healthz`. New additions
  (`/buckets/rename`, `/import/confirm`, `/static/*`) are purely additive.
- The `IDEABUCKET_HOST/PORT/DATA` environment variables are still honored.
  The new `RUSTYBOX_*` names are primary; when both are set, the Rusty Box
  name wins. The boot log tells you which name supplied each value.
- A v1.2.0 browser export (`idea-bucket.json`) imports unchanged — ids,
  colors, timestamps, tins and source lines round-trip byte-faithfully, and
  a Rusty Box export pastes straight back into the v1 browser app.

## What's new

- **Two designed themes.** "Daylight on the workbench" (light) and
  "lamp-lit cellar" (dark) are separate token sets — the dark theme is not an
  inversion: surfaces go warm-black, the accent cools to embers, and a lamp
  of old gold survives from v1. Pick Daylight or Lamplight in the colophon
  (or `?theme=dark`); the choice is remembered for a year, and the system
  preference applies until you do. Contrast is verified against WCAG floors
  (body ≥ 7:1, secondary ≥ 4.5:1, interactive ≥ 3:1) — 26/26 pairs pass in
  both themes, computed mechanically from the token file.
- **A real design-token system.** `templates/tokens.css` is the single
  source of truth: five palette ramps (paper, oak, rust, patina, iron), type
  and space scales, motion tokens, and the semantic aliases both themes
  consume. No magic hex values downstream. See `docs/ui-redesign.md` for the
  full record.
- **The box itself, on the stage.** The glass of v2.0 became an open wooden
  box whose pile of paper balls rises with your count. The old bucket
  vocabulary became tins: a rail of tins with per-tin rename and delete,
  a slot composer with filing chips and the optional "from:" line, cards and
  plates, print and reduced-motion rules — all CSS, still zero JavaScript.
- **Tin rename.** Rename a tin from its rail disclosure
  (`POST /buckets/rename`). The rename is position-preserving (an idea's own
  tin order never shifts), deduplicates, counts no-ops, and refuses blank
  names. Deleting a tin still only unfiles its ideas.
- **Two-step destructive import.** Replace no longer acts on the first
  submit: it shows a guard page ("Empty the box and replace everything?")
  with the counts, and nothing changes until you confirm. Merge stays
  immediate.
- **Styled dead ends.** A 404 page that says what happened ("Nothing here
  but the smell of oil") with a way back on every page. If the data file is
  corrupt at boot, it is quarantined to `<file>.corrupt-backup` and the
  front page says so, with the backup path, until you restore it.
- **Honest configuration.** Host, port and data path resolve in one audited
  module; blank values count as unset; a bad port error names the exact
  variable consulted; the boot log prints each value's provenance.
- **In-binary assets.** The favicon and social-card image are compiled into
  the binary and served from `/static/` — no asset files to lose, no network
  to fetch them from. System font stacks only: nothing to download, ever.

## Quality

- `cargo test --workspace`: **106 tests** (43 core, 27 server, 36
  integration), including a zero-`<script>` sweep over every served page,
  a theme drift check, and the two-step import guard.
- `cargo fmt --check` and `cargo clippy -D warnings` clean; `unsafe` remains
  forbidden workspace-wide.
- The 34-step smoke suite passes against the release binary: capture, search,
  filters, detail/edit/delete, tin delete (unfiles, never deletes), tin
  rename, dip, JSON and Markdown exports, import merge/replace/failure, 404,
  corrupt-store quarantine, and no-`<script>`/no-external-URL hygiene.
- The old single-file app and its Android wrapper remain frozen under
  `legacy/`, untouched.

## Upgrade notes

Pull the tag, run `cargo run -p rustybox-server` (or drop the release binary
in place of the old one). Your data file is picked up where it is; your old
service files keep working; your bookmarks keep working.

— Rusty Box v2.1.0, "Drop a thought in. Reach back in for old ones."
