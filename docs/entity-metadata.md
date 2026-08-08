# Entity metadata and FGD support

Source Weaver keeps unknown Source/Hammer classnames visible, then enriches classnames with metadata when it can.

## Built-in metadata

The core library includes built-in category and description metadata for common Half-Life 2 / Black Mesa style entities, including landmarks, changelevel triggers, props, logic entities, brush entities, overlays, lights, sound entities, NPCs, and player starts.

Unknown classnames are never hidden. If a classname matches a common prefix such as `trigger_`, `func_`, `prop_`, `logic_`, `npc_`, `light`, or `info_`, Source Weaver infers a broad category and marks the source as inferred. Completely unknown classnames remain category `other` with source `unknown`.

## Loading FGD files

The desktop app has a **Load FGD metadata...** action. Loaded FGD records override built-in or inferred metadata for matching classnames.

Supported lightweight FGD class declarations include lines like:

```fgd
@PointClass base(Targetname) = info_target : "Target point"
@SolidClass base(Targetname) = func_ladder : "Climbable ladder brush"
```

The parser extracts:

- classname
- point/solid/NPC annotation category when available
- quoted class description
- source FGD path
- supported property keys, value types, labels, defaults, descriptions, choices, and flags

The parser is intentionally lightweight; it does not attempt to evaluate the full FGD inheritance tree, resolve includes, evaluate editor helper metadata, or execute FGD expressions. Unknown or complex FGD content is skipped safely. Supported class declarations can include property definitions with labels, value types, default values, descriptions, choices, and flags. See `docs/fgd-support-matrix.md` for the exact supported/unsupported syntax matrix.

## Property metadata

Supported property examples include normal keyvalue definitions, choices, and flags:

```fgd
@PointClass base(Targetname) = trigger_custom : "Custom trigger"
[
    targetname(target_source) : "Name" : : "Entity name used by Source I/O"
    mode(choices) : "Mode" : 0 : "How this trigger starts"
    [
        0 : "Disabled" : "Starts disabled"
        1 : "Enabled" : "Starts enabled"
    ]
    spawnflags(flags) =
    [
        1 : "Starts enabled"
        2 : "Clients only"
    ]
]
```

`sourceweaver inspect <map.vmf> --fgd entities.fgd --json` includes selected property metadata under each entity's `metadata.properties` array. Text inspect output prints `property` lines for parsed property labels and descriptions.

## Desktop display

Entity and classname tables show:

- category
- friendly display name
- classname
- description
- FGD property count and tooltip details when property metadata is loaded

Search covers classname, category, friendly name, description, targetname, role data, property keys, property labels, property descriptions, value types, and choices. Entity rows can be sorted by category.

## Future expansion

If richer FGD behavior is needed, add fixtures for the exact FGD syntax before expanding the parser. The current parser focuses on class declarations and representative keyvalue metadata; it does not claim complete Hammer FGD language coverage. `@BaseClass` inheritance definitions are skipped rather than emitted as entity metadata.
