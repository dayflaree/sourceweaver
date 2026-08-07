#!/usr/bin/env bash
set -euo pipefail

current_version="${1:-}"
repo="${2:-dayflaree/sourceweaver}"
api_url="https://api.github.com/repos/${repo}/releases/latest"

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required to check GitHub releases" >&2
  exit 1
fi
if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required to parse GitHub release metadata" >&2
  exit 1
fi

response_file="$(mktemp)"
trap 'rm -f "$response_file"' EXIT

http_status="$(curl -sS -L \
  -H 'Accept: application/vnd.github+json' \
  -H 'X-GitHub-Api-Version: 2022-11-28' \
  -w '%{http_code}' \
  -o "$response_file" \
  "$api_url" || true)"

if [[ "$http_status" != "200" ]]; then
  echo "could not fetch latest release for ${repo}: HTTP ${http_status}" >&2
  if [[ -s "$response_file" ]]; then
    python3 - "$response_file" <<'PY' >&2
import json
import sys
try:
    payload = json.load(open(sys.argv[1], encoding="utf-8"))
except Exception:
    sys.exit(0)
message = payload.get("message")
if message:
    print(message)
PY
  fi
  exit 1
fi

python3 - "$response_file" "$current_version" <<'PY'
import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
current = sys.argv[2].strip()
latest = str(payload.get("tag_name") or "").strip()
assets = payload.get("assets") or []

def normalize(version: str) -> str:
    version = version.strip()
    return version[1:] if version.startswith("v") else version

print(f"repository: {payload.get('html_url', '').split('/releases/')[0] or 'unknown'}")
print(f"latest_tag: {latest or 'unknown'}")
print(f"latest_name: {payload.get('name') or latest or 'unknown'}")
print(f"published_at: {payload.get('published_at') or 'unknown'}")
print(f"prerelease: {str(bool(payload.get('prerelease'))).lower()}")
print(f"release_url: {payload.get('html_url') or 'unknown'}")
if current:
    if normalize(current) == normalize(latest):
        print(f"status: current ({current})")
    else:
        print(f"status: update_available (current={current}, latest={latest})")
else:
    print("status: current version not supplied")
print("assets:")
for asset in assets:
    name = asset.get("name") or "unnamed"
    size = asset.get("size")
    print(f"  - {name} ({size} bytes)")
PY
