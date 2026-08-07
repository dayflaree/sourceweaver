# Source Weaver requirements

## User workflow

Source Weaver automatically merges selected Source Engine VMF campaign maps into a single map while giving users full visibility and deletion control over entities, Hammer classnames, and brush categories.

Desktop-created project files must remain compatible with the CLI job format where possible. Saving a project must include selected VMFs, base map, landmark, output path, deletion rules, deletion safety settings, and predictable relative paths. Loading a project must resolve relative VMF/output paths from the project file directory and restore the UI setup.

## Supported platforms

- Linux desktop
- Windows desktop

The core VMF engine must stay platform-neutral. Platform-specific UI or packaging code must call into the same core engine.

## Map merge requirements

### VMF selection

Users must be able to select multiple `.vmf` files. One VMF acts as the base output document. Additional VMFs are appended into the base.

### Landmark alignment

When a landmark targetname is supplied, Source Weaver must locate an `info_landmark` entity with that targetname in the base map and in each incoming map. Incoming geometry and entities are translated by:

```text
base_landmark_origin - incoming_landmark_origin
```

If a map lacks the requested landmark, the tool must report that fact and leave that map unshifted for the current slice.

The desktop UI must discover `info_landmark` targetnames from selected VMFs, offer discovered values in a dropdown, and keep manual targetname entry available for unusual or messy maps. Before preview/export, it must show per-map status for the chosen landmark and warn when a selected map lacks it, has duplicate matching landmarks, or has a missing/invalid landmark origin.

### Campaign transitions

The engine must detect `trigger_changelevel` entities and expose target map plus landmark-related properties. Desktop inspection and automation reports must surface this transition data so later workflows can suggest map order and landmark pairs.

Given a selected set of campaign VMFs, Source Weaver must suggest a plausible map order by matching `trigger_changelevel` target map values against selected VMF filenames. It must suggest landmark pairings from transition landmark properties, warn about missing target maps or missing target landmarks, and keep user override paths available through manual ordering, base-map selection, and landmark entry. Merge workflows must also expose an explicit changelevel policy so stitched outputs can preserve, disable, delete, or rewrite internal transition destinations without implying compile or runtime validation.

## VMF integrity checks

Before writing cleaned or merged output, Source Weaver must validate the relevant VMFs and result document. It must error when an output has no editable top-level `world` block or has multiple top-level `world` blocks. It should warn about missing common VMF sections, missing IDs, duplicate numeric IDs, multiple ID fields, and non-numeric IDs on blocks that normally require stable Hammer IDs. CLI errors must include the VMF filename, and parse errors must include the parser byte position. The desktop UI and automation JSON reports must surface integrity warnings so users can inspect problems before export.

## Source tool validation

Source Weaver must provide a validation path that works on Linux without Hammer installed and can also consume real Source compiler logs. The CLI must validate a generated VMF structurally, parse captured VBSP logs for warnings, errors, and leaks, and optionally run a configured VBSP command when a user has Source tooling installed. External compiler/decompiler executions must have configurable timeouts and must not deadlock on large stdout/stderr output. Captured compile logs must require explicit success markers, so incomplete logs do not pass merely because they mention a tool name. CI should cover the portable path with fixture logs rather than depending on proprietary game tools.

The CLI must also provide an optional Source compile pipeline for users who can provide VBSP/VVIS/VRAD paths. Users must be able to configure tool paths through command flags or a TOML profile, select compile steps, capture per-step logs, and receive JSON reports that summarize exit codes, warnings, errors, and leaks. Source tools are never bundled.

## Regression fixtures and golden outputs

CI must cover representative VMF structures so parser, merge, preview, deletion, transition, and automation behavior cannot drift silently. Fixtures should include world brushes, triggers, clips, areaportals, skybox materials, displacement start positions, multiple/duplicate landmarks, malformed VMFs, transitions, and larger mixed maps. Golden snapshots should verify key merged VMF output and job-runner JSON reports.

Real VMF validation must be reproducible outside CI without requiring bundled game assets. A script may download public VMFs from a pinned source commit, run inspect/list/merge/validate, and document any environment limitations such as unavailable Hammer/Hammer++ on Linux.

## Displacement translation

Landmark-aligned translation must move displacement-bearing brush sides consistently. Side `plane` points and `dispinfo` `startposition` values are absolute VMF coordinates and must be translated together. Non-position displacement fields such as normals, distances, alphas, triangle tags, and allowed verts must be preserved unless a real-map fixture proves they need different treatment.

## Texture-axis translation

Landmark-aligned brush translation must preserve texture-lock behavior for VMF side axes. When translating a brush by offset `t`, `uaxis` and `vaxis` shift values must be adjusted by `-dot(axis_vector, t)` while preserving axis vectors and scale values. Fixture tests must cover positive and negative axes.

## ID references

Merge renumbering must keep known VMF ID references consistent. Incoming `id` keys must be renumbered to avoid base-map collisions, and known reference keys such as `parentid`, `groupid`, `visgroupid`, `sideid`, `solidid`, `entityid`, `nodeid`, and overlay `sides` must be rewritten when the old ID maps uniquely to a new ID. Ambiguous duplicate old IDs must not be guessed; they should remain visible through integrity warnings and documented behavior.

## Editor metadata policy

Merge output must preserve base top-level editor/document metadata so the resulting VMF remains Hammer-friendly. Incoming top-level editor metadata such as `versioninfo`, `viewsettings`, `visgroups`, `cameras`, `cordons`, and unknown global metadata must be ignored intentionally unless a fixture-backed policy says otherwise. Nested editor metadata inside appended solids/entities must be preserved with those objects.

### Skybox preservation

World solids from incoming maps must be appended to the base map, including brushes using skybox tool materials. This ensures each selected map can contribute its skybox shell.

### Entity preservation

All incoming top-level `entity` blocks must be appended unless a deletion rule removes them. This includes point entities and brush entities.

### ID collision handling

Incoming VMF `id` keys must be renumbered before insertion to reduce Hammer conflicts.

## Entity and brush inspection requirements

### Entity discovery

The tool must list every top-level VMF `entity` block and expose its Hammer `classname`, `targetname`, `origin`, solid count, and detected roles.

Source Weaver must not rely on a fixed Hammer entity whitelist. Unknown or game-specific classnames must still be shown.

Large VMFs must remain manageable in the desktop inspection UI. Entity rows must support text search across block name, classname, targetname, and roles; role filtering; visible-row counts; and sortable columns for index, block, classname, targetname, origin, solids, and roles. Classname summaries must support search, filtered counts, and sorting by classname or count. Map-source filtering for merged data depends on source-provenance tracking in the merged preview workflow.

Entity inspection must enrich classnames with useful metadata while preserving unknown/game-specific entities. Built-in or inferred categories must be available for common Source classname patterns. Desktop users must be able to load FGD files for class-level descriptions and selected property metadata, and loaded FGD metadata must override built-in/inferred metadata for matching classnames.

Merged previews must visually distinguish the base VMF and incoming VMFs. Source colors must be stable for the current selected-map order, and the preview legend must identify every selected VMF. Role coloring should remain readable when source coloring is active.

The 2D preview should render closed convex brush solids using reconstructed face polygons rather than only bounding rectangles. Axis-aligned boxes must project as expected in top/front/side views, wedge or angled solids must render more accurately than their bounds, and malformed/open solids may fall back to bounds without blocking the preview.

The preview UI must include a 3D geometry view that does not replace the faster 2D top/front/side views. Users must be able to pan, zoom, and adjust the 3D camera angle. Tool brushes, entity origins, landmarks, deletion overlays, and selection outlines must remain visible in the 3D preview.

Preview views must show `info_landmark` markers with targetname labels in top, front, and side views. The currently selected merge landmark must be visually distinguished, and merged previews must show offset arrows/labels whose values match the merge report/status output.

Entity-table row selection must support multiple selected rows through checkboxes, select-all and clear controls, and a visible selected count. Selection keys must include the VMF path and row identity data so later deletion actions can target selected rows without confusing rows from different maps.

Preview clicks must update entity-table selection for the selected VMF. Clicking an entity marker or solid preview should toggle the owning world/entity row where possible. Selected rows must be visibly highlighted in the entity table and preview, and selection state must survive switching top/front/side projections.

### Deletion safety modes

Brush-role deletion must distinguish world solids from brush-entity behavior. World solid role matches remove matching world solids. Brush-entity role matches must use an explicit mode: `whole-entity` removes matching brush entities as a unit and preserves the original behavior; `matching-solids` preserves the entity and removes only matching contained solids. Critical transition/player/logic entities must be protected by default from classname, targetname, and brush-role deletion unless protection is explicitly disabled.

Desktop deletion presets must generate transparent deletion criteria that users can inspect before applying. Presets should cover common cleanup workflows such as triggers, clips, areaportals, gameplay logic, world-only geometry, and world-plus-skybox cleanup. Preset previews must run the same deletion implementation as final export so preview counts match final deletion counts.

The desktop preview must visualize current deletion criteria without writing output. Users must be able to distinguish kept and removed content through at least highlight, dim, and hide modes. Exact removal counts must continue to come from the same in-memory pruning path used by export and merge preview.

Desktop cleanup exports must be gated by pending review and confirmation. A deletion preview must create a pending review with exact counts. Users must be able to undo/clear the pending review before export. If deletion criteria change after review, confirmation must be revoked and export must require a fresh preview. Destructive cleaned-copy and merge exports must refuse to write until the current criteria have been reviewed and confirmed.

## Desktop usability

The desktop app must support common workflows with minimal manual steps. Users should be able to add VMFs, projects/jobs, and FGD metadata through drag-and-drop as well as file dialogs. Recent VMF and project paths should be available during the session. Large or failing scan workflows must show visible parse progress/failure status, and important errors must appear in the UI without requiring terminal logs. Users must be able to adjust preview/table space and switch between dark and light visual themes.

## Release packaging

Linux users must be able to download a release tarball containing the desktop app, CLI, desktop entry, icon, docs, README, and license. Windows users must be able to download a release zip containing the desktop app, CLI, icon asset, docs, README, and license. Tagging a `v*` version must build both packages, upload workflow artifacts, and publish a GitHub Release with release notes. Required Linux runtime libraries and Windows runtime expectations must be documented.

## BSP import stance

Source Weaver must remain VMF-first. BSP input support must rely on user-selected external decompiler paths and must not bundle third-party decompilers, game BSPs, or decompiled content until redistribution and update-policy review is complete. Documentation must explain legal responsibilities, decompile quality limitations, and the recommended external workflow.

The CLI may run a user-selected BSPSource launcher, BSPSource jar, or generic BSP decompiler wrapper and must validate the generated VMF before reporting success. Reports must include tool kind, tool path, version probe when available, command arguments, input BSP, output VMF, exit code, log path, warnings/errors, entity counts, classname counts, and integrity status. The desktop app must be able to select a BSP, run a user-selected BSPSource launcher/jar or generic wrapper through the same boundary, import the validated VMF with clear warnings, and keep it as a normal VMF input after import.

## Compile profile stance

Source Weaver must help users create and validate compile profiles without requiring hand-written TOML. Missing VBSP/VVIS/VRAD paths, invalid game directories, and unusable tool paths must produce actionable reports. Wine/Proton/native wrapper examples must remain external-tool examples and must not imply that Source Weaver ships Valve tools or requires Hammer for normal Linux VMF merge/edit use. Desktop compile launch must remain optional, use the same external-tool profile boundary, keep the UI responsive, and report compile results separately from VMF export success.

## Model tooling stance

Model tooling must remain optional and must not complicate VMF workflows. Source Weaver may inspect basic MDL metadata natively and may run user-provided StudioMDL-compatible compiler wrappers with machine-readable reports. Crowbar and other third-party model tools must not be bundled, ported, or copied until licensing, attribution, naming, runtime, and redistribution obligations are reviewed.

## BSP packing stance

BSP content packing must remain optional and must use user-provided external packer paths. Source Weaver must not bundle packers, BSPs, or custom assets. The CLI may generate BSPZIP-compatible file lists from asset roots and relative include rules or use an existing file list. Reports must include command provenance, tool path, best-effort tool version, input/output BSPs, file-list path, requested files, missing files, warnings, packer exit status, log path, and packed file counts when detectable. Desktop packing UI can be added after compile workflow stabilization.

### Landmark discovery

The core engine must expose `info_landmark` discovery independently from the desktop UI. A discovered landmark record includes the targetname, parsed origin when available, and source entity index. Duplicate landmark targetnames within a map must be reported with counts so UI and automation layers can warn before merge.

### Brush role discovery

The tool must detect brush categories using classnames and side materials, including:

- triggers
- clips, including player and NPC clips
- areaportals
- occluders
- skybox brushes
- hint brushes
- skip brushes
- nodraw brushes
- water brushes
- world brushes
- brush entities
- raw `tools/...` materials

## Deletion requirements

Users must be able to delete map content by:

- classname
- targetname
- brush role

Deletion must support repeated or comma-separated filters so future UI bulk selections can map directly to the same core rules.

## Validation requirements

Every parser, merge, transform, classification, and prune change must have tests. Generated VMFs should be opened in Hammer or validated by compiler tooling in later milestones.
