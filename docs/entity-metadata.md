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

The parser is intentionally lightweight; it does not attempt to evaluate the full FGD inheritance tree or parse every keyvalue definition. Unknown or complex FGD content is skipped safely.

## Desktop display

Entity and classname tables show:

- category
- friendly display name
- classname
- description

Search covers classname, category, friendly name, description, targetname, and role data. Entity rows can be sorted by category.

## Future expansion

If richer FGD behavior is needed, add fixtures for the exact FGD syntax before expanding the parser. The current parser focuses on class-level metadata because Source Weaver's cleanup workflow primarily needs semantic grouping and readable descriptions.
