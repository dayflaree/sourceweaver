# Linux Wine/Proton Source compiler setup

Source Weaver is Linux-first for VMF merge/edit/preview/validation. VMF-to-BSP compilation is optional and uses external tools selected by the user. Source Weaver does not ship VBSP, VVIS, VRAD, Hammer, game SDKs, game content, or proprietary Valve binaries.

Use this document only when you already have legal access to the target game's compiler tools and content. Structural validation from `sourceweaver validate` is portable. A real compile is a separate external-tool test and should be reported as such.

Current evidence split:

- The Garry's Mod Source++ row below is real Proton-backed VBSP++/VVIS++/VRAD++ wrapper evidence. It is not native Linux compiler evidence, native Windows compiler evidence, stock Source SDK Base 2013 compiler-binary evidence, Hammer/Hammer++ evidence, or game-runtime map-load evidence.
- The pure system-Wine row remains blocked in this environment because `wine`, `wine64`, `wineserver`, `wineboot`, and `winetricks` were unavailable when checked. Do not describe the Proton-backed row as Wine validation.
- Native Windows Source compiler execution remains unvalidated until a Windows host runs the compiler tools natively and records paths, versions, command lines, logs, output BSP hashes, and redistribution boundaries.

## Compile command model

Source Weaver runs the selected tools in this shape:

```text
<vbsp> [-game <game-dir>] <map.vmf>
<vvis> [-game <game-dir>] <map.bsp>
<vrad> [-game <game-dir>] <map.bsp>
```

The `-game` argument is omitted only when no game directory is configured. VVIS and VRAD receive the BSP path produced by changing the input VMF extension to `.bsp`.

The Valve Developer Community command-sequence page lists VBSP, VVIS, VRAD, and the game executable as normal commands in compile sequences. Search snippets checked on 2026-08-06 also show the common `vbsp mymap.vmf`, `vvis mymap.bsp`, `vrad mymap.bsp` order for Source map compilation.

## Create a profile without hand-editing TOML

Use explicit tool paths or wrapper paths:

```bash
sourceweaver compile-profile create \
  --output hl2-tools.toml \
  --vbsp /path/to/vbsp-or-wrapper \
  --vvis /path/to/vvis-or-wrapper \
  --vrad /path/to/vrad-or-wrapper \
  --game /path/to/game/content-dir \
  --steps vbsp,vvis,vrad \
  --log-dir target/sourceweaver-compile-logs/hl2 \
  --timeout-seconds 900 \
  --validate \
  --json
```

Validate an existing profile before compiling:

```bash
sourceweaver compile-profile validate --profile hl2-tools.toml --json
```

Discover tools from explicit directories plus `PATH`:

```bash
sourceweaver compile-profile discover \
  --search-dir /path/to/source/bin \
  --output discovered-profile.toml \
  --game /path/to/game/content-dir \
  --log-dir target/sourceweaver-compile-logs/discovered \
  --json
```

Discovery never downloads tools and never guesses ownership. It reports missing tools with the directory/PATH search context.

## Wine wrapper examples

The `examples/wrappers/` directory contains generic scripts:

- `wine-source-tool.sh` runs the `SOURCE_TOOL_EXE` selected by the caller through Wine.
- `source-vbsp-wine.sh`, `source-vvis-wine.sh`, and `source-vrad-wine.sh` read `SOURCE_SDK_BIN` and select the matching `.exe`.
- `proton-source-tool.sh` demonstrates the environment variables needed to run an executable through a user-selected Proton prefix.

Example Wine run:

```bash
export SOURCE_SDK_BIN="$HOME/.steam/steam/steamapps/common/Half-Life 2/bin"
sourceweaver compile stitched.vmf \
  --profile examples/compile-profiles/hl2-wine.toml \
  --report target/sourceweaver-compile-report.json \
  --json
```

The sample profile contains placeholder paths. Copy it into your project and edit it with `sourceweaver compile-profile create` or a text editor.

## Proton wrapper notes

Proton command-line use depends on a legal Steam installation, a selected Proton build, and a writable compatibility-data prefix. Source Weaver does not choose those paths for you. A typical wrapper environment includes:

```bash
export PROTON="/path/to/steamapps/common/Proton 9.0/proton"
export STEAM_COMPAT_DATA_PATH="$HOME/.steam/steam/steamapps/compatdata/sourceweaver-tools"
export STEAM_COMPAT_CLIENT_INSTALL_PATH="$HOME/.steam/steam"
export SOURCE_TOOL_EXE="/path/to/vbsp.exe"
examples/wrappers/proton-source-tool.sh -game "/path/to/game" stitched.vmf
```

Create one small wrapper per tool or set `SOURCE_TOOL_EXE` in the environment that launches Source Weaver.


## Verified Proton compile row: Garry's Mod Source++ tools

The following local validation row was completed on 2026-08-07 and is recorded in `docs/source-compiler-smoke-test-matrix.md` Row C.

Environment:

```text
os: Linux OldBeast 7.0.0-29-generic x86_64
steam_root: /home/elijah/snap/steam/common/.local/share/Steam
proton_path: /home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/Proton 10.0/proton
proton_compat_version_file: /home/elijah/snap/steam/common/.local/share/hammerplusplus-gmod/compatdata/version = 10.1000-105
steam_compat_client_install_path: /home/elijah/snap/steam/common/.local/share/Steam
steam_compat_data_path: /home/elijah/snap/steam/common/.local/share/hammerplusplus-gmod/compatdata
steam_app_id: 4000
game_dir: /home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/garrysmod
```

Local wrapper shape:

```bash
#!/usr/bin/env bash
set -euo pipefail

STEAM_ROOT="${STEAM_ROOT:-$HOME/snap/steam/common/.local/share/Steam}"
GMOD_ROOT="$STEAM_ROOT/steamapps/common/GarrysMod"
DATA_ROOT="$HOME/snap/steam/common/.local/share/hammerplusplus-gmod"

case "${0##*/}" in
    vbspplusplus-gmod) TOOL=vbspplusplus.exe ;;
    vvisplusplus-gmod) TOOL=vvisplusplus.exe ;;
    vradplusplus-gmod) TOOL=vradplusplus.exe ;;
    *) exit 2 ;;
esac

PROTON="$STEAM_ROOT/steamapps/common/Proton 10.0/proton"
EXE="$GMOD_ROOT/bin/win64/$TOOL"
mkdir -p "$DATA_ROOT/compatdata"
export STEAM_COMPAT_CLIENT_INSTALL_PATH="$STEAM_ROOT"
export STEAM_COMPAT_DATA_PATH="$DATA_ROOT/compatdata"
export STEAM_COMPAT_APP_ID=4000
export SteamAppId=4000
export SteamGameId=4000
cd "$GMOD_ROOT/bin/win64"
exec "$PROTON" run "$EXE" "$@"
```

Source Weaver profile validation command used for the row:

```bash
sourceweaver compile-profile create   --output /tmp/sourceweaver-real-compiler-smoke-118/smoke-compile-profile.toml   --vbsp /home/elijah/.local/bin/vbspplusplus-gmod   --vvis /home/elijah/.local/bin/vvisplusplus-gmod   --vrad /home/elijah/.local/bin/vradplusplus-gmod   --game /home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/garrysmod   --steps vbsp,vvis,vrad   --log-dir /tmp/sourceweaver-real-compiler-smoke-118/logs   --timeout-seconds 1800   --validate   --json
```

Real compile command used for the row:

```bash
sourceweaver compile /tmp/sourceweaver-real-compiler-smoke-118/smoke_box.vmf   --profile /tmp/sourceweaver-real-compiler-smoke-118/smoke-compile-profile.toml   --steps vbsp,vvis,vrad   --log-dir /tmp/sourceweaver-real-compiler-smoke-118/logs   --timeout-seconds 1800   --report /tmp/sourceweaver-real-compiler-smoke-118/smoke-compile-report.json   --json
```

Observed result:

```text
sourceweaver_report_ok: true
vbsp_exit: 0
vvis_exit: 0
vrad_exit: 0
output_bsp: /tmp/sourceweaver-real-compiler-smoke-118/smoke_box.bsp
output_bsp_size: 65,808 bytes
leak_detected: false
```

Caveats:

- This was a real Proton-backed VBSP++/VVIS++/VRAD++ compile run, not a native Linux tool run.
- The smoke VMF, temporary validation material, generated BSP, and logs remain outside the repository because they depend on local game/runtime content and are evidence artifacts, not redistributable fixtures.
- The compiler transcript reported `Could not locate GameData file garrysmod.fgd` and an instance-collapse caveat, but the tiny smoke map used no instances and all Source Weaver step reports passed.
- No Hammer/Hammer++, HLMV, BSPZIP, game runtime map-load, SDK installer, proprietary model, proprietary BSP, or committed game content was run or bundled for this row.


## Wine compile row status: blocked in this environment

Checked on 2026-08-07 for issue #122:

```text
command -v wine: not found
command -v wine64: not found
command -v wineserver: not found
command -v wineboot: not found
command -v winetricks: not found
wine --version: command not found
wine64 --version: command not found
```

Windows Source++ compiler binaries were present under the local Garry's Mod install:

```text
/home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/bin/win64/vbspplusplus.exe
/home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/bin/win64/vvisplusplus.exe
/home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/bin/win64/vradplusplus.exe
```

Because no Wine executable or Wine prefix manager was available, no real Wine-based compile run was performed. The successful row in this repository state is the Proton-backed row above. Do not describe it as Wine validation.

To complete a future Wine row, install or select a legal Wine runtime, create a dedicated prefix, verify `wine --version`, run `compile-profile create --validate` with Wine wrapper paths, run `sourceweaver compile`, and record the same evidence fields used for the Proton row: Wine version, prefix path, wrapper scripts, commands, logs, reports, generated BSP path/size, warnings/errors, and ownership boundaries.

## Desktop use

After creating and validating a profile, open the desktop app and use **Optional external compile**. Select the same profile, choose logs/report paths, and enable **Run compile after successful Merge selected VMFs** when you want a post-export compile. Set `SOURCEWEAVER_CLI` before launching the desktop app if the CLI executable is not next to the desktop executable.

## Troubleshooting

### Missing tool path

Run:

```bash
sourceweaver compile-profile validate --profile hl2-tools.toml --json
```

The report lists each selected step, the resolved tool path, whether the path exists, and whether it appears executable.

### Missing game content

If `game` is configured, the validator requires it to exist and be a directory. The game directory should be the content/mod directory expected by the selected compiler, such as an `hl2` or mod folder. Source Weaver passes it as `-game <game-dir>`.

### Wine path translation

Prefer wrapper scripts that receive Linux paths from Source Weaver and handle Wine/Proton translation internally. Keep logs in a Linux directory via `compile.log_dir` so Source Weaver can read them without Wine path conversion.

### Permissions

On Unix, profile validation warns when a tool path exists but has no executable bit. Run `chmod +x wrapper.sh` for shell wrappers.

### Timeout failures

External tool runs default to 900 seconds. Increase `compile.timeout_seconds` for slow VIS/RAD runs or use `--timeout-seconds` on a one-off command.

### Compile failure is isolated from export success

A failed `sourceweaver compile` does not mean VMF merge/export failed. Read the generated JSON report and step logs to distinguish structural VMF validation, VBSP errors, VVIS errors, VRAD errors, leaks, missing content, and timeout failures.
