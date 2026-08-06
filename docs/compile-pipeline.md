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
  --report target/sourceweaver-compile-report.json \
  --json
```

Available flags:

- `--vbsp`, `--vvis`, `--vrad`: executable paths for each compile step
- `--game`: optional Source game/content directory passed as `-game <dir>`
- `--steps`: comma-separated step order, such as `vbsp`, `vbsp,vvis`, or `vbsp,vvis,vrad`
- `--log-dir`: directory where each step writes `<step>.log`
- `--report`: JSON report path
- `--json`: print the JSON report to stdout

VBSP receives the input `.vmf`. VVIS and VRAD receive the corresponding `.bsp` path by changing the input extension to `.bsp`.

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
```

Run it with:

```bash
sourceweaver compile stitched.vmf --profile hl2-tools.toml --report compile-report.json --json
```

Command-line paths override profile paths, so a profile can define defaults while a one-off run changes one tool path or the step list.

## Report contents

The compile report includes:

- VMF integrity summary before tool execution
- configured game directory
- log directory
- one entry per step with tool path, input path, exit code, log path, and parsed compile-log summary
- warning, error, and leak lines extracted from stdout/stderr

The command exits non-zero if VMF integrity has structural errors, any compile process exits non-zero, or any parsed log contains errors or leaks.

## Linux development workflow

This repository is developed on Linux and does not assume licensed Source tools are installed. The compile command is validated with fake compiler tools in local test runs so pipeline control flow, log capture, JSON reporting, and leak/error parsing are deterministic.

Real compile validation still requires a Source game/tool installation or captured logs from a machine that has those tools.
