import math

import pytest

from sourceweaver.geometry import (
    BoundsRelation,
    BrushRelation,
    BrushSpatialRecord,
    ConvexBrush,
    GeometryTolerances,
    GeometryTransformStatus,
    Plane,
    PlaneParseError,
    ReconstructionStatus,
    Vec3,
    classify_bounds_relation,
    classify_brush_relation,
    extract_brush_sources,
    find_potential_brush_intersections,
    reconstruct_convex_brush,
    translate_convex_brush_for_analysis,
)
from sourceweaver.vmf.document import VmfDocument

CUBE_PLANES = (
    "(0 0 0) (0 1 0) (1 1 0)",
    "(0 0 128) (128 0 128) (128 128 128)",
    "(0 0 0) (128 0 0) (128 0 128)",
    "(0 128 0) (0 128 128) (128 128 128)",
    "(0 0 0) (0 0 128) (0 128 128)",
    "(128 0 0) (128 128 0) (128 128 128)",
)


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


def test_vmf_plane_parser_preserves_numeric_source() -> None:
    plane = Plane.from_vmf("(0 0 0) (+128.0 -0.5 .25) (128 128 0)")

    assert plane.points[1].x.raw == "+128.0"
    assert plane.points[1].y.raw == "-0.5"
    assert plane.points[1].z.raw == ".25"
    assert math.isfinite(plane.normal.x)
    assert plane.distance == pytest.approx(0.0)


@pytest.mark.parametrize(
    "raw",
    [
        "(0 0 0) (1 0 0)",
        "(0 0 0) (1 0 0) (2 0 0)",
        "(0 0 nan) (1 0 0) (0 1 0)",
    ],
)
def test_invalid_vmf_planes_are_blocked(raw: str) -> None:
    with pytest.raises(PlaneParseError):
        Plane.from_vmf(raw)


def test_reconstructs_axis_aligned_cube_from_vmf_side_planes() -> None:
    planes = tuple(Plane.from_vmf(raw) for raw in CUBE_PLANES)

    result = reconstruct_convex_brush(planes, side_ids=tuple(str(i) for i in range(6)))

    assert result.status is ReconstructionStatus.VALID
    assert result.brush is not None
    assert len(result.brush.vertices) == 8
    assert len(result.brush.faces) == 6
    assert result.brush.volume == pytest.approx(128**3)
    assert {round(face.area, 6) for face in result.brush.faces} == {float(128**2)}
    assert result.brush.bounds_min.as_tuple() == (0.0, 0.0, 0.0)
    assert result.brush.bounds_max.as_tuple() == (128.0, 128.0, 128.0)


def test_open_brush_is_invalid_instead_of_guessed() -> None:
    planes = tuple(Plane.from_vmf(raw) for raw in CUBE_PLANES[:-1])

    result = reconstruct_convex_brush(planes)

    assert result.status is ReconstructionStatus.INVALID
    assert result.brush is None
    assert any(blocker.code == "BRUSH_UNBOUNDED_OR_OPEN" for blocker in result.blockers)


def test_minimum_feature_tolerances_block_sliver_brushes() -> None:
    planes = tuple(
        Plane.from_vmf(raw)
        for raw in (
            "(0 0 0) (0 1 0) (1 1 0)",
            "(0 0 0.001) (1 0 0.001) (1 1 0.001)",
            "(0 0 0) (1 0 0) (1 0 0.001)",
            "(0 1 0) (0 1 0.001) (1 1 0.001)",
            "(0 0 0) (0 0 0.001) (0 1 0.001)",
            "(1 0 0) (1 1 0) (1 1 0.001)",
        )
    )

    result = reconstruct_convex_brush(planes, tolerances=GeometryTolerances(min_edge_length=0.01))

    assert result.status is ReconstructionStatus.INVALID
    assert any(blocker.code == "BRUSH_EDGE_TOO_SHORT" for blocker in result.blockers)


def test_extracts_solid_sources_from_lossless_cst_spans() -> None:
    text = """world
{
    "id" "1"
    "classname" "worldspawn"
    solid
    {
        "id" "20"
        side { "id" "1" "plane" "(0 0 0) (0 1 0) (1 1 0)" }
        side { "id" "2" "plane" "(0 0 128) (128 0 128) (128 128 128)" }
        side { "id" "3" "plane" "(0 0 0) (128 0 0) (128 0 128)" }
        side { "id" "4" "plane" "(0 128 0) (0 128 128) (128 128 128)" }
        side { "id" "5" "plane" "(0 0 0) (0 0 128) (0 128 128)" }
        side { "id" "6" "plane" "(128 0 0) (128 128 0) (128 128 128)" }
    }
}
"""
    document = VmfDocument.from_bytes(text.encode("utf-8"))

    solids = extract_brush_sources(document.syntax)

    assert len(solids) == 1
    assert solids[0].solid_id == "20"
    assert [side.side_id for side in solids[0].sides] == ["1", "2", "3", "4", "5", "6"]
    assert all(
        text[side.plane_span.start : side.plane_span.end].startswith('"(')
        for side in solids[0].sides
    )
    assert solids[0].reconstruct().status is ReconstructionStatus.VALID


@pytest.mark.parametrize(
    ("other_min", "other_max", "expected"),
    [
        (Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0), BrushRelation.EQUAL_VOLUME),
        (Vec3(32.0, 32.0, 32.0), Vec3(96.0, 96.0, 96.0), BrushRelation.A_CONTAINS_B),
        (Vec3(128.0, 0.0, 0.0), Vec3(256.0, 128.0, 128.0), BrushRelation.TOUCHING),
        (Vec3(64.0, 0.0, 0.0), Vec3(192.0, 128.0, 128.0), BrushRelation.OVERLAPPING),
        (Vec3(256.0, 0.0, 0.0), Vec3(384.0, 128.0, 128.0), BrushRelation.DISJOINT),
    ],
)
def test_classifies_convex_brush_relations(
    other_min: Vec3, other_max: Vec3, expected: BrushRelation
) -> None:
    base = _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0))
    other = _cube_brush(other_min, other_max)

    assert classify_brush_relation(base, other) is expected


def test_classifies_reversed_containment() -> None:
    base = _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0))
    inner = _cube_brush(Vec3(32.0, 32.0, 32.0), Vec3(96.0, 96.0, 96.0))

    assert classify_brush_relation(inner, base) is BrushRelation.B_CONTAINS_A


def test_translates_convex_brush_for_analysis_without_mutation_authority() -> None:
    brush = _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0))
    offset = Vec3(512.0, -64.0, 32.0)

    result = translate_convex_brush_for_analysis(brush, offset)

    assert result.status is GeometryTransformStatus.VALID
    assert result.brush is not None
    assert result.mutation_authorized is False
    assert result.operation == "translation"
    assert result.brush.volume == pytest.approx(brush.volume)
    assert result.brush.bounds_min.as_tuple() == pytest.approx((512.0, -64.0, 32.0))
    assert result.brush.bounds_max.as_tuple() == pytest.approx((640.0, 64.0, 160.0))
    assert result.brush.vertices == tuple(vertex + offset for vertex in brush.vertices)
    for original, translated in zip(brush.faces, result.brush.faces, strict=True):
        assert translated.plane.raw.startswith("<generated:analysis-translation")
        assert translated.plane.normal.as_tuple() == pytest.approx(original.plane.normal.as_tuple())
        assert translated.plane.distance == pytest.approx(
            original.plane.distance + original.plane.normal.dot(offset)
        )
        assert translated.area == pytest.approx(original.area)
        assert translated.vertices == tuple(vertex + offset for vertex in original.vertices)


def test_nonfinite_translation_offset_is_blocked() -> None:
    brush = _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0))

    result = translate_convex_brush_for_analysis(brush, Vec3(math.inf, 0.0, 0.0))

    assert result.status is GeometryTransformStatus.INVALID
    assert result.brush is None
    assert any(blocker.code == "TRANSFORM_NONFINITE_OFFSET" for blocker in result.blockers)


def test_translation_world_bound_violation_is_blocked() -> None:
    brush = _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0))

    result = translate_convex_brush_for_analysis(brush, Vec3(70_000.0, 0.0, 0.0))

    assert result.status is GeometryTransformStatus.INVALID
    assert result.brush is None
    assert any(blocker.code == "BRUSH_WORLD_BOUNDS_EXCEEDED" for blocker in result.blockers)


def test_translated_brush_can_feed_relation_classification() -> None:
    source = _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0))
    candidate = _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0))

    transformed = translate_convex_brush_for_analysis(candidate, Vec3(128.0, 0.0, 0.0))

    assert transformed.status is GeometryTransformStatus.VALID
    assert transformed.brush is not None
    assert classify_brush_relation(source, transformed.brush) is BrushRelation.TOUCHING


@pytest.mark.parametrize(
    ("other_min", "other_max", "expected"),
    [
        (Vec3(128.0, 0.0, 0.0), Vec3(256.0, 128.0, 128.0), BoundsRelation.TOUCHING),
        (Vec3(64.0, 0.0, 0.0), Vec3(192.0, 128.0, 128.0), BoundsRelation.OVERLAPPING),
        (Vec3(256.0, 0.0, 0.0), Vec3(384.0, 128.0, 128.0), BoundsRelation.DISJOINT),
    ],
)
def test_classifies_axis_aligned_bounds_relation(
    other_min: Vec3, other_max: Vec3, expected: BoundsRelation
) -> None:
    base = _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0))
    other = _cube_brush(other_min, other_max)

    assert classify_bounds_relation(base, other) is expected


def test_bounds_relation_supports_expansion_for_seam_neighborhoods() -> None:
    base = _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0))
    nearby = _cube_brush(Vec3(144.0, 0.0, 0.0), Vec3(272.0, 128.0, 128.0))

    assert classify_bounds_relation(base, nearby) is BoundsRelation.DISJOINT
    assert classify_bounds_relation(base, nearby, expansion=8.0) is BoundsRelation.TOUCHING


def test_find_potential_brush_intersections_returns_deterministic_candidate_pairs() -> None:
    a_records = (
        BrushSpatialRecord(
            "a/second", _cube_brush(Vec3(256.0, 0.0, 0.0), Vec3(384.0, 128.0, 128.0))
        ),
        BrushSpatialRecord("a/first", _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0))),
    )
    b_records = (
        BrushSpatialRecord(
            "b/touching", _cube_brush(Vec3(128.0, 0.0, 0.0), Vec3(256.0, 128.0, 128.0))
        ),
        BrushSpatialRecord("b/far", _cube_brush(Vec3(512.0, 0.0, 0.0), Vec3(640.0, 128.0, 128.0))),
    )

    candidates = find_potential_brush_intersections(a_records, b_records)

    assert [
        (candidate.a_key, candidate.b_key, candidate.bounds_relation) for candidate in candidates
    ] == [
        ("a/first", "b/touching", BoundsRelation.TOUCHING),
        ("a/second", "b/touching", BoundsRelation.TOUCHING),
    ]


def test_potential_intersection_expansion_includes_nearby_pairs() -> None:
    a_records = (
        BrushSpatialRecord("a", _cube_brush(Vec3(0.0, 0.0, 0.0), Vec3(128.0, 128.0, 128.0))),
    )
    b_records = (
        BrushSpatialRecord("b", _cube_brush(Vec3(144.0, 0.0, 0.0), Vec3(272.0, 128.0, 128.0))),
    )

    assert find_potential_brush_intersections(a_records, b_records) == ()
    assert (
        find_potential_brush_intersections(a_records, b_records, expansion=8.0)[0].bounds_relation
        is BoundsRelation.TOUCHING
    )
