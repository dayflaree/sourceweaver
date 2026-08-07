# BSPZIP context profiles and wrappers

Source Weaver can run a user-provided BSPZIP, BSPZIP++, or compatible wrapper, but the executable often depends on the game/tool context it was launched from. Context profiles document that environment so JSON reports are reviewable and repeatable.

List profiles:

```bash
sourceweaver bspzip-context-profiles --json
```

Run packing with context fields:

```bash
sourceweaver pack map.bsp \
  --tool ./examples/wrappers/bspzip-linux-ld-library-path-wrapper.sh \
  --output packed.bsp \
  --asset-root /path/to/game \
  --include materials/custom/wall01.vmt \
  --context-profile linux-ld-library-path \
  --tool-cwd /path/to/game/bin \
  --library-path /path/to/game/bin \
  --report pack-report.json \
  --json
```

## Profile fields

`--context-profile <id>` records which documented context was intended. It does not change the command by itself.

`--tool-cwd <dir>` runs the packer from a specific working directory. Use this when a stock game-bin BSPZIP expects to run beside DLLs/shared libraries or when vproject-style auto-detection depends on the launch directory.

`--library-path <dir>` can be repeated. Source Weaver prepends those paths to `LD_LIBRARY_PATH` for the packer process and for the version probe. The report records configured paths and the environment key name, while the exact inherited environment is not expanded into the report.

`--game-dir <dir>` records the target game/content directory.

`--pass-game-dir` inserts `-game <dir>` before `-addlist`. Use it only for wrappers or packers that you have verified accept that argument. Stock BSPZIP argument support varies across branches, so Source Weaver requires this separate opt-in flag.

## Built-in context profiles

### `stock-game-bin`

Use for a user-provided Valve BSPZIP executable from a game `bin` directory. The Valve Developer Union BSPZIP guide documents running BSPZIP from the same game bin location as Hammer/SDK tools and using `-addlist <input.bsp> <filelist.txt> <output.bsp>` with internal/external path-pair file lists.

Recommended Source Weaver fields:

```bash
--context-profile stock-game-bin \
--tool-cwd /path/to/game/bin
```

### `linux-ld-library-path`

Use for Linux local toolchains or wrappers that need Source/Steam runtime library directories. Source Weaver sets `LD_LIBRARY_PATH` only for the packer process and version probe.

Recommended Source Weaver fields:

```bash
--context-profile linux-ld-library-path \
--tool-cwd /path/to/game/bin \
--library-path /path/to/game/bin \
--library-path /path/to/steam-runtime/lib
```

Example wrapper: `examples/wrappers/bspzip-linux-ld-library-path-wrapper.sh`.

### `bspzipplusplus-sdk2013-x64`

Use for a user-provided BSPZIP++ binary in a supported 64-bit SDK2013-based game/tool context. Hammer++ tools documentation checked on 2026-08-07 describes BSPZIP++ as a rewrite of SDK2013 BSPZIP, lists examples such as Team Fortress 2, Counter-Strike: Source, Day of Defeat: Source, and Half-Life 2: Deathmatch, and documents Garry's Mod as unsupported for BSPZIP++.

Recommended Source Weaver fields depend on where the user installed the tool:

```bash
--context-profile bspzipplusplus-sdk2013-x64 \
--tool-cwd /path/to/game/bin/x64
```

Check third-party redistribution terms before bundling any BSPZIP++ binary with a mod, tool pack, or release artifact.

### `explicit-game-arg-wrapper`

Use when a local wrapper or compatible packer requires `-game <dir>` before the BSPZIP operation.

```bash
sourceweaver pack map.bsp \
  --tool ./bspzip-wrapper.sh \
  --output packed.bsp \
  --asset-root /path/to/game \
  --include materials/custom/wall01.vmt \
  --context-profile explicit-game-arg-wrapper \
  --game-dir /path/to/game \
  --pass-game-dir \
  --json
```

Example wrapper: `examples/wrappers/bspzip-game-arg-wrapper.sh`.

## Report fields

Pack reports include `tool_context`:

```json
{
  "profile_id": "explicit-game-arg-wrapper",
  "profile_label": "Wrapper-compatible explicit -game context",
  "tool_cwd": "/path/to/game/bin",
  "game_dir": "/path/to/game",
  "pass_game_dir": true,
  "library_paths": ["/path/to/game/bin"],
  "environment_keys": ["LD_LIBRARY_PATH"],
  "warnings": ["-game <dir> is forwarded only because --pass-game-dir was supplied..."],
  "real_tool_validation": false
}
```

`real_tool_validation` remains `false` unless a real external packer is run and the evidence is recorded outside the profile registry. The bundled tests use fake wrapper-compatible tools only.

## Wrapper examples

- `examples/wrappers/bspzip-linux-ld-library-path-wrapper.sh` sets `LD_LIBRARY_PATH`, changes to the configured game/tool bin, and executes a local user-supplied packer.
- `examples/wrappers/bspzip-game-arg-wrapper.sh` prepends `-game <dir>` for wrapper-compatible tools.
- `examples/wrappers/bspzip-windows-game-bin-wrapper.ps1` runs from a Windows game bin directory so DLL and game-bin context matches a manual mapper command prompt.

## Boundary

Source Weaver does not bundle Valve BSPZIP, BSPZIP++, Hammer, Hammer++, SDK tools, Steam files, game runtimes, compiled BSPs, or game content. Context profiles document how a user-provided external packer should be launched. They are not real external-tool validation results.

## Sources checked

- Valve Developer Union, `Zipping Files Into a Map Using BSPZIP or VIDE`, checked 2026-08-07: https://valvedev.info/guides/zipping-files-into-a-map-using-bspzip-or-vide/
- Hammer++ tools page, checked 2026-08-07 for BSPZIP++ support and redistribution context: https://ficool2.github.io/HammerPlusPlus-Website/tools.html
- Source Weaver issue #107 acceptance criteria, checked 2026-08-07 for LD_LIBRARY_PATH, `-game`, platform profile, wrapper, and validation-boundary requirements.
