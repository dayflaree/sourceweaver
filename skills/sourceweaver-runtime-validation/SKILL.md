---
name: sourceweaver-runtime-validation
description: Prove generated Source/GMod maps through isolated deterministic gameplay, rendering, lifecycle, and performance scenarios.
---

# Runtime validation

## Trigger

Use when a change can affect gameplay, rendering, collision, PVS, environment state, save/load, or performance.

## Preconditions

- compiled candidate and baseline BSPs;
- exact GMod branch/build profile;
- isolated addon/content set;
- versioned scenario definitions;
- watchdog and machine-readable result channel.

## Procedure

1. Fingerprint game build, mounted content, addons, settings, and hardware-relevant configuration.
2. Launch an isolated listen or dedicated-server environment.
3. Load baseline, run warm-up, then execute deterministic scenarios.
4. Capture console, entity state, region telemetry, screenshots, PVS/render metrics, and timing.
5. Reset cleanly between runs.
6. Execute the candidate under identical conditions.
7. Compare mandatory assertions first, performance second.
8. Repeat metric routes enough to distinguish practical change from noise.
9. Treat crash, hang, forbidden console error, missing observation, or inconclusive mandatory assertion as a blocker.
10. Retain logs/results and terminate every process.

## Mandatory stitching coverage

Forward/reverse seam, vehicle, doors, lifecycle once-only behavior, NPC/script timing, environment/audio handoff, save/reload, death/reset, repeated cycles, and multiplayer policy where supported.

## Mandatory optimization coverage

Portal doors open/closed, scripted/dynamic views, breakables, collision near edits, visual regression samples, and repeated performance routes.

## References

- `docs/RUNTIME_VALIDATION.md`
- `docs/METRICS.md`
