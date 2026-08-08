# `func_instance` handling

Source Weaver's current `func_instance` strategy is **preservation-only with explicit warnings**.

Source Weaver preserves `func_instance` entities as ordinary VMF entities during parse, merge, transform, preview extraction, deletion review, and validation. It translates the instance entity `origin` during landmark-aligned merge just like other entity origins, preserves `file`, `targetname`, `angles`, `fixup_style`, and `replaceXX` keyvalues, and preserves nested `editor` metadata with the entity.

Source Weaver does **not** load referenced instance VMFs, resolve instance search roots, inline instance contents, transform instance child brushes/entities, apply `replaceXX` parameter substitution, evaluate `fixup_style`, or expand nested instances.

## Decision

Instance expansion is deferred. The current behavior is:

```text
func_instance: preserved only; referenced instance VMFs are not loaded, expanded, transformed, or compiled by Source Weaver.
```

The validation command now reports warnings for each `func_instance` so unsupported expansion is visible in JSON and text output.

## Current behavior

| Workflow | Behavior |
| --- | --- |
| Parse/serialize | Preserves `func_instance` blocks and unknown keyvalues. |
| Merge | Moves the `origin` key with the containing entity when a landmark offset applies. |
| ID remap | Remaps normal VMF IDs around preserved entities; it does not inspect referenced instance files. |
| Preview | Shows the `func_instance` marker/entity metadata only. It does not preview referenced instance geometry. |
| Validation | Warns that the instance file is preserved-only and not resolved/expanded. Missing `file` keys and replacement parameters are reported explicitly. |
| Compile | Leaves `func_instance` for user-provided Source compiler tools to interpret. Source Weaver does not prove compiler instance-collapse behavior unless a real compile row records it. |

## Warning examples

A resolved-looking relative path still receives a preservation warning:

```text
warning: map.vmf: entity[1] func_instance `synthetic_instance` references instance file `instances/synthetic_room.vmf`; Source Weaver preserves the entity but does not resolve, inline, transform, apply fixups to, or expand nested instance VMFs
```

A missing `file` key receives a stronger warning:

```text
warning: map.vmf: entity[2] func_instance `missing_file_instance` has no non-empty `file` key; Source Weaver preserves the entity but cannot resolve or expand an instance VMF
```

Replacement parameters are preserved but not applied:

```text
warning: map.vmf: entity[1] func_instance `synthetic_instance` has replacement parameter keys replace01; Source Weaver preserves those keyvalues but does not apply parameter replacement
```

## Future expansion requirements

A future expansion mode must be opt-in and must require explicit instance search roots. It must not guess from private Steam paths, SDK install paths, or game directories.

Minimum CLI shape before implementation:

```bash
sourceweaver merge \
  --instance-mode preserve|report|expand \
  --instance-root /path/to/maps \
  --instance-root /path/to/sdk_content/maps \
  -o merged.vmf base.vmf add.vmf
```

Expansion acceptance criteria before claims change:

- synthetic redistributable fixtures cover translation, yaw/pitch/roll rotation, nested instances, `fixup_style`, `replaceXX`, missing files, cyclic includes, and unsupported paths;
- expansion reports every instance path considered and every skipped file;
- expansion refuses absolute, parent-directory, or symlink-escape paths unless a documented allow rule exists;
- ID remapping, targetname fixups, outputs/inputs, visgroups/editor metadata, overlays, displacement start positions, texture axes, and entity origins are tested;
- preview differentiates expanded geometry from preserved placeholder entities;
- compile validation with real tools is recorded separately before claiming compile/runtime equivalence.

## External reference checked

- Strata Source `func_instance` reference, checked 2026-08-08: https://wiki.stratasource.org/entities/reference/func_instance . It describes `func_instance` as an entity for placing an instance map file, states it may be translated and rotated, documents the `file` key relative lookup behavior, `fixup_style`, and `replace` keys for parameter substitution. This supports keeping Source Weaver's current preservation behavior separate from true expansion semantics.
