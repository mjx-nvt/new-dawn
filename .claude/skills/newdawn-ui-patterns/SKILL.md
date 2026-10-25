---
name: newdawn-ui-patterns
description: UI implementation reference for New Dawn — Majoris tokens on an Apple glass shell, brand pack schema (documents only), glass rules, typography scale, ghost classes, section rhythm, buttons, sidebar/toolbar and drawer specs. Load when creating or modifying any screen, component or stylesheet. The non-negotiable rules live in CLAUDE.md; this skill carries the specifics.
---

# New Dawn UI patterns

If a rule here conflicts with `CLAUDE.md`, `CLAUDE.md` wins.

## Design system (D-033)
Majoris Design System colours and type on an Apple Liquid Glass shell: Apple owns the shell and the control shapes (capsules, hover lift, press scale), Majoris owns the palette and the typeface. The app's look is fixed; a workspace's brand pack never reaches the UI (its logo in the workspace badge is the one exception).

## Typeface (fixed for all tenants)
DM Sans via `@fontsource-variable/dm-sans` (`opsz.css` + `opsz-italic.css`, variable weight), imported once in `src/main.tsx`.
`--font: 'DM Sans Variable', 'DM Sans', -apple-system, system-ui, sans-serif`. Shell and content both use it. Weights: body 400, labels / buttons / chips / eyebrows 500, section titles 600, page titles 700. macOS draws its own menus and dialogs in SF — leave those alone. Never add another `font-family` (IBM Plex Serif is a document font, not a UI one).

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
**Everything in the pack is for exported documents** (`logo`, `colors`, `slogan`, `document_fonts`; `radius` and `button` are kept in the file for compatibility but edited nowhere). The UI shows the `logo` in the workspace badge (`kernel/theme/brandLogo.ts`) and otherwise ignores the pack. `primary_ramp` may be omitted — `kernel/brand/tokens.rs` derives it. The Workspace → Brand page edits the pack with a `DocumentPreview` (a paper-white first page), never a UI preview.

## CSS variables — all in `src/styles/tokens.css`, none set at runtime
`--font` · text sizes `--t-xs 11.5px`, `--t-sm 13px`, `--t-md 14.5px`, `--t-lg 17px`, `--t-xl 22px`, `--t-2xl 30px` · `--space-1…8` (4 / 8 / 12 / 16 / 24 / 32 / 40 / 48) · `--ink` (slate-900) · `--ink-2` (slate-600) · `--ink-3` (slate-500: muted / hints; ≥ 4.5:1) · `--hair` (0.5 px hairline) · `--surface` · `--surface-2` (slate-100) · `--canvas` · `--glass-tint` · `--shell-scrim` (neutral wash behind sidebar + toolbar; transparent in light, ~40 % black in dark) · `--focus` (= system `AccentColor`; selection + focus rings only) · `--chevron` (select arrow) · `--ai-from` / `--ai-to` / `--ai-gradient` (AI marker) · `--radius-window` / `--radius-card 10` / `--radius-chip 6` / `--radius-control 8` / `--radius-modal 14` / `--radius-pill` · `--button-shape` (= pill).

**Primary — Majoris orange**: `--primary` (= 500 `#F97316`) · `--primary-50…--primary-950` (the Majoris orange ramp) · `--on-primary` (white) · `--primary-text` (700 in light, 400 in dark — use for all orange text; 500/600 fail 4.5:1 on white). Fills: 500, hover 600, pressed 700. Badge pair: 100 background / 800 text.

**Status** (text on light at ≥ 4.5:1): `--status-positive` green-700 · `--status-warning` yellow-800 · `--status-negative` red-600; dark: green-400 / yellow-400 / red-400. Their 12 % backgrounds are `--status-*-tint` (rgba literals, same repaint rule as the primary tints). The other Majoris families (blue, cyan) are added to `tokens.css` when charts need them, not before.

**Tints**: `--primary-tint` / `--on-primary-tint` (active item, orange chip, workspace pill) and `--primary-tint-soft` (playing row, banner) — 100 / 50 in light, `rgba(251, 146, 60, .22 / .14)` (primary-400 with alpha) with `--primary-200` text in dark. **Always use the tokens; never write a per-rule `@media (prefers-color-scheme: dark)` override for a tint, and never `color-mix(…, transparent)` for a background** — WebKit does not repaint such a background when the appearance changes while the app runs (verified by pixel sampling both ways; a `:root` flip of rgba literals repaints).

**Dark mode**: variables flip under `prefers-color-scheme: dark` in `tokens.css` only.

## Glass
- **Native** (from Rust, `kernel/glass.rs`): the window background, sidebar and toolbar. The DOM leaves those regions fully transparent — no backgrounds, no borders except hairlines.
- **Content root**: `background: color-mix(in srgb, var(--canvas) 92%, transparent)`; under `@media (prefers-reduced-transparency: reduce)` → `background: var(--canvas)`.
- **In-page glass** `.glass`: `backdrop-filter: blur(24px) saturate(1.4); background: var(--glass-tint); border: 0.5px solid var(--hair); box-shadow: inset 0 0.5px 0 rgba(255,255,255,.35), 0 8px 32px rgba(0,0,0,.12)`. Under reduced transparency → `background: var(--surface)`, no blur. Use only for the AI drawer, popovers, modal sheets (native `<dialog>` with `showModal()`), quick-capture window and menus.
- Never tint glass with the primary colour.

## Typography scale
| Element | Size / weight | Colour | Notes |
|---|---|---|---|
| Page title | 30 / 700 | `--ink` | line-height 1.15, tracking -0.01em |
| Section title | 17 / 600 | `--ink` | sentence case (D-030: no uppercase section titles) |
| Section subtitle | 14.5 / 400 | `--ink-2` | regular, one line |
| Eyebrow | 11.5 / 500 | `--primary-text` | uppercase, tracking .08em — the only uppercase text |
| Field label / row label | 13 / 500 · 14.5 / 500 | `--ink-2` · `--ink` | sentence case |
| Field value | 14.5 / 500 | `--ink` | |
| Body | 14.5 / 400 | `--ink-2` | line-height 1.6 |
| Card title | 17 / 600 | `--ink` | |
| Meta / hint | 13 / 500 | `--ink-2` / `--ink-3` | at most one hint per group |
| Stat value | 22–28 / 800 | `--ink` | |
| "Not yet captured" | inherit / 400 | `--ink-3` | italic |

## Forms and actions (System Settings shape)
- **Grouped lists** `.group` > `.groupRow` (`.rowText` with `.rowLabel` + `.rowHint`, `.rowControl` on the right); `.groupTitle` above a group. Use for every settings pane.
- **One primary action** per screen state (`.btnPrimary`); secondary actions in a `⋯` `MenuButton` (`.menu` glass popover). Destructive actions are red: `.btnDanger` (text) and the `DangerButton` component for two-step confirms (red fill on the confirm step), `.menuItemDanger` in menus.
- **Pages fill the panel** (`.page` has no max-width); prose limits its own measure (`max-width: 76ch`). Wide meeting pages use two columns (content + a 340 px aside) via `@container content (min-width: 961px)`.
- Settings window (app-only): a glass sidebar of panes on the left (200 px, grouped App / AI on this Mac), the pane on the right with a 22/700 title. A workspace's own settings (Overview, Brand, Meeting types, People) are the Workspace page in the main window, as a segmented control.

## Ghost classes (every dossier stylesheet includes them)
```css
.ghostCard { border: 0.5px dashed var(--hair); background: transparent; opacity: .55; }
.ghostText, .fieldMuted { font-style: italic; color: var(--ink-3); font-weight: 400; }
.ghostPill { background: transparent; border: 0.5px dashed var(--hair); color: var(--ink-3); font-style: italic; }
.ghostAvatar { border: 0.5px dashed var(--hair); background: transparent; }
```
Field sections always render every label; list sections render real items or 2–3 ghost cards. Omit a section only when the data *cannot* exist for that entity type.

## Section rhythm
Group divider: eyebrow "Group A · Name" + hairline; 40 px above, 24 px below. Section: title + optional italic subtitle; 24 px between sections. Field grid: 2 columns, 1 column when the content panel is ≤ 640 px wide. `.content-root` is a size container named `content`: write responsive rules as `@container content (…)` so they follow the panel (sidebar open or not), not the window. Cards: `--radius-card`, 0.5 px `--hair` border, `--surface` background, 16 px padding.

## Buttons (Apple capsules, 14.5 / 500)
- `.btnPrimary`: `--primary` fill, white text; hover `--primary-600`, pressed `--primary-700`. One per screen state.
- `.btnSecondary`: neutral — `--surface-2` fill, `--ink` text, inset hairline; hover a touch darker. Never tinted.
- `.btnGhost`: transparent, `--ink-2` text, hover `--surface-2`.
- `.btnDanger` / `.btnDangerFilled`: red text / red fill (two-step confirm via `DangerButton`).
All: radius `--button-shape` (pill), press `scale(.97)`, height 28 px (compact) / 32 px (default); focus ring `0 0 0 3px color-mix(in srgb, var(--focus) 35%, transparent)`. Segmented controls are a pill of pills (`--surface-2` track, `--surface` selected segment).

## AI marker (the only gradient)
Every button that runs Whisper or Qwen: `className="btn … btnAi"` + `<SparkleIcon />` as its first child (`kernel/shell/icons.tsx`, Material Symbols `auto_awesome`), `aria-busy` while it is pending (the primary step button disappears once the job runs: the page shows its progress card instead). `.btnPrimary.btnAi` = gradient fill `--ai-gradient` (`--ai-from` blue-600 → `--ai-to` = `--primary`), white text; `.btnAi` alone = `--surface` with a 1 px gradient ring. AI-written content gets `<p className="fieldHint aiMark"><SparkleIcon /> Written by {model} · {date}</p>`. No other element uses a gradient, and no non-AI action gets the sparkle.

## Player
`WaveformPlayer` (`modules/meetings/`): a `.glass` strip — 36 px orange play/pause, canvas waveform (2 px bars, 1 px gap; played = `--primary`, rest = `--ink-3` at 45 %; current transcript line = `--primary` band at 14 %; `role="slider"`, click/drag seeks, ← → 5 s), `m:ss / m:ss` tabular, ±10 s ghost buttons with Material `replay` glyphs, speed as a segmented control. Peaks come from `get_waveform` (`useWaveform`), never decoded in the webview. The `<audio>` element is hidden and owned by the caller.

## Dropdowns
Always a native `<select className="input">` — `components.css` makes it a 28 px capsule with its own chevron (`--chevron`, a data-URI SVG in two greys), hover tint, `width: auto` (100 % inside `.field`). No custom select component; no per-page overrides.

## Sidebar & toolbar
Sidebar 240 px, transparent (native glass behind); **icon rail 76 px** (icons + tooltips, workspace mark only) and hidden are the other states. Default "auto": rail below a 1000 px window, labels above; ⟨⟨/⟩⟩ at the bottom and ⌃⌘S pick one (remembered per Mac), ⌥⌘S hides. Icons are inline SVG in `kernel/shell/icons.tsx` (16 pt, 1.6 stroke, currentColor). Items 28 px tall, radius `--radius-control`, 12 px side padding; active item `--primary-100` background with `--primary-800` text, weight 500 (dark: `--primary-400` at 22 % over the glass / `--primary-200` text). Workspace badge at the top: the workspace logo alone when the pack has one (name as tooltip), else a 20 px letter mark; click opens the switcher popover (`.glass`). Section labels 11/700 uppercase `--ink-3`.
Toolbar 52 px, unified, transparent, `data-tauri-drag-region="deep"`; left: back/forward (‹ ›, ⌘[ / ⌘]) + page title 15.5/600 + breadcrumb (hidden when narrow); centre: search field (⌘K, a capsule); right: Record control — ● pill (`--status-negative` dot, elapsed `mm:ss`, 5-bar level meter), becomes ■ while recording. Traffic lights sit inside the toolbar's left padding.

## Drawer (AI companion)
Right-side, 480 px at ≥ 1200 px else 420 px, full width < 720 px; `.glass`; ESC and backdrop close; focus trapped; streams text; footer shows model · tokens · time from `ai_log`. Opened with ⌘J.

## Status pills
Height 20 px, radius `--radius-chip`, 11/500. Positive = `--status-positive` on 12 % tint; warning / negative likewise; neutral = `--ink-2` on `--surface-2`; orange-tagged (`.chipPrimary`) = `--primary-800` on `--primary-100`.

## Motion
150 ms ease-out for hover/open; 220 ms for drawer/popovers; none under `prefers-reduced-motion`.
