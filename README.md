# Source Weaver

Source Weaver is a cross-platform desktop tool for combining Source Engine campaign VMFs into one Hammer-editable map. It is being built for workflows around games such as Half-Life 2, Black Mesa, and other Source 1 projects that use `.vmf` map sources.

The project is now a Rust workspace with three pieces:

- `sourceweaver-core`: VMF parser, inspector, deletion engine, transform logic, and merger.
- `sourceweaver-desktop`: native Linux/Windows desktop UI built with egui/eframe.
- `sourceweaver-cli`: command-line interface for scripting and validation.

## What Source Weaver does

Source Weaver takes selected VMF files and creates a single merged VMF. It is designed around campaign map stitching, where separate maps need to line up at transition landmarks and remain editable in Hammer afterward.

Current capabilities:

- Select multiple `.vmf` files in the desktop app.
- Pick a base map for the merged output.
- Align incoming maps to a shared `info_landmark` targetname.
- Preserve incoming world brushes, including skybox brushes.
- Preserve incoming point entities and brush entities.
- View detected Hammer entity classnames, including unknown and game-specific classnames.
- View individual world/entity records with classname, targetname, origin, solid count, and detected roles.
- Detect brush roles such as triggers, clips, areaportals, occluders, skybox, hint, skip, nodraw, and water.
- Preview bulk deletion rules.
- Save a cleaned copy of a selected VMF.
- Apply deletion rules during merge.
- Export a merged `.vmf` for Hammer.

## Build and run the desktop app

### Linux

Install the Rust stable toolchain, then run:

```bash
cargo run -p sourceweaver-desktop
```

Some Linux distributions require desktop GUI development libraries for egui/eframe and native file dialogs. On Debian/Ubuntu-style systems, install the common build dependencies if the GUI stack fails to compile:

```bash
sudo apt install build-essential pkg-config libgtk-3-dev libx11-dev libxcb1-dev libxkbcommon-dev libwayland-dev
```

### Windows

Install Rust stable from <https://rustup.rs/> and the Microsoft C++ Build Tools, then run from PowerShell or Windows Terminal:

```powershell
cargo run -p sourceweaver-desktop
```

A release executable can be built with:

```powershell
cargo build --release -p sourceweaver-desktop
```

The executable will be under `target\release\sourceweaver-desktop.exe` on Windows and `target/release/sourceweaver-desktop` on Linux. The `Desktop Builds` GitHub Actions workflow also builds Linux and Windows desktop binaries for tags and manual runs.

## Desktop workflow

1. Click **Add VMFs...** and select the campaign VMF files.
2. Select the base map in the left panel or in the **Base map** dropdown.
3. Enter the shared `info_landmark` targetname. Leave it blank to append maps without alignment.
4. Browse for an output `.vmf` path.
5. Inspect the selected map's entities and classnames in the inspection table.
6. Optionally add deletion rules by classname, targetname, or brush role.
7. Click **Preview deletion** to see how much content the cleanup rules would remove.
8. Click **Save cleaned selected VMF...** to export a cleaned copy of one VMF, or **Merge selected VMFs** to apply the rules during merge.


## Non-interactive automation

Source Weaver can be driven entirely without the desktop UI through TOML job files. This is the preferred workflow when an assistant or script is running the tool for you.

Create a starter job:

```bash
cargo run -p sourceweaver-cli -- job-template > sourceweaver-job.toml
```

Run it:

```bash
cargo run -p sourceweaver-cli -- run --job sourceweaver-job.toml
```

Preview it without writing a VMF:

```bash
cargo run -p sourceweaver-cli -- run --job sourceweaver-job.toml --dry-run
```

The job runner prints a JSON report and can also write one to disk. See `docs/automation.md` for the full workflow.

## CLI usage

The CLI remains available for scripting and regression testing.

Inspect all top-level VMF world/entity records and detected roles:

```bash
cargo run -p sourceweaver-cli -- inspect path/to/map.vmf
```

List every detected Hammer classname and count:

```bash
cargo run -p sourceweaver-cli -- list-types path/to/map.vmf
```

Delete matching content by classname, targetname, or brush role:

```bash
cargo run -p sourceweaver-cli -- prune path/to/map.vmf \
  -o cleaned.vmf \
  --drop-classname prop_static \
  --drop-role clip
```

Merge multiple VMFs using a shared `info_landmark` targetname:

```bash
cargo run -p sourceweaver-cli -- merge \
  -o stitched.vmf \
  --landmark map_transition \
  base.vmf next.vmf another.vmf
```

## Deletion roles

The current deletion/classification roles are:

```text
trigger
clip
areaportal
skybox
occluder
hint
skip
nodraw
water
world-brush
brush-entity
```

Raw `tools/...` materials are also surfaced internally so the UI can later expose more precise cleanup filters.

## Repository layout

```text
crates/sourceweaver-core/      VMF parser, inspection, deletion, transform, and merge engine
crates/sourceweaver-cli/       CLI for validating and using the core engine
crates/sourceweaver-desktop/   Native Linux/Windows desktop app
docs/                          Requirements, architecture, and roadmap notes
tests/fixtures/                Small VMF files used for local validation
```

## Important limitations

Source Weaver is still early in the rebuild.

Known limitations:

- No 2D/3D map preview yet.
- No BSP decompilation.
- No FGD-backed property labels yet.
- No automatic compile pipeline yet.
- Texture-axis and displacement edge cases may need additional handling as real campaign maps are tested.
- Very large merged maps can still hit Hammer or Source compiler limits.

## Design principle

The core engine should preserve data it does not understand. Source maps often contain game-specific entities and custom Hammer data, so Source Weaver should inspect and move VMF content conservatively instead of relying on a fixed whitelist.

## License

MIT
