# Rust Rewrite Plan — The Idea Bucket v2.0.0

The Idea Bucket (راستی) is today a single-file HTML/CSS/JS app (`index.html`, v1.2.0)
wrapped in Capacitor for Android. This plan rewrites the entire application in
**pure Rust**: every line of application logic, rendering, persistence, search,
export and import lives in Rust. No JavaScript ships to the client — the UI is
fully server-rendered, so the app stays a browser app with the same look and
feel, while the whole stack becomes one small Rust binary.

Repository note: this repo is called **rusty-box**. It was always going to end
up here.

## Goals

1. **Feature parity** with v1.2.0 for everything that matters in the data model
   and workflows (capture, buckets, filing, search, dip, edit, export, deep links).
2. **Pure Rust stack** — a Cargo workspace, one core library, one web server
   binary, zero client-side JavaScript.
3. **A complete test suite from day one** — the v1 jsdom suite found 8 real bugs;
   v2 gets the same rigor with `cargo test` (unit + HTTP integration tests).
4. **Best-practice GitHub flow** — feature branch, conventional commits, CI gate,
   pull request, no direct pushes to `main`, no secrets in the repo.

## Non-goals (v2.0.0)

- Android APK: Capacitor is inherently Java/Gradle/JS-glue, which contradicts
  "pure Rust". The v1 app is preserved under `legacy/` and can still ship APKs.
  A future path to mobile/desktop reuses `ideabucket-core` (e.g. egui or Tauri
  shell) — the domain crate is UI-agnostic by design.
- Drop/keyboard animations (`visualViewport` work): meaningless without
  client-side JS. CSS animations cover what is free (handle bob, liquid pulse,
  card entrance).
- Multi-user auth: v1 is a personal, local-first app. The server binds
  `127.0.0.1` by default on purpose.

## Architecture

```
Cargo.toml                  # workspace: lints, profiles, shared deps
crates/
  ideabucket-core/          # domain: no framework deps, fully unit-testable
    src/model.rs            #   Idea, normalization, constants (palette, limits)
    src/buckets.rs          #   naming rules, color hashing, bucket index
    src/store.rs            #   JSON file store: atomic save, corrupt quarantine, ops, search, dip
    src/export.rs           #   JSON + Markdown exporters (v1-compatible formats)
    src/import.rs           #   JSON import (replace/merge) with validation
  ideabucket-server/
    src/main.rs             #   config (env), router, graceful shutdown
    src/state.rs            #   AppState = Mutex<Store>
    src/routes.rs           #   handlers
    templates/*.html        #   askama, compile-time checked, auto-escaped
```

Key decisions:

- **axum + askama, zero JS.** Server-rendered HTML; every interaction is a form
  POST or a link. Escaping is guaranteed by askama (v1 needed a hand-rolled
  `escapeHtml` — and bug #6 of the jsdom suite was exactly an escaping miss).
- **Storage is a JSON file** with the *same shape* as v1's `idea-bucket-v1`
  payload and v1's JSON export: a flat array of ideas. This makes browser →
  server migration a copy/paste of an existing export into the import form.
  Saves are atomic (temp file + rename); corrupt payloads are quarantined as
  `<file>.corrupt-backup`, porting v1's quarantine behavior.
- **Ported algorithms, exactly.** Bucket colors reuse v1's palette and
  `h = (h*31 + code) % 9973` hash; the glass liquid level reuses
  `y = 325 − min(n,20)/20 × (325−150)`; blob placement formulas are the same.
- **`std::sync::Mutex`** around the store (no `.await` under lock), tokio
  multi-thread runtime for the network.

## Feature parity table

| v1 (index.html)                     | v2 (pure Rust)                                   |
|-------------------------------------|--------------------------------------------------|
| Capture ≤280 chars, Enter/drop btn  | `POST /drop` from composer form                  |
| Multi-bucket ideas, "file into"     | Composer + editor bucket checkboxes, new-bucket field |
| Bucket index (count sort, colors)   | Ported `bucket_index`, same palette + hash       |
| Bucket delete only unfiles ideas    | `POST /buckets/delete` — same guarantee          |
| Search over text + bucket names     | `GET /?q=…&bucket=…` (bookmarkable, shares glass) |
| Dip for inspiration (random, pool-aware, lastViewed) | `GET /dip` → redirect to `/idea/<id>` |
| Idea page: view / edit ≤2000 / delete | `GET /idea/<id>`, `POST /idea/<id>/edit`, `POST /idea/<id>/delete` |
| Deep links `/idea/<id>`, unknown bounces | Same route; unknown → flash + redirect to `/` |
| Export JSON (pretty, same shape)    | `GET /export/json`                               |
| Export Markdown (sections, "also in", unfiled) | `GET /export/md`, locale-neutral ISO dates |
| Corrupt-storage quarantine          | Corrupt-file quarantine (`.corrupt-backup`)      |
| Edit/empty-text guards, no silent loss | Same rules enforced server-side, validated in tests |
| — (v1 had no import)                | **New:** `POST /import` (paste exported JSON; replace or merge) — doubles as the browser→server migration path |
| Glass: level, blobs, counter, drip  | Same SVG, server-computed                        |
| Persian name راستی                  | Kept in the UI footer                            |

## Test plan (the "complete test suite" carried into v2)

- **core unit tests:** normalization (legacy entries, junk buckets, dedup,
  defaults), bucket naming rules, color hash vectors, index ordering,
  search/filter, dip determinism with seeded RNG, export formats (exact string
  fixtures), import (replace/merge, invalid payloads), atomic save, corrupt
  quarantine, empty-text rejection, 280/2000 limits.
- **server integration tests** (`tower::ServiceExt::oneshot`, no sockets):
  every route's happy path + failure paths, content types, attachment
  headers on exports, XSS escaping (idea text containing `<script>` must render
  escaped), 404/redirect behavior, full add → file → search → edit → dip →
  delete lifecycle, import round-trip.
- **CI gate:** `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.

## GitHub interaction practices (applied throughout)

1. Work happens on `feat/rust-rewrite`, never directly on `main`.
2. The v1 freeze is one isolated mechanical commit (`git mv`, no content edits)
   so review of the rewrite commits stays readable.
3. Conventional commit messages (`feat:`, `test:`, `ci:`, `docs:`, `chore:`).
4. The PR — not a push — is the only path to `main`; CI must be green first.
5. No secrets: credentials are used ephemerally for the push, never written to
   `.git/config`, files, or commits; the previously exposed PAT must be rotated.

## Steps

1. **Branch** `feat/rust-rewrite` off `main`; commit this plan.
2. **Freeze v1**: move `index.html`, `package.json`, `package-lock.json`,
   `capacitor.config.json`, `android/`, `tests/`, and the build scripts into
   `legacy/`; point the existing CI jobs at `legacy/` so v1 keeps building.
3. **Workspace scaffold**: root `Cargo.toml` (workspace lints: forbid unsafe,
   deny warnings in CI; release profile: thin LTO, stripped symbols), crate
   skeletons, `target/` in `.gitignore`.
4. **`ideabucket-core`**: model + buckets + store + export + import, with the
   full unit suite green.
5. **`ideabucket-server`**: config, state, routes, askama templates, zero-JS
   UI port of the v1 design (dark navy + gold, the bucket glass, chips, cards),
   with the HTTP integration suite green.
6. **Gate**: `fmt --check`, `clippy -D warnings`, `cargo test` — fix every
   finding until the whole workspace is clean.
7. **CI + docs**: a Rust job in `build.yml` (fmt/clippy/test with dep
   caching) beside the legacy jobs; README rewritten for v2 with a legacy
   section; `RELEASES.md` records v2.0.0 as a breaking change with migration
   notes.
8. **Commit history**: one conventional commit per logical step.
9. **Push + PR**: push the branch with ephemeral credentials, open the pull
   request with a full description (architecture, parity table, migration),
   let CI verify, leave the merge to the maintainer.
