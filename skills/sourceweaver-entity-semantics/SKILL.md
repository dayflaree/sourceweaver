---
name: sourceweaver-entity-semantics
description: Perform FGD-aware entity, targetname, I/O, instance, asset, and script transformations safely.
---

# Entity semantics

## Trigger

Use for entity keyvalues, outputs, targetnames, references, parent graphs, instances, resource paths, scripts, or lifecycle class policy.

## Procedure

1. Resolve the exact game profile and FGD source for each class.
2. Assign a schema provenance and type to every affected field.
3. Build ordered entity-I/O, target, parent, instance-fixup, and globalname graphs.
4. Preserve duplicate outputs and original comma/ESC separators.
5. Mark special names, wildcards, shared/global names, and ambiguous definitions.
6. Plan namespace/transform rewrites as one atomic graph transaction.
7. Parse output parameters according to the target input definition when available.
8. Scan scripts/console commands/custom entities for affected opaque dependencies.
9. Re-resolve the complete graph after edits.
10. Compile and run behavior scenarios for any lifecycle-affecting change.

## Blockers

- unknown type on an affected field;
- unresolved or ambiguous affected target;
- unsupported instance fixup behavior;
- opaque script dependency;
- branch-specific special token without a policy;
- graph connectivity or output ordering changes outside the manifest.

## Acceptance gates

- all affected definitions/references resolve as intended;
- special/global names retain semantics;
- output order and separators are preserved;
- semantic before/after graph diff is fully explained;
- runtime scenarios pass where behavior can change.

## References

- `docs/ENTITY_SEMANTICS.md`
- `docs/CAMPAIGN_LIFECYCLE.md`
