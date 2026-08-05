from pathlib import Path

from sourceweaver.compiler import CompilerRunPreflight, CompilerRunStatus
from sourceweaver.geometry import (
    BoundsRelation,
    BrushRelation,
    BrushSource,
    BrushSpatialRecord,
    ConvexBrush,
    Plane,
    ReconstructionStatus,
    Vec3,
    extract_brush_sources,
    reconstruct_convex_brush,
)
from sourceweaver.lifecycle import (
    LifecycleControllerEntityPlan,
    LifecycleControllerEntityStatus,
    build_lifecycle_controller_entity_plan,
    build_lifecycle_controller_output_plan,
    build_lifecycle_controller_plan,
    build_lifecycle_policy_matrix,
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
    SeamDeletionEvidence,
    SeamDeletionEvidenceStatus,
    SeamEvidenceBlockerCode,
    SeamEvidenceStatus,
    SingletonConflictCode,
    SingletonConflictStatus,
    StitchMaterializationBlockerCode,
    StitchMaterializationStatus,
    StitchPlanManifest,
    StitchPlanManifestBlockerCode,
    StitchPlanManifestStatus,
    StitchPreflightBlockerCode,
    StitchPreflightStatus,
    StitchRemovalAuthorityBlockerCode,
    StitchRemovalAuthorityStatus,
    TargetNameNamespaceBlockerCode,
    TargetNameNamespaceEditKind,
    TargetNameNamespaceStatus,
    TransitionBlockerCode,
    TranslationAlignmentHypothesis,
    build_import_id_allocation_plan,
    build_seam_confidence_report,
    build_seam_deletion_evidence,
    build_seam_overlap_evidence,
    build_singleton_conflict_report,
    build_stitch_plan_manifest,
    build_stitch_preflight_report,
    build_stitch_removal_authority_report,
    build_targetname_namespace_plan,
    build_transition_graph,
    build_translation_alignment_hypothesis,
    materialize_stitch_from_manifest,
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


def test_builds_targetname_namespace_plan_for_supported_semantic_references() -> None:
    source = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "logic_relay"
    "targetname" "alpha_relay"
}
"""
    )
    candidate = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "logic_relay"
    "targetname" "relay"
}
entity
{
    "id" "3"
    "classname" "logic_relay"
    "targetname" "child"
    "parentname" "relay"
    "OnTrigger" "relay,Trigger,,0,-1"
}
"""
    )

    plan = build_targetname_namespace_plan(source, candidate, prefix="beta__")

    assert plan.status is TargetNameNamespaceStatus.VALID
    assert plan.mutation_authorized is False
    assert plan.blockers == ()
    assert [
        (edit.kind, edit.entity_index, edit.original_value, edit.namespaced_value)
        for edit in plan.edits
    ] == [
        (TargetNameNamespaceEditKind.DEFINITION, 1, "relay", "beta__relay"),
        (TargetNameNamespaceEditKind.DEFINITION, 2, "child", "beta__child"),
        (TargetNameNamespaceEditKind.REFERENCE, 2, "relay", "beta__relay"),
        (TargetNameNamespaceEditKind.REFERENCE, 2, "relay", "beta__relay"),
    ]


def test_namespace_plan_blocks_unresolved_and_ambiguous_references() -> None:
    source = _semantic('world\n{\n    "id" "1"\n}\n')
    candidate = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "logic_relay"
    "targetname" "dup"
}
entity
{
    "id" "3"
    "classname" "logic_relay"
    "targetname" "dup"
}
entity
{
    "id" "4"
    "classname" "logic_relay"
    "parentname" "dup"
    "OnTrigger" "missing,Trigger,,0,-1"
}
"""
    )

    plan = build_targetname_namespace_plan(source, candidate, prefix="beta__")

    assert plan.status is TargetNameNamespaceStatus.BLOCKED
    assert plan.edits == ()
    assert [(blocker.code, blocker.entity_index, blocker.name) for blocker in plan.blockers] == [
        (TargetNameNamespaceBlockerCode.UNRESOLVED_REFERENCE, 3, "missing"),
        (TargetNameNamespaceBlockerCode.AMBIGUOUS_REFERENCE, 3, "dup"),
    ]


def test_namespace_plan_blocks_special_wildcard_empty_prefix_and_source_collisions() -> None:
    source = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "logic_relay"
    "targetname" "beta__relay"
}
"""
    )
    candidate = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "logic_relay"
    "targetname" "relay"
    "parentname" "relay*"
    "OnTrigger" "!self,Trigger,,0,-1"
}
"""
    )

    plan = build_targetname_namespace_plan(source, candidate, prefix="beta__")

    assert plan.status is TargetNameNamespaceStatus.BLOCKED
    assert [(blocker.code, blocker.entity_index, blocker.name) for blocker in plan.blockers] == [
        (TargetNameNamespaceBlockerCode.NAMESPACED_NAME_COLLISION, 1, "beta__relay"),
        (TargetNameNamespaceBlockerCode.WILDCARD_REFERENCE_UNSUPPORTED, 1, "relay*"),
        (TargetNameNamespaceBlockerCode.SPECIAL_REFERENCE_UNSUPPORTED, 1, "!self"),
    ]

    empty_prefix = build_targetname_namespace_plan(source, candidate, prefix="")

    assert empty_prefix.status is TargetNameNamespaceStatus.BLOCKED
    assert empty_prefix.blockers[0].code is TargetNameNamespaceBlockerCode.EMPTY_PREFIX


def test_namespace_plan_includes_conservative_fgd_keyvalue_references() -> None:
    source = _semantic('world\n{\n    "id" "1"\n}\n')
    candidate = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "logic_relay"
    "targetname" "relay"
}
entity
{
    "id" "3"
    "classname" "point_template"
    "targetname" "template"
    "Template01" "relay"
}
entity
{
    "id" "4"
    "classname" "env_entity_maker"
    "EntityTemplate" "template"
}
"""
    )

    plan = build_targetname_namespace_plan(source, candidate, prefix="beta__")

    assert plan.status is TargetNameNamespaceStatus.VALID
    assert [
        (edit.kind, edit.entity_index, edit.original_value, edit.namespaced_value)
        for edit in plan.edits
    ] == [
        (TargetNameNamespaceEditKind.DEFINITION, 1, "relay", "beta__relay"),
        (TargetNameNamespaceEditKind.DEFINITION, 2, "template", "beta__template"),
        (TargetNameNamespaceEditKind.REFERENCE, 2, "relay", "beta__relay"),
        (TargetNameNamespaceEditKind.REFERENCE, 3, "template", "beta__template"),
    ]


def test_singleton_conflict_report_passes_when_world_keys_and_singletons_are_compatible() -> None:
    source = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
    "skyname" "sky_day01_01"
}
entity
{
    "id" "2"
    "classname" "info_target"
}
"""
    )
    candidate = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
    "skyname" "sky_day01_01"
}
entity
{
    "id" "2"
    "classname" "info_target"
}
"""
    )

    report = build_singleton_conflict_report(source, candidate)

    assert report.status is SingletonConflictStatus.CLEAR
    assert report.mutation_authorized is False
    assert report.conflicts == ()


def test_singleton_conflict_report_blocks_world_key_and_singleton_class_conflicts() -> None:
    source = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
    "skyname" "sky_day01_01"
}
entity
{
    "id" "2"
    "classname" "env_fog_controller"
    "targetname" "fog_a"
}
"""
    )
    candidate = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
    "skyname" "sky_day01_02"
}
entity
{
    "id" "2"
    "classname" "env_fog_controller"
    "targetname" "fog_b"
}
"""
    )

    report = build_singleton_conflict_report(source, candidate)

    assert report.status is SingletonConflictStatus.BLOCKED
    assert [conflict.code for conflict in report.conflicts] == [
        SingletonConflictCode.WORLD_KEY_CONFLICT,
        SingletonConflictCode.SINGLETON_CLASS_CONFLICT,
    ]
    assert report.conflicts[0].key == "skyname"
    assert report.conflicts[0].source_value == "sky_day01_01"
    assert report.conflicts[0].candidate_value == "sky_day01_02"
    assert report.conflicts[1].classname == "env_fog_controller"
    assert report.conflicts[1].source_entity_indexes == (1,)
    assert report.conflicts[1].candidate_entity_indexes == (1,)


def test_singleton_conflict_report_detects_candidate_duplicate_singletons() -> None:
    source = _semantic('world\n{\n    "id" "1"\n    "classname" "worldspawn"\n}\n')
    candidate = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "logic_auto"
}
entity
{
    "id" "3"
    "classname" "logic_auto"
}
"""
    )

    report = build_singleton_conflict_report(source, candidate)

    assert report.status is SingletonConflictStatus.BLOCKED
    assert [(conflict.code, conflict.classname) for conflict in report.conflicts] == [
        (SingletonConflictCode.CANDIDATE_DUPLICATE_SINGLETON, "logic_auto")
    ]
    assert report.conflicts[0].candidate_entity_indexes == (1, 2)


def test_stitch_preflight_passes_when_all_evidence_gates_are_clear() -> None:
    source_doc = _document(
        """world
{
    "id" "1"
    "classname" "worldspawn"
    "skyname" "sky_day01_01"
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
    candidate_doc = _document(
        """world
{
    "id" "1"
    "classname" "worldspawn"
    "skyname" "sky_day01_01"
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
    source = build_semantic_document(source_doc)
    candidate = build_semantic_document(candidate_doc)
    alignment = build_translation_alignment_hypothesis(
        build_transition_graph(source),
        build_transition_graph(candidate),
        source_map_name="alpha",
        candidate_map_name="beta",
    )
    source_records = (
        BrushSpatialRecord(
            "alpha/solid",
            _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0)),
        ),
    )
    candidate_records = (
        BrushSpatialRecord(
            "beta/solid",
            _cube_brush(Vec3(128.0, 0.0, 0.0), Vec3(256.0, 128.0, 128.0)),
        ),
    )
    seam_confidence = build_seam_confidence_report(
        build_seam_deletion_evidence(
            build_seam_overlap_evidence(alignment, source_records, candidate_records)
        )
    )
    id_plan = build_import_id_allocation_plan(source, candidate, (), ())
    namespace_plan = build_targetname_namespace_plan(source, candidate, prefix="beta__")
    singleton_report = build_singleton_conflict_report(source, candidate)

    preflight = build_stitch_preflight_report(
        alignment,
        seam_confidence,
        id_plan,
        namespace_plan,
        singleton_report,
        candidate,
        (),
    )

    assert preflight.status is StitchPreflightStatus.READY_FOR_PLAN
    assert preflight.mutation_authorized is False
    assert preflight.blockers == ()
    assert preflight.imported_entity_count == 2
    assert preflight.imported_solid_count == 0
    assert preflight.imported_side_count == 0


def test_stitch_preflight_blocks_failed_evidence_gates() -> None:
    source = _semantic('world\n{\n    "id" "1"\n    "classname" "worldspawn"\n}\n')
    candidate = _semantic('world\n{\n    "id" "1"\n    "classname" "worldspawn"\n}\n')
    alignment = build_translation_alignment_hypothesis(
        build_transition_graph(source),
        build_transition_graph(candidate),
        source_map_name="alpha",
        candidate_map_name="beta",
    )
    seam_confidence = build_seam_confidence_report(
        build_seam_deletion_evidence(build_seam_overlap_evidence(alignment, (), ()))
    )
    id_plan = build_import_id_allocation_plan(source, candidate, (), ())
    namespace_plan = build_targetname_namespace_plan(source, candidate, prefix="beta__")
    singleton_report = build_singleton_conflict_report(source, candidate)

    preflight = build_stitch_preflight_report(
        alignment,
        seam_confidence,
        id_plan,
        namespace_plan,
        singleton_report,
        candidate,
        (),
    )

    assert preflight.status is StitchPreflightStatus.BLOCKED
    assert [blocker.code for blocker in preflight.blockers] == [
        StitchPreflightBlockerCode.ALIGNMENT_BLOCKED,
        StitchPreflightBlockerCode.SEAM_CONFIDENCE_BLOCKED,
    ]


def test_stitch_preflight_blocks_capacity_overages() -> None:
    source = _semantic('world\n{\n    "id" "1"\n    "classname" "worldspawn"\n}\n')
    candidate = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "info_target"
}
"""
    )
    candidate_brushes = (
        extract_brush_sources(
            _document(
                """world
{
    "id" "1"
    "classname" "worldspawn"
    solid
    {
        "id" "2"
        side { "id" "3" "plane" "(0 0 0) (0 1 0) (1 1 0)" }
    }
}
"""
            ).syntax
        )[0],
    )

    preflight = build_stitch_preflight_report(
        _valid_zero_offset_alignment(),
        build_seam_confidence_report(
            build_seam_deletion_evidence(
                build_seam_overlap_evidence(_valid_zero_offset_alignment(), (), ())
            )
        ),
        build_import_id_allocation_plan(source, candidate, (), ()),
        build_targetname_namespace_plan(source, candidate, prefix="beta__"),
        build_singleton_conflict_report(source, candidate),
        candidate,
        candidate_brushes,
        max_imported_entities=0,
        max_imported_solids=0,
        max_imported_sides=0,
    )

    assert preflight.status is StitchPreflightStatus.BLOCKED
    assert [blocker.code for blocker in preflight.blockers] == [
        StitchPreflightBlockerCode.SEAM_CONFIDENCE_BLOCKED,
        StitchPreflightBlockerCode.CAPACITY_EXCEEDED,
        StitchPreflightBlockerCode.CAPACITY_EXCEEDED,
        StitchPreflightBlockerCode.CAPACITY_EXCEEDED,
    ]


def test_builds_read_only_stitch_plan_manifest_from_ready_preflight() -> None:
    source_doc = _document(
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
    candidate_doc = _document(
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
    source = build_semantic_document(source_doc)
    candidate = build_semantic_document(candidate_doc)
    alignment = build_translation_alignment_hypothesis(
        build_transition_graph(source),
        build_transition_graph(candidate),
        source_map_name="alpha",
        candidate_map_name="beta",
    )
    source_records = (
        BrushSpatialRecord(
            "alpha/solid",
            _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0)),
        ),
    )
    candidate_records = (
        BrushSpatialRecord(
            "beta/duplicate",
            _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0)),
        ),
    )
    deletion = build_seam_deletion_evidence(
        build_seam_overlap_evidence(alignment, source_records, candidate_records)
    )
    seam_confidence = build_seam_confidence_report(deletion)
    id_plan = build_import_id_allocation_plan(source, candidate, (), ())
    namespace_plan = build_targetname_namespace_plan(source, candidate, prefix="beta__")
    singleton_report = build_singleton_conflict_report(source, candidate)
    preflight = build_stitch_preflight_report(
        alignment,
        seam_confidence,
        id_plan,
        namespace_plan,
        singleton_report,
        candidate,
        (),
    )

    manifest = build_stitch_plan_manifest(
        preflight,
        alignment,
        deletion,
        id_plan,
        namespace_plan,
    )

    assert manifest.status is StitchPlanManifestStatus.VALID
    assert manifest.mutation_authorized is False
    assert manifest.blockers == ()
    assert manifest.candidate_to_source_offset == Vec3(0.0, 0.0, 0.0)
    assert manifest.candidate_removals == ("beta/duplicate",)
    assert manifest.id_allocations == id_plan.allocations
    assert manifest.namespace_edits == namespace_plan.edits


def test_stitch_plan_manifest_blocks_when_preflight_is_blocked() -> None:
    alignment = _valid_zero_offset_alignment()
    deletion = build_seam_deletion_evidence(build_seam_overlap_evidence(alignment, (), ()))
    source = _semantic('world\n{\n    "id" "1"\n    "classname" "worldspawn"\n}\n')
    candidate = _semantic('world\n{\n    "id" "1"\n    "classname" "worldspawn"\n}\n')
    id_plan = build_import_id_allocation_plan(source, candidate, (), ())
    namespace_plan = build_targetname_namespace_plan(source, candidate, prefix="beta__")
    singleton_report = build_singleton_conflict_report(source, candidate)
    preflight = build_stitch_preflight_report(
        alignment,
        build_seam_confidence_report(deletion),
        id_plan,
        namespace_plan,
        singleton_report,
        candidate,
        (),
    )

    manifest = build_stitch_plan_manifest(
        preflight,
        alignment,
        deletion,
        id_plan,
        namespace_plan,
    )

    assert manifest.status is StitchPlanManifestStatus.BLOCKED
    assert manifest.candidate_to_source_offset is None
    assert [blocker.code for blocker in manifest.blockers] == [
        StitchPlanManifestBlockerCode.PREFLIGHT_BLOCKED
    ]


def test_synthetic_transition_fixture_pair_produces_ready_stitch_manifest() -> None:
    fixture_dir = Path("tests/fixtures/stitching")
    source_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_alpha.vmf").read_bytes(),
        path=fixture_dir / "transition_alpha.vmf",
    )
    candidate_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_beta.vmf").read_bytes(),
        path=fixture_dir / "transition_beta.vmf",
    )
    source = build_semantic_document(source_doc)
    candidate = build_semantic_document(candidate_doc)
    source_brushes = extract_brush_sources(source_doc.syntax)
    candidate_brushes = extract_brush_sources(candidate_doc.syntax)
    source_records = _brush_records_from_sources("transition_alpha", source_brushes)
    candidate_records = _brush_records_from_sources("transition_beta", candidate_brushes)
    alignment = build_translation_alignment_hypothesis(
        build_transition_graph(source),
        build_transition_graph(candidate),
        source_map_name="transition_alpha",
        candidate_map_name="transition_beta",
    )
    deletion = build_seam_deletion_evidence(
        build_seam_overlap_evidence(alignment, source_records, candidate_records)
    )
    seam_confidence = build_seam_confidence_report(deletion)
    id_plan = build_import_id_allocation_plan(source, candidate, source_brushes, candidate_brushes)
    namespace_plan = build_targetname_namespace_plan(source, candidate, prefix="beta__")
    singleton_report = build_singleton_conflict_report(source, candidate)
    preflight = build_stitch_preflight_report(
        alignment,
        seam_confidence,
        id_plan,
        namespace_plan,
        singleton_report,
        candidate,
        candidate_brushes,
    )

    manifest = build_stitch_plan_manifest(
        preflight,
        alignment,
        deletion,
        id_plan,
        namespace_plan,
    )

    assert alignment.status is AlignmentStatus.VALID
    assert seam_confidence.status is SeamConfidenceStatus.READY_FOR_REVIEW
    assert preflight.status is StitchPreflightStatus.READY_FOR_PLAN
    assert manifest.status is StitchPlanManifestStatus.VALID
    assert manifest.candidate_to_source_offset == Vec3(0.0, 0.0, 0.0)
    assert manifest.candidate_removals == ("transition_beta/world/1/solid/10",)
    assert len(manifest.id_allocations) == 9
    assert len(manifest.namespace_edits) == 2


def test_materializes_synthetic_transition_fixture_as_reversible_generated_vmf() -> None:
    fixture_dir = Path("tests/fixtures/stitching")
    source_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_alpha.vmf").read_bytes(),
        path=fixture_dir / "transition_alpha.vmf",
    )
    candidate_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_beta.vmf").read_bytes(),
        path=fixture_dir / "transition_beta.vmf",
    )
    manifest = _synthetic_fixture_manifest(source_doc, candidate_doc)

    materialized = materialize_stitch_from_manifest(source_doc, candidate_doc, manifest)

    assert materialized.status is StitchMaterializationStatus.VALID
    assert materialized.source_mutation_authorized is False
    assert materialized.blockers == ()
    assert materialized.output_bytes is not None
    assert materialized.output_bytes.startswith(source_doc.raw_bytes)
    assert b'"id" "29"' in materialized.output_bytes
    assert b'"targetname" "beta__shared_landmark"' in materialized.output_bytes
    assert b'"targetname" "beta__to_alpha"' in materialized.output_bytes
    assert b'"id" "10"' not in materialized.output_bytes.removeprefix(source_doc.raw_bytes)
    reparsed = VmfDocument.from_bytes(materialized.output_bytes, path=Path("stitched.vmf"))
    assert reparsed.render_bytes() == materialized.output_bytes
    assert [entry.kind for entry in materialized.provenance] == [
        "source_preserved",
        "candidate_entity_imported",
        "candidate_entity_imported",
    ]
    assert materialized.provenance[0].source_path == "tests/fixtures/stitching/transition_alpha.vmf"


def test_materializes_lifecycle_controller_relays_with_generated_provenance() -> None:
    fixture_dir = Path("tests/fixtures/stitching")
    source_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_alpha.vmf").read_bytes(),
        path=fixture_dir / "transition_alpha.vmf",
    )
    candidate_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_beta.vmf").read_bytes(),
        path=fixture_dir / "transition_beta.vmf",
    )
    manifest = _synthetic_fixture_manifest(source_doc, candidate_doc)
    controller_entities = _synthetic_controller_entity_plan()

    materialized = materialize_stitch_from_manifest(
        source_doc,
        candidate_doc,
        manifest,
        controller_entities=controller_entities,
    )

    assert materialized.status is StitchMaterializationStatus.VALID
    assert materialized.output_bytes is not None
    assert b'"id" "200"' in materialized.output_bytes
    assert b'"classname" "logic_relay"' in materialized.output_bytes
    assert b'"targetname" "sourceweaver_transition_beta_preload"' in materialized.output_bytes
    assert (
        VmfDocument.from_bytes(materialized.output_bytes, path=Path("stitched.vmf")).render_bytes()
        == materialized.output_bytes
    )
    assert [entry.kind for entry in materialized.provenance] == [
        "source_preserved",
        "candidate_entity_imported",
        "candidate_entity_imported",
        "lifecycle_controller_entity_generated",
        "lifecycle_controller_entity_generated",
        "lifecycle_controller_entity_generated",
        "lifecycle_controller_entity_generated",
        "lifecycle_controller_entity_generated",
    ]
    assert all(entry.source_span is None for entry in materialized.provenance[3:])


def test_materialization_blocks_blocked_lifecycle_controller_entity_plan() -> None:
    fixture_dir = Path("tests/fixtures/stitching")
    source_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_alpha.vmf").read_bytes(),
        path=fixture_dir / "transition_alpha.vmf",
    )
    candidate_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_beta.vmf").read_bytes(),
        path=fixture_dir / "transition_beta.vmf",
    )
    manifest = _synthetic_fixture_manifest(source_doc, candidate_doc)
    blocked_entities = build_lifecycle_controller_entity_plan(
        build_lifecycle_controller_plan(
            build_lifecycle_policy_matrix(_semantic('world\n{\n    "id" "1"\n}\n')),
            region_name="",
        ),
        first_entity_id=0,
    )

    materialized = materialize_stitch_from_manifest(
        source_doc,
        candidate_doc,
        manifest,
        controller_entities=blocked_entities,
    )

    assert blocked_entities.status is LifecycleControllerEntityStatus.BLOCKED
    assert materialized.status is StitchMaterializationStatus.BLOCKED
    assert materialized.output_bytes is None
    assert [blocker.code for blocker in materialized.blockers] == [
        StitchMaterializationBlockerCode.CONTROLLER_ENTITY_PLAN_BLOCKED
    ]


def test_materializes_lifecycle_controller_outputs_on_generated_relays() -> None:
    fixture_dir = Path("tests/fixtures/stitching")
    source_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_alpha.vmf").read_bytes(),
        path=fixture_dir / "transition_alpha.vmf",
    )
    candidate_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_beta.vmf").read_bytes(),
        path=fixture_dir / "transition_beta.vmf",
    )
    manifest = _synthetic_fixture_manifest(source_doc, candidate_doc)
    controller_entities = _targetnamed_controller_entity_plan()
    controller_outputs = build_lifecycle_controller_output_plan(controller_entities)

    materialized = materialize_stitch_from_manifest(
        source_doc,
        candidate_doc,
        manifest,
        controller_entities=controller_entities,
        controller_outputs=controller_outputs,
    )

    assert materialized.status is StitchMaterializationStatus.VALID
    assert materialized.output_bytes is not None
    assert b'"OnTrigger" "auto_controller,FireUser1,' in materialized.output_bytes
    assert b'"OnTrigger" "entry_trigger,FireUser1,' in materialized.output_bytes
    assert (
        VmfDocument.from_bytes(materialized.output_bytes, path=Path("stitched.vmf")).render_bytes()
        == materialized.output_bytes
    )


def test_materialization_blocks_blocked_lifecycle_controller_output_plan() -> None:
    fixture_dir = Path("tests/fixtures/stitching")
    source_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_alpha.vmf").read_bytes(),
        path=fixture_dir / "transition_alpha.vmf",
    )
    candidate_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_beta.vmf").read_bytes(),
        path=fixture_dir / "transition_beta.vmf",
    )
    manifest = _synthetic_fixture_manifest(source_doc, candidate_doc)
    controller_entities = _synthetic_controller_entity_plan()
    controller_outputs = build_lifecycle_controller_output_plan(controller_entities)

    materialized = materialize_stitch_from_manifest(
        source_doc,
        candidate_doc,
        manifest,
        controller_entities=controller_entities,
        controller_outputs=controller_outputs,
    )

    assert materialized.status is StitchMaterializationStatus.BLOCKED
    assert materialized.output_bytes is None
    assert [blocker.code for blocker in materialized.blockers] == [
        StitchMaterializationBlockerCode.CONTROLLER_OUTPUT_PLAN_BLOCKED
    ]


def test_materialization_blocks_invalid_manifest() -> None:
    source_doc = _document('world\n{\n    "id" "1"\n    "classname" "worldspawn"\n}\n')
    candidate_doc = _document('world\n{\n    "id" "1"\n    "classname" "worldspawn"\n}\n')
    alignment = build_translation_alignment_hypothesis(
        build_transition_graph(build_semantic_document(source_doc)),
        build_transition_graph(build_semantic_document(candidate_doc)),
        source_map_name="alpha",
        candidate_map_name="beta",
    )
    deletion = build_seam_deletion_evidence(build_seam_overlap_evidence(alignment, (), ()))
    manifest = build_stitch_plan_manifest(
        build_stitch_preflight_report(
            alignment,
            build_seam_confidence_report(deletion),
            build_import_id_allocation_plan(
                build_semantic_document(source_doc), build_semantic_document(candidate_doc), (), ()
            ),
            build_targetname_namespace_plan(
                build_semantic_document(source_doc),
                build_semantic_document(candidate_doc),
                prefix="beta__",
            ),
            build_singleton_conflict_report(
                build_semantic_document(source_doc), build_semantic_document(candidate_doc)
            ),
            build_semantic_document(candidate_doc),
            (),
        ),
        alignment,
        deletion,
        build_import_id_allocation_plan(
            build_semantic_document(source_doc), build_semantic_document(candidate_doc), (), ()
        ),
        build_targetname_namespace_plan(
            build_semantic_document(source_doc),
            build_semantic_document(candidate_doc),
            prefix="beta__",
        ),
    )

    materialized = materialize_stitch_from_manifest(source_doc, candidate_doc, manifest)

    assert materialized.status is StitchMaterializationStatus.BLOCKED
    assert materialized.output_bytes is None
    assert [blocker.code for blocker in materialized.blockers] == [
        StitchMaterializationBlockerCode.MANIFEST_BLOCKED
    ]


def test_authorizes_candidate_duplicate_removal_when_all_authority_gates_pass() -> None:
    fixture_dir = Path("tests/fixtures/stitching")
    source_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_alpha.vmf").read_bytes(),
        path=fixture_dir / "transition_alpha.vmf",
    )
    candidate_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_beta.vmf").read_bytes(),
        path=fixture_dir / "transition_beta.vmf",
    )
    manifest = _synthetic_fixture_manifest(source_doc, candidate_doc)
    deletion = _synthetic_fixture_deletion_evidence(source_doc, candidate_doc)
    seam_confidence = build_seam_confidence_report(deletion)

    authority = build_stitch_removal_authority_report(
        manifest,
        deletion,
        seam_confidence,
        _ready_compiler_preflight(),
        material_equivalent_candidate_keys=("transition_beta/world/1/solid/10",),
        runtime_acceptance_passed=True,
    )

    assert authority.status is StitchRemovalAuthorityStatus.AUTHORIZED
    assert authority.mutation_authorized is True
    assert authority.candidate_removals == ("transition_beta/world/1/solid/10",)
    assert authority.blockers == ()


def test_removal_authority_blocks_without_material_compiler_and_runtime_gates() -> None:
    fixture_dir = Path("tests/fixtures/stitching")
    source_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_alpha.vmf").read_bytes(),
        path=fixture_dir / "transition_alpha.vmf",
    )
    candidate_doc = VmfDocument.from_bytes(
        (fixture_dir / "transition_beta.vmf").read_bytes(),
        path=fixture_dir / "transition_beta.vmf",
    )
    manifest = _synthetic_fixture_manifest(source_doc, candidate_doc)
    deletion = _synthetic_fixture_deletion_evidence(source_doc, candidate_doc)
    seam_confidence = build_seam_confidence_report(deletion)

    authority = build_stitch_removal_authority_report(
        manifest,
        deletion,
        seam_confidence,
        CompilerRunPreflight(
            status=CompilerRunStatus.BLOCKED,
            tools=(),
            blockers=(),
        ),
        material_equivalent_candidate_keys=(),
        runtime_acceptance_passed=False,
    )

    assert authority.status is StitchRemovalAuthorityStatus.BLOCKED
    assert authority.mutation_authorized is False
    assert authority.candidate_removals == ()
    assert [blocker.code for blocker in authority.blockers] == [
        StitchRemovalAuthorityBlockerCode.MATERIAL_EQUIVALENCE_MISSING,
        StitchRemovalAuthorityBlockerCode.COMPILER_PREFLIGHT_BLOCKED,
        StitchRemovalAuthorityBlockerCode.RUNTIME_ACCEPTANCE_MISSING,
    ]


def test_removal_authority_blocks_unsafe_deletion_classes_even_if_manifest_lists_key() -> None:
    alignment = _valid_zero_offset_alignment()
    deletion = build_seam_deletion_evidence(
        build_seam_overlap_evidence(
            alignment,
            (
                BrushSpatialRecord(
                    "alpha/source",
                    _cube_brush(Vec3(32.0, 32.0, 32.0), Vec3(96.0, 96.0, 96.0)),
                ),
            ),
            (
                BrushSpatialRecord(
                    "beta/outer",
                    _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0)),
                ),
            ),
        )
    )
    manifest = StitchPlanManifest(
        status=StitchPlanManifestStatus.VALID,
        candidate_to_source_offset=Vec3(0.0, 0.0, 0.0),
        candidate_removals=("beta/outer",),
        id_allocations=(),
        namespace_edits=(),
        blockers=(),
    )

    authority = build_stitch_removal_authority_report(
        manifest,
        deletion,
        build_seam_confidence_report(deletion),
        _ready_compiler_preflight(),
        material_equivalent_candidate_keys=("beta/outer",),
        runtime_acceptance_passed=True,
    )

    assert authority.status is StitchRemovalAuthorityStatus.BLOCKED
    assert [blocker.code for blocker in authority.blockers] == [
        StitchRemovalAuthorityBlockerCode.SEAM_CONFIDENCE_BLOCKED,
        StitchRemovalAuthorityBlockerCode.UNSAFE_REMOVAL_CLASS,
    ]


def _synthetic_fixture_manifest(
    source_doc: VmfDocument, candidate_doc: VmfDocument
) -> StitchPlanManifest:
    source = build_semantic_document(source_doc)
    candidate = build_semantic_document(candidate_doc)
    source_brushes = extract_brush_sources(source_doc.syntax)
    candidate_brushes = extract_brush_sources(candidate_doc.syntax)
    alignment = build_translation_alignment_hypothesis(
        build_transition_graph(source),
        build_transition_graph(candidate),
        source_map_name="transition_alpha",
        candidate_map_name="transition_beta",
    )
    deletion = build_seam_deletion_evidence(
        build_seam_overlap_evidence(
            alignment,
            _brush_records_from_sources("transition_alpha", source_brushes),
            _brush_records_from_sources("transition_beta", candidate_brushes),
        )
    )
    seam_confidence = build_seam_confidence_report(deletion)
    id_plan = build_import_id_allocation_plan(source, candidate, source_brushes, candidate_brushes)
    namespace_plan = build_targetname_namespace_plan(source, candidate, prefix="beta__")
    preflight = build_stitch_preflight_report(
        alignment,
        seam_confidence,
        id_plan,
        namespace_plan,
        build_singleton_conflict_report(source, candidate),
        candidate,
        candidate_brushes,
    )
    return build_stitch_plan_manifest(
        preflight,
        alignment,
        deletion,
        id_plan,
        namespace_plan,
    )


def _synthetic_fixture_deletion_evidence(
    source_doc: VmfDocument, candidate_doc: VmfDocument
) -> SeamDeletionEvidence:
    source = build_semantic_document(source_doc)
    candidate = build_semantic_document(candidate_doc)
    source_brushes = extract_brush_sources(source_doc.syntax)
    candidate_brushes = extract_brush_sources(candidate_doc.syntax)
    alignment = build_translation_alignment_hypothesis(
        build_transition_graph(source),
        build_transition_graph(candidate),
        source_map_name="transition_alpha",
        candidate_map_name="transition_beta",
    )
    return build_seam_deletion_evidence(
        build_seam_overlap_evidence(
            alignment,
            _brush_records_from_sources("transition_alpha", source_brushes),
            _brush_records_from_sources("transition_beta", candidate_brushes),
        )
    )


def _ready_compiler_preflight() -> CompilerRunPreflight:
    return CompilerRunPreflight(
        status=CompilerRunStatus.READY,
        tools=(),
        blockers=(),
    )


def _synthetic_controller_entity_plan() -> LifecycleControllerEntityPlan:
    semantic = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "logic_auto"
}
"""
    )
    controller_plan = build_lifecycle_controller_plan(
        build_lifecycle_policy_matrix(semantic),
        region_name="transition_beta",
    )
    return build_lifecycle_controller_entity_plan(
        controller_plan,
        first_entity_id=200,
    )


def _targetnamed_controller_entity_plan() -> LifecycleControllerEntityPlan:
    semantic = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "logic_auto"
    "targetname" "auto_controller"
}
entity
{
    "id" "3"
    "classname" "trigger_once"
    "targetname" "entry_trigger"
}
"""
    )
    controller_plan = build_lifecycle_controller_plan(
        build_lifecycle_policy_matrix(semantic),
        region_name="transition_beta",
    )
    return build_lifecycle_controller_entity_plan(
        controller_plan,
        first_entity_id=200,
    )


def _brush_records_from_sources(
    map_name: str, brush_sources: tuple[BrushSource, ...]
) -> tuple[BrushSpatialRecord, ...]:
    records: list[BrushSpatialRecord] = []
    for brush_source in brush_sources:
        reconstruction = brush_source.reconstruct()
        assert reconstruction.status is ReconstructionStatus.VALID
        assert reconstruction.brush is not None
        records.append(
            BrushSpatialRecord(
                f"{map_name}/{brush_source.owner_kind}/{brush_source.owner_id}/solid/{brush_source.solid_id}",
                reconstruction.brush,
            )
        )
    return tuple(records)
