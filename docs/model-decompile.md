# External model decompile runner

Source Weaver can run a user-provided headless model decompile wrapper and capture a structured report. The command is intentionally generic because current Crowbar research did not confirm a stable official headless decompile CLI in the upstream README/source scan. Crowbar remains documented as a graphical Source/GoldSrc model decompiler and StudioMDL front-end.

## CLI

```bash
sourceweaver model-decompile model.mdl \
  --tool ./examples/wrappers/model-decompile-wrapper.sh \
  --output-dir decompiled/model \
  --tool-arg --input \
  --tool-arg '{input}' \
  --tool-arg --output \
  --tool-arg '{output-dir}' \
  --report model-decompile-report.json \
  --json
```

Default generic wrapper shape, used when no placeholder appears in `--tool-arg` values:

```text
<headless-wrapper> [tool-args] <input.mdl> <output-dir>
```

Template wrapper shape, used when any placeholder appears in `--tool-arg` values:

```text
<headless-wrapper> [expanded --tool-arg placeholders]
```

Supported placeholders:

- `{input}` expands to the input `.mdl` path;
- `{output-dir}` expands to the output directory;
- `{game}` expands to `--game <dir>` when a wrapper needs a game/content directory.

Use one `--tool-arg` per argument token so paths with spaces are preserved by the process API.

## Report fields

The JSON report includes:

- `tool` and `tool_kind`;
- `command_shape`, `command_args`, and raw `--tool-arg` values;
- whether placeholder expansion was used;
- input `.mdl` path and output directory;
- recursively discovered output files such as QC, SMD, DMX, VTA, or wrapper-specific logs;
- optional game/content directory;
- exit status;
- optional log path;
- parsed warning/error summary from stdout/stderr;
- external-tool boundary text;
- `real_tool_validation = false` for the generic runner itself.

`real_tool_validation` is false because Source Weaver only launched the selected wrapper and observed process/log/output facts. A real Crowbar, StudioMDL, HLMV, or other external-tool validation row requires the actual tool name/version, command evidence, logs, inputs, outputs, and ownership/license context to be recorded for that run.

## Wrapper example

`examples/wrappers/model-decompile-wrapper.sh` is a template for users who already have a legal local headless model decompiler or their own automation around a manual tool. It expects `SOURCEWEAVER_MODEL_DECOMPILER` and maps Source Weaver placeholders to a conventional `--input`, `--output`, and optional `--game` argument shape.

The wrapper is an example only. Review and adapt it to the actual local tool before use.

## Crowbar research boundary

Sources checked on 2026-08-07:

- Valve Developer Union Crowbar page: https://valvedev.info/tools/crowbar/
- Crowbar upstream README: https://raw.githubusercontent.com/ZeqMacaw/Crowbar/master/README.md
- Crowbar upstream source tree scan for command-line/startup/decompile handling: https://github.com/ZeqMacaw/Crowbar

Findings used for this implementation:

- VDU describes Crowbar as a graphical frontend for StudioMDL and a model decompiler for GoldSrc and Source games.
- VDU describes Crowbar decompile options for QC files, face flexes, LOD meshes, and physics meshes.
- The upstream README directs users to the Crowbar Steam group for official links and describes Visual Basic/Visual Studio builds.
- The source scan found GUI startup and command-line argument plumbing, but no stable documented headless decompile command shape suitable for Source Weaver to bake in as a tool-specific command.

Source Weaver therefore provides a generic headless wrapper runner rather than a Crowbar-specific runner. Source Weaver does not bundle Crowbar, copy Crowbar implementation details, redistribute model decompilers, run StudioMDL, run HLMV, install SDKs, or include proprietary game models/content.

## Validation evidence

The repository test suite uses a synthetic `.mdl` header and a local fake wrapper that writes QC/SMD fixture outputs. That verifies Source Weaver command shaping, placeholder expansion, log capture, output discovery, report fields, and boundary text. It is not real Crowbar validation and does not prove any real model was decompiled.
