# Campaign lifecycle synthesis

## Problem

A normal Source level transition unloads one BSP, loads another, restores selected state around a landmark, and runs the destination map's startup lifecycle. Combining maps into one BSP removes that boundary. Without a replacement, destination NPCs, ambient sounds, logic, scripts, and effects may activate immediately and source-region systems may never retire.

## Region model

Each imported map becomes a region with explicit states:

```text
UNLOADED_MODEL -> PRELOADED -> ACTIVE -> DORMANT -> RETIRED
```

The name `UNLOADED_MODEL` means logically absent; all entities still reside in the BSP unless a supported runtime mechanism removes them.

### PRELOADED

Static world geometry and visibility data exist. Dynamic region entities are disabled, hidden, asleep, or withheld according to class policy.

### ACTIVE

The region's gameplay systems run. Spawn logic, ambient systems, AI, triggers, and controllers are enabled in a deterministic order.

### DORMANT

The player has left, but backtracking is allowed. Persistent state is retained while expensive systems are disabled where semantics permit.

### RETIRED

Backtracking is disallowed or the region is explicitly complete. Disposable entities may be removed.

## Entity lifecycle registry

Every known class receives actions for:

- initial materialization;
- activate;
- deactivate;
- reset;
- save/load;
- remove;
- unsupported conditions.

Examples:

| Class family | Preload policy | Activation policy | Deactivation policy |
|---|---|---|---|
| `logic_auto` | Prevent automatic region-B startup | Replay mapped outputs once on region activation | Never replay unless reset policy says so |
| NPCs | Start disabled/asleep when supported | Enable/spawn in authored order | Sleep, disable, or persist based on state policy |
| Ambient sound | Silent | Start/fade in | Stop/fade out |
| Fog/tonemap | Retained controller | Select region controller | Hand off to next controller |
| `trigger_once` | Disabled outside region | Enable | Preserve fired state |
| Scripted sequences | Disable activation paths | Enable with dependencies | Abort only when safe |
| Doors | Preserve physical/authored state | Enable controls | Keep state for backtracking |

Unknown classes affecting activation are blockers.

## Implemented support envelope

Current code provides an evidence-only lifecycle policy matrix. It classifies these registry entries:

- `logic_auto` as startup logic;
- `ambient_generic` as ambient sound;
- `env_fog_controller` and `env_tonemap_controller` as environment controllers;
- `trigger_once` as trigger lifecycle;
- `prop_door_rotating` as door lifecycle;
- `npc_*` as NPC lifecycle;
- `weapon_*` and `item_*` as pickup lifecycle.

Transition scaffolding already handled by stitching (`info_landmark`, `trigger_changelevel`, `trigger_transition`) is ignored by the lifecycle matrix. Unknown activation-affecting classes block synthesis until a class policy is added. The matrix is evidence-only and does not generate controllers, rewrite entities, or authorize source mutation.

Current code also builds a deterministic lifecycle controller plan from a clear matrix. The plan expands each entity policy into ordered `preload`, `activate`, `deactivate`, `reset`, and `remove` steps for one named region. Empty region names and blocked policy matrices stop planning. The controller plan is read-only: it does not emit VMF controller entities, wire outputs, or prove runtime behavior.

## Synthesized controller

The stitcher generates a deterministic region controller using supported Source/GMod entities or an optional companion Lua runtime. The implementation is profile-specific.

The controller owns:

- current region;
- previous region;
- transition direction;
- one-shot activation tokens;
- persisted region variables;
- activation/deactivation queues;
- failure logging;
- runtime test instrumentation.

## Activation ordering

A topological order is derived from the entity I/O and parent graphs:

1. shared/global controllers;
2. environment controllers;
3. parents and movement bases;
4. required props and doors;
5. NPC relationships and makers;
6. scripts and relays;
7. triggers and one-shot startup events;
8. ambient and cosmetic systems.

Cycles are collapsed into strongly connected components and require a class-specific cycle policy. Unknown cycles block automatic synthesis.

## Transition state

The normal engine can carry movable/global entities near a landmark. In a merged map, SourceWeaver classifies each authored transition-relevant entity as:

- shared singleton;
- physically continuous entity already present;
- source-region persistent entity;
- destination duplicate to suppress;
- destination replacement to activate;
- unsupported custom state.

The player and vehicle continue physically through the seam. Inventory, globals, and game-mode state remain in the same runtime. Duplicated destination stand-ins must be suppressed.

## Backtracking policies

- **Bidirectional:** regions become dormant and can reactivate with preserved state.
- **Checkpointed:** crossing commits a snapshot; return triggers a controlled reset.
- **One-way:** source region retires and the seam closes.

The tool infers a proposal from authored triggers and geometry, then requires a declared policy when evidence is ambiguous.

## Runtime assertions

The generated controller exposes test-only observability:

- current region ID;
- activation token counts;
- enabled entity counts by class;
- unexpected early activations;
- duplicate singleton controllers;
- unresolved output targets;
- transition timestamps;
- persistent state checksums.

Acceptance requires forward, reverse, save/reload, death/respawn, and repeated-transition scenarios where applicable.
