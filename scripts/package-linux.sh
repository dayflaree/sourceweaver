#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-${GITHUB_REF_NAME:-dev}}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGE_NAME="sourceweaver-${VERSION}-linux-x86_64"
PACKAGE_DIR="$ROOT/target/package/$PACKAGE_NAME"
ARCHIVE="$ROOT/target/package/${PACKAGE_NAME}.tar.gz"

rm -rf "$PACKAGE_DIR" "$ARCHIVE"
mkdir -p "$PACKAGE_DIR/bin" \
  "$PACKAGE_DIR/share/applications" \
  "$PACKAGE_DIR/share/icons/hicolor/scalable/apps" \
  "$PACKAGE_DIR/docs"

cargo build --release -p sourceweaver-cli -p sourceweaver-desktop
cp "$ROOT/target/release/sourceweaver" "$PACKAGE_DIR/bin/sourceweaver"
cp "$ROOT/target/release/sourceweaver-desktop" "$PACKAGE_DIR/bin/sourceweaver-desktop"
cp "$ROOT/LICENSE" "$PACKAGE_DIR/LICENSE"
cp "$ROOT/README.md" "$PACKAGE_DIR/README.md"
cp -R "$ROOT/docs/." "$PACKAGE_DIR/docs/"
cp "$ROOT/packaging/linux/io.github.dayflaree.SourceWeaver.desktop" "$PACKAGE_DIR/share/applications/"
cp "$ROOT/packaging/linux/sourceweaver.svg" "$PACKAGE_DIR/share/icons/hicolor/scalable/apps/sourceweaver.svg"
cat > "$PACKAGE_DIR/SourceWeaver" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
APPDIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$APPDIR/bin/sourceweaver-desktop" "$@"
SH
chmod +x "$PACKAGE_DIR/SourceWeaver"

cat > "$PACKAGE_DIR/SourceWeaver.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Source Weaver
Comment=Merge, preview, and validate Source Engine VMFs
Exec=sh -c "APPDIR=\\$(dirname \\"\\$1\\"); shift; exec \\"\\$APPDIR/SourceWeaver\\" \\"\\$@\\"" sourceweaver-desktop %k
Icon=sourceweaver
Terminal=false
Categories=Development;
StartupWMClass=Source Weaver
DESKTOP
chmod +x "$PACKAGE_DIR/SourceWeaver.desktop"

cat > "$PACKAGE_DIR/install-linux.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

APPDIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
INSTALL_DIR="$DATA_HOME/sourceweaver"
APPLICATIONS_DIR="$DATA_HOME/applications"
ICON_DIR="$DATA_HOME/icons/hicolor/scalable/apps"
BIN_DIR="$HOME/.local/bin"

mkdir -p "$APPLICATIONS_DIR" "$ICON_DIR" "$BIN_DIR"

desktop_quote() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//\$/\\\$}"
  value="${value//\`/\\\`}"
  printf '"%s"' "$value"
}

TMP_INSTALL="${INSTALL_DIR}.tmp"
rm -rf "$TMP_INSTALL"
mkdir -p "$TMP_INSTALL"
cp -a \
  "$APPDIR/bin" \
  "$APPDIR/docs" \
  "$APPDIR/share" \
  "$APPDIR/SourceWeaver" \
  "$APPDIR/SourceWeaver.desktop" \
  "$APPDIR/LICENSE" \
  "$APPDIR/README.md" \
  "$APPDIR/RUNNING_ON_LINUX.md" \
  "$TMP_INSTALL/"
rm -rf "$INSTALL_DIR"
mv "$TMP_INSTALL" "$INSTALL_DIR"
ln -sf "$INSTALL_DIR/bin/sourceweaver" "$BIN_DIR/sourceweaver"
ln -sf "$INSTALL_DIR/bin/sourceweaver-desktop" "$BIN_DIR/sourceweaver-desktop"
cp "$INSTALL_DIR/share/icons/hicolor/scalable/apps/sourceweaver.svg" "$ICON_DIR/sourceweaver.svg"

DESKTOP_EXEC="$(desktop_quote "$INSTALL_DIR/bin/sourceweaver-desktop")"

cat > "$APPLICATIONS_DIR/io.github.dayflaree.SourceWeaver.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Source Weaver
Comment=Merge, preview, and validate Source Engine VMFs
Exec=$DESKTOP_EXEC
Icon=sourceweaver
Terminal=false
Categories=Development;
StartupWMClass=Source Weaver
DESKTOP
chmod +x "$APPLICATIONS_DIR/io.github.dayflaree.SourceWeaver.desktop"

command -v desktop-file-validate >/dev/null 2>&1 && desktop-file-validate "$APPLICATIONS_DIR/io.github.dayflaree.SourceWeaver.desktop"

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPLICATIONS_DIR" || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache "$DATA_HOME/icons/hicolor" || true

echo "Installed Source Weaver to $INSTALL_DIR"
echo "Desktop launcher: $APPLICATIONS_DIR/io.github.dayflaree.SourceWeaver.desktop"
echo "CLI symlink: $BIN_DIR/sourceweaver"
echo "Open it from your app menu, or run: $INSTALL_DIR/bin/sourceweaver-desktop"
SH
chmod +x "$PACKAGE_DIR/install-linux.sh"

cat > "$PACKAGE_DIR/RUNNING_ON_LINUX.md" <<'DOC'
# Running Source Weaver on Linux

Double-click one of these files in the extracted package:

- `SourceWeaver`
- `SourceWeaver.desktop`

Some file managers require you to right-click `SourceWeaver.desktop`, open Properties, and enable **Allow executing file as program**.

To install Source Weaver into your user app menu:

```bash
./install-linux.sh
```

Run directly from the extracted package:

```bash
./SourceWeaver
./bin/sourceweaver-desktop
./bin/sourceweaver --help
```

After running `install-linux.sh`, launch **Source Weaver** from your desktop environment's app menu or run `sourceweaver-desktop` / `sourceweaver` from a terminal.

See `docs/packaging.md` for required system libraries and troubleshooting.
DOC

if command -v desktop-file-validate >/dev/null 2>&1; then
  desktop-file-validate "$PACKAGE_DIR/SourceWeaver.desktop"
  desktop-file-validate "$PACKAGE_DIR/share/applications/io.github.dayflaree.SourceWeaver.desktop"
fi
"$PACKAGE_DIR/bin/sourceweaver" --help >/dev/null

(
  cd "$ROOT/target/package"
  tar -czf "$ARCHIVE" "$PACKAGE_NAME"
)

echo "$ARCHIVE"
