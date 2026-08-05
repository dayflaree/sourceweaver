from pathlib import Path

from sourceweaver.geometry import Vec3
from sourceweaver.semantics import SemanticDocument, build_semantic_document
from sourceweaver.stitching import (
    AlignmentBlockerCode,
    AlignmentStatus,
    TransitionBlockerCode,
    build_transition_graph,
    build_translation_alignment_hypothesis,
    normalize_map_name,
)
from sourceweaver.vmf import VmfDocument


def _semantic(text: str) -> SemanticDocument:
    return build_semantic_document(
        VmfDocument.from_bytes(text.encode("utf-8"), path=Path("synthetic.vmf"))
    )


def test_normalizes_map_names_for_matching_without_losing_raw_spelling() -> None:
    assert normalize_map_name("maps/D1_TrainStation_02.BSP") == "maps/d1_trainstation_02"
    assert normalize_map_name("d1_trainstation_02") == "d1_trainstation_02"


def test_extracts_changelevel_edge_landmark_and_transition_volume() -> None:
    semantic = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "info_landmark"
    "targetname" "landmark_a"
    "origin" "10 20 30"
}
entity
{
    "id" "3"
    "classname" "trigger_transition"
    "targetname" "landmark_a"
}
entity
{
    "id" "4"
    "classname" "trigger_changelevel"
    "targetname" "exit_trigger"
    "map" "D1_TrainStation_02.BSP"
    "landmark" "landmark_a"
}
"""
    )

    graph = build_transition_graph(semantic)

    assert [landmark.name for landmark in graph.landmarks] == ["landmark_a"]
    assert graph.landmarks[0].origin == Vec3(10.0, 20.0, 30.0)
    assert [transition.entity_index for transition in graph.transition_volumes] == [2]
    assert len(graph.edges) == 1
    edge = graph.edges[0]
    assert edge.changelevel_entity_index == 3
    assert edge.destination_raw == "D1_TrainStation_02.BSP"
    assert edge.destination_normalized == "d1_trainstation_02"
    assert edge.landmark_name == "landmark_a"
    assert [landmark.entity_index for landmark in edge.landmark_matches] == [1]
    assert edge.landmark_origin == Vec3(10.0, 20.0, 30.0)
    assert [volume.entity_index for volume in edge.transition_volume_matches] == [2]
    assert edge.blockers == ()


def test_missing_changelevel_landmark_is_reported_as_blocker() -> None:
    semantic = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "trigger_changelevel"
    "map" "next_map"
    "landmark" "missing_landmark"
}
"""
    )

    graph = build_transition_graph(semantic)

    assert len(graph.edges) == 1
    assert graph.edges[0].landmark_origin is None
    assert [blocker.code for blocker in graph.edges[0].blockers] == [
        TransitionBlockerCode.LANDMARK_NOT_FOUND
    ]


def test_duplicate_landmarks_are_ambiguous_instead_of_chosen() -> None:
    semantic = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "info_landmark"
    "targetname" "landmark_a"
    "origin" "0 0 0"
}
entity
{
    "id" "3"
    "classname" "info_landmark"
    "targetname" "landmark_a"
    "origin" "1 2 3"
}
entity
{
    "id" "4"
    "classname" "trigger_changelevel"
    "map" "next_map"
    "landmark" "landmark_a"
}
"""
    )

    graph = build_transition_graph(semantic)

    assert graph.edges[0].landmark_origin is None
    assert [blocker.code for blocker in graph.edges[0].blockers] == [
        TransitionBlockerCode.LANDMARK_AMBIGUOUS
    ]


def test_invalid_landmark_origin_blocks_transition_authority() -> None:
    semantic = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "info_landmark"
    "targetname" "landmark_a"
    "origin" "0 nope 0"
}
entity
{
    "id" "3"
    "classname" "trigger_changelevel"
    "map" "next_map"
    "landmark" "landmark_a"
}
"""
    )

    graph = build_transition_graph(semantic)

    assert graph.landmarks[0].origin is None
    assert [blocker.code for blocker in graph.edges[0].blockers] == [
        TransitionBlockerCode.LANDMARK_INVALID_ORIGIN
    ]


def test_changelevel_missing_required_keys_reports_stable_blockers() -> None:
    semantic = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "trigger_changelevel"
}
"""
    )

    graph = build_transition_graph(semantic)

    assert [blocker.code for blocker in graph.edges[0].blockers] == [
        TransitionBlockerCode.CHANGELEVEL_MISSING_MAP,
        TransitionBlockerCode.CHANGELEVEL_MISSING_LANDMARK,
    ]


def test_builds_unique_translation_alignment_hypothesis() -> None:
    source = build_transition_graph(
        _semantic(
            """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "info_landmark"
    "targetname" "shared_landmark"
    "origin" "100 200 300"
}
entity
{
    "id" "3"
    "classname" "trigger_changelevel"
    "map" "maps/BETA.BSP"
    "landmark" "shared_landmark"
}
"""
        )
    )
    candidate = build_transition_graph(
        _semantic(
            """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "info_landmark"
    "targetname" "shared_landmark"
    "origin" "10 20 30"
}
entity
{
    "id" "3"
    "classname" "trigger_changelevel"
    "map" "alpha"
    "landmark" "shared_landmark"
}
"""
        )
    )

    hypothesis = build_translation_alignment_hypothesis(
        source,
        candidate,
        source_map_name="ALPHA.bsp",
        candidate_map_name="maps/beta.bsp",
    )

    assert hypothesis.status is AlignmentStatus.VALID
    assert hypothesis.offset == Vec3(90.0, 180.0, 270.0)
    assert hypothesis.source_map_normalized == "alpha"
    assert hypothesis.candidate_map_normalized == "maps/beta"
    assert hypothesis.source_edge is source.edges[0]
    assert hypothesis.candidate_edge is candidate.edges[0]
    assert hypothesis.blockers == ()
    assert hypothesis.mutation_authorized is False


def test_alignment_blocks_multiple_source_edges_to_candidate() -> None:
    source = build_transition_graph(
        _semantic(
            """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "info_landmark"
    "targetname" "shared_landmark"
    "origin" "0 0 0"
}
entity
{
    "id" "3"
    "classname" "trigger_changelevel"
    "map" "beta"
    "landmark" "shared_landmark"
}
entity
{
    "id" "4"
    "classname" "trigger_changelevel"
    "map" "beta"
    "landmark" "shared_landmark"
}
"""
        )
    )
    candidate = build_transition_graph(
        _semantic(
            """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "info_landmark"
    "targetname" "shared_landmark"
    "origin" "0 0 0"
}
entity
{
    "id" "3"
    "classname" "trigger_changelevel"
    "map" "alpha"
    "landmark" "shared_landmark"
}
"""
        )
    )

    hypothesis = build_translation_alignment_hypothesis(
        source, candidate, source_map_name="alpha", candidate_map_name="beta"
    )

    assert hypothesis.status is AlignmentStatus.BLOCKED
    assert hypothesis.offset is None
    assert [blocker.code for blocker in hypothesis.blockers] == [
        AlignmentBlockerCode.SOURCE_EDGE_COUNT_UNSUPPORTED
    ]


def test_alignment_blocks_missing_reverse_edge() -> None:
    source = build_transition_graph(
        _semantic(
            """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "info_landmark"
    "targetname" "shared_landmark"
    "origin" "0 0 0"
}
entity
{
    "id" "3"
    "classname" "trigger_changelevel"
    "map" "beta"
    "landmark" "shared_landmark"
}
"""
        )
    )
    candidate = build_transition_graph(
        _semantic(
            """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "info_landmark"
    "targetname" "shared_landmark"
    "origin" "0 0 0"
}
"""
        )
    )

    hypothesis = build_translation_alignment_hypothesis(
        source, candidate, source_map_name="alpha", candidate_map_name="beta"
    )

    assert hypothesis.status is AlignmentStatus.BLOCKED
    assert [blocker.code for blocker in hypothesis.blockers] == [
        AlignmentBlockerCode.CANDIDATE_EDGE_COUNT_UNSUPPORTED
    ]


def test_alignment_propagates_transition_edge_blockers() -> None:
    source = build_transition_graph(
        _semantic(
            """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "trigger_changelevel"
    "map" "beta"
    "landmark" "missing_landmark"
}
"""
        )
    )
    candidate = build_transition_graph(
        _semantic(
            """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "info_landmark"
    "targetname" "missing_landmark"
    "origin" "0 0 0"
}
entity
{
    "id" "3"
    "classname" "trigger_changelevel"
    "map" "alpha"
    "landmark" "missing_landmark"
}
"""
        )
    )

    hypothesis = build_translation_alignment_hypothesis(
        source, candidate, source_map_name="alpha", candidate_map_name="beta"
    )

    assert hypothesis.status is AlignmentStatus.BLOCKED
    assert [blocker.code for blocker in hypothesis.blockers] == [
        AlignmentBlockerCode.SOURCE_EDGE_BLOCKED
    ]


def test_alignment_blocks_mismatched_landmark_names() -> None:
    source = build_transition_graph(
        _semantic(
            """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "info_landmark"
    "targetname" "source_landmark"
    "origin" "0 0 0"
}
entity
{
    "id" "3"
    "classname" "trigger_changelevel"
    "map" "beta"
    "landmark" "source_landmark"
}
"""
        )
    )
    candidate = build_transition_graph(
        _semantic(
            """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "info_landmark"
    "targetname" "candidate_landmark"
    "origin" "0 0 0"
}
entity
{
    "id" "3"
    "classname" "trigger_changelevel"
    "map" "alpha"
    "landmark" "candidate_landmark"
}
"""
        )
    )

    hypothesis = build_translation_alignment_hypothesis(
        source, candidate, source_map_name="alpha", candidate_map_name="beta"
    )

    assert hypothesis.status is AlignmentStatus.BLOCKED
    assert [blocker.code for blocker in hypothesis.blockers] == [
        AlignmentBlockerCode.LANDMARK_NAME_MISMATCH
    ]
