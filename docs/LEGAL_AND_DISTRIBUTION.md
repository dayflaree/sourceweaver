# Legal and distribution

This document records engineering policy, not legal advice.

## Repository contents

Allowed:

- original SourceWeaver code and documentation;
- synthetic VMF fixtures created for this project;
- hashes and metadata for locally installed tools;
- transformation algorithms and patch schemas;
- small redacted reproductions with documented permission/provenance.

Disallowed:

- Valve campaign VMFs or BSPs;
- decompiled maps redistributed without permission;
- game textures, models, sounds, scripts, or scenes;
- Hammer++, Tools++, Source compilers, or other binaries;
- third-party map content without a compatible license.

## Compilers

Hammer++’s Tools page explicitly asks authors to request permission before redistributing map compilers. SourceWeaver discovers user-installed compilers and stores only fingerprints/metadata.

## Source SDK

The Source SDK repository has its own license and restrictions. SourceWeaver references its public source for behavior research and does not copy SDK implementation code into the project.

## Map patches

Prefer transformation manifests containing:

- source file hash;
- object IDs/fingerprints;
- operations and parameters;
- generated tool brushes/entities;
- validation metrics.

Avoid patches that reproduce substantial original map text or assets. Users apply transformations locally to content they are authorized to modify.

## Reports

Public reports should redact:

- local filesystem paths;
- account names;
- proprietary entity/script text;
- asset inventories that expose private projects;
- full compiler binaries or dumps.
