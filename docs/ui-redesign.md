# The Rusty Box UI redesign (v2.1.0)

This document is the design-system record for the v2.1.0 redesign: what the
interface looks like, why, and how every claim in it is checkable. The single
source of truth is [`crates/rustybox-server/templates/tokens.css`]
(../crates/rustybox-server/templates/tokens.css); every rule in
`style.css` consumes those custom properties, and the numbers below are
computed from that file, not eyeballed.

Companion documents: [`release-notes-v2.1.0.md`](release-notes-v2.1.0.md)
(what shipped) and [`rust-rewrite-plan.md`](rust-rewrite-plan.md) (how v2.0
happened).

## Principles

1. **One box, honest materials.** The app is an old wooden box with rusted
   metal straps. The palette is named for its materials — paper, oak, rust,
   patina, iron — and the UI behaves like them: surfaces are matte, shadows
   are soft contact shadows, the accent (rust) is the lever, not wallpaper.
2. **Tokens, not magic values.** Every color, radius, duration and shadow in
   `style.css` is a `var(--…)`. There is exactly one sanctioned exception
   (see "The one documented exception" below). A token rename is a refactor;
   a new hex value downstream is a bug.
3. **Two designed themes, not an inversion.** "Daylight on the workbench"
   (light) and "lamp-lit cellar" (dark) are sibling token sets. The dark
   theme is not light theme's colors flipped: surfaces go warm-black, the
   accent cools down to embers, and one v1 keepsake survives — a lamp of old
   gold (`--lamp: #f0b429`) kept for tiny glints in the dark.
4. **Zero JavaScript, by construction.** Every interaction is a link or a
   form. CSS-only interactivity uses `<details>` (disclosures, the
   destructive-action confirm), `:checked` (composer filing chips), and
   `:has()` where a parent must react. The toast is a server-resolved flash
   message that animates itself in and out; no script is involved, and an
   integration test sweeps every served page for `<script`.
5. **Accessible by default.** WCAG contrast targets are treated as floors
   (see the verified table), the skip link is the first tab stop, focus is
   always visible (`:focus-visible` ring in `--focus-ring`), touch targets
   are at least 44px in their interactive box, the toast is
   `role="status" aria-live="polite"`, the quarantine notice is
   `role="alert"`, and every animation dies under
   `prefers-reduced-motion: reduce`.
6. **Offline, always.** System font stacks only (`Iowan Old Style`/`Palatino`
   for display, `system-ui` for body, `ui-monospace` for code). Textures are
   inline SVG data URIs. Icons are inline SVG strokes. Favicon and social
   card are compiled into the binary. No external URL appears in any served
   page.

## Palette

### Primitives (`:root`, theme-independent)

| Ramp | 50 | 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900 |
|---|---|---|---|---|---|---|---|---|---|---|
| `--paper` | `#faf6ec` | `#f4eee0` | `#e9dfc9` | `#d9cbae` | `#c4b18c` | `#ab9570` | `#8f7a58` | `#73603f` | `#56462d` | `#3b2f1e` |
| `--oak` | `#f7f0e4` | `#ecdfc9` | `#dcc6a4` | `#c8a97c` | `#b28a58` | `#9a7040` | `#7f5a33` | `#654629` | `#4b341f` | `#322316` |
| `--rust` | `#fbf0e8` | `#f6dcc9` | `#eebb9f` | `#e2946c` | `#d4713f` | `#c05a28` | `#a4471d` | `#863716` | `#682a11` | `#4a1d0b` |
| `--patina` | `#eef5f0` | `#d8e9de` | `#b5d5c3` | `#8bbaa2` | `#639d82` | `#468168` | `#356753` | `#2a5041` | `#213b31` | `#172921` |
| `--iron` | `#f5efe4` | `#e9e1d4` | `#d5c7ae` | `#b5a48d` | `#94806b` | `#75604c` | `#5d4a39` | `#46372a` | `#33281c` | `#261e14` (+ `--iron-950: #191309`) |

Plus the v1 keepsake: `--lamp: #f0b429`.

Ramps are ten steps so a theme can pick ink, muted ink and hairlines from one
ramp without ever mixing ramp families; the semantic layer below is the only
thing components consume.

### Semantic tokens

| Token | Light ("daylight on the workbench") | Dark ("lamp-lit cellar") |
|---|---|---|
| `--bg` | `--paper-100` | `--iron-950` |
| `--surface` | `#fbf7ec` | `#221a11` |
| `--surface-raised` | `#fffdf6` | `#2a2014` |
| `--surface-sunken` | `#efe7d5` | `#191309` |
| `--border` | `--iron-200` | `#4a3a26` |
| `--border-strong` | `--iron-300` | `#63503a` |
| `--text` | `--iron-900` | `--paper-100` |
| `--text-muted` | `--iron-600` | `--iron-300` |
| `--text-faint` | `--iron-500` | `--iron-400` |
| `--accent` | `--rust-600` | `--rust-300` |
| `--accent-strong` | `--rust-700` | `--rust-200` |
| `--accent-contrast` | `#fff6e8` | `--iron-950` |
| `--accent-soft` | `--rust-50` | `rgba(226, 148, 108, 0.12)` |
| `--patina` / `--success` | `--patina-700` | `--patina-300` |
| `--danger` | `#8c2f18` | `#f0987d` |
| `--focus-ring` | `--rust-500` | `--rust-400` |
| `--toast-bg` / `--toast-text` | `--iron-900` / `--paper-100` | `--paper-100` / `--iron-950` |

Theme selection is server-rendered: `?theme=light|dark` (the colophon
picker) sets a one-year `rustybox_theme` cookie; until then the
`prefers-color-scheme` media query applies, through an auto-dark block kept
byte-equivalent to the `[data-theme="dark"]` block by a drift check in the
contrast verifier.

## Verified contrast

Computed from `tokens.css` (resolve `var()` chains, WCAG relative luminance,
`(L1+0.05)/(L2+0.05)`). Requirements: body text ≥ 7:1, secondary text ≥
4.5:1, interactive ≥ 3:1. All 26 pairs pass.

| Pair | Light | Dark | Requirement |
|---|---|---|---|
| body text (`--text`) on `--bg` | **14.20:1** | **15.95:1** | 7:1 |
| body text on `--surface` | **15.35:1** | **14.83:1** | 7:1 |
| muted text on `--bg` | **7.25:1** | **7.61:1** | 4.5:1 |
| muted text on `--surface` | **7.83:1** | **7.08:1** | 4.5:1 |
| faint text on `--surface` | **5.55:1** | **4.54:1** | 4.5:1 |
| accent as text on `--bg` | **5.19:1** | **7.63:1** | 4.5:1 |
| accent-strong on `--bg` | **7.06:1** | **10.76:1** | 4.5:1 |
| `--accent-contrast` on accent fill | **5.60:1** | **7.63:1** | 4.5:1 |
| patina text on `--surface` | **8.45:1** | **7.88:1** | 4.5:1 |
| danger text on `--surface` | **7.74:1** | **7.76:1** | 4.5:1 |
| success text on `--surface` | **8.45:1** | **7.88:1** | 4.5:1 |
| focus ring on `--bg` | **3.83:1** | **5.48:1** | 3:1 |
| toast text on toast bg | **14.20:1** | **15.95:1** | 7:1 |

The verification is mechanical: parse `tokens.css`, resolve every `var()`,
and recompute the ratios — no hard-coded claimed values are trusted.

## Type

| Token | Size | Use |
|---|---|---|
| `--text-xs` | 0.72rem | tool notes, count notes |
| `--text-sm` | 0.84rem | toasts, meta lines, hints |
| `--text-base` | clamp(0.98rem → 1.06rem) | body |
| `--text-md` | clamp(1.08rem → 1.2rem) | tagline, results note |
| `--text-lg` | clamp(1.25rem → 1.45rem) | section headings |
| `--text-xl` | clamp(1.5rem → 1.85rem) | wordmark |
| `--text-display` | clamp(1.9rem → 2.75rem) | idea title on the plate |

Display headings use the serif stack with `text-wrap: balance`; body copy is
capped at `--measure: 68ch`. Leading comes in three steps (`1.2 / 1.35 /
1.6`).

## Space, radii, depth

- Space scale on a 4px base: `--space-1…8` = 0.25 / 0.5 / 0.75 / 1 / 1.5 /
  2 / 3 / 4 rem.
- Radii: `--radius-sm: 4px` (inputs, chips), `--radius: 8px` (buttons,
  toasts), `--radius-lg: 14px` (plates, cards), `--radius-full: 999px`
  (toast icon dot).
- Shadows come in four named recipes per theme (`--shadow-ambient`,
  `--shadow-contact`, `--shadow-card`, `--shadow-raise`) plus the two inset
  recipes (`--inset-seam` for raised metal, `--slot-inset` for the slot the
  thought drops into). Dark theme shadows are pure-black and heavier;
  light theme shadows are warm iron tinted.

## Textures

- `--grain`: inline SVG `feTurbulence` paper grain, tiled, on its own fixed
  layer so `--bg-grain-opacity` (0.05 light / 0.07 dark) controls it
  without touching the background color.
- `--planks`: repeating-linear-gradient plank seams for the box stage and
  rail woodwork.
- `--brushed`: fine horizontal lines for metal surfaces.
- All three are CSS only — no images, no requests, they print nothing.

## Motion

| Token | Value | Use |
|---|---|---|
| `--dur-fast` | 120ms | hovers, tin link taps |
| `--dur` | 220ms | disclosures, card lift |
| `--dur-slow` | 320ms | plate entrance |
| `--dur-lid` | 400ms | the toast "lid thunk" |
| `--ease-out` | cubic-bezier(0.22, 0.61, 0.36, 1) | settles, never bounces |
| `--ease-inout` | cubic-bezier(0.65, 0, 0.35, 1) | symmetric moves |
| `--ease-thunk` | cubic-bezier(0.2, 1.3, 0.4, 1) | the lid: fast drop, one overshoot, stop |

Signature animations:

- **Toast lid-thunk** (`@keyframes lid-thunk`): the flash message drops in
  from −14px with a single overshoot at 60%, then `toast-out` fades it away
  after 2.6s. Mechanical, never bouncy-silly.
- **View transitions**: `::view-transition-old/new(root)` fade/translate —
  progressive enhancement on top of plain navigation; browsers without it
  just navigate.
- **Reduced motion**: `@media (prefers-reduced-motion: reduce)` sets every
  animation and transition to 0.01ms, forces `scroll-behavior: auto`,
  renders the toast without animation (visible, then gone via the normal
  page load), and disables the view-transition animations. Nothing moves
  unless the user asked for it.

## CSS-only techniques (and their degradation)

| Technique | Where | If unsupported |
|---|---|---|
| `<details>/<summary>` | rail disclosures, rename/delete per tin, destructive confirm | content renders open/static; every action is still a plain form |
| `:checked` | composer filing chips, editor filing chips | checkboxes remain native and labeled |
| `:has()` | tin list rows reacting to their open `<details>` | rows lose only their open-state styling; all controls remain usable |
| `scrollbar-gutter: stable` | html | classic scrollbar shift on narrow viewports |
| view transitions | navigation polish | plain navigation |
| `clamp()` type | whole scale | falls back to the middle size |
| `aria-current="true"` | active tin in the rail | purely additive semantics |

No technique above carries information that is not also in the HTML: the
site degrades to a fully functional form-and-link document.

## Components ↔ templates

| Component | Lives in | Notes |
|---|---|---|
| Skip link | `base.html` | first tab stop, jumps to `#main` |
| Toast (flash) | `base.html` | kind+text resolved server-side; `toast-ok/-warn/-err` draw the icon via CSS `::before` |
| Quarantine notice | `base.html` | `role="alert"`, shows the backup path |
| Masthead lockup | `index.html` | box mark + wordmark + tagline |
| Tin rail | `index.html` | All + every tin, count per tin, rename/delete disclosures per tin (never on "All") |
| Rail tools | `index.html` | Reach in & dip, export/import disclosure, "where does the data live" disclosure |
| Slot composer | `index.html` | textarea, drop button, filing chips, `+ new tin`, source line |
| Box stage | `index.html` | open box; pile height and paper balls follow the visible count |
| Card list | `index.html` | cards with text, tin chips, edit/delete |
| Plate | `idea.html`, `404.html`, `confirm.html` | the narrow-page card with a tag row and close/cancel |
| Meta grid | `idea.html` | Created / Edited / Link |
| Destructive confirm | `idea.html` | `<details>` guard before the real delete button |
| Colophon + theme picker | `base.html` | Auto / Daylight / Lamplight, `aria-pressed` state |

## Page-by-page notes

- **Index (`/`)**: the only wide page. Rail (tins + tools) and content
  (composer → stage → cards) side by side ≥ ~900px, stacked below. The
  composer is always at the top of the content column — capturing never
  requires scrolling past the list.
- **Idea (`/idea/{id}`)**: plate layout, 44px actions, the link cell shows
  the shareable path. Edit mode swaps the plate body for the form; the
  close-x becomes "cancel editing".
- **404**: plate with the honest status code and a way back; the copy is
  "Nothing here but the smell of oil".
- **Import replace guard**: plate that repeats the counts, carries the
  payload in a hidden field, and offers exactly two actions (replace / keep).

## The one documented exception

`base.html` carries two `<meta name="theme-color">` tags with literal hex
values (`#f4eee0` light, `#191309` dark). These are browser-chrome hints —
they color the mobile address bar before any CSS loads, they cannot reference
`var()`, and they must match the two themes' `--bg` values. They are the only
hex values outside `tokens.css`, and this sentence is their paper trail.

## Screenshots

Captured headlessly at the listed widths with both themes:

| File | What it shows |
|---|---|
| `screenshots/home-light.png` | index, light, 1440px — rail, composer, box stage, cards |
| `screenshots/home-dark.png` | index, dark, 1440px — the lamp-lit cellar |
| `screenshots/idea-edit-dark.png` | idea plate in edit mode, dark, 1440px |
| `screenshots/home-mobile.png` | index, light, 360px — stacked layout, no horizontal scroll |

A width sweep at 360 / 768 / 1440 / 2560px across both themes on the index
and idea pages found no horizontal overflow (the reserved scrollbar gutter
is not overflow).
