#!/usr/bin/env bash
set -euo pipefail

CARGO="${CARGO:-cargo}"
OUT="${1:-/tmp/sourceweaver-material-preview-scope}"

rm -rf "$OUT"
mkdir -p "$OUT/material-root/materials/brick" "$OUT/material-root/materials/custom" "$OUT/material-root/materials/tools"

cat > "$OUT/textured-preview-scope.vmf" <<'VMF'
versioninfo { "editorversion" "400" }
world {
  "id" "1"
  solid {
    "id" "2"
    side { "id" "3" "plane" "(0 0 0) (64 0 0) (64 64 0)" "material" "Brick/Wall001" "uaxis" "[1 0 0 16] 0.25" "vaxis" "[0 -1 0 8] 0.5" }
    side { "id" "4" "plane" "(0 0 64) (64 64 64) (64 0 64)" "material" "custom\\MixedCase_Detail" "uaxis" "[0 1 0 -8] 0.25" "vaxis" "[1 0 0 4] 0.25" }
    side { "id" "5" "plane" "(0 0 0) (0 0 64) (64 0 64)" "material" "TOOLS/TOOLSTRIGGER" "uaxis" "[1 0 0 0] 0.25" "vaxis" "[0 0 -1 0] 0.25" }
    side { "id" "6" "plane" "(64 0 0) (64 0 64) (64 64 64)" "material" "MISSING/SYNTHETIC_ONLY" "uaxis" "[0 1 0 0] 0.25" "vaxis" "[0 0 -1 0] 0.25" }
  }
}
VMF

cat > "$OUT/material-root/materials/brick/wall001.vmt" <<'VMT'
LightmappedGeneric
{
  "$basetexture" "brick/wall001"
}
VMT
printf 'synthetic-vtf-scan-only' > "$OUT/material-root/materials/brick/wall001.vtf"
printf 'synthetic-preview-sidecar' > "$OUT/material-root/materials/brick/wall001.png"
cat > "$OUT/material-root/materials/custom/MixedCase_Detail.vmt" <<'VMT'
LightmappedGeneric
{
  "$basetexture" "custom/MixedCase_Detail"
}
VMT

"$CARGO" run -q -p sourceweaver-cli -- validate "$OUT/textured-preview-scope.vmf" --json > "$OUT/textured-preview-scope.validate.json"
"$CARGO" run -q -p sourceweaver-cli -- inspect "$OUT/textured-preview-scope.vmf" > "$OUT/textured-preview-scope.inspect.txt"
"$CARGO" test -q -p sourceweaver-core reconstructs_face_materials_aligned_with_faces -- --nocapture > "$OUT/core-preview-material-tests.txt"
"$CARGO" test -q -p sourceweaver-desktop material_preview -- --nocapture > "$OUT/desktop-material-preview-tests.txt"

python3 - <<'PY' "$OUT"
import json
from pathlib import Path
import sys
out = Path(sys.argv[1])
validation = json.loads((out / 'textured-preview-scope.validate.json').read_text())
assert validation['ok'] is True, validation
assert validation['complexity']['brush_solids'] == 1, validation['complexity']
assert validation['complexity']['sides'] == 4, validation['complexity']
core_tests = (out / 'core-preview-material-tests.txt').read_text()
desktop_tests = (out / 'desktop-material-preview-tests.txt').read_text()
assert 'test result: ok' in core_tests and '1 passed' in core_tests, core_tests
assert 'test result: ok' in desktop_tests and '2 passed' in desktop_tests, desktop_tests
summary = {
    'fixture': str(out / 'textured-preview-scope.vmf'),
    'material_root': str(out / 'material-root'),
    'validation_ok': validation['ok'],
    'brush_solids': validation['complexity']['brush_solids'],
    'sides': validation['complexity']['sides'],
    'scope': 'material-aware face colors from VMF material names and user-provided material roots; no VTF pixel decoding or Hammer-equivalent viewport claim',
    'unsupported': [
        'VTF pixel decoding',
        'Hammer-equivalent UV projection/rendering',
        'game-specific entity/model icons',
        'compile or runtime material availability proof',
        'bundled proprietary game content',
    ],
}
(out / 'scope-summary.json').write_text(json.dumps(summary, indent=2) + '\n')
PY

sha256sum \
  "$OUT/textured-preview-scope.vmf" \
  "$OUT/material-root/materials/brick/wall001.vmt" \
  "$OUT/material-root/materials/brick/wall001.vtf" \
  "$OUT/material-root/materials/brick/wall001.png" \
  "$OUT/material-root/materials/custom/MixedCase_Detail.vmt" \
  > "$OUT/SHA256SUMS"

printf 'Source Weaver material-preview scope validation complete: %s\n' "$OUT"
cat "$OUT/scope-summary.json"
cat "$OUT/SHA256SUMS"
