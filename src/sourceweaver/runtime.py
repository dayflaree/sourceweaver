"""Runtime acceptance scenario manifests for generated Source maps.

This module defines deterministic acceptance inputs and preflight blockers. It
intentionally does not launch GMod or claim runtime proof.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path

from sourceweaver.compiler import CompilerRunPreflight, CompilerRunStatus


class RuntimeAcceptanceStatus(StrEnum):
    """Final status for runtime acceptance manifest readiness."""

    READY = "ready"
    BLOCKED = "blocked"


class RuntimeAcceptanceBlockerCode(StrEnum):
    """Deterministic blockers for runtime acceptance manifest readiness."""

    COMPILER_PREFLIGHT_BLOCKED = "compiler_preflight_blocked"
    BASELINE_BSP_MISSING = "baseline_bsp_missing"
    CANDIDATE_BSP_MISSING = "candidate_bsp_missing"
    EMPTY_MAP_NAME = "empty_map_name"


class RuntimeScenarioId(StrEnum):
    """Mandatory runtime scenarios for the current stitching support envelope."""

    MAP_LOAD_SPAWN = "map_load_spawn"
    SEAM_FORWARD_WALK = "seam_forward_walk"
    SEAM_REVERSE_WALK = "seam_reverse_walk"
    LIFECYCLE_RELAY_CYCLE = "lifecycle_relay_cycle"
    SAVE_RELOAD_SEAM = "save_reload_seam"
    DEATH_RESPAWN_CLEANUP = "death_respawn_cleanup"
    REPEATED_TRANSITION_CYCLES = "repeated_transition_cycles"


@dataclass(frozen=True, slots=True)
class RuntimeScenario:
    """One deterministic runtime scenario definition."""

    scenario_id: RuntimeScenarioId
    map_name: str
    mandatory: bool
    assertions: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class RuntimeAcceptanceBlocker:
    """A reason runtime acceptance cannot be attempted."""

    code: RuntimeAcceptanceBlockerCode
    message: str
    path: Path | None = None


@dataclass(frozen=True, slots=True)
class RuntimeAcceptanceManifest:
    """Read-only runtime acceptance manifest for baseline/candidate BSPs."""

    status: RuntimeAcceptanceStatus
    map_name: str
    baseline_bsp: Path
    candidate_bsp: Path
    scenarios: tuple[RuntimeScenario, ...]
    blockers: tuple[RuntimeAcceptanceBlocker, ...]
    runtime_proof_authorized: bool = False


_RUNTIME_SCENARIO_ASSERTIONS: tuple[tuple[RuntimeScenarioId, tuple[str, ...]], ...] = (
    (
        RuntimeScenarioId.MAP_LOAD_SPAWN,
        (
            "candidate BSP loads without crash or forbidden console errors",
            "player spawns at a deterministic checkpoint",
        ),
    ),
    (
        RuntimeScenarioId.SEAM_FORWARD_WALK,
        (
            "player can traverse from source-side checkpoint into candidate region",
            "region lifecycle relay activation is observed once",
        ),
    ),
    (
        RuntimeScenarioId.SEAM_REVERSE_WALK,
        (
            "player can backtrack from candidate region to source-side checkpoint",
            "deactivation relay is observed without duplicate activation",
        ),
    ),
    (
        RuntimeScenarioId.LIFECYCLE_RELAY_CYCLE,
        (
            "preload, activate, deactivate, reset, and remove relays are addressable",
            "required relay outputs fire in deterministic order",
        ),
    ),
    (
        RuntimeScenarioId.SAVE_RELOAD_SEAM,
        ("save before, inside, and after seam reloads without lifecycle corruption",),
    ),
    (
        RuntimeScenarioId.DEATH_RESPAWN_CLEANUP,
        ("death and respawn near seam preserve cleanup policy",),
    ),
    (
        RuntimeScenarioId.REPEATED_TRANSITION_CYCLES,
        ("forward and reverse transition cycle repeats without accumulating errors",),
    ),
)


def build_runtime_acceptance_manifest(
    compiler_preflight: CompilerRunPreflight,
    *,
    baseline_bsp: Path,
    candidate_bsp: Path,
    map_name: str,
) -> RuntimeAcceptanceManifest:
    """Build a runtime acceptance manifest when compiled artifacts are present."""
    blockers = _runtime_acceptance_blockers(
        compiler_preflight,
        baseline_bsp=baseline_bsp,
        candidate_bsp=candidate_bsp,
        map_name=map_name,
    )
    if blockers:
        return RuntimeAcceptanceManifest(
            status=RuntimeAcceptanceStatus.BLOCKED,
            map_name=map_name,
            baseline_bsp=baseline_bsp,
            candidate_bsp=candidate_bsp,
            scenarios=(),
            blockers=blockers,
        )
    return RuntimeAcceptanceManifest(
        status=RuntimeAcceptanceStatus.READY,
        map_name=map_name,
        baseline_bsp=baseline_bsp,
        candidate_bsp=candidate_bsp,
        scenarios=_runtime_scenarios(map_name),
        blockers=(),
    )


def _runtime_acceptance_blockers(
    compiler_preflight: CompilerRunPreflight,
    *,
    baseline_bsp: Path,
    candidate_bsp: Path,
    map_name: str,
) -> tuple[RuntimeAcceptanceBlocker, ...]:
    blockers: list[RuntimeAcceptanceBlocker] = []
    if compiler_preflight.status is not CompilerRunStatus.READY:
        blockers.append(
            RuntimeAcceptanceBlocker(
                code=RuntimeAcceptanceBlockerCode.COMPILER_PREFLIGHT_BLOCKED,
                message="Compiler preflight must be ready before runtime acceptance.",
            )
        )
    if not baseline_bsp.is_file():
        blockers.append(
            RuntimeAcceptanceBlocker(
                code=RuntimeAcceptanceBlockerCode.BASELINE_BSP_MISSING,
                message="Baseline compiled BSP is missing.",
                path=baseline_bsp,
            )
        )
    if not candidate_bsp.is_file():
        blockers.append(
            RuntimeAcceptanceBlocker(
                code=RuntimeAcceptanceBlockerCode.CANDIDATE_BSP_MISSING,
                message="Candidate compiled BSP is missing.",
                path=candidate_bsp,
            )
        )
    if not map_name:
        blockers.append(
            RuntimeAcceptanceBlocker(
                code=RuntimeAcceptanceBlockerCode.EMPTY_MAP_NAME,
                message="Runtime acceptance map name must not be empty.",
            )
        )
    return tuple(blockers)


def _runtime_scenarios(map_name: str) -> tuple[RuntimeScenario, ...]:
    return tuple(
        RuntimeScenario(
            scenario_id=scenario_id,
            map_name=map_name,
            mandatory=True,
            assertions=assertions,
        )
        for scenario_id, assertions in _RUNTIME_SCENARIO_ASSERTIONS
    )
