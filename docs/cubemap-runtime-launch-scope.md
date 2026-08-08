# Cubemap runtime launch scope

Source Weaver's cubemap/buildcubemaps workflow is **plan-only**. The project will not launch Steam or a Source game runtime for cubemap building in the current release line.

## Decision

The current CLI remains a planner:

```text
runtime_launch_mode = "plan-only"
real_game_runtime_validation = false
```

`sourceweaver cubemap-workflow`, `sourceweaver cubemap-plan`, and `sourceweaver buildcubemaps` may write JSON reports, suggested launch commands, console-command sequences, and cfg helpers. They do not execute the suggested command, launch Steam, launch a game executable, monitor a process, enforce a timeout, tail console logs, or check BSP mutation.

This decision keeps Source Weaver inside its redistribution and evidence boundaries. Cubemap generation requires a real game runtime, a compiled BSP, user-owned content, and usually a writable game install or map directory. Those inputs can reveal private Steam paths and project content, and the `buildcubemaps` command mutates the BSP.

## Current report guarantees

A cubemap workflow report can state:

- which profile was selected;
- which launch arguments are suggested;
- which Steam or direct executable command shape the user supplied enough information to construct;
- which console/cfg commands should be run in the game runtime;
- that `buildcubemaps` writes cubemap textures into the BSP;
- that `real_game_runtime_validation` is `false`;
- that no Steam client, game runtime, SDK tool, BSPZIP, VBSP, VVIS, or VRAD process was launched by Source Weaver.

A report must not state:

- that cubemaps were built;
- that a runtime was launched;
- that console logs were captured;
- that the BSP changed;
- that reflections were visually verified;
- that the workflow timed out safely;
- that a game-specific cubemap workaround succeeded.

## Why runtime launch stays out of scope

A safe launcher would need all of these before implementation:

- explicit user-provided game executable or Steam app launch path;
- explicit game directory and map/BSP staging plan;
- timeout and process-tree cleanup;
- console-log discovery and capture per Source branch;
- before/after BSP hash, size, and timestamp checks;
- backup/restore policy for mutated BSPs;
- private-path redaction;
- failure classification for Steam, Proton, native Linux, and Windows runtimes;
- no redistribution of game content, runtime files, logs with private content, or generated BSPs.

That is closer to runtime validation work than cubemap planning, so it remains out of scope unless a future issue implements a dedicated, opt-in, evidence-preserving runtime launcher.

## Manual user workflow

The supported manual workflow is:

1. Compile the map with user-provided compiler tools.
2. Back up the compiled BSP.
3. Run `sourceweaver cubemap-workflow <map.bsp> --profile <profile> --write-cfg <cfg> --report <json>`.
4. Review the generated report and cfg helper.
5. Launch the game/runtime manually with the suggested arguments.
6. Run or `exec` the generated cfg in the game console.
7. Preserve the game console log and before/after BSP hashes outside the repository if adding evidence.
8. Repack/distribute only after cubemap building is complete and redistribution rights are clear.

## Future launcher acceptance criteria

If a future issue implements a launcher, it must add tests for command construction plus safe failure behavior. Minimum evidence must include:

- synthetic command-construction tests that do not launch a real runtime;
- timeout tests with a harmless local helper process;
- log-capture tests with synthetic logs;
- redaction tests for private paths;
- a real-runtime row only when a user-provided game/runtime is available and evidence stays outside the repository;
- release wording that distinguishes launcher plumbing from successful cubemap generation.

## External references checked

- Valve Developer Union cubemap guide, checked 2026-08-08: https://valvedev.info/guides/cubemaps-what-they-do-and-how-to-use-them/ . It says `buildcubemaps` is run in the game after loading the map, notes that cubemaps are embedded in the BSP, describes HDR/LDR, TF2/Source 2013 MP, L4D, Portal 2, and CS:GO caveats, and warns that cubemaps can break if the BSP is renamed after building. This supports keeping Source Weaver's automated scope to planning and evidence preparation.
