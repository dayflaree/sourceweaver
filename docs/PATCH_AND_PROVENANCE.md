# Patch and provenance model

## Patch manifest

Every candidate emits a machine-readable manifest containing:

```json
{
  "schema_version": "1.0",
  "run_id": "...",
  "source_files": [{"path": "map.vmf", "sha256": "..."}],
  "profile": {"id": "gmod-toolsplusplus", "compiler_hashes": {}},
  "policy_hash": "...",
  "transformations": [],
  "static_checks": [],
  "compile_runs": [],
  "runtime_runs": [],
  "metrics": {},
  "verdict": "accepted|rejected|review|required|blocked"
}
```

## Transformation entry

Each transformation records:

- stable transformation ID;
- rule version;
- source object UUIDs;
- generated object UUIDs;
- source text hashes and edits;
- semantic before/after representation;
- geometric before/after fingerprints;
- dependencies;
- evidence and confidence;
- expected compiler/runtime effect;
- rollback edits.

## Reproducibility

A run is reproducible only when these match:

- SourceWeaver commit/version;
- input hashes;
- FGD and asset search-path fingerprints;
- game profile version;
- compiler binary hashes;
- command lines;
- environment variables affecting compilers;
- policy and thresholds;
- random seed;
- hardware-sensitive mode flags when relevant.

## Review report

The human-facing report includes:

- a map-wide summary;
- before/after metrics;
- a visual and textual diff per candidate;
- rejected alternatives and reasons;
- unresolved risks;
- exact validation performed;
- links to logs and generated artifacts.

No “optimized successfully” message is emitted without the supporting measurements.
