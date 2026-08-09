#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

fail() {
  printf 'external certification evidence validation failed: %s\n' "$*" >&2
  exit 1
}

usage() {
  cat >&2 <<'EOF'
Usage:
  scripts/validate-external-certification-evidence.sh <evidence-bundle-dir>
  scripts/validate-external-certification-evidence.sh --self-test

The bundle validator checks evidence completeness, checksums, basic manifest shape,
claim lists, legal-boundary text, and obvious private host path leaks. It does not
prove that external Source tools succeeded.
EOF
}

run_self_test() {
  local fixture_root="$ROOT/tests/fixtures/external-certification-evidence"
  local tmp
  tmp="$(mktemp -d /tmp/sourceweaver-external-evidence-self-test.XXXXXX)"
  trap 'rm -rf "$tmp"' RETURN

  "$0" "$fixture_root/valid-runtime" >"$tmp/valid.out" 2>"$tmp/valid.err" || {
    cat "$tmp/valid.out" >&2
    cat "$tmp/valid.err" >&2
    fail "positive synthetic evidence fixture failed"
  }

  if "$0" "$fixture_root/missing-hash" >"$tmp/missing-hash.out" 2>"$tmp/missing-hash.err"; then
    cat "$tmp/missing-hash.out" >&2
    fail "missing-hash negative fixture unexpectedly passed"
  fi
  grep -q 'required file not listed in SHA256SUMS' "$tmp/missing-hash.err" || {
    cat "$tmp/missing-hash.err" >&2
    fail "missing-hash negative fixture failed for the wrong reason"
  }

  if "$0" "$fixture_root/private-path" >"$tmp/private-path.out" 2>"$tmp/private-path.err"; then
    cat "$tmp/private-path.out" >&2
    fail "private-path negative fixture unexpectedly passed"
  fi
  grep -q 'private path pattern' "$tmp/private-path.err" || {
    cat "$tmp/private-path.err" >&2
    fail "private-path negative fixture failed for the wrong reason"
  }

  printf 'external certification evidence validator self-test passed\n'
}

if [[ $# -ne 1 ]]; then
  usage
  exit 2
fi

if [[ "$1" == "--help" || "$1" == "-h" ]]; then
  usage
  exit 0
fi

if [[ "$1" == "--self-test" ]]; then
  run_self_test
  exit 0
fi

BUNDLE_INPUT="$1"
[[ -d "$BUNDLE_INPUT" ]] || fail "bundle directory does not exist: $BUNDLE_INPUT"
BUNDLE_DIR="$(cd "$BUNDLE_INPUT" && pwd -P)"

python3 - "$BUNDLE_DIR" <<'PY'
from __future__ import annotations

import hashlib
import json
import os
import re
import sys
from pathlib import Path
from typing import Any

bundle = Path(sys.argv[1])
required_files = [
    "evidence-manifest.json",
    "SHA256SUMS",
    "tool-versions.txt",
    "commands.sh",
    "validation-summary.md",
    "legal-boundary.md",
]
required_hashed_files = [name for name in required_files if name != "SHA256SUMS"]
allowed_tool_kinds = {"runtime", "hammer", "compiler", "hlmv", "signing"}
commit_re = re.compile(r"^[0-9a-fA-F]{7,40}$")
sha_line_re = re.compile(r"^([0-9a-fA-F]{64})[ \t][ \t*]?(.+)$")
private_patterns = [
    re.compile(r"(?i)(?:^|[\s\"'=])(/home/[A-Za-z0-9._-]+)(?:[/\s\"']|$)"),
    re.compile(r"(?i)(?:^|[\s\"'=])(/Users/[A-Za-z0-9._-]+)(?:[/\s\"']|$)"),
    re.compile(r"(?i)(?:^|[\s\"'=])([A-Z]:\\Users\\[A-Za-z0-9._-]+)(?:[\\\s\"']|$)"),
    re.compile(r"(?i)(?:^|[\s\"'=])(/root)(?:[/\s\"']|$)"),
    re.compile(r"(?i)(?:^|[\s\"'=])(~[/\\][^\s\"']*)"),
    re.compile(r"(?i)(?:^|[\s\"'=])(\$HOME[/\\][^\s\"']*)"),
]

errors: list[str] = []


def add_error(message: str) -> None:
    errors.append(message)


def rel(path: Path) -> str:
    try:
        return path.relative_to(bundle).as_posix()
    except ValueError:
        return path.as_posix()


for name in required_files:
    path = bundle / name
    if not path.is_file():
        add_error(f"required file missing: {name}")
    elif name != "SHA256SUMS" and path.stat().st_size == 0:
        add_error(f"required file is empty: {name}")

manifest: dict[str, Any] | None = None
manifest_path = bundle / "evidence-manifest.json"
if manifest_path.is_file():
    try:
        loaded = json.loads(manifest_path.read_text(encoding="utf-8"))
        if isinstance(loaded, dict):
            manifest = loaded
        else:
            add_error("evidence-manifest.json must contain a JSON object")
    except json.JSONDecodeError as error:
        add_error(f"evidence-manifest.json is invalid JSON: {error}")


def nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def nonempty_list(value: Any) -> bool:
    if not isinstance(value, list) or not value:
        return False
    for item in value:
        if item is None:
            return False
        if isinstance(item, str) and not item.strip():
            return False
        if isinstance(item, (dict, list)) and not item:
            return False
    return True


if manifest is not None:
    issue = manifest.get("issue")
    if not isinstance(issue, int) or issue <= 0:
        add_error("manifest field issue must be a positive integer")

    commit = manifest.get("sourceweaver_commit")
    if not nonempty_string(commit) or not commit_re.fullmatch(str(commit).strip()):
        add_error("manifest field sourceweaver_commit must be a 7-40 character hex commit")

    tool_kind = manifest.get("tool_kind")
    if not nonempty_string(tool_kind) or str(tool_kind).strip() not in allowed_tool_kinds:
        add_error("manifest field tool_kind must be one of runtime, hammer, compiler, hlmv, signing")

    if not nonempty_string(manifest.get("host_os")):
        add_error("manifest field host_os must be nonempty")

    if not nonempty_list(manifest.get("external_tool_versions")):
        add_error("manifest field external_tool_versions must be a nonempty list")

    if not nonempty_list(manifest.get("commands")):
        add_error("manifest field commands must be a nonempty list")

    artifacts = manifest.get("artifacts")
    if not isinstance(artifacts, list):
        add_error("manifest field artifacts must be a list")

    if not nonempty_list(manifest.get("validated_claims")):
        add_error("manifest field validated_claims must be a nonempty list")

    if not nonempty_list(manifest.get("unvalidated_claims")):
        add_error("manifest field unvalidated_claims must be a nonempty list")

    if not nonempty_string(manifest.get("redistribution_boundary")):
        add_error("manifest field redistribution_boundary must be nonempty")

    if nonempty_string(manifest.get("redistribution_boundary")):
        boundary_file = bundle / "legal-boundary.md"
        if boundary_file.is_file() and not boundary_file.read_text(encoding="utf-8", errors="replace").strip():
            add_error("legal-boundary.md must contain the redistribution/legal boundary text")

sha_path = bundle / "SHA256SUMS"
sha_entries: dict[str, str] = {}
if sha_path.is_file():
    raw_sha = sha_path.read_text(encoding="utf-8", errors="replace").splitlines()
    if not raw_sha:
        add_error("SHA256SUMS is empty")
    for line_number, line in enumerate(raw_sha, start=1):
        if not line.strip():
            continue
        match = sha_line_re.match(line)
        if not match:
            add_error(f"SHA256SUMS:{line_number}: invalid sha256sum line")
            continue
        expected_hash, listed_name = match.groups()
        listed_name = listed_name.strip()
        if listed_name.startswith("*"):
            listed_name = listed_name[1:]
        listed_path = Path(listed_name)
        if listed_path.is_absolute():
            add_error(f"SHA256SUMS:{line_number}: listed path must be relative: {listed_name}")
            continue
        if listed_path == Path("SHA256SUMS"):
            add_error(f"SHA256SUMS:{line_number}: SHA256SUMS must not list itself")
            continue
        if ".." in listed_path.parts:
            add_error(f"SHA256SUMS:{line_number}: listed path must not contain '..': {listed_name}")
            continue
        target = bundle / listed_path
        if not target.is_file():
            add_error(f"SHA256SUMS:{line_number}: listed file does not exist: {listed_name}")
            continue
        actual_hash = hashlib.sha256(target.read_bytes()).hexdigest()
        if actual_hash.lower() != expected_hash.lower():
            add_error(f"SHA256SUMS:{line_number}: hash mismatch for {listed_name}")
        sha_entries[listed_path.as_posix()] = expected_hash.lower()

for name in required_hashed_files:
    if name not in sha_entries:
        add_error(f"required file not listed in SHA256SUMS: {name}")

if manifest is not None and isinstance(manifest.get("artifacts"), list):
    for index, artifact in enumerate(manifest["artifacts"], start=1):
        artifact_path: str | None = None
        if isinstance(artifact, str):
            artifact_path = artifact
        elif isinstance(artifact, dict):
            raw_path = artifact.get("path") or artifact.get("file") or artifact.get("name")
            if isinstance(raw_path, str):
                artifact_path = raw_path
        if artifact_path:
            artifact_rel = Path(artifact_path)
            if artifact_rel.is_absolute() or ".." in artifact_rel.parts:
                add_error(f"manifest artifacts[{index}] path must be relative inside the bundle: {artifact_path}")
            elif artifact_rel.as_posix() not in sha_entries:
                add_error(f"manifest artifacts[{index}] is not listed in SHA256SUMS: {artifact_path}")

scan_files: set[Path] = set()
for name in required_files:
    path = bundle / name
    if path.is_file():
        scan_files.add(path)
for listed_name in sha_entries:
    path = bundle / listed_name
    if path.is_file() and path.stat().st_size <= 2_000_000:
        scan_files.add(path)

for path in sorted(scan_files):
    try:
        text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        continue
    for line_number, line in enumerate(text.splitlines(), start=1):
        for pattern in private_patterns:
            match = pattern.search(line)
            if match:
                add_error(f"{rel(path)}:{line_number}: private path pattern found: {match.group(1)}")
                break

if errors:
    for error in errors:
        print(error, file=sys.stderr)
    raise SystemExit(1)

print(f"external certification evidence structure passed: {bundle.name}")
PY

python3 "$ROOT/scripts/check-validation-claims.py"
printf 'Source Weaver external evidence validation passed: %s\n' "$BUNDLE_INPUT"
