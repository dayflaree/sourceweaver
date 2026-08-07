# Packaging Source Weaver

Source Weaver release packages are built as portable archives and AppImages instead of system installers.

## Linux package format

Linux releases use a tarball:

```text
sourceweaver-vX.Y.Z-linux-x86_64.tar.gz
```

The archive contains:

- `SourceWeaver` double-click shell launcher
- `SourceWeaver.desktop` double-click desktop launcher for file managers that trust executable desktop files
- `install-linux.sh` user-level app-menu installer
- `bin/sourceweaver-desktop`
- `bin/sourceweaver`
- `share/applications/io.github.dayflaree.SourceWeaver.desktop`
- `share/icons/hicolor/scalable/apps/sourceweaver.svg`
- `docs/`
- `README.md`
- `LICENSE`

Run from the extracted archive by double-clicking `SourceWeaver` or `SourceWeaver.desktop`, or from a terminal:

```bash
./SourceWeaver
./bin/sourceweaver-desktop
./bin/sourceweaver --help
```

Some Linux file managers require right-clicking `SourceWeaver.desktop`, opening Properties, and enabling **Allow executing file as program** before double-click launchers are trusted.

Install into the current user's app menu:

```bash
./install-linux.sh
```

The installer copies the package to `${XDG_DATA_HOME:-~/.local/share}/sourceweaver`, installs an application entry under `${XDG_DATA_HOME:-~/.local/share}/applications`, installs the SVG icon, and creates `~/.local/bin/sourceweaver` / `~/.local/bin/sourceweaver-desktop` symlinks.

### Linux system libraries

The desktop app uses `eframe/egui`, `winit`, and native file dialogs. Common runtime/build packages on Debian/Ubuntu-family systems include:

```bash
sudo apt-get install libgtk-3-0 libx11-6 libxcb1 libxkbcommon0 libwayland-client0
```

For local builds on CI-like Ubuntu systems, development packages are installed by the workflow:

```bash
sudo apt-get install libgtk-3-dev libx11-dev libxcb1-dev libxkbcommon-dev libwayland-dev
```

The Linux package is validated in GitHub Actions on `ubuntu-latest` by building the release binaries and creating the tarball.

## Linux AppImage format

Linux release workflows also build:

```text
sourceweaver-vX.Y.Z-linux-x86_64.AppImage
```

The AppImage contains:

- `usr/bin/sourceweaver-desktop`
- `usr/bin/sourceweaver`
- `usr/share/applications/io.github.dayflaree.SourceWeaver.desktop`
- `usr/share/icons/hicolor/scalable/apps/sourceweaver.svg`
- root `AppRun`
- root `io.github.dayflaree.SourceWeaver.desktop`
- root `sourceweaver.svg`
- docs, README, and LICENSE under `usr/share/sourceweaver/`

Run it directly:

```bash
chmod +x sourceweaver-vX.Y.Z-linux-x86_64.AppImage
./sourceweaver-vX.Y.Z-linux-x86_64.AppImage
```

Inspect AppImage runtime help:

```bash
./sourceweaver-vX.Y.Z-linux-x86_64.AppImage --appimage-help
```

Uninstall by deleting the AppImage file. If a desktop environment or helper such as AppImageLauncher integrated it into an app menu, remove the helper-created launcher through that tool or delete the generated desktop entry from the user's application directory.

### AppImage build details

`scripts/package-appimage.sh` creates a reproducible `SourceWeaver.AppDir` and then runs appimagetool when available:

```bash
scripts/package-appimage.sh v0.1.0
```

Build only the AppDir locally without appimagetool:

```bash
scripts/package-appimage.sh v0.1.0-local --appdir-only
```

The GitHub release workflow downloads `appimagetool-x86_64.AppImage` from the AppImage project continuous release and sets `ARCH=x86_64`. AppImage documentation checked on 2026-08-08 describes `AppRun` as the AppDir entry point and appimagetool as the tool that creates AppImages from AppDirs. It also documents appimagetool downloads from `https://github.com/AppImage/appimagetool/releases/continuous`.

### AppImage limitations

- AppImage launch was wired into the release workflow and AppDir layout was smoke-tested locally; cross-distribution GUI smoke testing still needs a clean Linux VM or runner artifact test.
- The AppImage starts the desktop app. The CLI binary is bundled at `usr/bin/sourceweaver` for extraction/debugging, while normal user launch is desktop-first.
- Some systems need FUSE support to run AppImages normally. AppImage runtime extraction modes may be used by testers when FUSE is unavailable.
- AppImage artifacts are not code-signed in this repository state.

## Windows package format

Windows releases use a zip archive:

```text
sourceweaver-vX.Y.Z-windows-x86_64.zip
```

The archive contains:

- `sourceweaver-desktop.exe`
- `sourceweaver.exe`
- `assets/sourceweaver.ico`
- `docs/`
- `README.md`
- `LICENSE`

Run from the extracted zip:

```powershell
.\sourceweaver-desktop.exe
.\sourceweaver.exe --help
```

No installer is required. The binary is built with the Rust stable Windows toolchain on the hosted `windows-latest` GitHub Actions runner. If Windows Defender or SmartScreen warns on unsigned artifacts, users can inspect the release checksum and build provenance in GitHub Actions.

## Local packaging commands

Linux tarball:

```bash
scripts/package-linux.sh v0.1.0
```

Linux AppImage:

```bash
scripts/package-appimage.sh v0.1.0
```

Windows PowerShell:

```powershell
scripts\package-windows.ps1 -Version v0.1.0
```

Both scripts write archives under `target/package/`.

## Why tarball/zip first?

Tarball and zip packages are deterministic, low-maintenance, and work with GitHub Actions without extra signing or installer infrastructure. AppImage, MSI, or code-signed installers can be added later once Source Weaver has stable release demand and signing keys.
