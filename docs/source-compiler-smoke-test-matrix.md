# Real Source compiler smoke-test matrix

This matrix defines how to record real VMF-to-BSP compiler compatibility for Source Weaver releases. It does not claim that real VBSP/VVIS/VRAD validation was run unless a completed row includes exact tools, logs, and output artifacts.

Source Weaver remains Linux-first for VMF merge/edit/preview/validation. Compilation is optional and uses user-provided external tools. Proprietary Valve tools, game assets, generated BSPs containing game content, and private logs must not be committed unless redistribution is explicitly allowed.

## Required release statement

Every release note that mentions compiler support must use this split:

- **Structural VMF validation:** portable Source Weaver checks and captured-log parsing run in CI/local validation.
- **Real compiler validation:** only the matrix rows marked `completed` were run with real external VBSP/VVIS/VRAD tools.
- **Hammer/Hammer++ open/save validation:** record separately using `docs/hammer-validation-workflow.md`.
- **Runtime map-load validation:** record separately using `docs/runtime-map-load-validation.md`.

When no row is completed for a release, write: `No real VBSP/VVIS/VRAD/Hammer/game runtime validation was run for this release.`

## Evidence to capture

For every completed row, record:

- Source Weaver commit SHA and version.
- Date, tester, OS, kernel/build, CPU architecture.
- Game/toolchain name and Steam app/tool source.
- Exact VBSP, VVIS, VRAD paths or wrapper scripts.
- Version banners or first 20 non-empty output lines from each tool.
- Wine/Proton/native environment details.
- `compile-profile.toml` with private paths redacted when needed.
- Input VMF source and whether it is redistributable.
- Full command line emitted by Source Weaver.
- Exit code for every step.
- `vbsp.log`, `vvis.log`, `vrad.log`, or redacted summaries.
- Generated BSP path and file size.
- Parsed Source Weaver report JSON.
- Warnings, errors, leak status, and follow-up issue links.
- Whether the BSP was launched in-game for runtime smoke testing.

## Manual smoke-test procedure

1. Build Source Weaver from the target commit.

   ```bash
   cargo build --workspace --release
   ```

2. Create a legal test VMF. Prefer a tiny map owned by the tester. Keep it out of the repo unless every asset and source file is redistributable.

3. Create or discover a compile profile.

   ```bash
   target/release/sourceweaver compile-profile create \
     --output smoke-compile-profile.toml \
     --vbsp /path/to/vbsp-or-wrapper \
     --vvis /path/to/vvis-or-wrapper \
     --vrad /path/to/vrad-or-wrapper \
     --game /path/to/game/content-dir \
     --steps vbsp,vvis,vrad \
     --log-dir smoke-logs \
     --timeout-seconds 1800 \
     --validate \
     --json | tee smoke-profile-validation.json
   ```

4. Run Source Weaver structural validation first.

   ```bash
   target/release/sourceweaver validate smoke-input.vmf --json | tee smoke-structural-validation.json
   ```

5. Run the real compile pipeline.

   ```bash
   target/release/sourceweaver compile smoke-input.vmf \
     --profile smoke-compile-profile.toml \
     --steps vbsp,vvis,vrad \
     --log-dir smoke-logs \
     --timeout-seconds 1800 \
     --report smoke-compile-report.json \
     --json | tee smoke-compile-stdout.json
   ```

6. Confirm the report JSON is valid.

   ```bash
   python3 -m json.tool smoke-compile-report.json >/dev/null
   python3 -m json.tool smoke-compile-stdout.json >/dev/null
   ```

7. Record the generated BSP path and size.

   ```bash
   ls -lh smoke-input.bsp
   ```

8. Optional runtime smoke test: launch the target game and load the compiled map. Record exact launch command, game build, console errors, missing materials/models/sounds, and whether the map loads to gameplay.

9. File follow-up issues for every warning, compiler failure, leak, path translation problem, missing content problem, or runtime failure. Link those issues in the matrix row.

## Matrix rows

### Row A: Source SDK Base 2013 Singleplayer-compatible tools through Wine

Status: `planned`

Purpose: baseline Linux/Wine path for a legal Steam tool install.

Public references checked on 2026-08-06:

- Valve Developer Community has a Source SDK Base 2013 getting-started page.
- Steam Community guidance describes installing Source SDK Base 2013 Singleplayer from Steam Library Tools.
- Public compile examples and Source command-sequence references show the standard order `vbsp` on VMF, then `vvis` and `vrad` on BSP.

Required row fields when executed:

```text
commit:
date:
tester:
os:
wine_version:
steam_install:
tool_source:
vbsp_path:
vvis_path:
vrad_path:
game_dir:
profile_path:
input_vmf:
input_vmf_redistributable: yes/no
command:
vbsp_exit:
vvis_exit:
vrad_exit:
output_bsp:
output_bsp_size:
warnings:
errors:
leak_detected:
runtime_smoke: not-run/pass/fail
runtime_notes:
follow_ups:
```

### Row B: Windows native game/tool install

Status: `planned`

Purpose: ensure Source Weaver-generated VMFs and reports match native Windows compiler behavior when a user has legal Windows Source tooling.

Required row fields are the same as Row A, replacing `wine_version` with the Windows build and native tool source.

### Row C: Proton wrapper path

Status: `completed`

Purpose: verify a Linux Proton wrapper path with legal local Garry's Mod Source++ compiler tools and a tiny tester-generated smoke VMF.

Completed evidence, 2026-08-07:

```text
commit: pending #118 implementation commit; see GitHub issue #118 closing comment for final SHA
date: 2026-08-07T14:46:06-06:00
tester: Elijah local environment via D0G/Hermes
os: Linux OldBeast 7.0.0-29-generic x86_64
runtime: Steam Proton 10.0 selected by /home/elijah/.local/bin/sourceplusplus-gmod
steam_install: /home/elijah/snap/steam/common/.local/share/Steam
tool_source: local Garry's Mod win64 Source++ tools under steamapps/common/GarrysMod/bin/win64
vbsp_path: /home/elijah/.local/bin/vbspplusplus-gmod -> sourceplusplus-gmod -> vbspplusplus.exe
vvis_path: /home/elijah/.local/bin/vvisplusplus-gmod -> sourceplusplus-gmod -> vvisplusplus.exe
vrad_path: /home/elijah/.local/bin/vradplusplus-gmod -> sourceplusplus-gmod -> vradplusplus.exe
vbsp_binary: /home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/bin/win64/vbspplusplus.exe, PE32+ x86-64, 1,136,128 bytes, mtime 2026-07-04 15:22
vvis_binary: /home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/bin/win64/vvisplusplus.exe, PE32+ x86-64, 617,984 bytes, mtime 2026-07-04 15:22
vrad_binary: /home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/bin/win64/vradplusplus.exe, PE32+ x86-64, 1,180,672 bytes, mtime 2026-07-04 15:22
game_dir: /home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/garrysmod
profile_path: /tmp/sourceweaver-real-compiler-smoke-118/smoke-compile-profile.toml
input_vmf: /tmp/sourceweaver-real-compiler-smoke-118/smoke_box.vmf
input_vmf_redistributable: not committed; tester-generated throwaway VMF in /tmp
validation_only_material: /home/elijah/snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod/garrysmod/materials/sourceweaver_smoke/white.vmt using local temporary VTF copy, not committed
command: cargo run -q -p sourceweaver-cli -- compile /tmp/sourceweaver-real-compiler-smoke-118/smoke_box.vmf --profile /tmp/sourceweaver-real-compiler-smoke-118/smoke-compile-profile.toml --steps vbsp,vvis,vrad --log-dir /tmp/sourceweaver-real-compiler-smoke-118/logs --timeout-seconds 1800 --report /tmp/sourceweaver-real-compiler-smoke-118/smoke-compile-report.json --json
vbsp_exit: 0
vvis_exit: 0
vrad_exit: 0
sourceweaver_report_ok: true
sourceweaver_step_ok: vbsp=true, vvis=true, vrad=true
output_bsp: /tmp/sourceweaver-real-compiler-smoke-118/smoke_box.bsp
output_bsp_size: 65,808 bytes
warnings: compiler transcript reported missing garrysmod.fgd and instance-collapse caveat; no Source Weaver parsed errors/warnings/leak
errors: none recorded in Source Weaver report
leak_detected: false
runtime_smoke: not-run
runtime_notes: no Hammer/Hammer++/HLMV/game-runtime validation was performed for this row
follow_ups: none for compiler success; later runtime validation belongs to #120+
```

Evidence retained outside the repo:

- `/tmp/sourceweaver-real-compiler-smoke-118/issue118-real-compile-evidence.md`
- `/tmp/sourceweaver-real-compiler-smoke-118/smoke-compile-report.json`
- `/tmp/sourceweaver-real-compiler-smoke-118/smoke-compile-stdout.json`
- `/tmp/sourceweaver-real-compiler-smoke-118/smoke_box.log`
- `/tmp/sourceweaver-real-compiler-smoke-118/logs/vbsp.log`
- `/tmp/sourceweaver-real-compiler-smoke-118/logs/vvis.log`
- `/tmp/sourceweaver-real-compiler-smoke-118/logs/vrad.log`
- `/tmp/sourceweaver-real-compiler-smoke-118/smoke_box.bsp`

Key transcript lines:

```text
vbspplusplus.exe ... /tmp/sourceweaver-real-compiler-smoke-118/smoke_box.vmf
Writing /tmp/sourceweaver-real-compiler-smoke-118/smoke_box.bsp
vvisplusplus.exe ... /tmp/sourceweaver-real-compiler-smoke-118/smoke_box.bsp
Wrote ZIP buffer, estimated size 58106, actual size 57892
vradplusplus.exe ... /tmp/sourceweaver-real-compiler-smoke-118/smoke_box.bsp
Ready to Finish
Total triangle count: 12
Writing \tmp\sourceweaver-real-compiler-smoke-118\smoke_box.bsp
```

Boundary:

- Real VBSP++/VVIS++/VRAD++ tools were run through a Proton wrapper and produced a BSP.
- No Hammer, Hammer++, HLMV, Crowbar, BSPSource, BSPZIP, game runtime, SDK installer, proprietary model, proprietary BSP, or committed game content was run or bundled.
- The smoke VMF, temporary material, generated BSP, and logs stay outside the repository because they use local game/runtime context and are evidence artifacts, not redistributable fixtures.

## Sanitized fixture policy

Do not commit proprietary tools, game content, generated BSPs that embed protected content, Steam paths that reveal account names, or private logs. Sanitized logs may be committed under `tests/fixtures/` only when:

- the log contains no private user paths or licensed asset contents;
- the tester confirms redistribution is allowed;
- the fixture is useful for parser regression coverage;
- the fixture header explains the tool/game source and redactions.

## Current release evidence

As of 2026-08-07, Source Weaver CI and local validation cover structural VMF validation, fake external compiler control flow, compile-profile create/validate/discover, JSON report parsing, desktop compile launch code compilation, fixture merge automation, and one completed real VBSP++/VVIS++/VRAD++ Proton smoke row.

Real compiler validation currently covers only Row C above. No Hammer, Hammer++, HLMV, game runtime map-load, BSPZIP packing, or real StudioMDL validation was run in this repository state.
