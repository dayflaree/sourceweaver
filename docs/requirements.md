# Source Weaver requirements

## Product statement

Source Weaver automatically merges selected Source Engine VMF campaign maps into a single map while giving users full visibility and deletion control over entities, Hammer classnames, and brush categories.

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
