#!/usr/bin/env bash
# Publishes the locally built, signed bundle as a GitHub release the app's updater can see:
#   scripts/release-local.sh            (version from src-tauri/tauri.conf.json)
# Needs: `pnpm tauri build` run with TAURI_SIGNING_PRIVATE_KEY set (so the .sig exists), and
# `gh` logged in with write access to the releases repo. Uploads the .dmg, the update bundle,
# its signature and latest.json (D-031). Asset names avoid spaces: GitHub would rename them.
set -euo pipefail
cd "$(dirname "$0")/.."

repo="mjx-nvt/new-dawn"
version=$(node -p "require('./src-tauri/tauri.conf.json').version")
tag="v$version"
bundle="src-tauri/target/release/bundle"
app_tgz="$bundle/macos/New Dawn.app.tar.gz"
sig="$app_tgz.sig"
dmg="$bundle/dmg/New Dawn_${version}_aarch64.dmg"
for f in "$app_tgz" "$sig" "$dmg"; do
  [ -f "$f" ] || { echo "missing $f — run: TAURI_SIGNING_PRIVATE_KEY=\"\$(cat ~/.tauri/newdawn.key)\" pnpm tauri build"; exit 1; }
done

out=$(mktemp -d)
cp "$app_tgz" "$out/New-Dawn-${version}-aarch64.app.tar.gz"
cp "$sig" "$out/New-Dawn-${version}-aarch64.app.tar.gz.sig"
cp "$dmg" "$out/New-Dawn-${version}-aarch64.dmg"
notes="${RELEASE_NOTES:-New Dawn $version.}"
python3 - "$out" "$version" "$repo" "$tag" "$notes" <<'EOF'
import json, sys, datetime
out, version, repo, tag, notes = sys.argv[1:6]
sig = open(f"{out}/New-Dawn-{version}-aarch64.app.tar.gz.sig").read().strip()
json.dump({
  "version": version,
  "notes": notes,
  "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
  "platforms": {
    "darwin-aarch64": {
      "signature": sig,
      "url": f"https://github.com/{repo}/releases/download/{tag}/New-Dawn-{version}-aarch64.app.tar.gz",
    }
  },
}, open(f"{out}/latest.json", "w"), indent=2)
EOF

gh release create "$tag" --repo "$repo" --title "New Dawn $version" --notes "$notes" \
  "$out/New-Dawn-${version}-aarch64.dmg" \
  "$out/New-Dawn-${version}-aarch64.app.tar.gz" \
  "$out/New-Dawn-${version}-aarch64.app.tar.gz.sig" \
  "$out/latest.json"
echo "published https://github.com/$repo/releases/tag/$tag"
rm -rf "$out"
