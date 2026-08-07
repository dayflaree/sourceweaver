# Cubemap/buildcubemaps workflow

Source Weaver can prepare a cubemap workflow report for a compiled BSP, but cubemap generation itself is a Source game-runtime action. The `buildcubemaps` console command renders cubemap textures from `env_cubemap` entities and writes those textures into the loaded `.bsp`. This must stay separate from VMF merge validation, VBSP/VVIS/VRAD compile validation, BSP packing, and release packaging.

## CLI planner

Generate a machine-readable plan and optional cfg helper:

```bash
sourceweaver cubemap-workflow map.bsp \
  --profile hl2-hdr \
  --steam-app-id 220 \
  --write-cfg cfg/sourceweaver_buildcubemaps.cfg \
  --report cubemap-report.json \
  --json
```

The command validates that the BSP path exists, infers the map name from the file stem, and reports:

- selected cubemap profile and research notes;
- suggested launch arguments such as `-dev`, `-console`, `-condebug`, window size, and profile-specific flags;
- optional direct executable command or Steam `-applaunch` command when the user supplied `--game-executable` or `--steam-app-id`;
- console commands to run inside the game runtime;
- optional cfg path when `--write-cfg` is used;
- warnings about BSP mutation, reload/restart caveats, and map naming;
- `real_game_runtime_validation = false` because Source Weaver did not launch or observe a real game runtime.

Aliases are available for scripting convenience:

```bash
sourceweaver cubemap-plan map.bsp --profile generic --json
sourceweaver buildcubemaps map.bsp --profile csgo --steam-app-id 730 --json
```

## Supported profiles

`generic` emits a conservative Source workflow:

```text
mat_specular 0
map <map>
sv_cheats 1
buildcubemaps
disconnect
mat_specular 1
map <map>
```

`hl2-hdr` emits separate LDR/HDR passes from the researched workflow:

```text
map <map>
sv_cheats 1
mat_hdr_level 0
reload
buildcubemaps
mat_hdr_level 2
reload
buildcubemaps
disconnect
```

`tf2-source2013mp` emits the Source 2013 MP / Team Fortress 2 specular-off workaround and notes that HDR targets may need a second HDR pass.

`csgo` adds `-insecure` to suggested launch arguments and emits a single HDR-oriented build pass. Run it only in an offline mapping workflow for a local CS:GO legacy/tool install.

`l4d` records the Left 4 Dead / Left 4 Dead 2 restart caveat after building.

`portal2` records the Portal 2 caveat that maps may need to be built from the `portal2_dlc2/maps` location rather than `portal2/maps`.

## Log capture and evidence

Use `-condebug` in the launch arguments so the target game writes console output to its console log location, commonly `console.log` or `qconsole.log` depending on branch/configuration. Preserve that log with the generated `cubemap-report.json` when adding real validation evidence.

A complete real validation record should include:

- Source Weaver cubemap workflow report JSON;
- target game, branch, Steam app ID or executable path, game directory, and launch arguments;
- exact BSP path before the run and whether it was backed up;
- console log captured from the game runtime;
- whether the BSP timestamp/size changed after `buildcubemaps`;
- visual/manual reflection check result after reload or game restart when required.

## Boundary

`sourceweaver cubemap-workflow` does not launch Steam, HL2, TF2, CS:GO, Portal 2, Left 4 Dead, Garry's Mod, or any other game runtime. It does not run Hammer, Hammer++, VBSP, VVIS, VRAD, BSPZIP, or BSPSource. It only creates a plan and optional cfg helper for a user-controlled runtime step.

Because `buildcubemaps` mutates the BSP, run it after the final compile and before final asset packing/distribution. If the target game needs BSPZIP `-deletecubemaps` or other cleanup before rebuilding, keep that as a separate user-confirmed BSPZIP step and preserve backups because cubemap deletion can remove embedded VTF data.

## Sources checked

- Valve Developer Union, `Cubemaps: What They Do, and How to Use Them`, checked 2026-08-07: https://valvedev.info/guides/cubemaps-what-they-do-and-how-to-use-them/
- Valve Developer Community command-line options mirror, checked 2026-08-07 for `-buildcubemaps`, `-condebug`, `+map`, and `+sv_cheats`: https://kogitae.fr/blog/command-line-options-valve-developer-community/
