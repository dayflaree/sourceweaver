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
- Save and load desktop project/job TOML files that are CLI-compatible where possible.
- Pick a base map for the merged output.
- Align incoming maps to a shared `info_landmark` targetname.
- Discover `info_landmark` targetnames from selected VMFs and choose one from a dropdown.
- Show missing, duplicate, and invalid landmark status before preview or export.
- Show VMF integrity status before preview/export, including missing common sections, duplicate IDs, and invalid world blocks.
- Validate generated VMFs for Source-tool readiness and parse captured VBSP logs.
- Preserve incoming world brushes, including skybox brushes.
- Preserve incoming point entities and brush entities.
- View detected Hammer entity classnames, including unknown and game-specific classnames.
- View individual world/entity records with classname, targetname, origin, solid count, and detected roles.
- Detect `trigger_changelevel` campaign transitions and show target map/landmark data.
- Search, role-filter, and sort large entity/classname tables.
- Select multiple entity-table rows with checkboxes for future cleanup actions.
- Preview scanned VMFs in Hammer-style 2D orthographic views.
- Preview the in-memory merged output before writing a VMF.
- Color merged-preview solids and entity markers by source VMF, with a source-map legend.
- Switch preview projection between top X/Y, front X/Z, and side Y/Z views.
- Pan and zoom the preview viewport.
- Draw brush bounds, face-plane triangles, entity origin markers, grid lines, and role-colored overlays.
- Draw `info_landmark` diamond markers with targetname labels, selected-landmark highlighting, and merged-preview offset arrows.
- Detect brush roles such as triggers, clips, areaportals, occluders, skybox, hint, skip, nodraw, and water.
- Preview bulk deletion rules.
- Apply transparent deletion presets for triggers, clips, areaportals, gameplay logic, world-only cleanup, and world-plus-skybox cleanup.
- Choose safe brush-entity deletion behavior and protect critical transition/player/logic entities by default.
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
3. Optionally click **Load project/job...** to restore a saved `.toml` setup, or **Save project...** to write the current setup for later CLI or desktop use.
4. Choose a discovered `info_landmark` targetname from the dropdown, or type one manually. Leave it blank to append maps without alignment.
5. Review the **Landmark status** table. It shows which selected VMFs contain the chosen landmark and warns about missing, duplicate, or invalid landmarks before preview/export.
6. Review the **VMF integrity status** table for structural errors and warnings before preview/export.
7. Browse for an output `.vmf` path.
8. Use the **Preview** tab to view the scanned VMF in top, front, or side projection.
9. Click **Preview selected merge** to build the exact current merge in memory without writing a file.
10. Switch **Preview source** between the selected VMF and the merged result.
11. Inspect the selected map's entities and classnames in the inspection tables. Entity rows can be searched, role-filtered, sorted, and selected with checkboxes; selections persist while switching tabs.
12. Optionally apply a deletion preset, then inspect the generated deletion rules.
13. In **Deletion safety**, choose whether brush-entity role matches delete whole entities or only matching contained solids. Critical transition/player/logic entities are protected by default.
14. Click **Preview deletion** to see how much content the cleanup rules would remove.
15. Click **Save cleaned selected VMF...** to export a cleaned copy of one VMF, or **Merge selected VMFs** to apply the rules during merge.


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

The job runner prints a JSON report and can also write one to disk. Reports include detected `trigger_changelevel` campaign transitions with target map and landmark data. See `docs/automation.md` for the full workflow.

Validate a generated VMF and optionally parse a captured VBSP log:

```bash
cargo run -p sourceweaver-cli -- validate stitched.vmf --json
cargo run -p sourceweaver-cli -- validate stitched.vmf --compile-log vbsp.log --json
```

When Source tooling is available, pass `--vbsp`, optional `--game`, and `--capture-log`. See `docs/compiler-validation.md` for Linux-friendly validation, captured-log parsing, and HL2/Black Mesa command examples.

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
cargo run -p sourceweaver-cli -- prune \
  path/to/map.vmf \
  -o cleaned.vmf \
  --drop-role clip \
  --brush-entity-mode matching-solids
```

Brush-entity deletion modes are explicit. `whole-entity` preserves the original behavior for brush-role matches by deleting matching brush entities. `matching-solids` keeps brush entities and removes only contained solids with matching roles. Critical transition/player/logic classnames are protected by default; direct CLI prune can opt out with `--allow-critical-deletion`, and job files can set `delete.protect_critical_entities = false`.

Desktop deletion presets are transparent. Each preset shows the generated criteria and can be previewed before applying; preview and export use the same pruning code path so counts match final deletion behavior.

Desktop project files use the same TOML shape as `sourceweaver run --job` where possible. Saving a project writes the current base VMF, input VMFs, landmark, output path, and deletion rules. Paths under the project file directory are saved relative to that directory; relative paths are resolved from the project file location when loaded.

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

- The current map preview is a 2D orthographic preview based on VMF brush plane points and bounds. It can preview single VMFs and the current in-memory merged output, but it is not yet a full textured Hammer 3D viewport.
- No BSP decompilation.
- No FGD-backed property labels yet.
- No automatic compile pipeline yet.
- Texture-axis and displacement edge cases may need additional handling as real campaign maps are tested.
- Very large merged maps can still hit Hammer or Source compiler limits.

## Design principle

The core engine should preserve data it does not understand. Source maps often contain game-specific entities and custom Hammer data, so Source Weaver should inspect and move VMF content conservatively instead of relying on a fixed whitelist.

## License

MIT
