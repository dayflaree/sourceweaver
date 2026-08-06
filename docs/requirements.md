# Source Weaver requirements

## User workflow

Source Weaver automatically merges selected Source Engine VMF campaign maps into a single map while giving users full visibility and deletion control over entities, Hammer classnames, and brush categories.

Desktop-created project files must remain compatible with the CLI job format where possible. Saving a project must include selected VMFs, base map, landmark, output path, deletion rules, deletion safety settings, and predictable relative paths. Loading a project must resolve relative VMF/output paths from the project file directory and restore the UI setup.

## Supported platforms

- Linux desktop
- Windows desktop

The core VMF engine must stay platform-neutral. Platform-specific UI or packaging code must call into the same core engine.

## Map merge requirements

### VMF selection

Users must be able to select multiple `.vmf` files. One VMF acts as the base output document. Additional VMFs are appended into the base.

### Landmark alignment

When a landmark targetname is supplied, Source Weaver must locate an `info_landmark` entity with that targetname in the base map and in each incoming map. Incoming geometry and entities are translated by:

```text
base_landmark_origin - incoming_landmark_origin
```

If a map lacks the requested landmark, the tool must report that fact and leave that map unshifted for the current slice.

The desktop UI must discover `info_landmark` targetnames from selected VMFs, offer discovered values in a dropdown, and keep manual targetname entry available for unusual or messy maps. Before preview/export, it must show per-map status for the chosen landmark and warn when a selected map lacks it, has duplicate matching landmarks, or has a missing/invalid landmark origin.

### Campaign transitions

The engine must detect `trigger_changelevel` entities and expose target map plus landmark-related properties. Desktop inspection and automation reports must surface this transition data so later workflows can suggest map order and landmark pairs.

## VMF integrity checks

Before writing cleaned or merged output, Source Weaver must validate the relevant VMFs and result document. It must error when an output has no editable top-level `world` block or has multiple top-level `world` blocks. It should warn about missing common VMF sections, missing IDs, duplicate numeric IDs, multiple ID fields, and non-numeric IDs on blocks that normally require stable Hammer IDs. CLI errors must include the VMF filename, and parse errors must include the parser byte position. The desktop UI and automation JSON reports must surface integrity warnings so users can inspect problems before export.

## Source tool validation

Source Weaver must provide a validation path that works on Linux without Hammer installed and can also consume real Source compiler logs. The CLI must validate a generated VMF structurally, parse captured VBSP logs for warnings, errors, and leaks, and optionally run a configured VBSP command when a user has Source tooling installed. CI should cover the portable path with fixture logs rather than depending on proprietary game tools.

### Skybox preservation

World solids from incoming maps must be appended to the base map, including brushes using skybox tool materials. This ensures each selected map can contribute its skybox shell.

### Entity preservation

All incoming top-level `entity` blocks must be appended unless a deletion rule removes them. This includes point entities and brush entities.

### ID collision handling

Incoming VMF `id` keys must be renumbered before insertion to reduce Hammer conflicts.

## Entity and brush inspection requirements

### Entity discovery

The tool must list every top-level VMF `entity` block and expose its Hammer `classname`, `targetname`, `origin`, solid count, and detected roles.

Source Weaver must not rely on a fixed Hammer entity whitelist. Unknown or game-specific classnames must still be shown.

Large VMFs must remain manageable in the desktop inspection UI. Entity rows must support text search across block name, classname, targetname, and roles; role filtering; visible-row counts; and sortable columns for index, block, classname, targetname, origin, solids, and roles. Classname summaries must support search, filtered counts, and sorting by classname or count. Map-source filtering for merged data depends on source-provenance tracking in the merged preview workflow.

Entity-table row selection must support multiple selected rows through checkboxes, select-all and clear controls, and a visible selected count. Selection keys must include the VMF path and row identity data so later deletion actions can target selected rows without confusing rows from different maps.

### Deletion safety modes

Brush-role deletion must distinguish world solids from brush-entity behavior. World solid role matches remove matching world solids. Brush-entity role matches must use an explicit mode: `whole-entity` removes matching brush entities as a unit and preserves the original behavior; `matching-solids` preserves the entity and removes only matching contained solids. Critical transition/player/logic entities must be protected by default from classname, targetname, and brush-role deletion unless protection is explicitly disabled.

Desktop deletion presets must generate transparent deletion criteria that users can inspect before applying. Presets should cover common cleanup workflows such as triggers, clips, areaportals, gameplay logic, world-only geometry, and world-plus-skybox cleanup. Preset previews must run the same deletion implementation as final export so preview counts match final deletion counts.

### Landmark discovery

The core engine must expose `info_landmark` discovery independently from the desktop UI. A discovered landmark record includes the targetname, parsed origin when available, and source entity index. Duplicate landmark targetnames within a map must be reported with counts so UI and automation layers can warn before merge.

### Brush role discovery

The tool must detect brush categories using classnames and side materials, including:

- triggers
- clips, including player and NPC clips
- areaportals
- occluders
- skybox brushes
- hint brushes
- skip brushes
- nodraw brushes
- water brushes
- world brushes
- brush entities
- raw `tools/...` materials

## Deletion requirements

Users must be able to delete map content by:

- classname
- targetname
- brush role

Deletion must support repeated or comma-separated filters so future UI bulk selections can map directly to the same core rules.

## Validation requirements

Every parser, merge, transform, classification, and prune change must have tests. Generated VMFs should be opened in Hammer or validated by compiler tooling in later milestones.
