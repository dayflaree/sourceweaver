# Validation claim guard

`scripts/check-validation-claims.py` scans `README.md`, `CHANGELOG.md`, `docs/**/*.md`, and common release-note draft paths for high-risk compatibility or release claims.

The guard fails on unsupported positive claims for:

- Hammer/Hammer++ compatibility or open/save certification;
- native Windows Source compiler validation;
- claims that a real game executable loaded a generated map successfully;
- rendered HLMV/HLMV++ preview success;
- signed Windows releases or artifacts;
- automatic update install, executable replacement, silent install, or rollback;
- textured Hammer-equivalent previews.

Boundary wording such as “not certified”, “not validated”, “failure evidence”, “when configured”, “manual handoff”, and “future work” is allowed because it prevents overclaiming.

Run it locally with:

```bash
python3 scripts/check-validation-claims.py --self-test
python3 scripts/check-validation-claims.py
```

CI runs both commands on every push and pull request.

## Updating the allowlist

`docs/validation-claim-allowlist.json` is for narrow, evidence-backed claims only. Add or widen an entry only after the evidence exists.

Each entry must include:

- `id`: stable identifier for the evidence boundary;
- `category`: one of the guard categories;
- `evidence_refs`: issue numbers, documentation anchors, scripts, or completed evidence rows;
- `files`: files or glob patterns where the narrow claim may appear;
- `line_patterns`: regular expressions for the exact wording that is allowed.

Do not add broad patterns such as `validated`, `compatible`, `signed`, or `automatic updates`. Use precise wording tied to a completed row, for example a specific issue number plus a document anchor.

When real evidence changes a boundary, update the evidence document first, then update this allowlist, then run:

```bash
python3 scripts/check-validation-claims.py --self-test
python3 scripts/check-validation-claims.py
```

Future examples that would require allowlist updates:

- Hammer/Hammer++ open/save certification after a real editor run, saved VMF diff, and evidence comment exist.
- Native Windows compiler validation after a Windows host runs real VBSP/VVIS/VRAD and records logs, versions, and hashes.
- Successful runtime map-load validation after a real game executable loads the map and records console/log evidence.
- Rendered HLMV/HLMV++ preview success after a rendered viewer window is observed and evidence is recorded.
- Production signing after a release run records real Authenticode/OpenPGP/update-signing credentials and verification output.
- Automatic update install/rollback after installer execution, recovery, rollback, and preference-persistence evidence exists.
