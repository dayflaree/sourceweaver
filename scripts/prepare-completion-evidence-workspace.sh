#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUT="${1:-/tmp/sourceweaver-completion-evidence}"
CARGO="${CARGO:-cargo}"

fail() {
  printf 'completion evidence workspace preparation failed: %s\n' "$*" >&2
  exit 1
}

usage() {
  cat >&2 <<'EOF'
Usage:
  scripts/prepare-completion-evidence-workspace.sh [output-dir]

Creates a non-repository evidence workspace seeded with Source Weaver-authored
legal fixtures, per-issue evidence bundle skeletons, redacted summaries, and
SHA-256 manifests for completion certification issues #141, #142, #145, and #146.

Set SOURCEWEAVER_COMPLETION_EVIDENCE_OVERWRITE=1 to replace an existing output
workspace.
EOF
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi

[[ -f "$ROOT/tests/fixtures/completion-certification/vmf/sourceweaver-cert-room.vmf" ]] || fail "missing synthetic VMF fixture"
[[ -f "$ROOT/tests/fixtures/completion-certification/PROVENANCE.md" ]] || fail "missing fixture provenance"
[[ -f "$ROOT/scripts/validate-external-certification-evidence.sh" ]] || fail "missing external evidence validator"

if [[ -e "$OUT" && -n "$(find "$OUT" -mindepth 1 -print -quit 2>/dev/null || true)" ]]; then
  if [[ "${SOURCEWEAVER_COMPLETION_EVIDENCE_OVERWRITE:-0}" != "1" ]]; then
    fail "output workspace already exists and is not empty: $OUT"
  fi
  rm -rf "$OUT"
fi
mkdir -p "$OUT"

COMMIT="$(git -C "$ROOT" rev-parse HEAD)"

python3 - "$ROOT" "$OUT" "$COMMIT" <<'PY'
from __future__ import annotations

import hashlib
import json
import shutil
import sys
from pathlib import Path
from textwrap import dedent

root = Path(sys.argv[1])
out = Path(sys.argv[2])
commit = sys.argv[3]
fixture_root = root / "tests" / "fixtures" / "completion-certification"
vmf_source = fixture_root / "vmf" / "sourceweaver-cert-room.vmf"
model_source = fixture_root / "model-source"
provenance = fixture_root / "PROVENANCE.md"

issue_specs = [
    {
        "issue": 141,
        "slug": "issue141-hammer-open-save",
        "tool_kind": "hammer",
        "title": "Hammer/Hammer++ open-save evidence seed",
        "fixture_paths": [(vmf_source, "input/sourceweaver-cert-room.vmf"), (provenance, "input/PROVENANCE.md")],
        "tool_version": "external Hammer or Hammer++ version pending manual evidence run",
        "command": "open input/sourceweaver-cert-room.vmf in the external editor, save a copy outside the repository, then record logs and hashes",
        "validated": "Legal synthetic VMF input and evidence bundle structure were prepared.",
        "unvalidated": "No real Hammer or Hammer++ open-save operation is recorded in this seed bundle.",
    },
    {
        "issue": 142,
        "slug": "issue142-windows-native-compile",
        "tool_kind": "compiler",
        "title": "Native Windows compiler evidence seed",
        "fixture_paths": [(vmf_source, "input/sourceweaver-cert-room.vmf"), (provenance, "input/PROVENANCE.md")],
        "tool_version": "external native Windows VBSP/VVIS/VRAD versions pending manual evidence run",
        "command": "compile input/sourceweaver-cert-room.vmf with native Windows VBSP, VVIS, and VRAD, then record logs and hashes outside the repository",
        "validated": "Legal synthetic VMF input and evidence bundle structure were prepared.",
        "unvalidated": "No native Windows VBSP, VVIS, or VRAD execution is recorded in this seed bundle.",
    },
    {
        "issue": 145,
        "slug": "issue145-runtime-map-load",
        "tool_kind": "runtime",
        "title": "Game-runtime map-load evidence seed",
        "fixture_paths": [(vmf_source, "input/sourceweaver-cert-room.vmf"), (provenance, "input/PROVENANCE.md")],
        "tool_version": "external game runtime version pending manual evidence run",
        "command": "load a legally compiled BSP derived from input/sourceweaver-cert-room.vmf in the target runtime and record sanitized logs and hashes outside the repository",
        "validated": "Legal synthetic VMF input and evidence bundle structure were prepared.",
        "unvalidated": "No real game-runtime map-load operation is recorded in this seed bundle.",
    },
    {
        "issue": 146,
        "slug": "issue146-hlmv-render",
        "tool_kind": "hlmv",
        "title": "HLMV/HLMV++ render evidence seed",
        "fixture_paths": [(model_source, "input/model-source"), (provenance, "input/PROVENANCE.md")],
        "tool_version": "external StudioMDL and HLMV/HLMV++ versions pending manual evidence run",
        "command": "compile the synthetic model source outside the repository, open the compiled model in HLMV or HLMV++, then record sanitized logs, screenshots when safe, and hashes",
        "validated": "Legal synthetic model source package and evidence bundle structure were prepared.",
        "unvalidated": "No compiled model package or rendered HLMV/HLMV++ window evidence is recorded in this seed bundle.",
    },
]

boundary = "Synthetic or legally owned fixture sources only; no proprietary game content, Steam files, external tool binaries, private assets, secrets, certificates, signing keys, or generated BSP/MDL outputs are included."


def copy_fixture(src: Path, dest: Path) -> None:
    if src.is_dir():
        if dest.exists():
            shutil.rmtree(dest)
        shutil.copytree(src, dest)
    else:
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dest)


def hash_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def all_files(path: Path) -> list[Path]:
    return sorted(child for child in path.rglob("*") if child.is_file())


def write_bundle(spec: dict[str, object]) -> None:
    bundle = out / str(spec["slug"])
    bundle.mkdir(parents=True, exist_ok=True)
    artifact_paths: list[str] = []
    for src, rel in spec["fixture_paths"]:  # type: ignore[index]
        dest = bundle / rel
        copy_fixture(Path(src), dest)
        if dest.is_dir():
            artifact_paths.extend(file.relative_to(bundle).as_posix() for file in all_files(dest))
        else:
            artifact_paths.append(dest.relative_to(bundle).as_posix())

    manifest = {
        "issue": spec["issue"],
        "sourceweaver_commit": commit,
        "tool_kind": spec["tool_kind"],
        "host_os": "workspace seed; replace with external evidence host OS during manual run",
        "external_tool_versions": [spec["tool_version"]],
        "commands": [spec["command"]],
        "artifacts": artifact_paths,
        "validated_claims": [spec["validated"]],
        "unvalidated_claims": [spec["unvalidated"]],
        "redistribution_boundary": boundary,
    }
    (bundle / "evidence-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    (bundle / "tool-versions.txt").write_text(str(spec["tool_version"]) + "\n", encoding="utf-8")
    (bundle / "commands.sh").write_text("#!/usr/bin/env bash\nset -euo pipefail\n# " + str(spec["command"]) + "\n", encoding="utf-8")
    (bundle / "commands.sh").chmod(0o755)
    (bundle / "validation-summary.md").write_text(dedent(f"""
        # {spec['title']}

        Validated now:

        - legal synthetic fixture source is present;
        - provenance notes are present;
        - evidence bundle structure is ready for the manual run;
        - all seed artifacts are covered by `SHA256SUMS`.

        Unvalidated now:

        - {spec['unvalidated']}

        Replace placeholder tool-version and command notes with exact external evidence before closing the dependent issue.
        """).lstrip(), encoding="utf-8")
    (bundle / "legal-boundary.md").write_text("# Legal boundary\n\n" + boundary + "\n", encoding="utf-8")

    hash_lines = []
    for file in all_files(bundle):
        rel = file.relative_to(bundle).as_posix()
        if rel == "SHA256SUMS":
            continue
        hash_lines.append(f"{hash_file(file)}  {rel}\n")
    (bundle / "SHA256SUMS").write_text("".join(sorted(hash_lines)), encoding="utf-8")

    manifest_copy = out / "manifests" / f"{spec['slug']}.json"
    manifest_copy.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(bundle / "evidence-manifest.json", manifest_copy)

    summary = out / "redacted-summaries" / f"{spec['slug']}.md"
    summary.parent.mkdir(parents=True, exist_ok=True)
    summary.write_text(dedent(f"""
        # {spec['title']}

        Issue: #{spec['issue']}
        Source Weaver commit: `{commit}`
        Fixture source: synthetic Source Weaver-authored files copied into `{spec['slug']}/input/`
        Legal boundary: {boundary}

        This is a seed summary for future external evidence. Replace this text with exact external tool versions, sanitized logs, artifact hashes, validated claims, and unvalidated claims after the manual run.
        """).lstrip(), encoding="utf-8")


(out / "README.md").write_text(dedent(f"""
    # Source Weaver completion evidence workspace

    Commit: `{commit}`

    This workspace was generated from repository-owned synthetic fixtures. It is safe to keep under `/tmp` and use as the staging area for external certification evidence. Keep generated BSPs, compiled model outputs, external tool binaries, proprietary content, screenshots with private content, private logs, signing keys, certificates, and secret values outside the repository.

    Seed bundles:

    - `issue141-hammer-open-save/`
    - `issue142-windows-native-compile/`
    - `issue145-runtime-map-load/`
    - `issue146-hlmv-render/`

    Shared folders:

    - `manifests/` contains manifest copies.
    - `redacted-summaries/` contains issue-comment starting points.

    Run `scripts/validate-external-certification-evidence.sh <bundle>` from the repository root after adding or replacing evidence files.
    """).lstrip(), encoding="utf-8")

for spec in issue_specs:
    write_bundle(spec)

# Workspace-wide hash list for easy archiving. Each per-issue bundle has its own SHA256SUMS as well.
hash_lines = []
for file in all_files(out):
    rel = file.relative_to(out).as_posix()
    if rel == "SHA256SUMS":
        continue
    hash_lines.append(f"{hash_file(file)}  {rel}\n")
(out / "SHA256SUMS").write_text("".join(sorted(hash_lines)), encoding="utf-8")
PY

for bundle in \
  "$OUT/issue141-hammer-open-save" \
  "$OUT/issue142-windows-native-compile" \
  "$OUT/issue145-runtime-map-load" \
  "$OUT/issue146-hlmv-render"; do
  "$ROOT/scripts/validate-external-certification-evidence.sh" "$bundle" >/dev/null
done

"$CARGO" run -q -p sourceweaver-cli -- validate \
  "$ROOT/tests/fixtures/completion-certification/vmf/sourceweaver-cert-room.vmf" \
  --rule-set hl2 \
  --json \
  > "$OUT/sourceweaver-cert-room-validation.json"

python3 -m json.tool "$OUT/sourceweaver-cert-room-validation.json" >/dev/null
sha256sum "$OUT/sourceweaver-cert-room-validation.json" >> "$OUT/SHA256SUMS"
printf 'Source Weaver completion evidence workspace prepared: %s\n' "$OUT"
find "$OUT" -maxdepth 2 -type f | sort
