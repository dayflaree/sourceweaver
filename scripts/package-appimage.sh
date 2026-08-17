#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-${GITHUB_REF_NAME:-dev}}"
MODE="${2:-}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARCH="${ARCH:-x86_64}"
PACKAGE_ROOT="$ROOT/target/package"
APPDIR="$PACKAGE_ROOT/SourceWeaver.AppDir"
APPIMAGE="$PACKAGE_ROOT/sourceweaver-${VERSION}-linux-${ARCH}.AppImage"
if [[ -n "${APPIMAGETOOL:-}" ]]; then
  APPIMAGETOOL_PATH="$APPIMAGETOOL"
elif [[ -x "$ROOT/target/tools/appimagetool-${ARCH}.AppImage" ]]; then
  APPIMAGETOOL_PATH="$ROOT/target/tools/appimagetool-${ARCH}.AppImage"
else
  APPIMAGETOOL_PATH="$PACKAGE_ROOT/appimagetool-${ARCH}.AppImage"
fi

case "$MODE" in
  ""|--appdir-only) ;;
  *)
    printf 'usage: %s [version] [--appdir-only]\n' "$0" >&2
    exit 2
    ;;
esac

rm -rf "$APPDIR" "$APPIMAGE"
mkdir -p \
  "$APPDIR/usr/bin" \
  "$APPDIR/usr/share/applications" \
  "$APPDIR/usr/share/icons/hicolor/scalable/apps" \
  "$APPDIR/usr/share/sourceweaver/docs"

cargo build --release -p sourceweaver-cli -p sourceweaver-desktop
install -Dm755 "$ROOT/target/release/sourceweaver" "$APPDIR/usr/bin/sourceweaver"
install -Dm755 "$ROOT/target/release/sourceweaver-desktop" "$APPDIR/usr/bin/sourceweaver-desktop"
install -Dm644 "$ROOT/LICENSE" "$APPDIR/usr/share/sourceweaver/LICENSE"
install -Dm644 "$ROOT/README.md" "$APPDIR/usr/share/sourceweaver/README.md"
cp -R "$ROOT/docs/." "$APPDIR/usr/share/sourceweaver/docs/"
install -Dm644 "$ROOT/packaging/linux/sourceweaver.svg" "$APPDIR/usr/share/icons/hicolor/scalable/apps/sourceweaver.svg"
install -Dm644 "$ROOT/packaging/linux/sourceweaver.svg" "$APPDIR/sourceweaver.svg"

cat > "$APPDIR/io.github.dayflaree.SourceWeaver.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Source Weaver
Comment=Merge, preview, and validate Source Engine VMFs
Exec=sourceweaver-desktop
Icon=sourceweaver
Terminal=false
Categories=Development;
Keywords=Source;Hammer;VMF;Half-Life;Black Mesa;Map;
StartupWMClass=Source Weaver
DESKTOP
install -Dm644 "$APPDIR/io.github.dayflaree.SourceWeaver.desktop" "$APPDIR/usr/share/applications/io.github.dayflaree.SourceWeaver.desktop"

cat > "$APPDIR/AppRun" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
APPDIR="${APPDIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"
export PATH="$APPDIR/usr/bin:$PATH"
export XDG_DATA_DIRS="$APPDIR/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
exec "$APPDIR/usr/bin/sourceweaver-desktop" "$@"
SH
chmod +x "$APPDIR/AppRun"

# Optional validation tools are present on GitHub-hosted Ubuntu after apt install.
if command -v desktop-file-validate >/dev/null 2>&1; then
  desktop-file-validate "$APPDIR/io.github.dayflaree.SourceWeaver.desktop"
  desktop-file-validate "$APPDIR/usr/share/applications/io.github.dayflaree.SourceWeaver.desktop"
fi

"$APPDIR/usr/bin/sourceweaver" --help >/dev/null
"$APPDIR/usr/bin/sourceweaver-desktop" --help >/dev/null

set +e
env -u DISPLAY -u WAYLAND_DISPLAY -u WAYLAND_SOCKET \
  "$APPDIR/usr/bin/sourceweaver-desktop" --check-display > "$PACKAGE_ROOT/appdir-check-display.stdout" 2> "$PACKAGE_ROOT/appdir-check-display.stderr"
display_check_status=$?
set -e
if [[ "$display_check_status" != "2" ]]; then
  printf 'expected AppDir no-display diagnostic exit 2, got %s\n' "$display_check_status" >&2
  cat "$PACKAGE_ROOT/appdir-check-display.stdout" >&2 || true
  cat "$PACKAGE_ROOT/appdir-check-display.stderr" >&2 || true
  exit 1
fi
grep -Fq 'Source Weaver Desktop needs a graphical Linux session.' "$PACKAGE_ROOT/appdir-check-display.stderr"

echo "AppDir: $APPDIR"

if [[ "$MODE" == "--appdir-only" ]]; then
  exit 0
fi

if [[ ! -x "$APPIMAGETOOL_PATH" ]]; then
  printf 'appimagetool not found or not executable at %s\n' "$APPIMAGETOOL_PATH" >&2
  printf 'The script also checks %s when APPIMAGETOOL is unset.\n' "$ROOT/target/tools/appimagetool-${ARCH}.AppImage" >&2
  printf 'Set APPIMAGETOOL=/path/to/appimagetool-x86_64.AppImage or run with --appdir-only.\n' >&2
  exit 1
fi

# appimagetool expects ARCH for reproducible architecture naming.
ARCH="$ARCH" "$APPIMAGETOOL_PATH" "$APPDIR" "$APPIMAGE"
chmod +x "$APPIMAGE"
"$APPIMAGE" --appimage-help >/dev/null
"$APPIMAGE" --help >/dev/null

echo "$APPIMAGE"
