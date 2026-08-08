# Runtime map-load validation workflow

Source Weaver can prepare a runtime map-load validation plan, but game runtime validation is complete only after a real game executable is launched and evidence is recorded. Portable VMF validation, Hammer/Hammer++ open/save validation, VBSP/VVIS/VRAD compile validation, BSP packing validation, and runtime map-load validation are separate evidence rows.

## CLI plan

```bash
sourceweaver runtime-map-load-workflow compiled_map.bsp \
  --game-dir /path/to/game \
  --game-exe /path/to/game-executable \
  --profile gmod \
  --map compiled_map \
  --console-log runtime-console.log \
  --compile-report smoke-compile-report.json \
  --pack-report pack-report.json \
  --merge-report merge-report.json \
  --report runtime-map-load-report.json \
  --json
```

The command emits:

- input BSP path and size;
- map name;
- target profile;
- game directory and optional executable path;
- launch argument plan such as `-dev -console -condebug +map <map>`;
- expected console log path;
- links to compile, pack, and merge reports;
- evidence requirements;
- manual validation steps;
- ownership caveats;
- external-tool boundary text;
- `real_game_runtime_validation = false`.

It does not launch Steam, Garry's Mod, Half-Life 2, any Source runtime, or any SDK tool.

## Manual run steps

1. Copy or mount the compiled BSP where the target game can load it. For Source 1 games this is usually `<game>/maps/<map>.bsp`.

2. Keep the producing evidence together:

   ```text
   merge report, if a merged VMF was used
   compile report, if VBSP/VVIS/VRAD produced the BSP
   pack report, if assets were packed into the BSP
   runtime map-load workflow report
   ```

3. Launch the target runtime using the tester's normal supported setup. Examples only:

   ```bash
   /path/to/gmod -dev -console -condebug +map sw_runtime_smoke
   ```

   ```bash
   /path/to/hl2.sh -game hl2 -dev -console -condebug +map sw_runtime_smoke
   ```

   ```bash
   STEAM_COMPAT_DATA_PATH=/path/to/compatdata \
   STEAM_COMPAT_CLIENT_INSTALL_PATH=/path/to/Steam \
   /path/to/proton run /path/to/game.exe -dev -console -condebug +map sw_runtime_smoke
   ```

4. Capture evidence:

   - exact game executable path;
   - game build/version or Steam app/build details;
   - launch command and working directory;
   - `console.log` or condebug output;
   - whether the game started;
   - whether the map loaded;
   - crash, hang, or timeout status;
   - missing material/model/sound/script warnings;
   - player spawn status;
   - lighting visibility;
   - obvious collision or trigger issues;
   - gameplay smoke notes;
   - screenshots or recordings when useful and safe to share.

5. If the runtime run fails, link the failure back to the source stage:

   - VMF merge/report issue;
   - compiler issue;
   - BSP packing or missing dependency issue;
   - game/runtime configuration issue;
   - content ownership or mount issue.

## JSON evidence template

```json
{
  "ok": false,
  "real_game_runtime_validation": true,
  "sourceweaver_commit": "",
  "date": "",
  "tester": "",
  "runtime": {
    "game": "garrysmod",
    "game_exe": "",
    "game_version_or_build": "",
    "game_dir": "",
    "working_dir": "",
    "launch_command": "",
    "runtime_mode": "native/linux|windows|wine|proton|steam"
  },
  "input_bsp": {
    "path": "",
    "size": 0,
    "sha256": "",
    "map_name": "",
    "redistributable": false
  },
  "linked_reports": {
    "merge_report": "",
    "compile_report": "",
    "pack_report": "",
    "hammer_validation": ""
  },
  "load_result": {
    "status": "not-run|pass|fail",
    "console_log": "",
    "missing_materials": [],
    "missing_models": [],
    "missing_sounds": [],
    "missing_scripts": [],
    "crashed": false,
    "hung": false,
    "timeout_seconds": 0,
    "player_spawned": false,
    "gameplay_smoke_notes": []
  },
  "attachments": {
    "screenshots": [],
    "recordings": [],
    "sanitized_logs": []
  },
  "follow_up_issues": []
}
```

## Completed runtime validation rows

### Issue #132: Garry's Mod dedicated server map-load failure row

Completed on 2026-08-08 with real runtime execution. This row records a failure result, not a successful gameplay load.

Input BSP:

```text
source: /tmp/sourceweaver-real-compiler-smoke-118/smoke_box.bsp
runtime copy: /home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/garrysmod/maps/sourceweaver_issue132_smoke_box.bsp
map name: sourceweaver_issue132_smoke_box
size: 65808 bytes
sha256: f421d86038e2e00cf426a53fccd66da2d25940a54247d7e79f5e33991ace39c5
ownership: generated local compile artifact from the real Source++ compiler smoke row; not committed or redistributed
```

Linked producing report:

```text
/tmp/sourceweaver-real-compiler-smoke-118/smoke-compile-report.json
```

Runtime:

```text
game: Garry's Mod dedicated server
game executable: /home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/bin/linux64/srcds
game dir: /home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/garrysmod
working dir: /home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/bin/linux64
runtime mode: native/linux Steam snap install
launch command: LD_LIBRARY_PATH=/home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/bin/linux64:${LD_LIBRARY_PATH:-} timeout 90 ./srcds -game garrysmod -console -condebug -insecure -noaddons -noworkshop +sv_lan 1 +maxplayers 1 +gamemode sandbox +map sourceweaver_issue132_smoke_box
```

Evidence artifacts:

```text
primary evidence directory: /tmp/sourceweaver-runtime-map-load-issue132-final
runtime plan: /tmp/sourceweaver-runtime-map-load-issue132-final/sourceweaver-runtime-map-load-plan.json
summary JSON: /tmp/sourceweaver-runtime-map-load-issue132-final/runtime-validation-summary.json
engine init stdout: /tmp/sourceweaver-runtime-map-load-issue132-final/attempt1-engine-init-stdout.log
engine init console: /tmp/sourceweaver-runtime-map-load-issue132-final/attempt1-engine-init-console.log
map-failure console: /tmp/sourceweaver-runtime-map-load-issue132-final/attempt2-map-failure-console.log
stderr logs: /tmp/sourceweaver-runtime-map-load-issue132-final/attempt1-engine-init-stderr.log and /tmp/sourceweaver-runtime-map-load-issue132-final/attempt2-map-failure-stderr.log
exit code files: /tmp/sourceweaver-runtime-map-load-issue132-final/attempt1-engine-init-exit-code.txt and /tmp/sourceweaver-runtime-map-load-issue132-final/attempt2-map-failure-exit-code.txt
```

Observed result:

```text
real_game_runtime_validation: true
load status: fail
runtime exit code: 100
console initialized: yes
Game.so loaded: yes
network/server startup: yes
map load success: no
crash: no native crash observed in srcds failure row
hang: no for the primary failure row
timeout: no for the primary failure row
player spawned: no; dedicated server validation did not create a client player
lighting visibility: not applicable because no rendered client loaded the map
collision/trigger smoke: not applicable because no client/player spawned
```

Important log excerpts:

```text
Console initialized.
Game.so loaded for "My Garry's Mod Server"
Game is ran with -noaddons, not loading legacy/folder addons!
Mounted 0 of 0 workshop addons!
Network: IP 127.0.1.1, mode MP, dedicated Yes, ports 27015 SV / 41543 CL
Warning! Singleplayer mode not available on dedicated server.
Executing dedicated server config file server.cfg
Using map cycle file cfg/mapcycle.txt.
Model models/Gibs/wood_gib01e.mdl not found and models/error.mdl couldn't be loaded
```

Failure classification:

- Real Garry's Mod dedicated runtime execution happened.
- The BSP was copied to the runtime `maps/` directory and the launch command requested `+map sourceweaver_issue132_smoke_box`.
- The runtime reached engine/server initialization and then failed before a successful map load because the local dedicated-server runtime could not resolve standard fallback model content.
- Additional temporary local symlink shims were tested for runtime library/resource/model probes. They were removed after evidence capture and are recorded in `/tmp/sourceweaver-runtime-map-load-issue132-shim-cleanup.log`.
- The failure is a runtime/content-mount issue, not a Source Weaver compile success claim and not a game-playable map-load pass.

Hashes for selected evidence artifacts:

```text
f421d86038e2e00cf426a53fccd66da2d25940a54247d7e79f5e33991ace39c5  smoke_box.bsp / runtime BSP copy
a83549d5db6822f38ed1b0af2944ebee8ae640faf421844775dcc596fecacea3  attempt1-engine-init-console.log
eea8254c7500ba3de996aa8ad6af399183f04e17d4a8102fde539dbc93a90012  attempt1-engine-init-exit-code.txt
ef2d373ab460605598df19c3ce48b9abd86edc010d1f8bfbeb4530db8defe297  attempt1-engine-init-stderr.log
eac3ecb93f9072b2dd72eceea0a88561dfc36d18946dfbee973f22d1cafb5e17  attempt1-engine-init-stdout.log
d1ce6932d4c8395f974c6e4feb09f5154ce5f671485e79d49c8e67a7751c56c8  attempt2-map-failure-console.log
eea8254c7500ba3de996aa8ad6af399183f04e17d4a8102fde539dbc93a90012  attempt2-map-failure-exit-code.txt
33372088700ece4d394f299ff4c9d018f75b2d39f23a063aa9b047545bf4d6f4  attempt2-map-failure-stderr.log
a2f23330f95a2ba12b8deccb1d7ed37144cd83941ed7c79b1dc3c5608e0b9675  bsp-sha256.txt
d6f60ef3cdd927f9a8829f8f3684eb9df1fed09c70461c5c2df9fe1dde67f8cd  runtime-validation-summary.json
ecfb53fab907a1c6facdac3888240721a19cda5e58be3233dea5601555ef5f9b  shim-cleanup.log
c7fbdd3eba1f2baa918d70b3e5e3a4277053d37114b7b109df75fa7dbb03357b  sourceweaver-runtime-map-load-plan.json
d2af24649567f69cfb0ff326e498c792da6840c0d0435af35518c3d3f5a4c25a  sourceweaver-runtime-map-load-plan-stdout.json
```

This row satisfies runtime execution evidence and unambiguous load-failure evidence. It does not satisfy, and must not be cited as, a successful in-game map-load, rendered lighting, player spawn, collision, or trigger smoke pass.

## Sanitization and ownership

Do not commit proprietary BSPs, extracted content, Steam account paths, screenshots of private maps, or full console logs that reveal private project/addon names unless sharing is permitted. Prefer issue comments with hashes, sizes, warning summaries, and private evidence paths when redistribution is unclear.

## Completion rule

A runtime map-load row is complete only when a real target game executable was launched and its console/load evidence was recorded. A `runtime-map-load-workflow` JSON plan, successful `sourceweaver compile`, successful Hammer open/save, or successful BSP packing run is not runtime validation by itself.
