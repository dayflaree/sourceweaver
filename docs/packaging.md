# Packaging Source Weaver

Source Weaver release packages are built as portable archives instead of installers.

## Linux package format

Linux releases use a tarball:

```text
sourceweaver-vX.Y.Z-linux-x86_64.tar.gz
```

The archive contains:

- `bin/sourceweaver-desktop`
- `bin/sourceweaver`
- `share/applications/io.github.dayflaree.SourceWeaver.desktop`
- `share/icons/hicolor/scalable/apps/sourceweaver.svg`
- `docs/`
- `README.md`
- `LICENSE`

Run from the extracted archive:

```bash
./bin/sourceweaver-desktop
./bin/sourceweaver --help
```

To install manually, copy `share/applications/io.github.dayflaree.SourceWeaver.desktop` to `~/.local/share/applications/`, copy the icon to `~/.local/share/icons/hicolor/scalable/apps/`, and add the archive's `bin` directory to your `PATH` or edit the desktop entry `Exec=` path.

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

Linux:

```bash
scripts/package-linux.sh v0.1.0
```

Windows PowerShell:

```powershell
scripts\package-windows.ps1 -Version v0.1.0
```

Both scripts write archives under `target/package/`.

## Why tarball/zip first?

Tarball and zip packages are deterministic, low-maintenance, and work with GitHub Actions without extra signing or installer infrastructure. AppImage, MSI, or code-signed installers can be added later once Source Weaver has stable release demand and signing keys.
