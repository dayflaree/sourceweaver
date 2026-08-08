# Model decompile/compile tooling

Source Weaver remains VMF-first. Model tooling is optional support for users who already work with Source model assets and external tools. Source Weaver does not bundle Crowbar, StudioMDL, model source files, compiled models, game SDKs, or game assets. Any future managed model-tool download or redistributable fixture must pass `docs/third-party-redistribution-policy.md`.

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

- Source Weaver should not port, vendor, bundle, or copy Crowbar code in this phase. The third-party redistribution policy keeps Crowbar in the user-provided-only category until a separate review approves a different category.
- A direct Crowbar port would need separate license review, attribution/name handling, UI/runtime replacement, and a clear decision on ShareAlike obligations.
- Source Weaver can safely start with independent, native metadata inspection and user-configured external compiler/decompiler boundaries.

## Feature mapping

| Crowbar area | Source Weaver approach |
| --- | --- |
| MDL/QC/SMD decompile | User-provided headless wrapper runner first; no copied Crowbar implementation. |
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
- Source MDL mesh metadata for supported v44-v49 `IDST` layouts:
  - bodypart count and bodypart table offset;
  - bodypart names, model counts, bases, and model table offsets;
  - model names, mesh counts, vertex counts, mesh table offsets, and vertex offsets;
  - mesh material indexes, vertex counts, vertex offsets, flex counts, and mesh IDs;
  - totals for bodyparts, models, meshes, and model-level vertices;
- Source MDL animation and sequence metadata for supported v44-v49 `IDST` layouts:
  - local animation count and animation table offset;
  - animation names, FPS, flags, frame counts, movement counts, animation block/index fields, IK-rule counts, and section frame counts;
  - local sequence count and sequence table offset;
  - sequence labels, activity names, activity IDs/weights, event counts, blend counts, group sizes, fade times, last frame values, next-sequence/pose indexes, IK/autolayer/IK-lock counts, keyvalue size, and activity-modifier counts;
- Source MDL material dependency metadata for supported v44-v49 `IDST` layouts:
  - texture/material-name count and table offset;
  - material search directory count and directory table offset;
  - texture names, flags, and used counters;
  - normalized material directories;
  - generated `materials/*.vmt` internal paths;
  - optional filesystem resolution against repeated `--asset-root` values;
  - missing and ambiguous material reports for future packing/UI workflows;
- companion-file metadata for sibling model files:
  - `.vvd` `vertexFileHeader_t` magic, version, checksum, LOD count, LOD vertex counts, fixup count/table offset, vertex data offset, and tangent data offset;
  - `.dx90.vtx`, `.dx80.vtx`, `.sw.vtx`, and `.vtx` optimized-model header version, cache/bone limits, checksum, LOD count, material replacement offset, bodypart count, and bodypart offset;
  - `.phy` probe metadata including VPHY section count;
  - missing sibling `.vvd` / common `.vtx` reports and MDL/VVD/VTX checksum mismatch reports;
- warnings and errors.

This is a metadata sanity check only. It parses the MDL bodypart/model/mesh, local animation/sequence descriptor, texture/material-directory, and companion-file header tables with bounds checks and version-aware warnings. It does not decode animation frame data, bone curves, event payloads, VVD/VTX vertex buffers, PHY collision meshes, QC/SMD/DMX source, VMT shader data, VTF texture data, or external tool output.

Resolve model material dependencies against local content roots:

```bash
sourceweaver model-inspect models/props/example.mdl \
  --asset-root /path/to/game \
  --asset-root /path/to/mod \
  --json
```

Material dependencies are reported as Source-internal `materials/*.vmt` paths. A path is `resolved` when exactly one configured asset root contains it, `missing` when none contain it, and `ambiguous` when more than one asset root contains the same internal path. Source Weaver reports the first configured root as the selected path for ambiguous entries and records a warning.

Sibling companion files are discovered from the MDL file stem in the same directory. Source Weaver currently probes `.vvd`, `.dx90.vtx`, `.dx80.vtx`, `.sw.vtx`, `.vtx`, and `.phy`. VVD/VTX checksums are compared to the MDL checksum when those headers are present. PHY support is deliberately probe-only: Source Weaver counts `VPHY` metadata markers but does not decode collision solids or triangle data.

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

## CLI: decompile MDL through external headless wrapper

`model-decompile` runs a user-provided headless wrapper and records command provenance, logs, output paths, and parsed warning/error summaries:

```bash
sourceweaver model-decompile models/props/example.mdl \
  --tool ./examples/wrappers/model-decompile-wrapper.sh \
  --output-dir decompiled/example \
  --tool-arg --input \
  --tool-arg '{input}' \
  --tool-arg --output \
  --tool-arg '{output-dir}' \
  --report model-decompile-report.json \
  --json
```

See `docs/model-decompile.md` for placeholder expansion, report fields, wrapper examples, and the Crowbar boundary. Source Weaver does not assume Crowbar has a supported headless CLI unless current Crowbar documentation confirms one. If Crowbar is used through Wine or manually by the user, Source Weaver treats it as an external user workflow and avoids bundling Crowbar binaries.

Classify already-generated QC/SMD/DMX/VTA outputs without launching a decompiler:

```bash
sourceweaver model-source-manifest decompiled/example --json
```

This supports external-tool delegated workflows by recording which source-like files were produced while keeping ownership/licensing and tool validation separate.

Build a reviewable model package manifest, optionally copying resolved files into package-relative paths:

```bash
sourceweaver model-package models/props/example.mdl \
  --asset-root /path/to/game \
  --asset-root /path/to/mod \
  --output-dir package-root \
  --copy \
  --report model-package-report.json \
  --json
```

`model-package` includes the input MDL, sibling VVD/VTX/PHY companion files, and resolved model material dependencies from the `model-inspect` metadata pipeline. Default mode is review-only. `--copy` must be paired with `--output-dir` and copies only resolved local files selected from the input path or configured asset roots. Missing companion/material files, ambiguous materials, and ownership caveats are recorded in the report.

Package manifests do not bundle assets by themselves and do not grant redistribution rights. They are a review/copy workflow for user-owned local content.

Create a model preview report or launch a user-provided HLMV-compatible viewer:

```bash
sourceweaver model-preview models/props/example.mdl \
  --asset-root /path/to/game \
  --hlmv /path/to/hlmv-or-wrapper \
  --launch \
  --log model-preview.log \
  --report model-preview-report.json \
  --json
```

Native preview is currently metadata-only. The report is suitable for preview cards and validation dashboards because it summarizes MDL name/version, mesh counts, local animation/sequence counts, material candidates, companion-file counts, and warnings. Source Weaver does not yet render textured models, play animations, visualize PHY collision, or read VVD/VTX vertex buffers for native preview.

HLMV integration is optional and external. Source Weaver launches only a user-provided `--hlmv` path when `--launch` is supplied, passes the MDL path as an argument, captures process exit/log output, and records that it did not inspect the rendered viewer window. This keeps HLMV/game SDK validation separate from Source Weaver native metadata parsing.

## Release wording

Use precise wording:

- Allowed: `Source Weaver can inspect basic MDL header metadata and run user-provided StudioMDL-compatible model compile or headless model-decompile wrappers.`
- Allowed: `Crowbar research is documented; Source Weaver does not bundle or port Crowbar.`
- Not allowed without evidence: `Source Weaver decompiles models like Crowbar.`
- Not allowed without actual tool runs: `Model compile was validated with real StudioMDL.`

## Current validation evidence

This implementation is validated with synthetic MDL headers, synthetic Source-style bodypart/model/mesh tables, synthetic local animation/sequence descriptor tables, synthetic texture/material-directory tables, synthetic VVD/VTX/PHY companion headers, synthetic model package copy trees, synthetic QC/QCI/SMD/DMX/VTA source-output manifests, fake HLMV-compatible preview wrappers, fake StudioMDL-compatible shell tools, fake model-decompile wrappers, one completed real StudioMDL++ model-compile validation row recorded in `docs/source-compiler-smoke-test-matrix.md`, and one completed real Crowbar 0.74 model-decompile validation row recorded below. No real HLMV or game runtime validation is implied by model-tooling evidence.

## Real Crowbar 0.74 decompile validation row

Completed on 2026-08-08 for issue #136.

Input and ownership:

```text
input_mdl: /home/elijah/.hermes/work/gmod_2924927318_dayflare/extracted/models/dayflare/ravenholm/female_01.mdl
input_companions: female_01.dx80.vtx, female_01.dx90.vtx, female_01.phy, female_01.sw.vtx, female_01.vvd
input_status: user-provided external Garry's Mod workshop/model content under the local Hermes work area
redistribution: not committed and not redistributed
```

Tooling:

```text
crowbar: /home/elijah/.hermes/tools/crowbar_0_74/Crowbar.exe
crowbar_sha256: b723a406a7f99a5565c10dd6e8c8de02e8988f6162e7fe44bd0e9ca9d58ebad9
headless_reflection_runner: /home/elijah/.hermes/work/crowbar_reflect_runner/CrowbarHeadlessDecompile.exe
headless_reflection_runner_sha256: 1a006851579eaa6608cb1e7f93d0f99defcd82009a67247585b573375f8c6283
proton: /home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/Proton 10.0/proton
proton_compat_version: 10.1000-105
steam_compat_app_id: 4000
steam_compat_data_path: /tmp/sourceweaver-crowbar-real-validation-issue136-normalized/compatdata
game_dir: /home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/garrysmod
```

Source Weaver command:

```bash
SOURCEWEAVER_CROWBAR_COMPATDATA=/tmp/sourceweaver-crowbar-real-validation-issue136-normalized/compatdata \
SOURCEWEAVER_CROWBAR_RAW_LOG=/tmp/sourceweaver-crowbar-real-validation-issue136-normalized/raw-proton-crowbar.log \
cargo run -q -p sourceweaver-cli -- model-decompile \
  /home/elijah/.hermes/work/gmod_2924927318_dayflare/extracted/models/dayflare/ravenholm/female_01.mdl \
  --tool /tmp/sourceweaver-crowbar-real-validation-issue136-normalized/crowbar-proton-normalized-wrapper.sh \
  --tool-arg /home/elijah/.hermes/tools/crowbar_0_74/Crowbar.exe \
  --tool-arg '{input}' \
  --tool-arg '{output-dir}' \
  --game /home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/garrysmod \
  --output-dir /tmp/sourceweaver-crowbar-real-validation-issue136-normalized/output \
  --log /tmp/sourceweaver-crowbar-real-validation-issue136-normalized/sourceweaver-model-decompile.log \
  --report /tmp/sourceweaver-crowbar-real-validation-issue136-normalized/sourceweaver-model-decompile-report.json \
  --timeout-seconds 300 \
  --real-tool-validation \
  --json
```

Observed Source Weaver result:

```text
report_ok: true
sourceweaver_exit: 0
wrapper_exit: 0
source_outputs_total_files: 46
qc_or_qci_files: 2
smd_files: 41
dmx_files: 0
vta_files: 1
other_files: 2
manifest_ok: true
real_tool_validation: true
```

Manifest command:

```bash
cargo run -q -p sourceweaver-cli -- model-source-manifest \
  /tmp/sourceweaver-crowbar-real-validation-issue136-normalized/output \
  --json > /tmp/sourceweaver-crowbar-real-validation-issue136-normalized/sourceweaver-model-source-manifest.json
```

Artifact hashes:

```text
d12c19d9c80d273415493bed8c2ff6ca8f64ada60db1bbae9ff7dfe6b6061308  sourceweaver-model-decompile-report.json
df869bb6b14a348a61cdb87ac8ebe80ce2671f9b49d679af5634c8a6ab03d3bc  sourceweaver-model-source-manifest.json
fbca63ab5d70d83aff7dd629a689eb5bfb5c086dabda7b97607213b8717e5dd8  raw-proton-crowbar.log
```

Caveats and limitations:

- Crowbar itself was used through a local Proton-backed reflection runner, so this row can honestly say Crowbar 0.74 was exercised. It does not claim full Crowbar parity or GUI workflow parity.
- The raw Proton/Crowbar process returned exit code 1 after generating files because the headless reflection runner hit a console `System.IO.IOException` while printing its QC summary. The normalizing wrapper recorded that raw failure, verified generated QC/SMD outputs, and returned exit 0 to Source Weaver only after outputs existed.
- Proton reported a missing FreeType font library warning. It did not prevent decompile output generation.
- The decompiled outputs are user-provided external model-derived artifacts and remain outside the repository under `/tmp/sourceweaver-crowbar-real-validation-issue136-normalized/output`.
- Source Weaver does not bundle Crowbar, the reflection runner, Proton, Steam, Garry's Mod content, the input MDL, companion files, QC/SMD/VTA outputs, or any decompiled model assets.

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
