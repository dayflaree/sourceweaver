# Model decompile/compile tooling

Source Weaver remains VMF-first. Model tooling is optional support for users who already work with Source model assets and external tools. Source Weaver does not bundle Crowbar, StudioMDL, model source files, compiled models, game SDKs, or game assets.

## Crowbar research

Crowbar was checked live on 2026-08-06.

Observed facts:

- Upstream repo: https://github.com/ZeqMacaw/Crowbar
- Repo default branch: `master`
- Repo language metadata: Visual Basic .NET
- GitHub license metadata: `NOASSERTION` / Other
- Latest GitHub release observed: `v0.74` / Crowbar 0.74, published 2023-02-20, asset `Crowbar_2023-02-16_0.74.7z`
- Active branches observed include `Version0.75` and feature/fix branches.
- `README.md` says Crowbar is a GoldSource and Source Engine modding tool, built with Visual Basic in Visual Studio Community 2017, and released/debugged as x86.
- `Crowbar/Crowbar.vbproj` is a Windows Forms executable targeting .NET Framework 4.0 with x86 platform targets.
- `CrowbarSteamPipe.vbproj` references Steamworks.NET x86.
- `LICENSE.txt` uses Creative Commons Attribution-ShareAlike 3.0 Unported, credits ZeqMacaw, and asks modified releases to use a different name or clear modified-by credit.

Conclusion:

- Source Weaver should not port, vendor, bundle, or copy Crowbar code in this phase.
- A direct Crowbar port would need separate license review, attribution/name handling, UI/runtime replacement, and a clear decision on ShareAlike obligations.
- Source Weaver can safely start with independent, native metadata inspection and user-configured external compiler/decompiler boundaries.

## Feature mapping

| Crowbar area | Source Weaver approach |
| --- | --- |
| MDL/QC/SMD decompile | Future external-tool integration first; no copied Crowbar implementation. |
| Model compile through StudioMDL | User-provided `studiomdl` or wrapper with logs and reports. |
| Model package/extract workflows | Future research; keep separate from VMF merge/edit. |
| Model metadata inspection | Native lightweight MDL header inspection is acceptable and testable. |
| Game/tool setup | Reuse the external-tool profile/reporting approach; never guess ownership or install paths. |
| Desktop UI | Future optional panel after CLI behavior stabilizes. |

## CLI: inspect MDL metadata

`model-inspect` reads a small MDL header prefix without decompiling model assets:

```bash
sourceweaver model-inspect models/props/example.mdl --json
```

The report includes:

- file path and size;
- magic (`IDST` or `IDSQ` expected);
- version;
- checksum;
- embedded model name;
- header data length;
- warnings and errors.

This is a metadata sanity check only. It does not parse meshes, animations, materials, VVD, VTX, PHY, or QC data.

## CLI: compile QC through external StudioMDL

`model-compile` runs a user-provided StudioMDL-compatible tool or wrapper:

```bash
sourceweaver model-compile model.qc \
  --studiomdl /path/to/studiomdl-or-wrapper \
  --game /path/to/game/content-dir \
  --tool-arg -nop4 \
  --log model-compile.log \
  --report model-compile-report.json \
  --json
```

Command shape:

```text
studiomdl [tool-args] [-game <game-dir>] <model.qc>
```

Use `--tool-arg` once per additional StudioMDL option. Use a wrapper script for Wine, Proton, or game-specific path setup.

The report includes:

- tool path;
- command shape and arguments;
- input QC path;
- game path when configured;
- exit code;
- log path;
- parsed warning/error/leak summary from captured output.

## Future decompile boundary

A future `model-decompile` command should start as an external-tool runner that records command provenance and output paths. It should not assume Crowbar has a supported headless CLI unless current Crowbar documentation confirms one. If Crowbar is used through Wine or manually by the user, Source Weaver should document it as an external user workflow and avoid bundling Crowbar binaries.

## Release wording

Use precise wording:

- Allowed: `Source Weaver can inspect basic MDL header metadata and run user-provided StudioMDL-compatible model compile tools.`
- Allowed: `Crowbar research is documented; Source Weaver does not bundle or port Crowbar.`
- Not allowed without evidence: `Source Weaver decompiles models like Crowbar.`
- Not allowed without actual tool runs: `Model compile was validated with real StudioMDL.`

## Current validation evidence

This implementation is validated with synthetic MDL headers and fake StudioMDL-compatible shell tools. No real Crowbar, StudioMDL, HLMV, game SDK, model decompile, model compile, or game runtime validation was run in this repository state.

## Desktop model tooling panel

The desktop app exposes optional model tooling in the **Optional external compile** panel under **Optional model tooling**.

Model inspect controls:

- MDL file path;
- **Inspect MDL metadata** action;
- command preview;
- JSON report display;
- stdout/stderr tail display.

Model compile controls:

- QC file path;
- StudioMDL-compatible executable or wrapper path;
- optional game path;
- whitespace-separated tool args;
- log path, JSON report path, and timeout seconds;
- **Run model compile** action.

Model compile runs in a background worker and uses `sourceweaver model-compile` with a user-provided tool. Source Weaver does not bundle StudioMDL, Crowbar, HLMV, game SDKs, game content, model assets, QC/SMD/DMX files, or wrappers. A model compile failure is reported in the model tooling panel and attention dialog, and it remains separate from VMF merge/export, BSP compile, and BSP packing results.

No real model decompile/compile/runtime validation is implied unless the actual external tool was run and its report/log evidence is recorded.
