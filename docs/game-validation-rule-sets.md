# Game validation rule sets

Source Weaver's generic VMF validation checks portable structure: parsing, top-level `world`, common sections, and numeric ID warnings. Game validation rule sets add opt-in map/profile semantics on top of that generic path.

Rule sets are built into Source Weaver as portable checks. They do not run Hammer, Hammer++, VBSP, VVIS, VRAD, a game runtime, or any external SDK. They do not require a game install.

## CLI usage

```bash
sourceweaver validate map.vmf --rule-set hl2 --json
```

Use `--rule-set none` or omit the flag for generic VMF integrity only. The JSON report keeps rule-set findings under `rule_set`, separate from generic `integrity` findings and compile-log findings.

## Desktop usage

The desktop **VMF integrity status** panel has a **Rule set** selector. Changing it rescans loaded VMFs and shows rule-set error/warning counts separately from generic integrity counts. The same boundary applies in the desktop UI: selecting a rule set performs Source Weaver checks only.

## Built-in rule sets

### `hl2`: Half-Life 2 single-player

Scope: portable structural checks for HL2/Source 2013 single-player VMFs. This profile is intended to catch common HL2 campaign-map issues that can be detected from VMF text alone.

Current checks:

- warn when no top-level `info_player_start` is present;
- warn when an `info_landmark` lacks a usable `targetname`;
- warn when an `info_landmark` lacks a numeric three-component `origin`;
- error when a `trigger_changelevel` lacks a `map` target;
- warn when a `trigger_changelevel` lacks a `landmark` key;
- warn when a `trigger_changelevel` references an `info_landmark` targetname absent from the same VMF.

Fixtures:

- `tests/fixtures/hl2_ruleset_ok.vmf` covers a minimal passing HL2-style VMF.
- `tests/fixtures/hl2_ruleset_warnings.vmf` covers separate rule-set errors and warnings.

## Validation boundary

A passing rule-set report means only that Source Weaver's portable checks passed. Real game validation still needs the appropriate rung to be run and recorded: Hammer/Hammer++ open checks, VBSP/VVIS/VRAD compile logs, or game runtime map-load evidence.
