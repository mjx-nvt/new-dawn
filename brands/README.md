# Brand packs (schema v2)

One folder per starter brand: `tokens.json` (`newdawn-brand-tokens/v2`, see `.claude/skills/newdawn-ui-patterns`), `logo/*.svg`, optional `fonts/` (document fonts only) and `templates/meeting-minutes/template.docx`.
A workspace's `brand/` folder is filled from the pack with its slug (else `default`) the first time its brand is read; the starter stays untouched. Its minutes template comes from the starter's `templates/` when there is one, otherwise it is **made from the brand** — logo, slogans, primary colour and fonts — from `src-tauri/templates/minutes-base.docx` (D-040).
A pack is for exported documents only (D-033): logo (the white version is made from it), slogan EN/VI, primary colour (the document palette is derived from it), document fonts (heading Playfair Display and body Be Vietnam Pro by default). The app's own typeface is DM Sans and never changes.
Packs are bundled into the app as resources (`tauri.conf.json` → `bundle.resources`).
