#!/usr/bin/env bash
set -euo pipefail

CARGO="${CARGO:-cargo}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${1:-/tmp/sourceweaver-real-validation}"
SOURCE_SHA="184f8c5eec17313724155f91f2f99133c12c464a"
BASE_URL="https://raw.githubusercontent.com/rubycho/labescape-hl2/${SOURCE_SHA}/maps"

rm -rf "$OUT"
mkdir -p "$OUT/vmfs" "$OUT/fake-tools" "$OUT/compile-logs"

curl -fL --retry 3 --retry-delay 2 "$BASE_URL/hl2-chap2.vmf" -o "$OUT/vmfs/hl2-chap2.vmf"
curl -fL --retry 3 --retry-delay 2 "$BASE_URL/hl2-chap3.vmf" -o "$OUT/vmfs/hl2-chap3.vmf"

"$CARGO" run -q -p sourceweaver-cli -- inspect "$OUT/vmfs/hl2-chap2.vmf" > "$OUT/hl2-chap2.inspect.txt"
"$CARGO" run -q -p sourceweaver-cli -- inspect "$OUT/vmfs/hl2-chap3.vmf" > "$OUT/hl2-chap3.inspect.txt"
"$CARGO" run -q -p sourceweaver-cli -- list-types "$OUT/vmfs/hl2-chap2.vmf" > "$OUT/hl2-chap2.types.txt"
"$CARGO" run -q -p sourceweaver-cli -- list-types "$OUT/vmfs/hl2-chap3.vmf" > "$OUT/hl2-chap3.types.txt"

"$CARGO" run -q -p sourceweaver-cli -- merge \
  -o "$OUT/hl2-chap2-chap3-merged.vmf" \
  --landmark landmark2 \
  "$OUT/vmfs/hl2-chap2.vmf" \
  "$OUT/vmfs/hl2-chap3.vmf" \
  > "$OUT/merge.txt"

"$CARGO" run -q -p sourceweaver-cli -- validate \
  "$OUT/hl2-chap2-chap3-merged.vmf" \
  --compile-log "$ROOT/tests/fixtures/vbsp-success.txt" \
  --json > "$OUT/validate.json"

cat > "$OUT/fake-tools/source-tool-ok" <<'TOOL'
#!/usr/bin/env bash
name=$(basename "$0")
echo "Valve Software - $name"
echo "input: ${@: -1}"
echo "0 errors, 0 warnings"
echo "VBSP finished successfully"
TOOL
chmod +x "$OUT/fake-tools/source-tool-ok"
ln -sf source-tool-ok "$OUT/fake-tools/vbsp"

cat > "$OUT/profile.toml" <<EOF
[tools]
vbsp = "$OUT/fake-tools/vbsp"

[compile]
steps = ["vbsp"]
log_dir = "$OUT/compile-logs"
EOF

"$CARGO" run -q -p sourceweaver-cli -- compile \
  "$OUT/hl2-chap2-chap3-merged.vmf" \
  --profile "$OUT/profile.toml" \
  --report "$OUT/compile-report.json" \
  --json > "$OUT/compile-stdout.json"

python3 - <<'PY' "$OUT"
import json
import pathlib
import sys
out = pathlib.Path(sys.argv[1])
validate = json.loads((out / 'validate.json').read_text())
compile_report = json.loads((out / 'compile-report.json').read_text())
assert validate['ok'] is True, validate
assert compile_report['ok'] is True, compile_report
assert (out / 'hl2-chap2-chap3-merged.vmf').stat().st_size > 100000
PY

printf 'Source Weaver public VMF validation complete: %s\n' "$OUT"
cat "$OUT/merge.txt"
