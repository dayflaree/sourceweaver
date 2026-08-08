# Hammer and Hammer++ validation workflow

Source Weaver portable validation proves only VMF text structure, Source Weaver rule sets, and captured compile-log parsing. Hammer and Hammer++ validation is a separate external editor workflow. It is complete only when a tester opens the generated VMF in a real editor, records the exact editor/game configuration, saves a copy, and reviews the resulting diff.

This repository does not bundle Hammer, Hammer++, Source SDK tools, Steam files, game content, generated BSPs, screenshots, or editor logs. Do not commit proprietary VMFs, generated BSPs, screenshots that expose private projects, or game assets unless redistribution rights are verified.

`docs/compatibility-matrix.md` records Hammer/Hammer++ open/save as `not validated`. Future issue #141 preserves the real-editor certification work and should remain open until a runnable editor produces a completed evidence row.

## Status in this repository state

Checked on 2026-08-07:

- No `hammer`, `hammer.exe`, `hammerplusplus`, or `hammerplusplus.exe` executable was found in `PATH`.
- `/home/elijah/snap/steam/common/.local/share/hammerplusplus-gmod` exists as a Proton compatibility-data directory, not as a directly runnable Hammer/Hammer++ executable.
- Therefore no real Hammer or Hammer++ open/save validation was run for this issue.

## Required evidence for a completed editor-open row

Record the following in the issue, release notes, or a linked private evidence bundle:

```text
sourceweaver_commit:
date:
tester:
os:
runtime: native/windows/wine/proton
hammer_kind: Hammer/Hammer++
hammer_path:
hammer_version_or_build:
game_or_sdk:
game_config_name:
game_dir:
fgd_paths:
input_vmf:
input_vmf_redistributable: yes/no
portable_validation_report:
compile_matrix_row: optional link when the same VMF was compiled
editor_open_result: pass/fail
editor_open_warnings:
editor_console_log:
save_action: no-save/save-as/overwrite
saved_vmf:
saved_vmf_diff_summary:
sourceweaver_validate_saved_vmf:
screenshots_or_recording: optional sanitized links
runtime_map_load: not-run/pass/fail
follow_up_issues:
```

## Manual validation steps

1. Build or select the Source Weaver commit under test.

   ```bash
   git rev-parse HEAD
   cargo build --workspace
   ```

2. Generate or select the VMF to test. Prefer a small tester-owned fixture. Keep proprietary maps outside the repository.

3. Run Source Weaver portable validation before opening the editor.

   ```bash
   sourceweaver validate generated.vmf --json > hammer-portable-validation.json
   python3 -m json.tool hammer-portable-validation.json >/dev/null
   ```

4. Record file identity.

   ```bash
   sha256sum generated.vmf > hammer-input.sha256
   wc -c generated.vmf > hammer-input-size.txt
   ```

5. Start the real editor using the tester's normal supported setup. Examples only:

   ```powershell
   # Windows native example
   "C:\\path\\to\\hammer.exe" "C:\\path\\to\\generated.vmf"
   ```

   ```bash
   # Proton/Wine wrapper example, adjusted by the tester
   STEAM_COMPAT_DATA_PATH=/path/to/compatdata \
   STEAM_COMPAT_CLIENT_INSTALL_PATH=/path/to/Steam \
   /path/to/proton run /path/to/hammerplusplus.exe Z:\\path\\to\\generated.vmf
   ```

6. Capture editor information before changing anything:

   - Hammer or Hammer++ executable path;
   - version/build text from About dialog, title bar, file properties, or launcher metadata;
   - game configuration name;
   - game directory;
   - FGD paths loaded by the editor;
   - warnings or dialogs shown during open;
   - console log if available.

7. Inspect the opened VMF:

   - world geometry visible;
   - entities visible and selectable;
   - no forced deletion dialogs;
   - no broken displacement, solid, or entity warnings beyond known fixture issues;
   - material/model placeholders are understood as expected for the selected game config.

8. Save as a new file, never overwrite the original evidence input.

   ```text
   generated.hammer-saved.vmf
   ```

9. Diff and validate the saved VMF.

   ```bash
   diff -u generated.vmf generated.hammer-saved.vmf > hammer-saved.diff || true
   sourceweaver validate generated.hammer-saved.vmf --json > hammer-saved-validation.json
   python3 -m json.tool hammer-saved-validation.json >/dev/null
   ```

10. Summarize modifications from the saved diff:

    - benign editor metadata churn;
    - ID renumbering;
    - texture-axis changes;
    - entity key changes;
    - deleted or rewritten solids/entities;
    - any issue that needs a Source Weaver fix.

11. Optional runtime smoke is separate. A Hammer open/save pass does not prove the map compiles or loads in-game. Link compiler matrix rows and runtime rows separately.

## JSON summary template

Use this shape when attaching a machine-readable summary:

```json
{
  "ok": false,
  "real_editor_validation": true,
  "sourceweaver_commit": "",
  "date": "",
  "editor": {
    "kind": "hammerplusplus",
    "path": "",
    "version_or_build": "",
    "runtime": "proton",
    "game_config_name": "",
    "game_dir": "",
    "fgd_paths": []
  },
  "input_vmf": {
    "path": "",
    "sha256": "",
    "redistributable": false,
    "portable_validation_report": ""
  },
  "open_result": {
    "status": "not-run",
    "warnings": [],
    "console_log": "",
    "screenshots": []
  },
  "save_result": {
    "status": "not-run",
    "saved_vmf": "",
    "diff": "",
    "diff_summary": [],
    "saved_validation_report": ""
  },
  "separate_validation": {
    "compiler_matrix_row": "",
    "runtime_map_load": "not-run"
  },
  "external_tool_boundary": [
    "Hammer/Hammer++ validation is external editor evidence only.",
    "Portable Source Weaver validation, real compiler validation, and game-runtime validation are separate rows."
  ]
}
```

## Sanitization rules

Before sharing evidence publicly:

- redact private user paths if needed;
- redact Steam account names or private project names;
- do not attach proprietary VMFs, BSPs, models, materials, or screenshots without permission;
- prefer issue comments with summarized warnings, hashes, tool versions, and diff categories;
- keep full private evidence paths in local notes when public redistribution is unclear.

## Completion rule

A Hammer/Hammer++ validation row is complete only when `editor_open_result` and `save_result` are both recorded from a real editor run. A planned workflow, fake wrapper, portable validation pass, or VBSP/VVIS/VRAD compile row is not a Hammer/Hammer++ validation pass.
