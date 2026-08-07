# Campaign batch workflow

Source Weaver can run a multi-step campaign stitching plan with `sourceweaver campaign-run`. A campaign plan is a TOML file containing ordered stitch steps. Each step reuses the same validation, merge, cleanup, transition, adjacency, and reporting pipeline as a normal `sourceweaver run --job` job.

This workflow is a Source Weaver automation layer. It does not run Hammer, Hammer++, VBSP, VVIS, VRAD, a game runtime, or any external SDK tool.

## CLI

```bash
sourceweaver campaign-run --plan tests/jobs/campaign-plan.toml --dry-run
sourceweaver campaign-run --plan tests/jobs/campaign-plan.toml --report target/campaign-summary.json
```

`--dry-run` overrides the plan and every step so merged VMFs are not written. JSON reports are still produced.

## Plan format

```toml
name = "fixture campaign batch"
dry_run = true
report = "../../target/test-output/campaign-plan-summary.json"

[[steps]]
name = "adjacency stitch"
base = "../fixtures/campaign_adjacency_01.vmf"
inputs = ["../fixtures/campaign_adjacency_02.vmf", "../fixtures/campaign_adjacency_03.vmf"]
output = "../../target/test-output/campaign_step_adjacency.vmf"
report = "../../target/test-output/campaign_step_adjacency.json"
landmark = "adj_lm"
changelevel_policy = "rewrite-internal"
changelevel_scope = "internal-only"

[steps.delete]
protect_critical_entities = true

[[steps]]
name = "transition cleanup"
base = "../fixtures/changelevel_d1_a.vmf"
inputs = ["../fixtures/changelevel_d1_b.vmf"]
output = "../../target/test-output/campaign_step_cleanup.vmf"
report = "../../target/test-output/campaign_step_cleanup.json"
landmark = "lm_exit"
changelevel_policy = "delete"
changelevel_scope = "all"

[[steps.preserve_external_transition]]
map = "external_entry"
landmark = "lm_exit"
targetname = "to_external"

[steps.delete]
protect_critical_entities = true
```

Each `[[steps]]` entry supports the normal job fields: `base`, `inputs`, `output`, `landmark`, `changelevel_policy`, `changelevel_scope`, `preserve_external_transition`, and `[steps.delete]` cleanup rules. Paths are resolved relative to the campaign plan file.

## Summary report

The command prints a campaign summary JSON report to stdout and writes it to `report` when configured. The summary links every generated artifact:

- plan-level summary report path;
- every step output VMF path;
- every step report JSON path;
- whether the step wrote output;
- integrity counts;
- transition count;
- campaign adjacency edge count;
- changelevel policy, scope, changed count, and preserved count;
- full embedded per-step `AutomationReport` objects under `step_reports`.

In dry-run mode, `outputs_written` is `0`, every step has `output_written = false`, and output VMFs are not written. Per-step report JSON files and the campaign summary JSON can still be written so reviewers can inspect the planned changes before exporting any VMF.

## Fixture coverage

- `tests/jobs/campaign-plan.toml` covers two planned stitch steps.
- The first step exercises a three-map adjacency stitch and writes `campaign_step_adjacency.json` in dry-run mode.
- The second step exercises transition cleanup preserve rules and writes `campaign_step_cleanup.json` in dry-run mode.
- CLI tests verify the summary report, embedded step reports, artifact paths, dry-run no-write behavior, and help output.
