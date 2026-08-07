# Optional Source compile pipeline

Source Weaver can run a user-configured Source compile pipeline when VBSP, VVIS, and VRAD tools are available. The pipeline is optional and works alongside the portable `validate` command and captured-log parser.

## CLI command

```bash
sourceweaver compile stitched.vmf \
  --vbsp /path/to/vbsp \
  --vvis /path/to/vvis \
  --vrad /path/to/vrad \
  --game /path/to/game-dir \
  --steps vbsp,vvis,vrad \
  --log-dir target/sourceweaver-compile-logs \
  --timeout-seconds 900 \
  --report target/sourceweaver-compile-report.json \
  --json
```

Available flags:

- `--vbsp`, `--vvis`, `--vrad`: executable paths for each compile step
- `--game`: optional Source game/content directory passed as `-game <dir>`
- `--steps`: comma-separated step order, such as `vbsp`, `vbsp,vvis`, or `vbsp,vvis,vrad`
- `--log-dir`: directory where each step writes `<step>.log`
- `--timeout-seconds`: per-tool timeout; defaults to 900 seconds
- `--report`: JSON report path
- `--json`: print the JSON report to stdout

VBSP receives the input `.vmf`. VVIS and VRAD receive the corresponding `.bsp` path by changing the input extension to `.bsp`.

## Compile profile helper

Use `compile-profile` to create, validate, or discover profiles before running real external tools:

```bash
sourceweaver compile-profile create \
  --output hl2-tools.toml \
  --vbsp /path/to/vbsp-or-wrapper \
  --vvis /path/to/vvis-or-wrapper \
  --vrad /path/to/vrad-or-wrapper \
  --game /path/to/game-dir \
  --steps vbsp,vvis,vrad \
  --log-dir target/sourceweaver-compile-logs \
  --timeout-seconds 900 \
  --validate \
  --json

sourceweaver compile-profile validate --profile hl2-tools.toml --json
sourceweaver compile-profile discover --search-dir /path/to/source/bin --output discovered.toml --game /path/to/game-dir --json
sourceweaver compile-profile discover --steam-root /path/to/Steam --output discovered-steam.toml --json
```

Profile validation checks selected steps, missing tool paths, file/executable status, game directory status, and timeout settings. Discovery searches explicit `--search-dir` values plus `PATH`; by default it also scans common Steam library layouts such as `steamapps/common/<game>/bin`, `bin/win64`, `bin/x64`, `bin/linux64`, `hl2/bin`, and `sdk_content/bin`. Use `--steam-root` for additional Steam libraries or `--no-steam-discovery` for explicit/PATH-only discovery.

Discovery reports candidates for VBSP, VVIS, VRAD, BSPZIP/BSPZIP++, and StudioMDL/StudioMDL++ compatible names. Each candidate includes a `source`, `confidence`, and runtime caveats. Source Weaver does not run, install, or validate discovered tools; users must confirm the selected path and matching game/runtime context before profile use.

## Profile TOML

A profile can store game/tool paths:

```toml
[tools]
vbsp = "/path/to/vbsp"
vvis = "/path/to/vvis"
vrad = "/path/to/vrad"
game = "/path/to/game-dir"

[compile]
steps = ["vbsp", "vvis", "vrad"]
log_dir = "target/sourceweaver-compile-logs"
timeout_seconds = 900
```

Run it with:

```bash
sourceweaver compile stitched.vmf --profile hl2-tools.toml --report compile-report.json --json
```

Command-line paths override profile paths, so a profile can define defaults while a one-off run changes one tool path or the step list.


## Desktop compile runner

The desktop app includes an **Optional external compile** panel below the merge controls. It can:

- select a compile profile TOML;
- choose step order, log directory, report JSON path, and timeout;
- run the compile after a successful **Merge selected VMFs** export;
- run compile manually for the current output VMF;
- keep the UI responsive by launching the compile command in a background worker;
- show the command, summary, report JSON, and stdout/stderr tails when the run finishes.

The desktop runner calls the Source Weaver CLI `compile` workflow. Packaged installs should include the CLI next to the desktop executable. Development or custom installs can set `SOURCEWEAVER_CLI=/path/to/sourceweaver` before launching the desktop app.

A compile failure is reported separately from merge/export success. The exported VMF can still be valid even when VBSP, VVIS, VRAD, Wine, Proton, game content, or a timeout causes compile failure.

## Report contents

The compile report includes:

- VMF integrity summary before tool execution
- configured game directory
- log directory
- one entry per step with tool path, input path, exit code, log path, and parsed compile-log summary
- warning, error, and leak lines extracted from stdout/stderr

The command exits non-zero if VMF integrity has structural errors, any compile process exits non-zero, any tool times out, or any parsed log contains errors or leaks. Captured output must include an explicit success marker, such as `0 errors` or `VBSP finished`; a truncated compiler banner is reported as unsuccessful.

## Linux development workflow

This repository is developed on Linux and does not assume licensed Source tools are installed. The compile command is validated with fake compiler tools in local test runs so pipeline control flow, log capture, JSON reporting, and leak/error parsing are deterministic.

Real compile validation still requires a Source game/tool installation or captured logs from a machine that has those tools. Use `docs/source-compiler-smoke-test-matrix.md` to record exactly which real toolchains were tested. See `docs/linux-source-compiler-setup.md` for Wine/Proton wrappers, sample profiles, and troubleshooting.
