#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUT="${1:-/tmp/sourceweaver-completion-prerequisites}"
REQUIRE_READY="${SOURCEWEAVER_COMPLETION_PREREQS_REQUIRE_READY:-0}"

fail() {
  printf 'completion prerequisite probe failed: %s\n' "$*" >&2
  exit 1
}

usage() {
  cat >&2 <<'EOF'
Usage:
  scripts/probe-completion-prerequisites.sh [output-dir]

Creates a non-repository prerequisite probe workspace for the remaining external
completion blockers:

  #156 external Source GUI/runtime evidence workstation
  #157 native Windows Source compiler certification host
  #158 production signing credentials and repository names

The probe records names, paths after HOME redaction, tool presence, environment
shape, repository secret/variable names when visible to gh, and SHA-256 manifests.
It never prints secret values and does not copy external tools, game content,
SDK files, screenshots, BSPs, MDLs, certificates, or private keys.

Set SOURCEWEAVER_COMPLETION_PREREQS_OVERWRITE=1 to replace an existing output
workspace.
Set SOURCEWEAVER_COMPLETION_PREREQS_REQUIRE_READY=1 to exit nonzero when any
prerequisite set is still blocked.
EOF
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi

if [[ -e "$OUT" && -n "$(find "$OUT" -mindepth 1 -print -quit 2>/dev/null || true)" ]]; then
  if [[ "${SOURCEWEAVER_COMPLETION_PREREQS_OVERWRITE:-0}" != "1" ]]; then
    fail "output workspace already exists and is not empty: $OUT"
  fi
  rm -rf "$OUT"
fi
mkdir -p "$OUT"

COMMIT="$(git -C "$ROOT" rev-parse HEAD)"
TIMESTAMP_UTC="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
HOME_DISPLAY="\${HOME}"

sanitize() {
  local value="$*"
  if [[ -n "${HOME:-}" ]]; then
    value="${value//${HOME}/${HOME_DISPLAY}}"
  fi
  printf '%s' "$value"
}

write_header() {
  local title="$1"
  {
    printf '# %s\n\n' "$title"
    printf 'timestamp_utc=%s\n' "$TIMESTAMP_UTC"
    printf 'sourceweaver_commit=%s\n' "$COMMIT"
    printf 'repo=%s\n' "$(sanitize "$ROOT")"
    printf '\n'
  }
}

command_path() {
  local name="$1"
  command -v "$name" 2>/dev/null || true
}

command_version() {
  local cmd="$1"
  if [[ -z "$cmd" ]]; then
    return 0
  fi
  case "$(basename "$cmd" | tr '[:upper:]' '[:lower:]')" in
    gpg)
      "$cmd" --version 2>/dev/null | head -n 1 || true
      ;;
    gh)
      "$cmd" --version 2>/dev/null | head -n 1 || true
      ;;
    git)
      "$cmd" --version 2>/dev/null | head -n 1 || true
      ;;
    rustc|cargo|wine|wine64|steamcmd|pwsh|powershell|powershell.exe)
      "$cmd" --version 2>/dev/null | head -n 1 || true
      ;;
    *)
      return 0
      ;;
  esac
}

record_tool_presence() {
  local output="$1"
  shift
  for name in "$@"; do
    local path
    path="$(command_path "$name")"
    if [[ -n "$path" ]]; then
      printf '%s: present at %s\n' "$name" "$(sanitize "$path")" >> "$output"
      local version
      version="$(command_version "$path")"
      if [[ -n "$version" ]]; then
        printf '%s version: %s\n' "$name" "$(sanitize "$version")" >> "$output"
      fi
    else
      printf '%s: absent from PATH\n' "$name" >> "$output"
    fi
  done
}

find_named_candidates() {
  local output="$1"
  shift
  if [[ -z "${HOME:-}" || ! -d "$HOME" ]]; then
    printf 'HOME search skipped: HOME is not a directory\n' >> "$output"
    return 0
  fi
  printf 'HOME search root: %s\n' "$(sanitize "$HOME")" >> "$output"
  local find_expr=()
  for pattern in "$@"; do
    if (( ${#find_expr[@]} > 0 )); then
      find_expr+=( -o )
    fi
    find_expr+=( -iname "$pattern" )
  done
  local tmp
  tmp="$(mktemp)"
  if timeout 30s find "$HOME" -xdev -type f \( "${find_expr[@]}" \) -print 2>/dev/null | head -n 80 > "$tmp"; then
    if [[ -s "$tmp" ]]; then
      while IFS= read -r line; do
        printf 'candidate: %s\n' "$(sanitize "$line")" >> "$output"
      done < "$tmp"
    else
      printf 'candidate search: no matches\n' >> "$output"
    fi
  else
    printf 'candidate search: timed out or unavailable after 30 seconds\n' >> "$output"
  fi
  rm -f "$tmp"
}

secret_names_json() {
  if command -v gh >/dev/null 2>&1; then
    gh secret list --repo dayflaree/sourceweaver --json name --jq '[.[].name]' 2>/dev/null || printf 'null\n'
  else
    printf 'null\n'
  fi
}

variable_names_json() {
  if command -v gh >/dev/null 2>&1; then
    gh variable list --repo dayflaree/sourceweaver --json name --jq '[.[].name]' 2>/dev/null || printf 'null\n'
  else
    printf 'null\n'
  fi
}

write_sha256sums() {
  local dir="$1"
  (
    cd "$dir"
    find . -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 sha256sum > SHA256SUMS
  )
}

issue156_dir="$OUT/issue156-gui-runtime-workstation"
issue157_dir="$OUT/issue157-native-windows-host"
issue158_dir="$OUT/issue158-signing-provisioning"
mkdir -p "$issue156_dir" "$issue157_dir" "$issue158_dir"

issue156_probe="$issue156_dir/probe.txt"
{
  write_header 'Issue #156 Source GUI/runtime workstation prerequisite probe'
  printf 'DISPLAY=%s\n' "${DISPLAY:-absent}"
  printf 'WAYLAND_DISPLAY=%s\n' "${WAYLAND_DISPLAY:-absent}"
  printf 'XDG_SESSION_TYPE=%s\n' "${XDG_SESSION_TYPE:-absent}"
  printf 'DESKTOP_SESSION=%s\n' "${DESKTOP_SESSION:-absent}"
  printf 'OS=%s\n' "$(uname -a)"
  if [[ -r /etc/os-release ]]; then
    printf 'os-release:\n'
    sed -n '1,8p' /etc/os-release
  fi
  printf '\nPATH tool presence:\n'
} > "$issue156_probe"
record_tool_presence "$issue156_probe" wine wine64 protontricks steam steamcmd hammer hammer.exe hammerplusplus.exe hlmv hlmv.exe hlmvplusplus.exe sourceweaver >> "$issue156_probe"
{
  printf '\nHOME candidate search for external GUI/runtime tools:\n'
} >> "$issue156_probe"
find_named_candidates "$issue156_probe" 'hammer.exe' 'hammerplusplus.exe' 'hlmv.exe' 'hlmvplusplus.exe' 'hlmv*.exe' 'hlfaceposer.exe' 'hl2.exe' 'gmod.exe' 'srcds.exe' 'srcds_linux'

issue157_probe="$issue157_dir/probe.txt"
{
  write_header 'Issue #157 native Windows compiler host prerequisite probe'
  printf 'OS=%s\n' "$(uname -a)"
  if [[ -r /etc/os-release ]]; then
    printf 'os-release:\n'
    sed -n '1,8p' /etc/os-release
  fi
  printf '\nPATH tool presence:\n'
} > "$issue157_probe"
record_tool_presence "$issue157_probe" powershell.exe pwsh cmd.exe vbsp vbsp.exe vvis vvis.exe vrad vrad.exe wine wine64 steam steamcmd sourceweaver >> "$issue157_probe"
{
  printf '\nHOME candidate search for Source compiler tools:\n'
} >> "$issue157_probe"
find_named_candidates "$issue157_probe" 'vbsp.exe' 'vvis.exe' 'vrad.exe' 'vbsp' 'vvis' 'vrad'

issue158_probe="$issue158_dir/probe.txt"
secret_json_file="$issue158_dir/repository-secret-names.json"
variable_json_file="$issue158_dir/repository-variable-names.json"
secret_names_json > "$secret_json_file"
variable_names_json > "$variable_json_file"
{
  write_header 'Issue #158 production signing provisioning prerequisite probe'
  printf 'Workflow final-mode required names from scripts/validate-final-release-environment.sh and desktop-builds.yml:\n'
  printf 'SOURCEWEAVER_WINDOWS_SIGNING_PFX_BASE64\n'
  printf 'SOURCEWEAVER_WINDOWS_SIGNING_PFX_PASSWORD\n'
  printf 'SOURCEWEAVER_WINDOWS_TIMESTAMP_URL\n'
  printf 'SOURCEWEAVER_WINDOWS_SIGNTOOL\n'
  printf 'SOURCEWEAVER_GPG_PRIVATE_KEY_BASE64\n'
  printf 'SOURCEWEAVER_UPDATE_SIGNING_KEY_BASE64\n'
  printf '\nRepository secret names visible to gh are stored in repository-secret-names.json. Values are never requested.\n'
  printf 'Repository variable names visible to gh are stored in repository-variable-names.json. Values are never requested.\n'
  printf '\nLocal environment presence, names only:\n'
  for name in \
    SOURCEWEAVER_WINDOWS_SIGNING_PFX_BASE64 \
    SOURCEWEAVER_WINDOWS_SIGNING_PFX_PASSWORD \
    SOURCEWEAVER_WINDOWS_TIMESTAMP_URL \
    SOURCEWEAVER_WINDOWS_SIGNTOOL \
    SOURCEWEAVER_GPG_PRIVATE_KEY_BASE64 \
    SOURCEWEAVER_GPG_PASSPHRASE \
    SOURCEWEAVER_UPDATE_SIGNING_KEY_BASE64; do
    if [[ -n "${!name:-}" ]]; then
      printf '%s=present\n' "$name"
    else
      printf '%s=absent\n' "$name"
    fi
  done
  printf '\nLocal signing tool presence:\n'
} > "$issue158_probe"
record_tool_presence "$issue158_probe" signtool signtool.exe osslsigncode gpg gh >> "$issue158_probe"
{
  printf '\nFinal-release policy dry run:\n'
  if "$ROOT/scripts/validate-final-release-policy.sh" >/tmp/sourceweaver-prereq-policy.out 2>&1; then
    printf 'scripts/validate-final-release-policy.sh: success\n'
  else
    printf 'scripts/validate-final-release-policy.sh: failed\n'
    sed -n '1,80p' /tmp/sourceweaver-prereq-policy.out
  fi
  rm -f /tmp/sourceweaver-prereq-policy.out
} >> "$issue158_probe"

python3 - "$ROOT" "$OUT" "$COMMIT" "$TIMESTAMP_UTC" "$secret_json_file" "$variable_json_file" <<'PY'
from __future__ import annotations

import json
import os
import platform
import shutil
import sys
from pathlib import Path

root = Path(sys.argv[1])
out = Path(sys.argv[2])
commit = sys.argv[3]
timestamp = sys.argv[4]
secret_path = Path(sys.argv[5])
variable_path = Path(sys.argv[6])

def load_json(path: Path):
    try:
        return json.loads(path.read_text())
    except Exception:
        return None

def which_any(names: list[str]) -> list[str]:
    return [name for name in names if shutil.which(name)]

secret_names = load_json(secret_path)
variable_names = load_json(variable_path)
if not isinstance(secret_names, list):
    secret_names = []
if not isinstance(variable_names, list):
    variable_names = []

has_display = bool(os.environ.get("DISPLAY") or os.environ.get("WAYLAND_DISPLAY"))
has_gui_runner = bool(which_any(["wine", "wine64", "protontricks", "steam", "steamcmd"]))
has_hammer = bool(which_any(["hammer", "hammer.exe", "hammerplusplus.exe"]))
has_hlmv = bool(which_any(["hlmv", "hlmv.exe", "hlmvplusplus.exe"]))
has_runtime = bool(which_any(["steam", "steamcmd", "srcds", "srcds_linux"]))
issue156_ready = has_display and has_gui_runner and has_hammer and has_hlmv and has_runtime

system = platform.system().lower()
has_native_windows_process = system == "windows" or bool(shutil.which("cmd.exe") or shutil.which("powershell.exe"))
has_compilers = all(shutil.which(name) for name in ["vbsp.exe", "vvis.exe", "vrad.exe"])
issue157_ready = has_native_windows_process and has_compilers

required_secrets = {
    "SOURCEWEAVER_WINDOWS_SIGNING_PFX_BASE64",
    "SOURCEWEAVER_WINDOWS_SIGNING_PFX_PASSWORD",
    "SOURCEWEAVER_GPG_PRIVATE_KEY_BASE64",
    "SOURCEWEAVER_UPDATE_SIGNING_KEY_BASE64",
}
required_variables = {
    "SOURCEWEAVER_WINDOWS_TIMESTAMP_URL",
    "SOURCEWEAVER_WINDOWS_SIGNTOOL",
}
secret_set = set(secret_names)
variable_set = set(variable_names)
env_or_repo_secrets = {name for name in required_secrets if name in secret_set or os.environ.get(name)}
env_or_repo_variables = {name for name in required_variables if name in variable_set or os.environ.get(name)}
issue158_ready = required_secrets <= env_or_repo_secrets and required_variables <= env_or_repo_variables

summary = {
    "timestamp_utc": timestamp,
    "sourceweaver_commit": commit,
    "issues": {
        "156": {
            "title": "Provision external Source GUI/runtime evidence workstation",
            "ready": issue156_ready,
            "checks": {
                "display_or_wayland_present": has_display,
                "gui_runner_present_on_path": has_gui_runner,
                "hammer_or_hammerplusplus_present_on_path": has_hammer,
                "hlmv_or_hlmvplusplus_present_on_path": has_hlmv,
                "runtime_or_steam_tool_present_on_path": has_runtime,
            },
            "boundary": "Probe only. It does not launch GUI tools, copy proprietary content, or certify external-tool behavior.",
        },
        "157": {
            "title": "Provision native Windows Source compiler certification host",
            "ready": issue157_ready,
            "checks": {
                "native_windows_process_context_detected": has_native_windows_process,
                "vbsp_vvis_vrad_exe_present_on_path": has_compilers,
            },
            "boundary": "Probe only. It does not run VBSP/VVIS/VRAD and does not certify native Windows compile behavior.",
        },
        "158": {
            "title": "Provision production signing credentials and repository secrets",
            "ready": issue158_ready,
            "checks": {
                "required_secret_names_present_or_env_set": sorted(env_or_repo_secrets),
                "missing_secret_names": sorted(required_secrets - env_or_repo_secrets),
                "required_variable_names_present_or_env_set": sorted(env_or_repo_variables),
                "missing_variable_names": sorted(required_variables - env_or_repo_variables),
            },
            "boundary": "Probe only. It lists names and local presence flags, never secret values or key material.",
        },
    },
}
(out / "prerequisite-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
(out / "README.md").write_text(
    "# Source Weaver completion prerequisite probe\n\n"
    f"Commit: `{commit}`\n\n"
    "This workspace records the current prerequisite state for #156, #157, and #158. "
    "It is safe to cite as blocker evidence because private paths are redacted to `${HOME}`, "
    "repository signing values are never requested, and external binaries or game assets are not copied.\n\n"
    "Useful files:\n\n"
    "- `prerequisite-summary.json` machine-readable readiness summary.\n"
    "- `issue156-gui-runtime-workstation/probe.txt` GUI/runtime workstation probe.\n"
    "- `issue157-native-windows-host/probe.txt` native Windows compiler host probe.\n"
    "- `issue158-signing-provisioning/probe.txt` signing-name and local tool probe.\n"
    "- `issue158-signing-provisioning/repository-secret-names.json` repository secret names visible to `gh`; no values.\n"
    "- `issue158-signing-provisioning/repository-variable-names.json` repository variable names visible to `gh`; no values.\n"
    "- `SHA256SUMS` hash manifest.\n"
)
PY

for dir in "$issue156_dir" "$issue157_dir" "$issue158_dir"; do
  write_sha256sums "$dir"
done
write_sha256sums "$OUT"

if [[ "$REQUIRE_READY" == "1" ]]; then
  python3 - "$OUT/prerequisite-summary.json" <<'PY'
import json
import sys
from pathlib import Path
summary = json.loads(Path(sys.argv[1]).read_text())
blocked = [num for num, info in summary["issues"].items() if not info["ready"]]
if blocked:
    print("completion prerequisites still blocked: " + ", ".join(f"#{num}" for num in blocked), file=sys.stderr)
    sys.exit(1)
PY
fi

printf 'Source Weaver completion prerequisite probe written: %s\n' "$OUT"
find "$OUT" -maxdepth 2 -type f | sort
