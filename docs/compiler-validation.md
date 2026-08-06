# Source tool validation

Source Weaver can run a Linux-friendly validation path without Hammer or VBSP installed, and it can consume real Source compiler logs when those tools are available on Windows or through a game/tool install. External VBSP execution defaults to a 900-second timeout. Captured logs must include explicit success markers such as `0 errors` or `VBSP finished`; truncated banners are treated as incomplete logs.

For multi-step VBSP/VVIS/VRAD execution, use `sourceweaver compile`; see `docs/compile-pipeline.md`.

## Portable validation on Linux

Run structural VMF and Source-tool readiness checks:

```bash
cargo run -p sourceweaver-cli -- validate path/to/merged.vmf --json
```

This validates the VMF can be parsed again and checks the same integrity rules used before desktop/CLI writes, including the required top-level `world` block and ID warnings.

## Game/profile rule sets

Add `--rule-set <id>` to run portable game/mod/profile semantics after generic integrity checks:

```bash
cargo run -p sourceweaver-cli -- validate path/to/merged.vmf \
  --rule-set hl2 \
  --json
```

Use `--rule-set none` or omit the flag for generic VMF integrity only. The JSON report keeps rule-set findings in `rule_set`, separate from generic `integrity` findings and compile-log findings. The initial `hl2` profile is documented in `docs/game-validation-rule-sets.md` and covered by `tests/fixtures/hl2_ruleset_ok.vmf` and `tests/fixtures/hl2_ruleset_warnings.vmf`.

Rule sets are Source Weaver checks only. They do not run Hammer, Hammer++, VBSP, VVIS, VRAD, or a game runtime, and they do not require a game install.

## Validate a captured VBSP log

When someone compiles on Windows or another machine, save the full VBSP output and parse it on Linux:

```bash
cargo run -p sourceweaver-cli -- validate path/to/merged.vmf \
  --compile-log path/to/vbsp.log \
  --json
```

The report includes:

- integrity errors and warnings
- compile-log errors and warnings
- leak detection
- whether the log looks successful

The parser flags common lines such as `Error`, `WARNING`, `**** leaked ****`, and `Entity ... leaked!`.

## Run VBSP when installed

When VBSP is available, Source Weaver can execute it and capture stdout/stderr:

```bash
cargo run -p sourceweaver-cli -- validate path/to/merged.vmf \
  --vbsp /path/to/vbsp \
  --game /path/to/game-dir \
  --capture-log target/vbsp.log \
  --timeout-seconds 900 \
  --json
```

The command exits non-zero if the VMF fails structural validation, if the VBSP process exits non-zero, or if the captured/loaded compile log contains errors or leaks.

## Known game configurations

### Half-Life 2

Typical Windows Source SDK layouts place VBSP under a `bin` directory and game content under `Half-Life 2/hl2`:

```powershell
sourceweaver validate stitched.vmf `
  --vbsp "C:\Program Files (x86)\Steam\steamapps\common\Half-Life 2\bin\vbsp.exe" `
  --game "C:\Program Files (x86)\Steam\steamapps\common\Half-Life 2\hl2" `
  --capture-log vbsp-hl2.log `
  --json
```

### Black Mesa

Black Mesa installations commonly provide their own SDK/bin tools and a `bms` game directory:

```powershell
sourceweaver validate stitched.vmf `
  --vbsp "C:\Program Files (x86)\Steam\steamapps\common\Black Mesa\bin\vbsp.exe" `
  --game "C:\Program Files (x86)\Steam\steamapps\common\Black Mesa\bms" `
  --capture-log vbsp-bms.log `
  --json
```

Paths vary by Steam library and tool install. The validation command does not assume a hard-coded SDK layout.

## CI fixture coverage

CI cannot assume licensed Source tools are installed. Instead, CI builds a merged fixture VMF, runs `sourceweaver validate`, and parses `tests/fixtures/vbsp-success.txt`. Failure-log parsing is covered by core tests using `tests/fixtures/vbsp-leak-error.txt`-style content.

## Validation notes for issue #47

This repository was validated on Linux with portable checks and synthetic VBSP log fixtures. Real Hammer/Hammer++ opening and VBSP execution still require a machine with the relevant Source game/tool installation. When such logs are captured, attach the JSON output from `sourceweaver validate --compile-log <log> --json` to the issue or release notes.
