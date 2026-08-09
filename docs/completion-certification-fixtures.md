# Completion certification fixtures and evidence workspace

Source Weaver completion certification uses repository-owned synthetic fixture sources plus a non-repository evidence workspace under `/tmp`. The workspace is for manual external evidence gathered for #141, #142, #145, and #146.

Generate the workspace with:

```bash
SOURCEWEAVER_COMPLETION_EVIDENCE_OVERWRITE=1 \
  scripts/prepare-completion-evidence-workspace.sh /tmp/sourceweaver-completion-evidence
```

The script creates:

```text
/tmp/sourceweaver-completion-evidence/
  issue141-hammer-open-save/
  issue142-windows-native-compile/
  issue145-runtime-map-load/
  issue146-hlmv-render/
  manifests/
  redacted-summaries/
  README.md
  SHA256SUMS
  sourceweaver-cert-room-validation.json
```

Each issue directory is a seed evidence bundle with the required #160 bundle files:

```text
evidence-manifest.json
SHA256SUMS
tool-versions.txt
commands.sh
validation-summary.md
legal-boundary.md
input/...
```

The generator validates each issue bundle with `scripts/validate-external-certification-evidence.sh` and validates the synthetic VMF through Source Weaver with the HL2 rule-set. The seed bundles document what is prepared and what is still unresolved. Replace placeholder external tool-version and command notes with exact sanitized evidence before closing the dependent issues.


## Prerequisite probe workspace

Before collecting the manual external evidence, refresh the prerequisite blocker state with:

```bash
SOURCEWEAVER_COMPLETION_PREREQS_OVERWRITE=1 \
  scripts/probe-completion-prerequisites.sh /tmp/sourceweaver-completion-prerequisites
```

The prerequisite probe is for #156, #157, and #158. It writes sanitized, non-repository evidence under `/tmp`:

```text
/tmp/sourceweaver-completion-prerequisites/
  issue156-gui-runtime-workstation/probe.txt
  issue157-native-windows-host/probe.txt
  issue158-signing-provisioning/probe.txt
  issue158-signing-provisioning/repository-secret-names.json
  issue158-signing-provisioning/repository-variable-names.json
  prerequisite-summary.json
  README.md
  SHA256SUMS
```

The probe records tool presence, display/session shape, native Windows/compiler presence, repository secret and variable names visible to `gh`, and local signing-tool availability. It redacts the home directory to `${HOME}`, requests repository secret names only, and never prints secret values, key material, external binaries, game content, screenshots, BSPs, MDLs, or private assets. It does not certify Hammer/Hammer++, HLMV/HLMV++, runtime map loads, native Source compiler execution, or production signing; it only proves whether the prerequisites for those manual runs are available.

Set `SOURCEWEAVER_COMPLETION_PREREQS_REQUIRE_READY=1` when a human expects all prerequisites to exist and wants the probe to exit nonzero if #156, #157, or #158 remains blocked.

## Repository fixture sources

```text
tests/fixtures/completion-certification/PROVENANCE.md
tests/fixtures/completion-certification/vmf/sourceweaver-cert-room.vmf
tests/fixtures/completion-certification/model-source/models/sourceweaver/sw_cert_cube.qc
tests/fixtures/completion-certification/model-source/models/sourceweaver/sw_cert_cube_ref.smd
tests/fixtures/completion-certification/model-source/models/sourceweaver/sw_cert_cube_idle.smd
tests/fixtures/completion-certification/model-source/materials/models/sourceweaver/synthetic_checker.vmt
```

`sourceweaver-cert-room.vmf` is a small synthetic room with six world brushes, one player start, one light, and one `info_landmark`. The model source package is original QC/SMD/VMT text for a simple cube. No compiled model outputs are committed.

## Legal boundary

The committed fixtures are Source Weaver-authored text files under the repository license. They contain no proprietary map data, Steam files, Source SDK binaries, Hammer/Hammer++ binaries, HLMV/HLMV++ binaries, VBSP/VVIS/VRAD binaries, BSPs, MDLs, VTFs, private assets, screenshots, private logs, signing keys, certificates, passphrases, GitHub secret values, or production signing credentials.

Generated BSPs, compiled model outputs, external tool binaries, screenshots, and raw logs belong in the `/tmp` workspace or another external evidence location. Commit only sanitized scripts, docs, tests, and redacted summaries.

## Validation boundary

This workflow prepares legal fixture sources and evidence directories. It does not record real external editor, compiler, model viewer, game runtime, or signing evidence. The dependent issues remain responsible for the actual manual evidence rows:

- #141 records real editor open-save evidence.
- #142 records native Windows compiler evidence.
- #145 records game-runtime map-load evidence.
- #146 records model viewer render evidence.
