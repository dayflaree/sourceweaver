#!/usr/bin/env bash
set -euo pipefail

CARGO="${CARGO:-cargo}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${1:-/tmp/sourceweaver-high-risk-map-cases}"

rm -rf "$OUT"
mkdir -p "$OUT"

python3 - <<'PY' "$OUT"
from pathlib import Path
import sys

out = Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=True)

def side(side_id, z, material="BRICK/SYNTHETIC_WALL", disp=False):
    disp_block = ''
    if disp:
        disp_block = f'''
      dispinfo {{
        "power" "2"
        "startposition" "[0 0 {z}]"
        "normals" "0 0 1 0 0 1 0 0 1"
        "distances" "0 0 0"
        "offsets" "0 0 0 0 0 0 0 0 0"
        "alphas" "0 0 0"
      }}'''
    return f'''
    side {{
      "id" "{side_id}"
      "plane" "(0 0 {z}) (64 0 {z}) (64 64 {z})"
      "material" "{material}"
      "uaxis" "[1 0 0 0] 0.25"
      "vaxis" "[0 -1 0 0] 0.25"{disp_block}
    }}'''

# Displacement-heavy synthetic fixture: 1536 dispinfo blocks, the 75% warning threshold for Source Weaver's 2048 heuristic.
parts = ['versioninfo { "editorversion" "400" }', 'world { "id" "1"']
next_id = 2
for index in range(1536):
    parts.append(f'  solid {{ "id" "{next_id}"')
    next_id += 1
    parts.append(side(next_id, index, "NATURE/SYNTHETIC_DISPLACEMENT", disp=True))
    next_id += 1
    parts.append('  }')
parts.append('}')
(out / 'displacement-heavy.vmf').write_text('\n'.join(parts) + '\n')

# Textured/material edge fixture: mixed case, slash/backslash naming, tool material, missing material, and texture axes.
(out / 'textured-material-edges.vmf').write_text(r'''
versioninfo { "editorversion" "400" }
world {
  "id" "1"
  solid {
    "id" "2"
    side { "id" "3" "plane" "(0 0 0) (64 0 0) (64 64 0)" "material" "Brick/Synthetic_Wall" "uaxis" "[1 0 0 16] 0.25" "vaxis" "[0 -1 0 8] 0.5" }
    side { "id" "4" "plane" "(0 0 64) (64 64 64) (64 0 64)" "material" "custom\\MixedCase_Detail" "uaxis" "[0 1 0 -8] 0.25" "vaxis" "[1 0 0 4] 0.25" }
    side { "id" "5" "plane" "(0 0 0) (0 0 64) (64 0 64)" "material" "TOOLS/TOOLSNODRAW" "uaxis" "[1 0 0 0] 0.25" "vaxis" "[0 0 -1 0] 0.25" }
    side { "id" "6" "plane" "(64 0 0) (64 0 64) (64 64 64)" "material" "MISSING/SYNTHETIC_ONLY" "uaxis" "[0 1 0 0] 0.25" "vaxis" "[0 0 -1 0] 0.25" }
  }
}
'''.lstrip())

# Nested/hidden visgroup and func_instance fixture pair.
(out / 'groups-instances-base.vmf').write_text(r'''
versioninfo { "editorversion" "400" }
viewsettings { "bSnapToGrid" "1" }
visgroups {
  visgroup {
    "id" "10"
    "name" "base_root"
    "visgroupid" "10"
    "visible" "1"
    visgroup { "id" "11" "name" "base_hidden_child" "visgroupid" "11" "visible" "0" }
  }
}
world {
  "id" "100"
  solid { "id" "101" side { "id" "102" "plane" "(0 0 0) (64 0 0) (64 64 0)" "material" "BRICK/BASE" } editor { "visgroupid" "11" "visgroupshown" "0" } }
}
entity { "id" "103" "classname" "info_landmark" "targetname" "lm" "origin" "0 0 0" }
'''.lstrip())
(out / 'groups-instances-incoming.vmf').write_text(r'''
versioninfo { "editorversion" "999" }
viewsettings { "bSnapToGrid" "0" }
visgroups {
  visgroup { "id" "20" "name" "incoming_root_should_not_be_top_level" "visgroupid" "20" "visible" "0" }
}
world {
  "id" "1"
  solid { "id" "2" side { "id" "3" "plane" "(128 0 0) (192 0 0) (192 64 0)" "material" "BRICK/INCOMING" } editor { "visgroupid" "20" "visgroupshown" "0" } }
}
entity { "id" "4" "classname" "info_landmark" "targetname" "lm" "origin" "128 0 0" }
entity { "id" "5" "classname" "func_instance" "targetname" "synthetic_instance" "file" "instances/synthetic_room.vmf" "origin" "128 16 0" editor { "visgroupid" "20" "visgroupshown" "0" } }
'''.lstrip())

# Large campaign heuristic pair: merged overlays exceed Source Weaver's VMF-only overlay warning threshold.
def campaign_map(path, start_id, prefix, overlay_count):
    lines = ['versioninfo { "editorversion" "400" }', 'viewsettings { "bSnapToGrid" "1" }', f'world {{ "id" "{start_id}" }}']
    entity_id = start_id + 1
    for index in range(overlay_count):
        lines.append(f'entity {{ "id" "{entity_id}" "classname" "info_overlay" "targetname" "{prefix}_overlay_{index}" "sides" "1" }}')
        entity_id += 1
    path.write_text('\n'.join(lines) + '\n')

campaign_map(out / 'large-campaign-a.vmf', 1, 'a', 300)
campaign_map(out / 'large-campaign-b.vmf', 10000, 'b', 300)
PY

"$CARGO" run -q -p sourceweaver-cli -- validate "$OUT/displacement-heavy.vmf" --json > "$OUT/displacement-heavy.validate.json"
"$CARGO" run -q -p sourceweaver-cli -- validate "$OUT/textured-material-edges.vmf" --json > "$OUT/textured-material-edges.validate.json"
"$CARGO" run -q -p sourceweaver-cli -- inspect "$OUT/textured-material-edges.vmf" > "$OUT/textured-material-edges.inspect.txt"
"$CARGO" test -q -p sourceweaver-desktop material_preview -- --nocapture > "$OUT/material-preview-tests.txt"

"$CARGO" run -q -p sourceweaver-cli -- merge \
  -o "$OUT/groups-instances-merged.vmf" \
  --landmark lm \
  "$OUT/groups-instances-base.vmf" \
  "$OUT/groups-instances-incoming.vmf" \
  > "$OUT/groups-instances.merge.txt"
"$CARGO" run -q -p sourceweaver-cli -- validate "$OUT/groups-instances-merged.vmf" --json > "$OUT/groups-instances.validate.json"

"$CARGO" run -q -p sourceweaver-cli -- merge \
  -o "$OUT/large-campaign-merged.vmf" \
  "$OUT/large-campaign-a.vmf" \
  "$OUT/large-campaign-b.vmf" \
  > "$OUT/large-campaign.merge.txt"
"$CARGO" run -q -p sourceweaver-cli -- validate "$OUT/large-campaign-merged.vmf" --json > "$OUT/large-campaign.validate.json"

python3 - <<'PY' "$OUT"
import json
from pathlib import Path
import sys
out = Path(sys.argv[1])

def load(name):
    return json.loads((out / name).read_text())

disp = load('displacement-heavy.validate.json')
assert disp['ok'] is True, disp
assert disp['complexity']['displacements'] == 1536, disp['complexity']
assert any(risk['metric'] == 'displacements' for risk in disp['complexity']['risks']), disp['complexity']

textured = load('textured-material-edges.validate.json')
assert textured['ok'] is True, textured
inspect = (out / 'textured-material-edges.inspect.txt').read_text()
assert 'worldspawn' in inspect or 'world' in inspect, inspect[:200]
material_tests = (out / 'material-preview-tests.txt').read_text()
assert 'test result: ok' in material_tests and '2 passed' in material_tests, material_tests

grouped = load('groups-instances.validate.json')
assert grouped['ok'] is True, grouped
merged_group = (out / 'groups-instances-merged.vmf').read_text()
assert 'func_instance' in merged_group
assert 'instances/synthetic_room.vmf' in merged_group
assert 'base_hidden_child' in merged_group
assert 'incoming_root_should_not_be_top_level' not in merged_group
assert '"visgroupshown" "0"' in merged_group
assert '"origin" "0 16 0"' in merged_group

large = load('large-campaign.validate.json')
assert large['ok'] is True, large
assert large['complexity']['overlays'] == 600, large['complexity']
assert any(risk['metric'] == 'overlays' for risk in large['complexity']['risks']), large['complexity']
PY

sha256sum \
  "$OUT/displacement-heavy.vmf" \
  "$OUT/textured-material-edges.vmf" \
  "$OUT/groups-instances-merged.vmf" \
  "$OUT/large-campaign-merged.vmf" \
  > "$OUT/SHA256SUMS"

printf 'Source Weaver high-risk map validation complete: %s\n' "$OUT"
cat "$OUT/SHA256SUMS"
