# Source Weaver

Source Weaver is a cross-platform Source Engine VMF tool for combining campaign maps into one Hammer-editable map. It is being built for workflows around games such as Half-Life 2, Black Mesa, and other Source 1 projects that use `.vmf` map sources.

The project has been restarted from a clean repository. The current implementation is a Rust VMF core plus a command-line interface. A desktop UI is planned after the merge, inspection, and deletion engine is stable.

## What Source Weaver is meant to do

Source Weaver takes selected VMF files and creates a single merged VMF. It is designed around campaign map stitching, where separate maps need to line up at transition landmarks and remain editable in Hammer afterward.

Core goals:

- Merge multiple selected `.vmf` files into one output `.vmf`.
- Align incoming maps to a shared `info_landmark` targetname.
- Preserve incoming world brushes, including skybox brushes.
- Preserve incoming point entities and brush entities.
- Detect every Hammer entity classname present in the VMF, including unknown or game-specific classnames.
- Classify common brush roles such as triggers, clips, areaportals, occluders, skybox, hint, skip, nodraw, and water.
- Delete map content in bulk by classname, targetname, or brush role.

## Current status

Implemented now:

- VMF KeyValues-style parser and writer.
- Ordered VMF tree model that keeps unknown blocks and keys.
- Entity inspection command.
- Classname summary command.
- Bulk prune command.
- Landmark-aligned merge command.
- Incoming ID renumbering to reduce Hammer conflicts.
- Tests and VMF fixtures for parser, transform, classification, prune, and merge behavior.

Planned next:

- Desktop UI for Linux and Windows.
- File picker for selecting VMFs.
- Entity table with filtering, sorting, and bulk selection.
- Deletion preview before applying cleanup rules.
- Merge warnings for missing landmarks and duplicate names.
- Hammer/compiler validation workflows.

## Repository layout

```text
crates/sourceweaver-core/   VMF parser, inspection, deletion, transform, and merge engine
crates/sourceweaver-cli/    CLI for validating and using the core engine
docs/                       Requirements, architecture, and roadmap notes
tests/fixtures/             Small VMF files used for local validation
```

## Requirements

- Rust stable toolchain
- Linux, Windows, or another platform supported by Rust

The current CLI has no external runtime dependencies beyond the Rust standard library and the local `sourceweaver-core` crate.

## Build and test

```bash
cargo fmt --check
cargo test --workspace
cargo build --workspace
```

## CLI usage

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

The first VMF is used as the base document. Each additional VMF contributes its world solids and entities. When `--landmark` is supplied, matching `info_landmark` origins are used to translate incoming geometry and entities into the base map's coordinate space.

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

## Important limitations

Source Weaver is early in the rebuild. The current CLI is useful for validating the VMF engine, but it is not yet the final desktop tool.

Known limitations:

- No graphical interface yet.
- No BSP decompilation.
- No FGD-backed property labels yet.
- No automatic compile pipeline yet.
- Texture-axis and displacement edge cases may need additional handling as real campaign maps are tested.
- Very large merged maps can still hit Hammer or Source compiler limits.

## Design principle

The core engine should preserve data it does not understand. Source maps often contain game-specific entities and custom Hammer data, so Source Weaver should inspect and move VMF content conservatively instead of relying on a fixed whitelist.

## License

MIT
