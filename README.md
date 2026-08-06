# Source Weaver

Source Weaver is a Linux and Windows tool for automatically combining Source Engine campaign VMFs into a single editable Hammer map. It is designed for projects such as Half-Life 2 and Black Mesa where multiple campaign maps need to be stitched together, inspected, cleaned, and exported as one VMF.

## Goals

- Merge selected `.vmf` files into one Hammer-editable `.vmf`.
- Move each incoming map to a shared `info_landmark` targetname so campaign transitions line up.
- Preserve and append world brushes, including skybox brushes, so each selected map contributes its skybox shell.
- Detect and list all top-level VMF `entity` blocks and their Hammer `classname` values without requiring a fixed whitelist.
- Detect important brush roles such as triggers, clips, areaportals, occluders, skybox, hint, skip, nodraw, water, world brushes, and brush entities.
- Delete entities and brushes by classname, targetname, or brush role, including bulk-style deletion rules.
- Keep the VMF engine separate from the UI so Linux and Windows builds share the same parser, merger, and deletion behavior.

## Current rebuild slice

The repository has been restarted with a Rust core and CLI. The CLI is intentionally the first deliverable because it exercises the map logic before a desktop UI is added.

```bash
cargo test --workspace
cargo run -p sourceweaver-cli -- inspect path/to/map.vmf
cargo run -p sourceweaver-cli -- list-types path/to/map.vmf
cargo run -p sourceweaver-cli -- prune path/to/map.vmf -o cleaned.vmf --drop-classname prop_static --drop-role clip
cargo run -p sourceweaver-cli -- merge -o stitched.vmf --landmark landmark_name base.vmf next.vmf another.vmf
```

## Planned desktop workflow

1. Select multiple VMFs.
2. Choose a base map and landmark targetname.
3. View every detected Hammer entity type and every individual entity record.
4. Filter by classname, targetname, role, map source, and brush material role.
5. Bulk-select matching entities or brushes.
6. Preview deletion rules before applying them.
7. Merge selected VMFs into one output file.
8. Open the generated VMF in Hammer for compile-time validation.

## Non-goals for the first slice

- BSP decompilation.
- Automatic leak fixing.
- Full compile pipeline integration.
- Custom FGD parsing.
- 3D preview rendering.

These are candidates for later milestones once VMF parsing, merging, and deletion are stable.
