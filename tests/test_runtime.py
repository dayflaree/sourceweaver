from pathlib import Path

from sourceweaver.compiler import CompilerRunPreflight, CompilerRunStatus
from sourceweaver.runtime import (
    RuntimeAcceptanceBlockerCode,
    RuntimeAcceptanceStatus,
    RuntimeScenarioId,
    build_runtime_acceptance_manifest,
)


def _ready_compiler() -> CompilerRunPreflight:
    return CompilerRunPreflight(
        status=CompilerRunStatus.READY,
        tools=(),
        blockers=(),
    )


def test_runtime_acceptance_manifest_is_ready_for_existing_compiled_artifacts(
    tmp_path: Path,
) -> None:
    baseline = tmp_path / "baseline.bsp"
    candidate = tmp_path / "candidate.bsp"
    baseline.write_bytes(b"VBSP\0baseline")
    candidate.write_bytes(b"VBSP\0candidate")

    manifest = build_runtime_acceptance_manifest(
        _ready_compiler(),
        baseline_bsp=baseline,
        candidate_bsp=candidate,
        map_name="transition_alpha_beta",
    )

    assert manifest.status is RuntimeAcceptanceStatus.READY
    assert manifest.blockers == ()
    assert manifest.baseline_bsp == baseline
    assert manifest.candidate_bsp == candidate
    assert [scenario.scenario_id for scenario in manifest.scenarios] == [
        RuntimeScenarioId.MAP_LOAD_SPAWN,
        RuntimeScenarioId.SEAM_FORWARD_WALK,
        RuntimeScenarioId.SEAM_REVERSE_WALK,
        RuntimeScenarioId.LIFECYCLE_RELAY_CYCLE,
        RuntimeScenarioId.SAVE_RELOAD_SEAM,
        RuntimeScenarioId.DEATH_RESPAWN_CLEANUP,
        RuntimeScenarioId.REPEATED_TRANSITION_CYCLES,
    ]
    assert all(scenario.mandatory for scenario in manifest.scenarios)


def test_runtime_acceptance_manifest_blocks_missing_bsp_artifacts(tmp_path: Path) -> None:
    baseline = tmp_path / "missing-baseline.bsp"
    candidate = tmp_path / "missing-candidate.bsp"

    manifest = build_runtime_acceptance_manifest(
        _ready_compiler(),
        baseline_bsp=baseline,
        candidate_bsp=candidate,
        map_name="transition_alpha_beta",
    )

    assert manifest.status is RuntimeAcceptanceStatus.BLOCKED
    assert manifest.scenarios == ()
    assert [(blocker.code, blocker.path) for blocker in manifest.blockers] == [
        (RuntimeAcceptanceBlockerCode.BASELINE_BSP_MISSING, baseline),
        (RuntimeAcceptanceBlockerCode.CANDIDATE_BSP_MISSING, candidate),
    ]


def test_runtime_acceptance_manifest_blocks_unready_compiler_preflight(tmp_path: Path) -> None:
    baseline = tmp_path / "baseline.bsp"
    candidate = tmp_path / "candidate.bsp"
    baseline.write_bytes(b"VBSP\0baseline")
    candidate.write_bytes(b"VBSP\0candidate")

    manifest = build_runtime_acceptance_manifest(
        CompilerRunPreflight(
            status=CompilerRunStatus.BLOCKED,
            tools=(),
            blockers=(),
        ),
        baseline_bsp=baseline,
        candidate_bsp=candidate,
        map_name="transition_alpha_beta",
    )

    assert manifest.status is RuntimeAcceptanceStatus.BLOCKED
    assert manifest.scenarios == ()
    assert [blocker.code for blocker in manifest.blockers] == [
        RuntimeAcceptanceBlockerCode.COMPILER_PREFLIGHT_BLOCKED
    ]


def test_runtime_acceptance_manifest_blocks_empty_map_name(tmp_path: Path) -> None:
    baseline = tmp_path / "baseline.bsp"
    candidate = tmp_path / "candidate.bsp"
    baseline.write_bytes(b"VBSP\0baseline")
    candidate.write_bytes(b"VBSP\0candidate")

    manifest = build_runtime_acceptance_manifest(
        _ready_compiler(),
        baseline_bsp=baseline,
        candidate_bsp=candidate,
        map_name="",
    )

    assert manifest.status is RuntimeAcceptanceStatus.BLOCKED
    assert [blocker.code for blocker in manifest.blockers] == [
        RuntimeAcceptanceBlockerCode.EMPTY_MAP_NAME
    ]
