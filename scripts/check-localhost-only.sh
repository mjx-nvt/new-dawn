#!/usr/bin/env bash
# New Dawn — fail if any URL in app code points anywhere but localhost.
# Rule 3 in CLAUDE.md: no network except localhost. Exceptions, outside the scanned folders:
# model downloads in kernel/ai/models.rs (D-014) and the updater endpoint in
# src-tauri/tauri.conf.json (D-031: github.com, only for the check the user runs or the daily one).
# Lines that are comments
# (//, #, *, /*, <!--) are ignored so documentation links are allowed, and
# XML/SVG/OOXML namespace URLs inside xmlns="…" are ignored because they are identifiers, not requests.
# One exception (CLAUDE.md rule 3, D-014): huggingface.co model downloads in kernel/ai/models.rs.
set -euo pipefail

cd "$(dirname "$0")/.."

allowed='https?://(localhost|127\.0\.0\.1)([:/]|$)|https?://(ipc|asset)\.localhost|https?://schema\.tauri\.app|xmlns(:[a-z0-9]+)?="http://(www\.w3\.org|schemas\.openxmlformats\.org)/'

violations=$(grep -rnE 'https?://' --include='*.rs' --include='*.ts' --include='*.tsx' --include='*.js' --include='*.json' --include='*.html' --include='*.css' src-tauri/src src 2>/dev/null \
  | grep -vE '^[^:]+:[0-9]+:[[:space:]]*(//|#|\*|/\*|<!--)' \
  | grep -vE "$allowed" \
  | grep -vE '^src-tauri/src/kernel/ai/models\.rs:[0-9]+:.*https://huggingface\.co/' \
  || true)

if [[ -n "$violations" ]]; then
  echo "❌ Non-localhost URLs found in app code:"
  echo "$violations"
  exit 1
fi

echo "✅ localhost-only check passed"
