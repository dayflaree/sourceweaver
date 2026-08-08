# Third-party tool redistribution policy

Source Weaver is a VMF-first editor and workflow coordinator. It may run user-selected external tools, but it must not silently bundle, download, extract, or redistribute third-party tools, SDKs, game files, map files, or model assets without a documented policy review.

This policy applies to release artifacts, test fixtures, managed-download helpers, desktop workflows, CLI commands, documentation examples, and issue evidence.

## Research notes

Research checked on 2026-08-07 reinforced the conservative default used here:

- Valve Steamworks SDK Access Agreement pages require partner access for the full terms, so this repository treats Valve/Steamworks/Source SDK components and game-distributed tools as non-redistributable unless an authorized maintainer records explicit permission for the exact artifact and use.
- Hammer++ tools documentation says permission is required to redistribute its map compilers in a Source mod, so Source Weaver keeps Hammer++/Tools++ compilers user-provided-only unless upstream permission and a policy review approve another category.
- Existing Source Weaver BSPSource research records a pinned managed-download helper for BSPSource `v1.4.8` with checksum verification, while still avoiding bundling BSPSource in release artifacts.
- Existing Source Weaver Crowbar research records Crowbar as an external tool with no bundled or ported implementation in this repository state.

## Policy categories

| Category | Meaning | Current examples | Default behavior |
| --- | --- | --- | --- |
| Never bundled | Source Weaver must not ship, cache, download, mirror, or package these assets. | Valve tools, Hammer/Hammer++, game runtimes, Steam client components, game content, proprietary BSPs/maps/models/materials, user project assets, private logs with sensitive paths. | User must provide local installation or private evidence. Repository and release artifacts exclude the files. |
| User-provided only | Source Weaver may accept a path to a tool or file the user already has, capture command/log provenance, and keep outputs outside the repo unless redistribution is verified. | VBSP, VVIS, VRAD, BSPZIP/BSPZIP++, StudioMDL, HLMV, Crowbar, VMEX, generic wrappers, game directories, custom asset roots. | No managed download. No bundling. No real-tool claim unless the tool was actually run and evidence is recorded. |
| Managed-download candidate | Source Weaver may offer an explicit opt-in downloader/cache only after license, provenance, checksums, update, support, and removal policy are documented. | Pinned BSPSource release ZIP. | Download must require user acceptance, verify size/checksum, use pinned URLs/digests, and never imply the tool is bundled. |
| Redistributable candidate | Source Weaver may commit or ship the asset only after redistribution rights, attribution, dependency notices, provenance, and update/removal obligations are recorded. | Source Weaver-authored synthetic fixtures; Cargo dependencies under their published licenses; future tiny fixtures with complete rights evidence. | Allowed only with review record and attribution. |

## Never bundled

These categories are out of scope for Source Weaver release artifacts and managed downloads:

- Valve proprietary tools and SDK executables, including Hammer, VBSP, VVIS, VRAD, BSPZIP, StudioMDL, HLMV, SDK launchers, Steam client components, and game-bin DLLs/shared libraries.
- Hammer++ binaries or companion files unless the upstream publisher explicitly grants redistribution for the exact files and use case, and a separate review approves it.
- Game runtimes, depots, mods, DLC, VPKs, BSPs, MDLs, VTFs, VMTs, materials, sounds, scripts, maps, or extracted game content.
- Proprietary or community maps/assets unless the repository records explicit redistribution permission for the exact fixture or artifact.
- Steam account data, installation manifests, auth tokens, private user paths, private project logs, or screenshots containing non-redistributable content.
- Third-party installer bundles that carry their own license prompts or downloaders.

Source Weaver may document how users point to locally installed tools. That documentation must not instruct contributors to commit or redistribute the tools.

## User-provided-only integrations

User-provided-only integrations are allowed when Source Weaver only receives a local path and records reproducible evidence.

Required command/report fields:

- tool path or wrapper path;
- tool kind and user-selected profile/preset;
- command arguments after redacting secrets;
- working directory when relevant;
- tool version probe when available;
- input and output paths;
- exit code;
- stdout/stderr log path or captured log summary;
- clear statement that Source Weaver did not bundle or validate redistribution rights for the tool itself.

Current user-provided-only tools:

| Tool or asset | Reason | Notes |
| --- | --- | --- |
| VBSP, VVIS, VRAD | Proprietary/game SDK distribution context varies by game and Steam install. | Use compile profiles and captured logs. |
| BSPZIP/BSPZIP++ | Branch-specific tool behavior and game-bin dependency context vary. | Use user-selected packer path and context profiles. |
| StudioMDL | Distributed through game/SDK contexts and often tied to game content. | Use user-provided compiler/wrapper only. |
| HLMV | Viewer requires local game/model/material context. | Source Weaver can launch a user-selected path but cannot inspect rendered output automatically. |
| Hammer/Hammer++ | Interactive editor validation requires local installation. | Evidence belongs in issue comments or private evidence folders. |
| Crowbar | External GUI/tooling with separate project licensing and runtime dependencies. | Source Weaver can run generic user wrappers but does not vendor or port Crowbar. |
| VMEX | Legacy/obsolete decompiler without verified redistribution permission. | Documentation-only wrapper example; no managed download. |
| Game content and custom assets | Ownership and redistribution rights vary. | Scan or package only from user-selected roots; do not commit unless rights are proven. |

## Managed-download candidates

Managed downloads are exceptional. Before adding or changing a managed download, open or update an issue with this checklist:

1. Exact upstream project and publisher.
2. Exact version, tag, commit, release URL, and asset URL.
3. License text and SPDX identifier when available.
4. Dependency licenses and runtime requirements.
5. Redistribution permission analysis for Source Weaver's intended use.
6. SHA-256 digest and file size from a trusted source or computed from the downloaded asset.
7. Transport and provenance notes, including whether GitHub release digests are available.
8. User consent text shown before download.
9. Cache path and cleanup/removal process.
10. Update policy: pinned version, no auto-adoption of latest upstream, and renewed review for every bump.
11. Support boundary: Source Weaver verifies/downloads only; upstream bugs remain upstream.
12. Removal policy if a license, vulnerability, publisher request, or provenance problem appears.

Managed downloads must:

- require an explicit command flag or desktop confirmation;
- download to a cache rather than the repository;
- write `.partial` files before atomic rename;
- verify checksum and size before reporting success;
- avoid extracting or executing unless a separate workflow explicitly asks;
- include a local verification command for already-downloaded files;
- work without network access when users provide a local file.

## Current managed-download status

| Candidate | Current status | License/provenance | Dependencies | Update policy | Removal policy |
| --- | --- | --- | --- | --- | --- |
| BSPSource `v1.4.8` ZIP | Managed download helper exists for pinned, explicit, checksum-verified user downloads. Source Weaver does not bundle it. | Upstream `ata4/bspsrc` was reviewed in `docs/bspsource-managed-download.md`; BSPSource itself records Unlicense text and dependencies with Apache-2.0/BSD-style notices. | Java runtime is user-provided and not bundled. | Pinned version only. Every bump requires updated URLs, sizes, SHA-256 digests, license notes, and validation evidence. | Remove or disable the helper if upstream license/provenance changes, checksums mismatch, malware/vulnerability concerns appear, or the publisher requests removal. |
| VMEX | Not a managed-download candidate. | Legacy/obsolete and no verified binary redistribution license. | Unknown/legacy runtime context. | No update path. | Keep documentation-only wrapper example or remove if it causes confusion. |
| Crowbar | Not a managed-download candidate in this repository state. | Separate project/license/runtime review would be required for exact binary redistribution. | Windows/.NET/Steamworks-related context may apply depending on build. | No Source Weaver-managed updates. | Keep as user-provided-wrapper workflow only. |
| Hammer++ | Not a managed-download candidate. | Redistribution permission must come from upstream for exact files/use. | Source SDK/game-bin context. | No Source Weaver-managed updates. | Documentation/user-provided only. |

## Redistributable candidates

An asset can be committed or shipped only when all of these facts are recorded:

- origin and author;
- license permitting redistribution in the repository or release artifact;
- attribution requirements;
- modification history;
- exact source URL, tag, commit, or generation command;
- checksums for binary or generated artifacts;
- confirmation that no proprietary game data or private user content is embedded;
- dependency license notices when the artifact includes third-party code or assets;
- owner decision approving inclusion.

Preferred redistributable fixtures are Source Weaver-authored synthetic files under a simple permissive license, such as CC0-1.0 for fixture data. Real game-derived fixtures must remain outside the repository until rights are proven for every embedded asset.

## Attribution and notices

Every approved redistribution must update at least one of:

- `README.md` for user-visible bundled components;
- release notes for artifact-level notices;
- a dedicated third-party notice file if bundled dependencies/assets need notices;
- the relevant feature document for tool-specific caveats.

Do not rely on transitive package manager metadata alone when Source Weaver ships a copied binary or asset.

## Update and removal rules

Third-party updates are not automatic. Updating a pinned third-party version requires a commit that records:

- old version and new version;
- upstream release notes or changelog link;
- license/provenance changes;
- new checksums and sizes;
- validation run;
- user-facing behavior changes.

Remove or disable a third-party integration when:

- license terms become unclear or incompatible;
- checksums or publisher provenance cannot be verified;
- the upstream project is compromised or requests removal;
- a vulnerability cannot be mitigated promptly;
- the tool becomes unsupported and unsafe for the documented workflow.

## Policy review gate

Before implementing a new managed third-party download or bundling any third-party binary/asset, the issue must include this decision record:

```text
third_party_policy_review:
  name:
  category: never-bundled | user-provided-only | managed-download-candidate | redistributable-candidate
  upstream_url:
  version_or_commit:
  license:
  dependency_licenses:
  redistribution_allowed: yes/no/unknown
  attribution_required:
  provenance_source:
  sha256:
  size_bytes:
  update_policy:
  removal_policy:
  user_consent_text:
  validation_evidence:
  reviewer:
  decision: approved/deferred/rejected
```

If any field is unknown, the default decision is deferred. Deferred tools can still be documented as user-provided-only when that does not require Source Weaver to download or redistribute them.

## Evidence rules

Do not claim real external-tool validation unless the real tool was run. A complete evidence record includes:

- exact tool name and version;
- command line and working directory;
- input ownership/redistribution status;
- output ownership/redistribution status;
- log/report path or redacted excerpt;
- exit code;
- checksum of output artifacts when relevant;
- statement of what was not validated.

Fake wrappers, synthetic fixtures, and captured logs validate Source Weaver plumbing only. They do not validate real Valve tools, BSPSource, Crowbar, StudioMDL, HLMV, game runtimes, or proprietary assets.
