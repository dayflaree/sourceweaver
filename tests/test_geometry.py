import math

import pytest

from sourceweaver.geometry import (
    GeometryTolerances,
    Plane,
    PlaneParseError,
    ReconstructionStatus,
    extract_brush_sources,
    reconstruct_convex_brush,
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
