# Runtime validation

## Purpose

Compilation proves structural acceptance. Runtime validation proves that the target GMod branch can load the BSP and that authored behavior still satisfies deterministic scenarios.

## Harness modes

- local listen server for rapid visual tests;
- dedicated server for server-authoritative behavior;
- client connected to dedicated server for networking/PVS tests;
- optional headless or low-render mode for logic-only scenarios;
- stable and dev branches qualified separately.

## Test addon

A small SourceWeaver GMod addon provides test-only instrumentation:

- scenario command dispatch;
- entity lookup and key state assertions;
- trigger touch/event observation;
- current PVS/leaf/area sampling where engine APIs permit;
- player/vehicle teleport through legal test routes;
- screenshot and console capture;
- region lifecycle telemetry;
- watchdog and completion result file.

The addon is generated or installed separately and is never required by an exported map unless the selected lifecycle implementation explicitly uses a runtime component.

## Scenario format

```yaml
id: seam_forward_walk
map: generated_map
setup:
  - reset_map
  - wait_for_idle
steps:
  - spawn_player: checkpoint_a
  - follow_route: route_to_seam
  - assert_region: region_b
  - assert_no_console_errors
  - assert_entity_state: {name: region_b_door, enabled: true}
  - capture_metrics: seam_after_cross
teardown:
  - write_result
```

Scenario inputs are versioned and deterministic. Random gameplay systems receive a fixed seed where possible.

## Mandatory stitching scenarios

- map load and player spawn;
- forward seam traversal on foot;
- forward traversal in each supported vehicle class;
- reverse traversal/backtracking policy;
- source/destination door sequence;
- destination `logic_auto` equivalent fires exactly once;
- NPC and scripted sequence activation timing;
- ambient, soundscape, fog, tonemap, color correction, and sky transition;
- transition-relevant entity continuity;
- save and reload before/inside/after seam;
- death/respawn and map cleanup;
- repeated transition cycles;
- multiplayer join-in-progress when supported;
- no early activation from inactive regions.

## Mandatory optimization scenarios

- load and traverse all visibility sample routes;
- open/close every portal-linked door;
- capture open/closed visibility metrics;
- exercise breakables, cameras, mirrors, and scripted views that could reveal nodraw faces;
- compare reference screenshots within a qualified tolerance;
- assert collision and trigger behavior around changed brushes;
- verify no new console errors or missing resources.

## Verdicts

- **pass:** every mandatory assertion succeeds;
- **fail:** a defined assertion fails, process crashes, hangs, or emits a forbidden error;
- **inconclusive:** observability is insufficient or the environment changed;
- **skipped:** scenario is outside the selected support envelope and cannot contribute to acceptance.

An inconclusive mandatory scenario blocks automatic acceptance.

## Implemented support envelope

Current code builds a read-only runtime acceptance manifest when compiler preflight is ready and both baseline and candidate BSP artifacts exist. The manifest contains mandatory scenarios for map load/spawn, forward and reverse seam traversal, lifecycle relay cycling, save/reload, death/respawn cleanup, and repeated transition cycles.

The manifest is a preflight artifact. It does not launch GMod, execute scenarios, collect telemetry, or issue pass/fail runtime verdicts. Missing compiled BSPs, blocked compiler preflight, and empty map names block the manifest.

## Safety

- Run generated maps in an isolated test game directory/profile.
- Disable unrelated Workshop addons.
- Record mounted content and addon hashes.
- Restrict console commands to a test allowlist.
- enforce timeouts and process-tree cleanup;
- retain crash dumps and logs without collecting unrelated personal data.
