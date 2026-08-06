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
cat > "$PACKAGE_DIR/RUNNING_ON_LINUX.md" <<'DOC'
# Running Source Weaver on Linux

Run from the extracted package:

```bash
./bin/sourceweaver-desktop
./bin/sourceweaver --help
```

To integrate the desktop entry manually, copy the files under `share/` into the matching XDG directories or add this package's `bin` directory to `PATH`.

See `docs/packaging.md` for required system libraries and troubleshooting.
DOC

(
  cd "$ROOT/target/package"
  tar -czf "$ARCHIVE" "$PACKAGE_NAME"
)

echo "$ARCHIVE"
