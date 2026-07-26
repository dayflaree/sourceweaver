# Definition of done

## Any code change

- tests cover success and failure paths;
- lint and strict typing pass;
- no source-integrity regression;
- user-facing behavior documented;
- errors are actionable and preserve provenance;
- no proprietary assets or binaries are added.

## A transformation rule

- support envelope is explicit;
- all source and semantic fields touched are typed;
- transformation is deterministic and reversible;
- static invariants are complete;
- synthetic legal fixture exists;
- compiler qualification passes;
- runtime scenarios pass when behavior may change;
- metrics and acceptance threshold are defined;
- unknown conditions block rather than guess;
- patch report explains the change.

## A compiler fingerprint

- executable hash and size recorded;
- provenance/distribution policy verified;
- qualification fixture suite passes;
- repeated compile determinism classified;
- log parser recognizes all expected output;
- no unreviewed error-like messages;
- game runtime loads produced BSPs.

## A map-stitch result

- both inputs remain unchanged;
- alignment and seam evidence retained;
- all IDs and affected references valid;
- no unresolved overlap or singleton conflict;
- region lifecycle scenarios pass;
- baseline/candidate compiler and runtime reports complete;
- accepted output and patch manifest are atomically published to the worktree;
- reviewer can trace every output object to a source or generation rule.

## A visibility optimization result

- correctness gates pass;
- full VVIS result available;
- relevant door/dynamic-view scenarios pass;
- repeated runtime measurements show a practical gain;
- leaf/PVS/portal growth remains within policy;
- regression images/metrics are reviewed or automatically within qualified tolerance.
