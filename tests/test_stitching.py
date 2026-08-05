from pathlib import Path

from sourceweaver.geometry import (
    BoundsRelation,
    BrushRelation,
    BrushSpatialRecord,
    ConvexBrush,
    Plane,
    ReconstructionStatus,
    Vec3,
    extract_brush_sources,
    reconstruct_convex_brush,
)
from sourceweaver.semantics import SemanticDocument, build_semantic_document
from sourceweaver.stitching import (
    AlignmentBlockerCode,
    AlignmentStatus,
    ImportIdAllocationBlockerCode,
    ImportIdAllocationStatus,
    ImportIdKind,
    SeamConfidenceBlockerCode,
    SeamConfidenceStatus,
    SeamDeletionClass,
    SeamDeletionEvidenceStatus,
    SeamEvidenceBlockerCode,
    SeamEvidenceStatus,
    TransitionBlockerCode,
    TranslationAlignmentHypothesis,
    build_import_id_allocation_plan,
    build_seam_confidence_report,
    build_seam_deletion_evidence,
    build_seam_overlap_evidence,
    build_transition_graph,
    build_translation_alignment_hypothesis,
    normalize_map_name,
)
from sourceweaver.vmf import VmfDocument


def _semantic(text: str) -> SemanticDocument:
    return build_semantic_document(
        VmfDocument.from_bytes(text.encode("utf-8"), path=Path("synthetic.vmf"))
    )


def _document(text: str) -> VmfDocument:
    return VmfDocument.from_bytes(text.encode("utf-8"), path=Path("synthetic.vmf"))


def _point(point: Vec3) -> str:
    return f"({point.x:g} {point.y:g} {point.z:g})"


def _cube_plane_strings(minimum: Vec3, maximum: Vec3) -> tuple[str, ...]:
    x0, y0, z0 = minimum.as_tuple()
    x1, y1, z1 = maximum.as_tuple()
    return (
        f"{_point(Vec3(x0, y0, z0))} {_point(Vec3(x0, y1, z0))} {_point(Vec3(x1, y1, z0))}",
        f"{_point(Vec3(x0, y0, z1))} {_point(Vec3(x1, y0, z1))} {_point(Vec3(x1, y1, z1))}",
        f"{_point(Vec3(x0, y0, z0))} {_point(Vec3(x1, y0, z0))} {_point(Vec3(x1, y0, z1))}",
        f"{_point(Vec3(x0, y1, z0))} {_point(Vec3(x0, y1, z1))} {_point(Vec3(x1, y1, z1))}",
        f"{_point(Vec3(x0, y0, z0))} {_point(Vec3(x0, y0, z1))} {_point(Vec3(x0, y1, z1))}",
        f"{_point(Vec3(x1, y0, z0))} {_point(Vec3(x1, y1, z0))} {_point(Vec3(x1, y1, z1))}",
    )


def _cube_brush(minimum: Vec3, maximum: Vec3) -> ConvexBrush:
    result = reconstruct_convex_brush(
        tuple(Plane.from_vmf(raw) for raw in _cube_plane_strings(minimum, maximum))
    )
    assert result.status is ReconstructionStatus.VALID
    assert result.brush is not None
    return result.brush


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


def test_builds_seam_overlap_evidence_from_alignment_offset() -> None:
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
    "origin" "128 0 0"
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
    alignment = build_translation_alignment_hypothesis(
        source, candidate, source_map_name="alpha", candidate_map_name="beta"
    )
    source_records = (
        BrushSpatialRecord(
            "alpha/world/solid/1",
            _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0)),
        ),
    )
    candidate_records = (
        BrushSpatialRecord(
            "beta/world/solid/1",
            _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0)),
        ),
    )

    evidence = build_seam_overlap_evidence(alignment, source_records, candidate_records)

    assert evidence.status is SeamEvidenceStatus.VALID
    assert evidence.mutation_authorized is False
    assert evidence.blockers == ()
    assert len(evidence.translated_candidate_records) == 1
    translated = evidence.translated_candidate_records[0]
    assert translated.key == "beta/world/solid/1"
    assert translated.brush.bounds_min == Vec3(128.0, 0.0, 0.0)
    assert translated.brush.bounds_max == Vec3(256.0, 128.0, 128.0)
    assert len(evidence.brush_pairs) == 1
    pair = evidence.brush_pairs[0]
    assert pair.source_key == "alpha/world/solid/1"
    assert pair.candidate_key == "beta/world/solid/1"
    assert pair.bounds_relation is BoundsRelation.TOUCHING
    assert pair.brush_relation is BrushRelation.TOUCHING


def test_seam_overlap_evidence_blocks_invalid_alignment() -> None:
    source = build_transition_graph(
        _semantic(
            """world
{
    "id" "1"
    "classname" "worldspawn"
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
"""
        )
    )
    alignment = build_translation_alignment_hypothesis(
        source, candidate, source_map_name="alpha", candidate_map_name="beta"
    )

    evidence = build_seam_overlap_evidence(alignment, (), ())

    assert evidence.status is SeamEvidenceStatus.BLOCKED
    assert evidence.brush_pairs == ()
    assert [blocker.code for blocker in evidence.blockers] == [
        SeamEvidenceBlockerCode.ALIGNMENT_BLOCKED
    ]


def test_seam_overlap_evidence_blocks_untranslatable_candidate_brush() -> None:
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
    "origin" "70000 0 0"
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
    alignment = build_translation_alignment_hypothesis(
        source, candidate, source_map_name="alpha", candidate_map_name="beta"
    )
    candidate_records = (
        BrushSpatialRecord(
            "beta/world/solid/1",
            _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0)),
        ),
    )

    evidence = build_seam_overlap_evidence(alignment, (), candidate_records)

    assert evidence.status is SeamEvidenceStatus.BLOCKED
    assert evidence.translated_candidate_records == ()
    assert [(blocker.code, blocker.record_key) for blocker in evidence.blockers] == [
        (SeamEvidenceBlockerCode.CANDIDATE_TRANSFORM_BLOCKED, "beta/world/solid/1")
    ]
    assert evidence.blockers[0].geometry_blocker_codes == ("BRUSH_WORLD_BOUNDS_EXCEEDED",)


def _valid_zero_offset_alignment() -> TranslationAlignmentHypothesis:
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
    return build_translation_alignment_hypothesis(
        source, candidate, source_map_name="alpha", candidate_map_name="beta"
    )


def test_classifies_equal_seam_pair_as_candidate_duplicate_evidence() -> None:
    alignment = _valid_zero_offset_alignment()
    source_records = (
        BrushSpatialRecord(
            "alpha/world/solid/1",
            _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0)),
        ),
    )
    candidate_records = (
        BrushSpatialRecord(
            "beta/world/solid/1",
            _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0)),
        ),
    )
    overlap = build_seam_overlap_evidence(alignment, source_records, candidate_records)

    deletion = build_seam_deletion_evidence(overlap)

    assert deletion.status is SeamDeletionEvidenceStatus.VALID
    assert deletion.mutation_authorized is False
    assert deletion.blockers == ()
    assert [
        (item.source_key, item.candidate_key, item.deletion_class) for item in deletion.items
    ] == [
        (
            "alpha/world/solid/1",
            "beta/world/solid/1",
            SeamDeletionClass.CANDIDATE_EQUAL_VOLUME_DUPLICATE,
        )
    ]
    assert deletion.items[0].remove_candidate is True
    assert deletion.items[0].remove_source is False


def test_classifies_contained_candidate_as_candidate_containment_evidence() -> None:
    alignment = _valid_zero_offset_alignment()
    source_records = (
        BrushSpatialRecord(
            "alpha/world/solid/outer",
            _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0)),
        ),
    )
    candidate_records = (
        BrushSpatialRecord(
            "beta/world/solid/inner",
            _cube_brush(Vec3(32.0, 32.0, 32.0), Vec3(96.0, 96.0, 96.0)),
        ),
    )
    overlap = build_seam_overlap_evidence(alignment, source_records, candidate_records)

    deletion = build_seam_deletion_evidence(overlap)

    assert [item.deletion_class for item in deletion.items] == [
        SeamDeletionClass.CANDIDATE_CONTAINED_IN_SOURCE
    ]
    assert deletion.items[0].remove_candidate is True
    assert deletion.items[0].remove_source is False


def test_classifies_touching_and_overlapping_pairs_as_preserve_evidence() -> None:
    alignment = _valid_zero_offset_alignment()
    source_records = (
        BrushSpatialRecord(
            "alpha/touch-source",
            _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0)),
        ),
        BrushSpatialRecord(
            "alpha/overlap-source",
            _cube_brush(Vec3(256.0, 0.0, 0.0), Vec3(384.0, 128.0, 128.0)),
        ),
    )
    candidate_records = (
        BrushSpatialRecord(
            "beta/touch-candidate",
            _cube_brush(Vec3(128.0, 0.0, 0.0), Vec3(256.0, 128.0, 128.0)),
        ),
        BrushSpatialRecord(
            "beta/overlap-candidate",
            _cube_brush(Vec3(320.0, 0.0, 0.0), Vec3(448.0, 128.0, 128.0)),
        ),
    )
    overlap = build_seam_overlap_evidence(alignment, source_records, candidate_records)

    deletion = build_seam_deletion_evidence(overlap)

    assert [
        (item.source_key, item.candidate_key, item.deletion_class) for item in deletion.items
    ] == [
        (
            "alpha/overlap-source",
            "beta/overlap-candidate",
            SeamDeletionClass.PRESERVE_UNSAFE_OVERLAP,
        ),
        (
            "alpha/overlap-source",
            "beta/touch-candidate",
            SeamDeletionClass.PRESERVE_TOUCHING_SEAM,
        ),
        (
            "alpha/touch-source",
            "beta/touch-candidate",
            SeamDeletionClass.PRESERVE_TOUCHING_SEAM,
        ),
    ]
    assert all(not item.remove_candidate and not item.remove_source for item in deletion.items)


def test_deletion_evidence_blocks_when_seam_evidence_is_blocked() -> None:
    source = build_transition_graph(
        _semantic(
            """world
{
    "id" "1"
    "classname" "worldspawn"
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
"""
        )
    )
    alignment = build_translation_alignment_hypothesis(
        source, candidate, source_map_name="alpha", candidate_map_name="beta"
    )
    overlap = build_seam_overlap_evidence(alignment, (), ())

    deletion = build_seam_deletion_evidence(overlap)

    assert deletion.status is SeamDeletionEvidenceStatus.BLOCKED
    assert deletion.items == ()
    assert deletion.blockers == overlap.blockers


def test_seam_confidence_accepts_touching_and_candidate_removal_evidence() -> None:
    alignment = _valid_zero_offset_alignment()
    source_records = (
        BrushSpatialRecord(
            "alpha/source",
            _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0)),
        ),
    )
    candidate_records = (
        BrushSpatialRecord(
            "beta/duplicate",
            _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0)),
        ),
        BrushSpatialRecord(
            "beta/touching",
            _cube_brush(Vec3(128.0, 0.0, 0.0), Vec3(256.0, 128.0, 128.0)),
        ),
    )
    overlap = build_seam_overlap_evidence(alignment, source_records, candidate_records)
    deletion = build_seam_deletion_evidence(overlap)

    report = build_seam_confidence_report(deletion)

    assert report.status is SeamConfidenceStatus.READY_FOR_REVIEW
    assert report.mutation_authorized is False
    assert report.blockers == ()
    assert report.pair_count == 2
    assert report.candidate_removal_count == 1
    assert report.touching_pair_count == 1
    assert report.unsafe_overlap_count == 0
    assert report.source_removal_count == 0


def test_seam_confidence_blocks_unsafe_overlap() -> None:
    alignment = _valid_zero_offset_alignment()
    source_records = (
        BrushSpatialRecord(
            "alpha/source",
            _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0)),
        ),
    )
    candidate_records = (
        BrushSpatialRecord(
            "beta/overlap",
            _cube_brush(Vec3(64.0, 0.0, 0.0), Vec3(192.0, 128.0, 128.0)),
        ),
    )
    deletion = build_seam_deletion_evidence(
        build_seam_overlap_evidence(alignment, source_records, candidate_records)
    )

    report = build_seam_confidence_report(deletion)

    assert report.status is SeamConfidenceStatus.BLOCKED
    assert [(blocker.code, blocker.item_index) for blocker in report.blockers] == [
        (SeamConfidenceBlockerCode.UNSAFE_OVERLAP, 0)
    ]
    assert report.unsafe_overlap_count == 1


def test_seam_confidence_blocks_source_removal_candidates() -> None:
    alignment = _valid_zero_offset_alignment()
    source_records = (
        BrushSpatialRecord(
            "alpha/inner",
            _cube_brush(Vec3(32.0, 32.0, 32.0), Vec3(96.0, 96.0, 96.0)),
        ),
    )
    candidate_records = (
        BrushSpatialRecord(
            "beta/outer",
            _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0)),
        ),
    )
    deletion = build_seam_deletion_evidence(
        build_seam_overlap_evidence(alignment, source_records, candidate_records)
    )

    report = build_seam_confidence_report(deletion)

    assert report.status is SeamConfidenceStatus.BLOCKED
    assert [(blocker.code, blocker.item_index) for blocker in report.blockers] == [
        (SeamConfidenceBlockerCode.SOURCE_REMOVAL_UNSUPPORTED, 0)
    ]
    assert report.source_removal_count == 1


def test_seam_confidence_blocks_empty_or_blocked_deletion_evidence() -> None:
    alignment = _valid_zero_offset_alignment()
    empty = build_seam_deletion_evidence(build_seam_overlap_evidence(alignment, (), ()))

    empty_report = build_seam_confidence_report(empty)

    assert empty_report.status is SeamConfidenceStatus.BLOCKED
    assert [blocker.code for blocker in empty_report.blockers] == [
        SeamConfidenceBlockerCode.EMPTY_SEAM_EVIDENCE
    ]

    source = build_transition_graph(_semantic('world\n{\n    "id" "1"\n}\n'))
    candidate = build_transition_graph(_semantic('world\n{\n    "id" "1"\n}\n'))
    blocked_alignment = build_translation_alignment_hypothesis(
        source, candidate, source_map_name="alpha", candidate_map_name="beta"
    )
    blocked = build_seam_deletion_evidence(build_seam_overlap_evidence(blocked_alignment, (), ()))

    blocked_report = build_seam_confidence_report(blocked)

    assert blocked_report.status is SeamConfidenceStatus.BLOCKED
    assert [blocker.code for blocker in blocked_report.blockers] == [
        SeamConfidenceBlockerCode.DELETION_EVIDENCE_BLOCKED
    ]


def test_builds_deterministic_import_id_allocation_plan() -> None:
    source_doc = _document(
        """world
{
    "id" "1"
    "classname" "worldspawn"
    solid
    {
        "id" "20"
        side { "id" "21" "plane" "(0 0 0) (0 1 0) (1 1 0)" }
    }
}
entity
{
    "id" "40"
    "classname" "info_target"
}
"""
    )
    candidate_doc = _document(
        """world
{
    "id" "2"
    "classname" "worldspawn"
    solid
    {
        "id" "3"
        side { "id" "4" "plane" "(0 0 0) (0 1 0) (1 1 0)" }
    }
}
entity
{
    "id" "40"
    "classname" "info_target"
}
"""
    )

    plan = build_import_id_allocation_plan(
        build_semantic_document(source_doc),
        build_semantic_document(candidate_doc),
        extract_brush_sources(source_doc.syntax),
        extract_brush_sources(candidate_doc.syntax),
    )

    assert plan.status is ImportIdAllocationStatus.VALID
    assert plan.blockers == ()
    assert plan.mutation_authorized is False
    assert [(item.kind, item.original_id, item.allocated_id) for item in plan.allocations] == [
        (ImportIdKind.SOLID, "3", "41"),
        (ImportIdKind.SIDE, "4", "42"),
        (ImportIdKind.ENTITY, "40", "43"),
    ]


def test_import_id_allocation_blocks_duplicate_candidate_ids() -> None:
    source_doc = _document('world\n{\n    "id" "1"\n}\n')
    candidate_doc = _document(
        """world
{
    "id" "2"
    "classname" "worldspawn"
}
entity
{
    "id" "7"
    "classname" "info_target"
}
entity
{
    "id" "7"
    "classname" "info_target"
}
"""
    )

    plan = build_import_id_allocation_plan(
        build_semantic_document(source_doc),
        build_semantic_document(candidate_doc),
        (),
        (),
    )

    assert plan.status is ImportIdAllocationStatus.BLOCKED
    assert plan.allocations == ()
    assert [(blocker.code, blocker.kind, blocker.raw_id) for blocker in plan.blockers] == [
        (ImportIdAllocationBlockerCode.DUPLICATE_CANDIDATE_ID, ImportIdKind.ENTITY, "7")
    ]


def test_import_id_allocation_blocks_missing_and_invalid_candidate_ids() -> None:
    source_doc = _document('world\n{\n    "id" "1"\n}\n')
    candidate_doc = _document(
        """world
{
    "id" "2"
    "classname" "worldspawn"
    solid
    {
        side { "id" "abc" "plane" "(0 0 0) (0 1 0) (1 1 0)" }
    }
}
entity
{
    "classname" "info_target"
}
"""
    )

    plan = build_import_id_allocation_plan(
        build_semantic_document(source_doc),
        build_semantic_document(candidate_doc),
        (),
        extract_brush_sources(candidate_doc.syntax),
    )

    assert plan.status is ImportIdAllocationStatus.BLOCKED
    assert [(blocker.code, blocker.kind, blocker.raw_id) for blocker in plan.blockers] == [
        (ImportIdAllocationBlockerCode.MISSING_CANDIDATE_ID, ImportIdKind.SOLID, None),
        (ImportIdAllocationBlockerCode.NON_NUMERIC_ID, ImportIdKind.SIDE, "abc"),
        (ImportIdAllocationBlockerCode.MISSING_CANDIDATE_ID, ImportIdKind.ENTITY, None),
    ]
