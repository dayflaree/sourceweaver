# Entity semantics and reference rewriting

## Rule

Strings are never rewritten based solely on appearance. A value such as `255 0 0` may be a color, a vector, or arbitrary script data. Transform and namespace operations require a schema-backed type.

## Schema sources

Priority order:

1. project/game override schema;
2. exact target game FGD files;
3. HammerAddons/srctools unified FGD database;
4. built-in conservative Source base schema;
5. unknown.

The report records which definition supplied each type.

## Typed categories

- world position;
- local position;
- direction/vector;
- Euler angle;
- targetname definition;
- targetname reference;
- entity class;
- model/material/sound/script/scene resource;
- output event and input;
- output parameter with input-specific typing;
- color;
- scalar/integer/flags;
- branch-specific opaque value.

## Targetname graph

The graph retains:

- duplicate definitions;
- wildcard references;
- special names (`!activator`, `!caller`, `!self`, and branch-specific tokens);
- case-insensitive matching behavior where applicable;
- parent relations;
- instance fixup scopes;
- global names;
- unresolved and ambiguous references.

## Namespacing

Map-local names from the imported map receive a deterministic prefix, for example:

```text
d1_trainstation_06__door_1
```

A name is exempt when it is:

- a Source special token;
- declared global by policy;
- intentionally shared across regions;
- controlled by a user-approved reconciliation rule.

All typed references and outputs are rewritten in the same transaction. The transaction fails if any affected reference is unknown or ambiguous.

## Implemented FGD-typed keyvalue coverage

Current code extends the CST-backed targetname graph with a conservative built-in Source schema for typed keyvalue references outside `parentname` and outputs. Covered classes/keys include:

- `env_entity_maker.EntityTemplate`;
- `filter_activator_name.filtername`;
- `func_button.target`;
- `func_door.filtername` and `func_door_rotating.filtername`;
- `func_tracktrain.target`;
- `logic_branch_listener.Branch01` through `Branch16`;
- `math_counter.OutValue`;
- `momentary_rot_button.target`;
- `path_track.target` and `path_track.altpath`;
- `point_template.Template01` through `Template16`;
- `trigger_multiple.filtername` and `trigger_once.filtername`.

These references resolve through the same targetname graph as parent/output references. Unknown key strings remain opaque and are not rewritten by appearance.

## Outputs

Source output values may use comma or ESC separators depending on branch and tooling. The semantic adapter must parse both and materialize using the original separator unless the target profile requires conversion.

Outputs remain ordered and duplicate event keys remain distinct.

## Scripts and opaque behavior

VScript, map Lua, RunScriptCode, console commands, choreography, and custom GMod entities may contain embedded names or coordinates. These are classified as:

- statically understood and transformable;
- understood enough to detect dependency but not rewrite;
- opaque.

An opaque dependency touching a transformed object blocks automatic acceptance. AI may explain the risk; it cannot waive the blocker.
