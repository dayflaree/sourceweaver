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

## Sanitization and ownership

Do not commit proprietary BSPs, extracted content, Steam account paths, screenshots of private maps, or full console logs that reveal private project/addon names unless sharing is permitted. Prefer issue comments with hashes, sizes, warning summaries, and private evidence paths when redistribution is unclear.

## Completion rule

A runtime map-load row is complete only when a real target game executable was launched and its console/load evidence was recorded. A `runtime-map-load-workflow` JSON plan, successful `sourceweaver compile`, successful Hammer open/save, or successful BSP packing run is not runtime validation by itself.
