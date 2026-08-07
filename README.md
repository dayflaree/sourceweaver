# Source Weaver

Source Weaver is a cross-platform desktop tool for combining Source Engine campaign VMFs into one Hammer-editable map. It is being built for workflows around games such as Half-Life 2, Black Mesa, and other Source 1 projects that use `.vmf` map sources.

The project is now a Rust workspace with three pieces:

- `sourceweaver-core`: VMF parser, inspector, deletion engine, transform logic, and merger.
- `sourceweaver-desktop`: native Linux/Windows desktop UI built with egui/eframe.
- `sourceweaver-cli`: command-line interface for scripting and validation.

## What Source Weaver does

Source Weaver takes selected VMF files and creates a single merged VMF. It is designed around campaign map stitching, where separate maps need to line up at transition landmarks and remain editable in Hammer afterward.

Current capabilities:

- Select multiple `.vmf` files in the desktop app, decompile/import BSP-derived VMFs with user-selected BSPSource tools, export a merged VMF, and optionally launch an external compile profile afterward.
- Save and load desktop project/job TOML files that are CLI-compatible where possible.
- Pick a base map for the merged output.
- Align incoming maps to a shared `info_landmark` targetname.
- Discover `info_landmark` targetnames from selected VMFs and choose one from a dropdown.
- Show missing, duplicate, and invalid landmark status before preview or export.
- Show VMF integrity status before preview/export, including missing common sections, duplicate IDs, and invalid world blocks.
- Validate generated VMFs for Source-tool readiness and parse captured VBSP logs.
- Run optional user-configured VBSP/VVIS/VRAD compile pipelines, create/validate compile profiles, and capture parsed JSON reports.
- Run optional BSP content packing with user-provided `bspzip`-compatible tools, explicit or VMF-discovered asset lists, context profiles, wrappers, and JSON reports.
- Generate cubemap/buildcubemaps runtime workflow reports and cfg helpers without launching game runtimes.
- Inspect basic MDL model headers plus Source-style mesh, animation, and sequence tables, and run user-provided StudioMDL-compatible model compile tools or headless model-decompile wrappers.
- Run optional user-selected BSPSource decompile commands or generic wrappers and validate generated VMFs before import.
- Preserve incoming world brushes, including skybox brushes.
- Preserve incoming point entities and brush entities.
- View detected Hammer entity classnames, including unknown and game-specific classnames.
- View individual world/entity records with classname, targetname, origin, solid count, and detected roles.
- Detect `trigger_changelevel` campaign transitions, show target map/landmark data, and report campaign adjacency graph edges with confidence levels.
- Preserve, disable, delete, or rewrite internal `trigger_changelevel` destinations during merge through explicit policies, cleanup scopes, and external preserve selectors.
- Suggest campaign map order and landmark pairs from detected transitions.
- Enrich entity/classname tables with built-in, inferred, and optionally loaded FGD metadata, including FGD-backed property labels, descriptions, defaults, choices, and flags where parsed.
- Drag and drop `.vmf`, `.toml`, and `.fgd` files into the desktop app, with in-session recent files/projects.
- Show parse progress, error dialogs, dark/light theme controls, and adjustable preview height.
- Search, role-filter, and sort large entity/classname tables.
- Select multiple entity-table rows with checkboxes for future cleanup actions.
- Click preview entity markers or solid bounds to select matching entity/world table rows.
- Preview scanned VMFs in Hammer-style 2D orthographic views.
- Preview the in-memory merged output before writing a VMF.
- Color merged-preview solids and entity markers by source VMF, with a source-map legend.
- Switch preview projection between top X/Y, front X/Z, and side Y/Z views.
- Pan and zoom the preview viewport.
- Draw brush bounds, face-plane triangles, entity origin markers, grid lines, and role-colored overlays.
- Reconstruct and draw closed convex brush face polygons for more accurate 2D preview shapes.
- Switch to a lightweight 3D isometric preview with yaw/pitch camera controls.
- Draw `info_landmark` diamond markers with targetname labels, selected-landmark highlighting, and merged-preview offset arrows.
- Detect brush roles such as triggers, clips, areaportals, occluders, skybox, hint, skip, nodraw, and water.
- Preview bulk deletion rules.
- Visualize deletion rules in the selected-map preview by highlighting, dimming, or hiding matched content.
- Review pending deletion counts, undo the pending review, and explicitly confirm cleanup export before writing destructive changes.
- Apply transparent deletion presets for triggers, clips, areaportals, gameplay logic, world-only cleanup, and world-plus-skybox cleanup.
- Choose safe brush-entity deletion behavior and protect critical transition/player/logic entities by default.
- Save a cleaned copy of a selected VMF.
- Apply deletion rules during merge.
- Export a merged `.vmf` for Hammer.
- Guard parser, merge, prune, preview, and automation behavior with VMF fixture/golden regression tests.
- Reproduce public real-VMF validation with a pinned two-map Source 1 workflow script.
- Translate displacement side planes and `dispinfo` `startposition` values during landmark-aligned moves.
- Adjust VMF `uaxis`/`vaxis` texture shifts during brush translation to preserve texture-lock behavior.
- Renumber incoming VMF IDs, remap known ID reference fields during merge, and warn on unsupported suspected ID-reference keys.
- Preserve base editor metadata while intentionally ignoring conflicting incoming top-level editor sections.
- Package Linux tarball and Windows zip releases from version tags.

## Build and run the desktop app

Download packaged releases from the GitHub Releases page, or build locally from source. Linux releases are `.tar.gz` archives and Windows releases are `.zip` archives. See `docs/packaging.md` and `docs/release.md` for package contents, runtime notes, and the tag-based release process.

### Linux

Install the Rust stable toolchain, then run:

```bash
cargo run -p sourceweaver-desktop
```

To build a double-clickable Linux package locally:

```bash
scripts/package-linux.sh v0.1.0-local
```

Extract the archive under `target/package/`, then double-click `SourceWeaver` or run `./install-linux.sh` to add **Source Weaver** to your user app menu.

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

The executable will be under `target\\release\\sourceweaver-desktop.exe` on Windows and `target/release/sourceweaver-desktop` on Linux. The `Desktop Release Builds` GitHub Actions workflow packages Linux and Windows release archives for tags and manual runs.

## Desktop workflow

1. Click **Add VMFs...** and select the campaign VMF files, or drag `.vmf` files onto the desktop window.
2. Select the base map in the left panel or in the **Base map** dropdown.
3. Optionally click **Load project/job...** to restore a saved `.toml` setup, drag a `.toml` job/project onto the window, or **Save project...** to write the current setup for later CLI or desktop use.
4. Review **Campaign suggestions** for a transition-derived map order and landmark pairs. Apply the suggestion or keep the manual order/base/landmark settings.
5. Choose a discovered `info_landmark` targetname from the dropdown, or type one manually. Leave it blank to append maps without alignment.
6. Review the **Landmark status** table. It shows which selected VMFs contain the chosen landmark and warns about missing, duplicate, or invalid landmarks before preview/export.
7. Choose a **Changelevel policy** when transition entities should be preserved, disabled, deleted, or rewritten for a stitched campaign output.
7. Review the **VMF integrity status** table for structural errors and warnings before preview/export.
8. Browse for an output `.vmf` path.
9. Use the **Preview** tab to view the scanned VMF in top, front, or side projection.
10. Click **Preview selected merge** to build the exact current merge in memory without writing a file.
11. Switch **Preview source** between the selected VMF and the merged result.
12. Optionally click **Load FGD metadata...** or drag `.fgd` files onto the window to enrich entity descriptions from Hammer FGD files.
13. Inspect the selected map's entities and classnames in the inspection tables. Entity rows can be searched, role-filtered, sorted, selected with checkboxes, or selected by clicking preview markers/solids; selections persist while switching tabs and view projections.
14. Optionally apply a deletion preset, then inspect the generated deletion rules.
15. In **Deletion safety**, choose whether brush-entity role matches delete whole entities or only matching contained solids. Critical transition/player/logic entities are protected by default.
16. Use **Deletion preview** in the preview toolbar to highlight, dim, or hide content matched by current cleanup rules. Click **Preview deletion** to create a pending cleanup review with exact in-memory removal counts.
17. Review **Pending cleanup review**, then click **Confirm cleanup export**. Use **Undo pending review** to clear the pending destructive action.
18. Click **Save cleaned selected VMF...** to export a cleaned copy of one VMF, or **Merge selected VMFs** to apply the confirmed rules during merge.

The desktop toolbar also provides a dark/light theme toggle. The left panel shows parse progress, recent VMF/project shortcuts, and visible parse failures. Important failures also appear in a dismissible error dialog.


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

The job runner prints a JSON report and can also write one to disk. Reports include detected `trigger_changelevel` campaign transitions plus suggested campaign order/landmark pairs. See `docs/automation.md` for the full workflow.

Validate a generated VMF and optionally parse a captured VBSP log:

```bash
cargo run -p sourceweaver-cli -- validate stitched.vmf --json
cargo run -p sourceweaver-cli -- validate stitched.vmf --rule-set hl2 --json
cargo run -p sourceweaver-cli -- validate stitched.vmf --compile-log vbsp.log --json
```

Use `--rule-set hl2` for portable Half-Life 2 single-player VMF semantics. Rule-set findings are reported separately from generic integrity findings and do not run Hammer, VBSP, VVIS, VRAD, or a game runtime. The same validation report also includes `entity_semantics` findings for duplicate targetnames and missing common target references plus a `complexity` summary for VMF-only Source/Hammer limit heuristics. When Source tooling is available, pass `--vbsp`, optional `--game`, and `--capture-log`. External compiler/decompiler runs default to a 900-second timeout; use `--timeout-seconds` for slower tools or quick failure tests. Captured compiler logs must include explicit success markers such as `0 errors` or `VBSP finished`; a truncated tool banner does not count as a successful compile. See `docs/compiler-validation.md`, `docs/bspsource-managed-download.md`, `docs/external-decompiler-presets.md`, `docs/bsp-derived-fixtures.md`, `docs/bsp-packing.md`, `docs/cubemap-workflow.md`, `docs/material-preview.md`, `docs/changelevel-policies.md`, `docs/transition-cleanup-rules.md`, `docs/deletion-presets.md`, `docs/campaign-adjacency.md`, `docs/campaign-batch-workflow.md`, `docs/entity-semantic-validation.md`, `docs/map-complexity.md`, and `docs/game-validation-rule-sets.md` for Linux-friendly validation, BSPSource managed download policy, external decompiler preset research, BSP-derived fixture boundaries, BSP packing integration, cubemap/buildcubemaps runtime workflow planning, material-aware preview limits, captured-log parsing, changelevel policies, transition cleanup rules, custom deletion presets, campaign adjacency inference, campaign batch plans, semantic checks, complexity heuristics, rule-set scope, and HL2/Black Mesa command examples.

Create and validate a compile profile without hand-editing TOML, then run a user-configured compile pipeline when VBSP/VVIS/VRAD are available:

```bash
cargo run -p sourceweaver-cli -- compile-profile create \
  --output hl2-tools.toml \
  --vbsp /path/to/vbsp-or-wrapper \
  --vvis /path/to/vvis-or-wrapper \
  --vrad /path/to/vrad-or-wrapper \
  --game /path/to/game-dir \
  --steps vbsp,vvis,vrad \
  --log-dir target/sourceweaver-compile-logs \
  --validate \
  --json

cargo run -p sourceweaver-cli -- compile stitched.vmf \
  --profile hl2-tools.toml \
  --steps vbsp,vvis,vrad \
  --log-dir target/sourceweaver-compile-logs \
  --timeout-seconds 900 \
  --report compile-report.json \
  --json
```

See `docs/linux-source-compiler-setup.md` for Wine/Proton wrappers, sample profiles, troubleshooting, and the boundary between VMF validation and real compiler validation. The desktop app also has an **Optional external compile** panel that can run the same profile after a successful merge/export without blocking the UI.

See `docs/compile-pipeline.md` for profile format, report fields, desktop compile behavior, and Linux-friendly validation notes. See `docs/source-compiler-smoke-test-matrix.md` for real-tool smoke-test evidence requirements.

Run a user-selected BSPSource decompiler and validate the generated VMF:

```bash
cargo run -p sourceweaver-cli -- bsp-import map.bsp \
  --bspsource /path/to/bspsrc.sh \
  --output decompiled_map.vmf \
  --log decompile.log \
  --timeout-seconds 900 \
  --report bsp-import-report.json \
  --json
```

For jar-only BSPSource distributions, use `--bspsource-jar /path/to/bspsrc.jar` and optionally `--java /path/to/java`. `--tool ./custom-wrapper.sh` remains available for unusual decompilers or argument orders. `--preset <id>` applies a documented BSPSource argument preset before raw `--tool-arg` values. In the desktop app, use **Decompile BSP...** or the **BSP decompile import** panel to select a `.bsp`, a BSPSource launcher/jar, a named preset, and an output VMF. Successful output is validated, imported, and marked with categorized decompile-quality warnings such as unsupported lumps, skipped data, quality risks, tool errors, and non-fatal tool configuration noise. **Add BSP-derived VMF...** remains available for VMFs decompiled outside Source Weaver.

Pack custom assets into a compiled BSP with a user-provided `bspzip`-compatible tool:

```bash
cargo run -p sourceweaver-cli -- pack map.bsp \
  --tool /path/to/bspzip \
  --output packed.bsp \
  --asset-root /path/to/game \
  --include materials/custom/wall01.vmt \
  --include materials/custom/wall01.vtf \
  --report pack-report.json \
  --json
```

See `docs/bsp-packing.md` for generated file lists, version/provenance report fields, and legal/asset ownership notes.

Generate a pack list from common VMF material, model, sound, script, and particle references before packing:

```bash
cargo run -p sourceweaver-cli -- pack map.bsp \
  --tool /path/to/bspzip \
  --output packed.bsp \
  --asset-root /path/to/game \
  --discover-from-vmf merged.vmf \
  --report pack-report.json \
  --json
```

The `discovered_dependencies` report is reviewable before distribution and records missing or ambiguous assets. Source Weaver still does not bundle or validate BSPZIP itself unless the user-provided external packer is actually run.

List documented BSPZIP/BSPZIP++ context profiles and wrapper boundaries:

```bash
cargo run -p sourceweaver-cli -- bspzip-context-profiles --json
```

Use `--context-profile`, `--tool-cwd`, repeated `--library-path`, `--game-dir`, and explicit `--pass-game-dir` on `sourceweaver pack` when a user-provided packer needs game-bin, LD_LIBRARY_PATH, or wrapper-compatible `-game` context. See `docs/bspzip-context-profiles.md` for profile details and wrapper examples.

Prepare a cubemap/buildcubemaps runtime plan for a compiled BSP:

```bash
cargo run -p sourceweaver-cli -- cubemap-workflow map.bsp \
  --profile hl2-hdr \
  --steam-app-id 220 \
  --write-cfg cfg/sourceweaver_buildcubemaps.cfg \
  --report cubemap-report.json \
  --json
```

The cubemap workflow command writes a report and optional cfg helper only. It does not launch Steam, a Source game runtime, Hammer, Hammer++, VBSP, VVIS, VRAD, BSPZIP, or BSPSource. See `docs/cubemap-workflow.md` for profile caveats, log capture expectations, and real-runtime evidence requirements.

Inspect a model header, run a user-provided StudioMDL-compatible wrapper, or launch a user-provided headless model-decompile wrapper:

```bash
cargo run -p sourceweaver-cli -- model-inspect models/props/example.mdl --json

cargo run -p sourceweaver-cli -- model-compile model.qc \
  --studiomdl /path/to/studiomdl-or-wrapper \
  --game /path/to/game/content-dir \
  --tool-arg -nop4 \
  --log model-compile.log \
  --report model-compile-report.json \
  --json

cargo run -p sourceweaver-cli -- model-decompile models/props/example.mdl \
  --tool ./examples/wrappers/model-decompile-wrapper.sh \
  --output-dir decompiled/example \
  --tool-arg --input \
  --tool-arg '{input}' \
  --tool-arg --output \
  --tool-arg '{output-dir}' \
  --log model-decompile.log \
  --report model-decompile-report.json \
  --json
```

See `docs/model-tooling.md` and `docs/model-decompile.md` for Crowbar research, licensing notes, wrapper usage, and model-tooling boundaries.

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

Desktop project files use the same TOML shape as `sourceweaver run --job` where possible. Saving a project writes the current base VMF, input VMFs, landmark, output path, changelevel policy, and deletion rules. Paths under the project file directory are saved relative to that directory; relative paths are resolved from the project file location when loaded.

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
packaging/                     Linux desktop entry/icon and Windows icon assets
scripts/package-linux.sh       Linux release tarball builder
scripts/package-windows.ps1    Windows release zip builder
scripts/validate-public-vmfs.sh Public real-VMF validation smoke script
tests/fixtures/                Small VMF files used for local validation
tests/golden/                  Golden VMF and JSON snapshots used by regression tests
```

For real-map smoke validation, run `scripts/validate-public-vmfs.sh /tmp/sourceweaver-real-validation`. It downloads two adjacent public Source 1 VMFs from a pinned commit, inspects them, merges them with a real landmark, validates the merged output, and exercises the compile-pipeline report path with a fake VBSP tool. See `docs/real-vmf-validation.md`.

## Important limitations

Source Weaver is still early in the rebuild.

Known limitations:

- The current map preview includes 2D orthographic views and a lightweight 3D isometric viewport based on reconstructed convex brush face polygons with bounds fallback. It can preview single VMFs and the current in-memory merged output, but it is not yet a full textured Hammer clone. See `docs/preview-geometry.md` and `docs/3d-preview.md`.
- No bundled/internal BSP decompilation. BSP import can run user-selected BSPSource launchers/jars or generic external wrappers and imports the generated VMF; Source Weaver remains VMF-first. See `docs/bsp-import.md`.
- FGD support parses supported class declarations and representative property metadata syntax, including labels, descriptions, defaults, choices, and flags. It intentionally skips unsupported full-FGD language features safely.
- Compile, BSP packing, BSP decompile, and model compile integrations require user-provided Source tool paths; Source tools, Hammer, Crowbar, StudioMDL, game content, model assets, and custom assets are not bundled. Desktop compile launch uses the Source Weaver CLI compile pipeline and remains separate from VMF export success.
- Texture-axis translation adjusts `uaxis`/`vaxis` offsets with fixture coverage; see `docs/texture-axes.md`. Displacement translation currently moves side planes and `dispinfo` `startposition`; see `docs/displacements.md`.
- Incoming IDs are renumbered during merge, known reference fields are remapped, and unsupported suspected ID-reference keys are surfaced as warnings; see `docs/id-renumbering.md`.
- Top-level editor metadata is preserved from the base VMF and intentionally not merged from incoming VMFs; see `docs/editor-metadata.md`.
- Entity metadata uses built-in semantics, inferred categories, optional FGD class descriptions, and selected FGD property metadata; see `docs/entity-metadata.md`.
- Very large merged maps can still hit Hammer or Source compiler limits.

## Design principle

The core engine should preserve data it does not understand. Source maps often contain game-specific entities and custom Hammer data, so Source Weaver should inspect and move VMF content conservatively instead of relying on a fixed whitelist.

## License

MIT
