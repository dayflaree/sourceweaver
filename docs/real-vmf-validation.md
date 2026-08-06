# Real VMF end-to-end validation

Issue #43 was validated on Linux with two public, real Source 1 VMFs from the `rubycho/labescape-hl2` repository.

## Source maps

Repository: https://github.com/rubycho/labescape-hl2

Pinned commit used by `scripts/validate-public-vmfs.sh`:

```text
184f8c5eec17313724155f91f2f99133c12c464a
```

Maps:

- `maps/hl2-chap2.vmf`
- `maps/hl2-chap3.vmf`

These maps are adjacent in their transition data. `hl2-chap2.vmf` contains `trigger_changelevel` entities targeting `hl2-chap3` with landmark `landmark2`, and `hl2-chap3.vmf` contains a reverse transition targeting `hl2-chap2` with the same landmark.

## Reproduction script

Run:

```bash
scripts/validate-public-vmfs.sh /tmp/sourceweaver-real-validation
```

The script:

1. Downloads the two VMFs from the pinned commit.
2. Runs `sourceweaver inspect` on both VMFs.
3. Runs `sourceweaver list-types` on both VMFs.
4. Merges them with `--landmark landmark2`.
5. Validates the merged VMF with the portable `sourceweaver validate` path and sample VBSP success log.
6. Runs the optional `sourceweaver compile` pipeline against the merged VMF using a fake VBSP script to validate compile-pipeline control flow, log capture, and JSON report shape on Linux.

## Local validation result

Command run locally on this Linux development machine:

```bash
scripts/validate-public-vmfs.sh /tmp/sourceweaver-real-validation
```

Observed merge summary:

```text
merged maps: 2
appended world solids: 47
appended entities: 62
offset /tmp/sourceweaver-real-validation-script/vmfs/hl2-chap2.vmf 0 0 0
offset /tmp/sourceweaver-real-validation-script/vmfs/hl2-chap3.vmf 2240 -56 4
wrote /tmp/sourceweaver-real-validation-script/hl2-chap2-chap3-merged.vmf
```

The merged VMF was written successfully and was about 178 KiB in this run.

`sourceweaver validate` returned JSON with:

```text
ok: true
integrity errors: 0
integrity warnings: 3
```

The warnings were duplicate numeric IDs in the source `hl2-chap2.vmf`; merge renumbering and ID-reference remapping still produced a valid output VMF.

The optional compile pipeline with a fake VBSP tool returned JSON with:

```text
ok: true
steps: [("vbsp", true, 0 errors, 0 warnings)]
```

## Hammer/Hammer++ limitation

This validation was run on Linux, and Hammer/Hammer++ is not available in this environment. The output VMF was therefore not opened interactively in Hammer. Instead, Source Weaver performed the checks available on Linux:

- parse/inspect both real VMFs
- list entity classnames
- discover real transition/landmark data
- merge with a real landmark
- parse and structurally validate the merged VMF
- run the optional compile-pipeline code path with deterministic fake compiler output
- parse a sample VBSP success log

When a Windows machine with Hammer/Hammer++ and Source tools is available, rerun the script's output VMF through Hammer and real VBSP/VVIS/VRAD, then attach the logs to this document or the relevant release notes.

## Follow-up policy

Any real-map parser, merge, Hammer-open, or compile issue discovered from future runs should become a focused follow-up issue with:

- exact map names and source commit/build
- Source Weaver command used
- generated report JSON
- Hammer/VBSP/VVIS/VRAD logs when available
- smallest legal VMF fixture that reproduces the defect
