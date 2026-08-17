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

grep -Fq "Exec=\"$INSTALLED_DIR/bin/sourceweaver-desktop\"" \
  "$XDG_DATA_HOME/applications/io.github.dayflaree.SourceWeaver.desktop"

printf 'Linux package smoke test passed: %s\n' "$ARCHIVE"
