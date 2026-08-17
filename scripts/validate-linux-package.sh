#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-${GITHUB_REF_NAME:-dev}}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGE_NAME="sourceweaver-${VERSION}-linux-x86_64"
ARCHIVE="${2:-$ROOT/target/package/${PACKAGE_NAME}.tar.gz}"
SMOKE_DIR="${SOURCEWEAVER_LINUX_SMOKE_DIR:-}"
CLEANUP_SMOKE=0

if [[ -z "$SMOKE_DIR" ]]; then
  SMOKE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/sourceweaver-linux-package-smoke.XXXXXX")"
  CLEANUP_SMOKE=1
fi

cleanup() {
  if [[ "$CLEANUP_SMOKE" == "1" ]]; then
    rm -rf "$SMOKE_DIR"
  fi
}
trap cleanup EXIT

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    printf 'missing required package file: %s\n' "$path" >&2
    exit 1
  fi
}

require_executable() {
  local path="$1"
  require_file "$path"
  if [[ ! -x "$path" ]]; then
    printf 'package file is not executable: %s\n' "$path" >&2
    exit 1
  fi
}

validate_desktop_file() {
  local path="$1"
  require_file "$path"
  if command -v desktop-file-validate >/dev/null 2>&1; then
    desktop-file-validate "$path"
  fi
}

validate_headless_display_diagnostic() {
  local executable="$1"
  local stdout_path="$SMOKE_DIR/check-display.stdout"
  local stderr_path="$SMOKE_DIR/check-display.stderr"
  local status

  set +e
  env -u DISPLAY -u WAYLAND_DISPLAY -u WAYLAND_SOCKET \
    "$executable" --check-display > "$stdout_path" 2> "$stderr_path"
  status=$?
  set -e

  if [[ "$status" != "2" ]]; then
    printf 'expected no-display diagnostic exit 2 from %s, got %s\n' "$executable" "$status" >&2
    cat "$stdout_path" >&2 || true
    cat "$stderr_path" >&2 || true
    exit 1
  fi
  grep -Fq 'Source Weaver Desktop needs a graphical Linux session.' "$stderr_path"
  grep -Fq 'DISPLAY, WAYLAND_DISPLAY, or WAYLAND_SOCKET' "$stderr_path"
}

validate_virtual_display_startup() {
  local executable="$1"
  if ! command -v Xvfb >/dev/null 2>&1 || ! command -v xwininfo >/dev/null 2>&1; then
    printf 'Skipping virtual display GUI startup probe; Xvfb or xwininfo is unavailable.\n'
    return
  fi

  local display_number="${SOURCEWEAVER_XVFB_DISPLAY:-:$((200 + (RANDOM % 300)))}"
  local xvfb_stdout="$SMOKE_DIR/xvfb.stdout"
  local xvfb_stderr="$SMOKE_DIR/xvfb.stderr"
  local app_stdout="$SMOKE_DIR/gui-startup.stdout"
  local app_stderr="$SMOKE_DIR/gui-startup.stderr"
  local xvfb_pid=""
  local app_pid=""
  local window_found=0

  Xvfb "$display_number" -screen 0 1280x800x24 > "$xvfb_stdout" 2> "$xvfb_stderr" &
  xvfb_pid=$!

  cleanup_virtual_display() {
    set +e
    if [[ -n "$app_pid" ]]; then
      kill "$app_pid" 2>/dev/null || true
      wait "$app_pid" 2>/dev/null || true
    fi
    if [[ -n "$xvfb_pid" ]]; then
      kill "$xvfb_pid" 2>/dev/null || true
      wait "$xvfb_pid" 2>/dev/null || true
    fi
    set -e
  }

  sleep 1
  DISPLAY="$display_number" "$executable" > "$app_stdout" 2> "$app_stderr" &
  app_pid=$!

  for _ in $(seq 1 30); do
    if ! kill -0 "$app_pid" 2>/dev/null; then
      printf 'Source Weaver exited before creating a GUI window.\n' >&2
      cat "$app_stdout" >&2 || true
      cat "$app_stderr" >&2 || true
      cleanup_virtual_display
      exit 1
    fi
    if DISPLAY="$display_number" xwininfo -root -tree 2>/dev/null | grep -Fq '"Source Weaver"'; then
      window_found=1
      break
    fi
    sleep 0.5
  done

  cleanup_virtual_display

  if [[ "$window_found" != "1" ]]; then
    printf 'Source Weaver did not create a visible X window under Xvfb.\n' >&2
    cat "$app_stdout" >&2 || true
    cat "$app_stderr" >&2 || true
    exit 1
  fi
}

if [[ ! -f "$ARCHIVE" ]]; then
  printf 'Linux package archive not found: %s\n' "$ARCHIVE" >&2
  printf 'Run scripts/package-linux.sh %s first.\n' "$VERSION" >&2
  exit 1
fi

rm -rf "$SMOKE_DIR/extract" "$SMOKE_DIR/home with spaces" "$SMOKE_DIR/xdg data"
mkdir -p "$SMOKE_DIR/extract" "$SMOKE_DIR/home with spaces" "$SMOKE_DIR/xdg data"

tar -xzf "$ARCHIVE" -C "$SMOKE_DIR/extract"
PKG="$SMOKE_DIR/extract/$PACKAGE_NAME"

require_executable "$PKG/SourceWeaver"
require_executable "$PKG/SourceWeaver.desktop"
require_executable "$PKG/install-linux.sh"
require_executable "$PKG/bin/sourceweaver"
require_executable "$PKG/bin/sourceweaver-desktop"
require_file "$PKG/share/icons/hicolor/scalable/apps/sourceweaver.svg"
require_file "$PKG/RUNNING_ON_LINUX.md"
require_file "$PKG/README.md"
require_file "$PKG/LICENSE"
validate_desktop_file "$PKG/SourceWeaver.desktop"
validate_desktop_file "$PKG/share/applications/io.github.dayflaree.SourceWeaver.desktop"

"$PKG/bin/sourceweaver" --help >/dev/null
"$PKG/bin/sourceweaver-desktop" --help >/dev/null
validate_headless_display_diagnostic "$PKG/bin/sourceweaver-desktop"
validate_virtual_display_startup "$PKG/SourceWeaver"

INSTALL_HOME="$SMOKE_DIR/home with spaces"
XDG_DATA_HOME="$SMOKE_DIR/xdg data"
HOME="$INSTALL_HOME" XDG_DATA_HOME="$XDG_DATA_HOME" "$PKG/install-linux.sh" > "$SMOKE_DIR/install.log"

INSTALLED_DIR="$XDG_DATA_HOME/sourceweaver"
require_executable "$INSTALLED_DIR/SourceWeaver"
require_executable "$INSTALLED_DIR/bin/sourceweaver"
require_executable "$INSTALLED_DIR/bin/sourceweaver-desktop"
require_file "$INSTALLED_DIR/share/icons/hicolor/scalable/apps/sourceweaver.svg"
validate_desktop_file "$XDG_DATA_HOME/applications/io.github.dayflaree.SourceWeaver.desktop"

[[ -L "$INSTALL_HOME/.local/bin/sourceweaver" ]]
[[ -L "$INSTALL_HOME/.local/bin/sourceweaver-desktop" ]]
HOME="$INSTALL_HOME" XDG_DATA_HOME="$XDG_DATA_HOME" "$INSTALL_HOME/.local/bin/sourceweaver" --help >/dev/null
HOME="$INSTALL_HOME" XDG_DATA_HOME="$XDG_DATA_HOME" "$INSTALL_HOME/.local/bin/sourceweaver-desktop" --help >/dev/null
validate_headless_display_diagnostic "$INSTALL_HOME/.local/bin/sourceweaver-desktop"

grep -Fq "Exec=\"$INSTALLED_DIR/bin/sourceweaver-desktop\"" \
  "$XDG_DATA_HOME/applications/io.github.dayflaree.SourceWeaver.desktop"

printf 'Linux package smoke test passed: %s\n' "$ARCHIVE"
