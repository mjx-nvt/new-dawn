# Brand packs (schema v2)

One folder per starter brand: `tokens.json` (`newdawn-brand-tokens/v2`, see `.claude/skills/newdawn-ui-patterns`), `logo/*.svg`, optional `fonts/` (document fonts only) and `templates/<meeting-type>/template.docx`.
When a tenant is created the chosen pack is copied into that tenant's `brand/` folder and becomes editable there; the starter stays untouched.
App UI typeface is fixed (Be Vietnam Pro). A pack may set: logo, slogan EN/VI, colours, document fonts (heading/body, exported documents only), corner-radius style, button style, starter meeting types.
Packs are bundled into the app as resources (`tauri.conf.json` → `bundle.resources`).
