# Linux/Wine Source compiler setup

Source Weaver is Linux-first for VMF merge/edit/preview/validation. VMF-to-BSP compilation is optional and uses external tools selected by the user. Source Weaver does not ship VBSP, VVIS, VRAD, Hammer, game SDKs, game content, or proprietary Valve binaries.

Use this document only when you already have legal access to the target game's compiler tools and content. Structural validation from `sourceweaver validate` is portable. A real compile is a separate external-tool test and should be reported as such.

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
