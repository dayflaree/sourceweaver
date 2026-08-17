#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-${GITHUB_REF_NAME:-dev}}"
MODE="${2:-}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARCH="${ARCH:-x86_64}"
PACKAGE_ROOT="$ROOT/target/package"
APPDIR="$PACKAGE_ROOT/SourceWeaver.AppDir"
APPIMAGE="$PACKAGE_ROOT/sourceweaver-${VERSION}-linux-${ARCH}.AppImage"
APPIMAGETOOL="${APPIMAGETOOL:-${PACKAGE_ROOT}/appimagetool-${ARCH}.AppImage}"

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
"$APPDIR/usr/bin/sourceweaver-desktop" --help >/dev/null 2>&1 || true

echo "AppDir: $APPDIR"

if [[ "$MODE" == "--appdir-only" ]]; then
  exit 0
fi

if [[ ! -x "$APPIMAGETOOL" ]]; then
  printf 'appimagetool not found or not executable at %s\n' "$APPIMAGETOOL" >&2
  printf 'Set APPIMAGETOOL=/path/to/appimagetool-x86_64.AppImage or run with --appdir-only.\n' >&2
  exit 1
fi

# appimagetool expects ARCH for reproducible architecture naming.
ARCH="$ARCH" "$APPIMAGETOOL" "$APPDIR" "$APPIMAGE"
chmod +x "$APPIMAGE"
"$APPIMAGE" --appimage-help >/dev/null

echo "$APPIMAGE"
