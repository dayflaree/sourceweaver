# Game and compiler profiles

## Purpose

Source 1 branches differ in BSP versions, entity definitions, static-prop formats, compiler flags, limits, materials, scripts, and runtime behavior. Profiles prevent cross-branch assumptions.

## Profile contents

- profile ID and schema version;
- game/application identity and branch;
- game root discovery rules;
- FGD and search paths;
- compiler executable candidate names;
- required executable fingerprints or qualification status;
- compiler arguments per tier;
- BSP and static-prop formats;
- world bounds and probed limits;
- tool materials;
- entity class policies;
- lifecycle implementation;
- output separator behavior;
- supported optimization features;
- runtime launch/test commands;
- warning and metric parsers;
- tolerances and safety margins.

## GMod Tools++ profile

The initial profile prefers locally installed:

- `vbspplusplus.exe`;
- `vvisplusplus.exe`;
- `vradplusplus.exe`;
- `bspzipplusplus.exe`.

It does not assume stock SDK 2013 hard limits. It discovers and fingerprints the executables, then loads limits from a qualified fingerprint record or runs safe synthetic probes.

`func_viscluster` generation is disabled for this profile because the current VBSP++ documentation deprecates it under VVIS++.

## Stable versus dev

GMod stable, x86-64, prerelease, and dev branches may differ. Each branch receives a separate profile qualification record. A profile cannot silently follow a moving branch without invalidating cached results.

## Profile inheritance

A profile may inherit common Source definitions, then override:

- entity classes/keys;
- compiler behavior;
- BSP format;
- limits;
- lifecycle rules;
- runtime commands.

Every final resolved profile is serialized and hashed into the run manifest.

## Probing limits

Where tools support configurable soft limits, probes determine the effective engine/compiler combination:

1. generate legal synthetic maps increasing one dimension at a time;
2. compile with the exact toolchain;
3. load in the target runtime;
4. record compiler and engine failure thresholds;
5. retain a conservative safety margin;
6. repeat when any executable or game build fingerprint changes.

Do not probe with user maps or risk overwriting game content.
