#!/usr/bin/env bash
set -euo pipefail

CARGO="${CARGO:-cargo}"
OUT="${1:-/tmp/sourceweaver-signed-update-validation}"
PRIVATE_KEY="BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc="
PUBLIC_KEY="6kpsY+KcUgq+9VB7Ey7F+ZVHdq6+vnuSQh7qaRRG0iw="

rm -rf "$OUT"
mkdir -p "$OUT/artifacts" "$OUT/download" "$OUT/rejected"

printf 'synthetic signed update artifact\n' > "$OUT/artifacts/sourceweaver-v0.2.0-linux-x86_64.AppImage"
SOURCEWEAVER_UPDATE_SIGNING_KEY_BASE64="$PRIVATE_KEY" \
  "$CARGO" run -q -p sourceweaver-cli -- update manifest \
    --artifact-dir "$OUT/artifacts" \
    --output "$OUT/sourceweaver-update-manifest.json" \
    --version v0.2.0 \
    --release-url https://github.com/dayflaree/sourceweaver/releases/tag/v0.2.0 \
    --base-download-url "$OUT/artifacts" \
    --published-at 2026-08-07T00:00:00Z \
    > "$OUT/manifest-generation.txt"

"$CARGO" run -q -p sourceweaver-cli -- update check \
  --manifest "$OUT/sourceweaver-update-manifest.json" \
  --public-key "$PUBLIC_KEY" \
  --current-version v0.1.0 \
  --target linux-x86_64 \
  --json \
  > "$OUT/check.json"

"$CARGO" run -q -p sourceweaver-cli -- update check \
  --manifest "$OUT/sourceweaver-update-manifest.json" \
  --public-key "$PUBLIC_KEY" \
  --current-version v0.1.0 \
  --target linux-x86_64 \
  --download-dir "$OUT/download" \
  --install --confirm-install \
  --json \
  > "$OUT/download-and-handoff.json"

python3 - <<'PY' "$OUT"
import json
from pathlib import Path
import sys
out = Path(sys.argv[1])
check = json.loads((out / 'check.json').read_text())
download = json.loads((out / 'download-and-handoff.json').read_text())
assert check['availability'] == 'update_available', check
assert check['downloaded_path'] is None, check
assert download['downloaded_path'], download
assert download['install_handoff'] is True, download
assert Path(download['downloaded_path']).is_file(), download
manifest = json.loads((out / 'sourceweaver-update-manifest.json').read_text())
manifest['signature'] = manifest['signature'][::-1]
(out / 'wrong-signature-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
artifact = json.loads((out / 'check.json').read_text())['artifact']
Path(out / 'artifacts' / artifact['name']).write_text('corrupt artifact bytes\n')
PY

if "$CARGO" run -q -p sourceweaver-cli -- update check \
  --manifest "$OUT/wrong-signature-manifest.json" \
  --public-key "$PUBLIC_KEY" \
  --current-version v0.1.0 \
  --target linux-x86_64 \
  --download-dir "$OUT/rejected" \
  > "$OUT/wrong-signature.txt" 2>&1; then
  echo "wrong-signature manifest unexpectedly passed" >&2
  exit 1
fi

if "$CARGO" run -q -p sourceweaver-cli -- update check \
  --manifest "$OUT/sourceweaver-update-manifest.json" \
  --public-key "$PUBLIC_KEY" \
  --current-version v0.1.0 \
  --target linux-x86_64 \
  --download-dir "$OUT/rejected" \
  > "$OUT/corrupt-artifact.txt" 2>&1; then
  echo "corrupt artifact unexpectedly passed" >&2
  exit 1
fi

if [[ -e "$OUT/rejected/sourceweaver-v0.2.0-linux-x86_64.AppImage" ]]; then
  echo "rejected artifact was written despite failed verification" >&2
  exit 1
fi

sha256sum \
  "$OUT/sourceweaver-update-manifest.json" \
  "$OUT/check.json" \
  "$OUT/download-and-handoff.json" \
  "$OUT/download/sourceweaver-v0.2.0-linux-x86_64.AppImage" \
  > "$OUT/SHA256SUMS"

printf 'Source Weaver signed update validation complete: %s\n' "$OUT"
cat "$OUT/SHA256SUMS"
