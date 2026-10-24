---
name: newdawn-ui-patterns
description: UI implementation reference for New Dawn — token names, brand pack schema, glass rules, typography scale, ghost classes, section rhythm, buttons, sidebar/toolbar and drawer specs. Load when creating or modifying any screen, component or stylesheet. The non-negotiable rules live in CLAUDE.md; this skill carries the specifics.
---

# New Dawn UI patterns

If a rule here conflicts with `CLAUDE.md`, `CLAUDE.md` wins.

## Typeface (fixed for all tenants)
Be Vietnam Pro via `@fontsource/be-vietnam-pro` (weights 400, 500, 600, 700; 400 italic), imported once in `src/main.tsx`.
`--font: 'Be Vietnam Pro', -apple-system, system-ui, sans-serif`. Shell and content both use it. macOS draws its own menus and dialogs in SF — leave those alone. Never add another `font-family`.

## Brand pack schema — `brands/<slug>/tokens.json`, `"$schema": "newdawn-brand-tokens/v2"`
```json
{
  "name": "VIFC Fintech Hub", "slug": "vifc",
  "logo": { "primary": "logo/x.svg", "on_dark": "logo/x_white.svg", "variants": ["original", "full_white"] },
  "slogan": { "en": "…", "vi": "…" },
  "colors": {
    "primary": "#0132AD",
    "primary_ramp": { "50": "#…", "…": "…", "950": "#…" },
    "secondary": null, "accent": null, "text": "#0B1220", "canvas": "#FFFFFF",
    "status": { "positive": "#1F7A4D", "warning": "#9A6A00", "negative": "#B3261E" }
  },
  "document_fonts": { "heading": "Playfair Display", "body": "Be Vietnam Pro", "files": [] },
  "radius": { "preset": "sharp|soft|round", "card": 10, "chip": 6, "control": 8, "modal": 14 },
  "button": { "shape": "square|rounded|pill", "primary": "filled|soft|outline", "secondary": "soft|outline|ghost" },
  "meeting_types_starter": [ { "name": "Partner call", "language": "en", "template": "templates/meeting-minutes/template.docx" } ]
}
```
UI-brandable: `logo`, `colors`, `radius`, `button` (→ CSS variables on `.content-root`; `--brand` = `colors.primary`). Document-only: `slogan`, `document_fonts` (used by the minutes export; the UI typeface is always Be Vietnam Pro). `primary_ramp` may be omitted — `kernel/brand/tokens.rs` derives it. The UI never reads `tokens.json` directly; it receives the resolved token set from the `get_brand` command.

## CSS variables
**Base (never brand)** — defined in `src/styles/tokens.css`, flipped for dark mode:
`--font` · `--ink` (primary text) · `--ink-2` (secondary) · `--ink-3` (muted / labels) · `--hair` (0.5 px hairline colour) · `--surface` · `--surface-2` · `--glass-tint` · `--shell-scrim` (neutral wash behind sidebar + toolbar; transparent in light, ~40 % black in dark) · `--on-brand` (text on brand fills) · `--radius-pill` · `--focus` (= system `AccentColor`; selection + focus rings — never the brand `--accent`) · `--space-1…6` (4 / 8 / 12 / 16 / 24 / 32) · `--radius-window` · text sizes `--t-xs 11px`, `--t-sm 12.5px`, `--t-md 13.5px`, `--t-lg 15.5px`, `--t-xl 20px`, `--t-2xl 28px`.

**Brand (from tokens)** — set on `.content-root` by `applyBrand()`:
`--brand` · `--brand-50…--brand-950` · `--brand-text` (derived: `--brand` in light, `--brand-400` in dark — use for all brand-coloured text) · `--secondary` · `--canvas` · `--status-positive` / `--status-warning` / `--status-negative` · `--radius-card` / `--radius-chip` / `--radius-control` / `--radius-modal` · `--button-shape` (`6px` | `var(--radius-control)` | `999px`) · `--button-primary` / `--button-secondary` (keywords consumed by `.btn`) · `--logo-url` / `--logo-dark-url`.

**Dark mode**: base variables flip under `prefers-color-scheme: dark`. Brand ramp is used *inverted* for tinted surfaces (brand-50 ↔ brand-950) and `--brand-400` replaces `--brand-500` for brand-coloured text on dark backgrounds.

## Glass
- **Native** (from Rust, `kernel/glass.rs`): the window background, sidebar and toolbar. The DOM leaves those regions fully transparent — no backgrounds, no borders except hairlines.
- **Content root**: `background: color-mix(in srgb, var(--canvas) 92%, transparent)`; under `@media (prefers-reduced-transparency: reduce)` → `background: var(--canvas)`.
- **In-page glass** `.glass`: `backdrop-filter: blur(24px) saturate(1.4); background: var(--glass-tint); border: 0.5px solid var(--hair); box-shadow: inset 0 0.5px 0 rgba(255,255,255,.35), 0 8px 32px rgba(0,0,0,.12)`. Under reduced transparency → `background: var(--surface)`, no blur. Use only for the AI drawer, popovers, modal sheets (native `<dialog>` with `showModal()`), quick-capture window and menus.
- Never tint glass with the brand colour.

## Typography scale
| Element | Size / weight | Colour | Notes |
|---|---|---|---|
| Page title | 28 / 700 | `--ink` | line-height 1.1 |
| Section title | 12 / 700 | `--ink` | uppercase, tracking .08em |
| Section subtitle | 12.5 / 400 | `--ink-2` | italic |
| Group eyebrow | 11 / 700 | `--brand-text` | uppercase, tracking .1em |
| Field label | 11 / 700 | `--ink-3` | uppercase, tracking .08em |
| Field value | 13.5 / 500 | `--ink` | |
| Body | 13.5 / 400 | `--ink-2` | line-height 1.6 |
| Card title | 15.5 / 700 | `--ink` | |
| Meta | 11.5 / 600 | `--ink-2` | |
| Stat value | 22–28 / 800 | `--ink` | |
| "Not yet captured" | inherit / 400 | `--ink-3` | italic |

## Ghost classes (every dossier stylesheet includes them)
```css
.ghostCard { border: 0.5px dashed var(--hair); background: transparent; opacity: .55; }
.ghostText, .fieldMuted { font-style: italic; color: var(--ink-3); font-weight: 400; }
.ghostPill { background: transparent; border: 0.5px dashed var(--hair); color: var(--ink-3); font-style: italic; }
.ghostAvatar { border: 0.5px dashed var(--hair); background: transparent; }
```
Field sections always render every label; list sections render real items or 2–3 ghost cards. Omit a section only when the data *cannot* exist for that entity type.

## Section rhythm
Group divider: eyebrow "Group A · Name" + hairline; 40 px above, 24 px below. Section: title + optional italic subtitle; 24 px between sections. Field grid: 2 columns at ≥ 960 px, 1 column below. Cards: `--radius-card`, 0.5 px `--hair` border, `--surface` background, 16 px padding.

## Buttons
`.btn` reads `--button-shape` for radius and `--button-primary` / `--button-secondary` for style:
- `filled`: `--brand` background, white text; hover `--brand-600`.
- `soft`: `--brand-100` background, `--brand-700` text (dark: `--brand-900` / `--brand-200`).
- `outline`: transparent, 0.5 px `--brand` border, `--brand` text.
- `ghost`: transparent, `--ink-2` text, hover `--surface-2`.
Height 28 px (compact) / 32 px (default); focus ring `0 0 0 3px color-mix(in srgb, var(--focus) 35%, transparent)`.

## Sidebar & toolbar
Sidebar 240 px, transparent (native glass behind). Items 28 px tall, radius `--radius-control`, 12 px side padding; active item `--brand-100` background with `--brand-700` text (dark: `--brand-400` at 22 % over the glass / `--brand-200` text). Workspace badge at the top: logo mark 20 px + name 13.5/600 + chevron; click opens the switcher popover (`.glass`). Section labels 11/700 uppercase `--ink-3`.
Toolbar 52 px, unified, transparent; left: page title 15.5/600 + breadcrumb; centre: search field (⌘K); right: Record control — ● pill (`--status-negative` dot, elapsed `mm:ss`, 5-bar level meter), becomes ■ while recording. Traffic lights sit inside the toolbar's left padding.

## Drawer (AI companion)
Right-side, 480 px at ≥ 1200 px else 420 px, full width < 720 px; `.glass`; ESC and backdrop close; focus trapped; streams text; footer shows model · tokens · time from `ai_log`. Opened with ⌘J.

## Status pills
Height 20 px, radius `--radius-chip`, 11/600. Positive = `--status-positive` on 12 % tint; warning / negative likewise; neutral = `--ink-2` on `--surface-2`; brand-tagged = `--brand-700` on `--brand-100`.

## Motion
150 ms ease-out for hover/open; 220 ms for drawer/popovers; none under `prefers-reduced-motion`.
