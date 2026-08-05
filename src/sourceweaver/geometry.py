"""Read-only Source brush geometry primitives.

The kernel in this module reconstructs convex brush geometry from VMF side
planes while retaining source numeric provenance. It produces validation results
only; it does not authorize VMF rewrites or generated brush materialization.
"""

from __future__ import annotations

import math
import re
from collections.abc import Iterable
from dataclasses import dataclass
from enum import StrEnum
from itertools import combinations
from typing import Final

from sourceweaver.semantics import SourceSpan
from sourceweaver.vmf.parser import BlockNode, PairNode, ParsedVmf

_NUMBER_RE: Final[str] = r"[+-]?(?:(?:\d+(?:\.\d*)?)|(?:\.\d+))(?:[eE][+-]?\d+)?"
_POINT_RE: Final[re.Pattern[str]] = re.compile(
    rf"\(\s*({_NUMBER_RE})\s+({_NUMBER_RE})\s+({_NUMBER_RE})\s*\)"
)
_PLANE_RE: Final[re.Pattern[str]] = re.compile(
    rf"^\s*{_POINT_RE.pattern}\s+{_POINT_RE.pattern}\s+{_POINT_RE.pattern}\s*$"
)


class PlaneParseError(ValueError):
    """Raised when a VMF side plane cannot be parsed safely."""


class ReconstructionStatus(StrEnum):
    """Final convex-brush reconstruction status."""

    VALID = "valid"
    INVALID = "invalid"


class PointPlaneRelation(StrEnum):
    """Tolerance-aware relation between a point and an outward plane."""

    INSIDE = "inside"
    ON = "on"
    OUTSIDE = "outside"


class BrushRelation(StrEnum):
    """Conservative relation between two validated convex brushes."""

    EQUAL_VOLUME = "equal_volume"
    A_CONTAINS_B = "a_contains_b"
    B_CONTAINS_A = "b_contains_a"
    TOUCHING = "touching"
    OVERLAPPING = "overlapping"
    DISJOINT = "disjoint"


class BoundsRelation(StrEnum):
    """Conservative relation between two axis-aligned brush bounds."""

    TOUCHING = "touching"
    OVERLAPPING = "overlapping"
    DISJOINT = "disjoint"


class GeometryTransformStatus(StrEnum):
    """Final status for an analysis-only geometry transform."""

    VALID = "valid"
    INVALID = "invalid"


@dataclass(frozen=True, slots=True)
class NumericValue:
    """A parsed numeric coordinate with exact source spelling retained."""

    raw: str
    value: float

    @classmethod
    def parse(cls, raw: str) -> NumericValue:
        value = float(raw)
        if not math.isfinite(value):
            raise PlaneParseError(f"Plane coordinate is not finite: {raw!r}")
        return cls(raw=raw, value=value)


@dataclass(frozen=True, slots=True)
class Vec3:
    """Three-dimensional vector used for exact-authority geometry checks."""

    x: float
    y: float
    z: float

    def __add__(self, other: Vec3) -> Vec3:
        return Vec3(self.x + other.x, self.y + other.y, self.z + other.z)

    def __sub__(self, other: Vec3) -> Vec3:
        return Vec3(self.x - other.x, self.y - other.y, self.z - other.z)

    def scale(self, scalar: float) -> Vec3:
        return Vec3(self.x * scalar, self.y * scalar, self.z * scalar)

    def dot(self, other: Vec3) -> float:
        return self.x * other.x + self.y * other.y + self.z * other.z

    def cross(self, other: Vec3) -> Vec3:
        return Vec3(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )

    def length(self) -> float:
        return math.sqrt(self.dot(self))

    def normalized(self) -> Vec3:
        length = self.length()
        if length == 0.0:
            raise PlaneParseError("Cannot normalize a zero-length vector")
        return self.scale(1.0 / length)

    def distance_to(self, other: Vec3) -> float:
        return (self - other).length()

    def as_tuple(self) -> tuple[float, float, float]:
        return (self.x, self.y, self.z)


@dataclass(frozen=True, slots=True)
class PlanePoint:
    """A point from a VMF plane string, retaining coordinate spellings."""

    x: NumericValue
    y: NumericValue
    z: NumericValue

    @classmethod
    def from_strings(cls, x: str, y: str, z: str) -> PlanePoint:
        return cls(NumericValue.parse(x), NumericValue.parse(y), NumericValue.parse(z))

    @property
    def vector(self) -> Vec3:
        return Vec3(self.x.value, self.y.value, self.z.value)


@dataclass(frozen=True, slots=True)
class Plane:
    """A normalized VMF side plane with outward normal and source points."""

    normal: Vec3
    distance: float
    points: tuple[PlanePoint, PlanePoint, PlanePoint]
    raw: str

    @classmethod
    def from_vmf(cls, raw: str, *, degeneracy_epsilon: float = 1e-9) -> Plane:
        """Parse one VMF side plane string into a normalized plane.

        VMF side planes are three points. Source uses outward normals with brush
        interior on the ``normal·point <= distance`` side; reconstruction keeps
        that convention and blocks invalid solids instead of flipping planes.
        """
        match = _PLANE_RE.fullmatch(raw)
        if match is None:
            raise PlaneParseError(f"Invalid VMF plane format: {raw!r}")

        values = match.groups()
        points = (
            PlanePoint.from_strings(values[0], values[1], values[2]),
            PlanePoint.from_strings(values[3], values[4], values[5]),
            PlanePoint.from_strings(values[6], values[7], values[8]),
        )
        a, b, c = (point.vector for point in points)
        normal = (b - a).cross(c - a)
        length = normal.length()
        if length <= degeneracy_epsilon:
            raise PlaneParseError("VMF plane points are collinear or coincident")
        normalized = normal.scale(1.0 / length)
        return cls(normal=normalized, distance=normalized.dot(a), points=points, raw=raw)

    def signed_distance(self, point: Vec3) -> float:
        """Return positive distance on the outside side of the plane."""
        return self.normal.dot(point) - self.distance

    def classify(self, point: Vec3, *, tolerance: float) -> PointPlaneRelation:
        signed = self.signed_distance(point)
        if signed > tolerance:
            return PointPlaneRelation.OUTSIDE
        if signed < -tolerance:
            return PointPlaneRelation.INSIDE
        return PointPlaneRelation.ON

    def coplanar_with(
        self, other: Plane, *, angular_tolerance: float, distance_tolerance: float
    ) -> bool:
        """Return whether two planes represent the same oriented support plane."""
        return (
            self.normal.distance_to(other.normal) <= angular_tolerance
            and abs(self.distance - other.distance) <= distance_tolerance
        )


@dataclass(frozen=True, slots=True)
class GeometryTolerances:
    """Profile-controlled tolerances for conservative brush validation."""

    plane_distance: float = 1e-5
    vertex_merge: float = 1e-4
    coplanar_normal: float = 1e-6
    min_edge_length: float = 0.01
    min_face_area: float = 0.01
    min_volume: float = 0.01
    world_bound: float = 65_536.0
    world_margin: float = 0.0


@dataclass(frozen=True, slots=True)
class GeometryBlocker:
    """A deterministic reason automatic geometry authority was denied."""

    code: str
    message: str
    side_id: str | None = None


@dataclass(frozen=True, slots=True)
class FaceGeometry:
    """Reconstructed polygon for one brush side."""

    side_id: str | None
    plane: Plane
    vertices: tuple[Vec3, ...]
    area: float


@dataclass(frozen=True, slots=True)
class ConvexBrush:
    """A validated convex brush reconstructed from side half-spaces."""

    vertices: tuple[Vec3, ...]
    faces: tuple[FaceGeometry, ...]
    volume: float
    bounds_min: Vec3
    bounds_max: Vec3


@dataclass(frozen=True, slots=True)
class BrushSpatialRecord:
    """One brush enrolled in deterministic broad-phase geometry checks."""

    key: str
    brush: ConvexBrush


@dataclass(frozen=True, slots=True)
class BrushIntersectionCandidate:
    """One AABB-filtered brush pair requiring exact relation checks."""

    a_key: str
    b_key: str
    bounds_relation: BoundsRelation


@dataclass(frozen=True, slots=True)
class BrushReconstruction:
    """Result of reconstructing one brush from VMF side planes."""

    status: ReconstructionStatus
    brush: ConvexBrush | None
    blockers: tuple[GeometryBlocker, ...]
    tolerances: GeometryTolerances


@dataclass(frozen=True, slots=True)
class BrushTransformResult:
    """Result of a geometry transform that carries no mutation authority."""

    status: GeometryTransformStatus
    brush: ConvexBrush | None
    blockers: tuple[GeometryBlocker, ...]
    tolerances: GeometryTolerances
    operation: str
    mutation_authorized: bool = False


@dataclass(frozen=True, slots=True)
class BrushSideSource:
    """One VMF ``side`` block with exact source spans for its plane keyvalue."""

    side_id: str | None
    plane_raw: str
    plane_span: SourceSpan
    block_span: SourceSpan
    plane: Plane


@dataclass(frozen=True, slots=True)
class BrushSource:
    """One VMF ``solid`` block represented as read-only geometry input."""

    solid_id: str | None
    owner_kind: str
    owner_id: str | None
    block_span: SourceSpan
    sides: tuple[BrushSideSource, ...]

    def reconstruct(self, tolerances: GeometryTolerances | None = None) -> BrushReconstruction:
        return reconstruct_convex_brush(
            tuple(side.plane for side in self.sides),
            side_ids=tuple(side.side_id for side in self.sides),
            tolerances=tolerances,
        )


def _source_span(start: int, end: int, line: int, column: int) -> SourceSpan:
    return SourceSpan(start=start, end=end, line=line, column=column)


def _block_span(block: BlockNode) -> SourceSpan:
    return _source_span(block.start, block.end, block.key.line, block.key.column)


def _first_pair(block: BlockNode, key: str) -> PairNode | None:
    folded = key.casefold()
    for entry in block.entries:
        if isinstance(entry, PairNode) and entry.key.value.casefold() == folded:
            return entry
    return None


def _direct_blocks(block: BlockNode, key: str) -> tuple[BlockNode, ...]:
    folded = key.casefold()
    return tuple(
        entry
        for entry in block.entries
        if isinstance(entry, BlockNode) and entry.key.value.casefold() == folded
    )


def _side_source(side_block: BlockNode) -> BrushSideSource | GeometryBlocker:
    plane_pair = _first_pair(side_block, "plane")
    side_id_pair = _first_pair(side_block, "id")
    side_id = side_id_pair.value.value if side_id_pair is not None else None
    if plane_pair is None:
        return GeometryBlocker(
            code="SIDE_MISSING_PLANE",
            message="Side block has no direct plane keyvalue.",
            side_id=side_id,
        )
    try:
        plane = Plane.from_vmf(plane_pair.value.value)
    except PlaneParseError as exc:
        return GeometryBlocker(
            code="SIDE_INVALID_PLANE",
            message=str(exc),
            side_id=side_id,
        )
    return BrushSideSource(
        side_id=side_id,
        plane_raw=plane_pair.value.value,
        plane_span=_source_span(
            plane_pair.value.start,
            plane_pair.value.end,
            plane_pair.value.line,
            plane_pair.value.column,
        ),
        block_span=_block_span(side_block),
        plane=plane,
    )


def extract_brush_sources(parsed: ParsedVmf) -> tuple[BrushSource, ...]:
    """Extract VMF world/entity solids whose side planes can be parsed safely.

    Solids with malformed or missing side planes are omitted from returned
    geometry sources because they cannot be used as exact-authority input. The
    linter/report layer will expose detailed blockers as this kernel is wired
    into higher-level analysis.
    """
    solids: list[BrushSource] = []
    owner_blocks = tuple(
        block for block in parsed.blocks() if block.key.value.casefold() in {"world", "entity"}
    )
    for owner_block in owner_blocks:
        owner_id_pair = _first_pair(owner_block, "id")
        owner_id = owner_id_pair.value.value if owner_id_pair is not None else None
        for solid_block in _direct_blocks(owner_block, "solid"):
            side_sources: list[BrushSideSource] = []
            failed = False
            for side_block in _direct_blocks(solid_block, "side"):
                source = _side_source(side_block)
                if isinstance(source, GeometryBlocker):
                    failed = True
                    break
                side_sources.append(source)
            if failed:
                continue
            solid_id_pair = _first_pair(solid_block, "id")
            solids.append(
                BrushSource(
                    solid_id=solid_id_pair.value.value if solid_id_pair is not None else None,
                    owner_kind=owner_block.key.value.casefold(),
                    owner_id=owner_id,
                    block_span=_block_span(solid_block),
                    sides=tuple(side_sources),
                )
            )
    return tuple(solids)


def reconstruct_convex_brush(
    planes: Iterable[Plane],
    *,
    side_ids: Iterable[str | None] | None = None,
    tolerances: GeometryTolerances | None = None,
) -> BrushReconstruction:
    """Reconstruct and validate a convex brush from outward VMF side planes."""
    active_tolerances = tolerances or GeometryTolerances()
    plane_tuple = tuple(planes)
    side_id_tuple = tuple(side_ids) if side_ids is not None else tuple(None for _ in plane_tuple)
    blockers: list[GeometryBlocker] = []

    if len(plane_tuple) < 4:
        blockers.append(
            GeometryBlocker(
                code="BRUSH_TOO_FEW_PLANES",
                message="A bounded convex brush needs at least four side planes.",
            )
        )
    if len(side_id_tuple) != len(plane_tuple):
        blockers.append(
            GeometryBlocker(
                code="BRUSH_SIDE_ID_MISMATCH",
                message="Side ID count does not match plane count.",
            )
        )
    if blockers:
        return BrushReconstruction(
            status=ReconstructionStatus.INVALID,
            brush=None,
            blockers=tuple(blockers),
            tolerances=active_tolerances,
        )

    vertices = _deduplicate_vertices(
        _candidate_vertices(plane_tuple, active_tolerances),
        active_tolerances.vertex_merge,
    )
    if len(vertices) < 4:
        blockers.append(
            GeometryBlocker(
                code="BRUSH_UNBOUNDED_OR_OPEN",
                message="Plane set did not produce enough bounded half-space vertices.",
            )
        )
        return BrushReconstruction(
            status=ReconstructionStatus.INVALID,
            brush=None,
            blockers=tuple(blockers),
            tolerances=active_tolerances,
        )

    faces: list[FaceGeometry] = []
    for plane, side_id in zip(plane_tuple, side_id_tuple, strict=True):
        face_vertices = tuple(
            vertex
            for vertex in vertices
            if abs(plane.signed_distance(vertex)) <= active_tolerances.plane_distance
        )
        if len(face_vertices) < 3:
            blockers.append(
                GeometryBlocker(
                    code="BRUSH_UNBOUNDED_OR_OPEN",
                    message="A side plane did not produce a closed polygonal face.",
                    side_id=side_id,
                )
            )
            continue
        ordered = _sort_face_vertices(face_vertices, plane.normal)
        area = _polygon_area(ordered, plane.normal)
        if area < active_tolerances.min_face_area:
            blockers.append(
                GeometryBlocker(
                    code="BRUSH_FACE_TOO_SMALL",
                    message=(
                        f"Face area {area:.6g} is below the configured minimum "
                        f"{active_tolerances.min_face_area:.6g}."
                    ),
                    side_id=side_id,
                )
            )
        edge_blocker = _edge_length_blocker(ordered, active_tolerances, side_id)
        if edge_blocker is not None:
            blockers.append(edge_blocker)
        faces.append(FaceGeometry(side_id=side_id, plane=plane, vertices=ordered, area=area))

    bounds_min, bounds_max = _bounds(vertices)
    bound_limit = active_tolerances.world_bound - active_tolerances.world_margin
    for vertex in vertices:
        if max(abs(vertex.x), abs(vertex.y), abs(vertex.z)) > bound_limit:
            blockers.append(
                GeometryBlocker(
                    code="BRUSH_WORLD_BOUNDS_EXCEEDED",
                    message="Brush vertex exceeds configured world bounds.",
                )
            )
            break

    if len(faces) != len(plane_tuple):
        blockers.append(
            GeometryBlocker(
                code="BRUSH_UNBOUNDED_OR_OPEN",
                message="Not every side plane produced a valid face.",
            )
        )

    volume = _brush_volume(vertices, faces)
    if volume < active_tolerances.min_volume:
        blockers.append(
            GeometryBlocker(
                code="BRUSH_VOLUME_TOO_SMALL",
                message=(
                    f"Brush volume {volume:.6g} is below the configured minimum "
                    f"{active_tolerances.min_volume:.6g}."
                ),
            )
        )

    if blockers:
        return BrushReconstruction(
            status=ReconstructionStatus.INVALID,
            brush=None,
            blockers=tuple(_unique_blockers(blockers)),
            tolerances=active_tolerances,
        )

    return BrushReconstruction(
        status=ReconstructionStatus.VALID,
        brush=ConvexBrush(
            vertices=vertices,
            faces=tuple(faces),
            volume=volume,
            bounds_min=bounds_min,
            bounds_max=bounds_max,
        ),
        blockers=(),
        tolerances=active_tolerances,
    )


def classify_brush_relation(
    brush_a: ConvexBrush,
    brush_b: ConvexBrush,
    *,
    tolerances: GeometryTolerances | None = None,
) -> BrushRelation:
    """Classify two validated convex brushes without authorizing mutation.

    The classifier uses half-space containment checks and a convex separating-axis
    test. A result of ``EQUAL_VOLUME`` or ``TOUCHING`` is geometry evidence only;
    automatic duplicate removal still requires semantic/material/compiler gates.
    """
    active_tolerances = tolerances or GeometryTolerances()
    a_contains_b = _brush_contains_vertices(
        brush_a, brush_b.vertices, tolerance=active_tolerances.plane_distance
    )
    b_contains_a = _brush_contains_vertices(
        brush_b, brush_a.vertices, tolerance=active_tolerances.plane_distance
    )
    if a_contains_b and b_contains_a:
        return BrushRelation.EQUAL_VOLUME
    if a_contains_b:
        return BrushRelation.A_CONTAINS_B
    if b_contains_a:
        return BrushRelation.B_CONTAINS_A

    overlap = _sat_overlap_depth(brush_a, brush_b, active_tolerances)
    if overlap is None:
        return BrushRelation.DISJOINT
    if overlap <= active_tolerances.plane_distance:
        return BrushRelation.TOUCHING
    return BrushRelation.OVERLAPPING


def classify_bounds_relation(
    brush_a: ConvexBrush,
    brush_b: ConvexBrush,
    *,
    expansion: float = 0.0,
    tolerances: GeometryTolerances | None = None,
) -> BoundsRelation:
    """Classify expanded axis-aligned bounds for deterministic broad phase."""
    active_tolerances = tolerances or GeometryTolerances()
    _validate_expansion(expansion)
    a_min, a_max = _expanded_bounds(brush_a, expansion)
    b_min, b_max = _expanded_bounds(brush_b, expansion)
    axis_overlaps = (
        min(a_max.x, b_max.x) - max(a_min.x, b_min.x),
        min(a_max.y, b_max.y) - max(a_min.y, b_min.y),
        min(a_max.z, b_max.z) - max(a_min.z, b_min.z),
    )
    if any(overlap < -active_tolerances.plane_distance for overlap in axis_overlaps):
        return BoundsRelation.DISJOINT
    if any(abs(overlap) <= active_tolerances.plane_distance for overlap in axis_overlaps):
        return BoundsRelation.TOUCHING
    return BoundsRelation.OVERLAPPING


def find_potential_brush_intersections(
    records_a: Iterable[BrushSpatialRecord],
    records_b: Iterable[BrushSpatialRecord],
    *,
    expansion: float = 0.0,
    tolerances: GeometryTolerances | None = None,
) -> tuple[BrushIntersectionCandidate, ...]:
    """Return deterministic AABB candidate pairs for later exact checks.

    This is a conservative broad phase. Returned candidates are evidence that an
    exact brush relation check is needed; they are not duplicate-removal or seam
    mutation authority.
    """
    candidates: list[BrushIntersectionCandidate] = []
    for record_a in records_a:
        for record_b in records_b:
            relation = classify_bounds_relation(
                record_a.brush,
                record_b.brush,
                expansion=expansion,
                tolerances=tolerances,
            )
            if relation is BoundsRelation.DISJOINT:
                continue
            candidates.append(
                BrushIntersectionCandidate(
                    a_key=record_a.key,
                    b_key=record_b.key,
                    bounds_relation=relation,
                )
            )
    return tuple(sorted(candidates, key=lambda candidate: (candidate.a_key, candidate.b_key)))


def translate_convex_brush_for_analysis(
    brush: ConvexBrush,
    offset: Vec3,
    *,
    tolerances: GeometryTolerances | None = None,
) -> BrushTransformResult:
    """Translate a validated convex brush for relation/seam analysis only.

    The returned brush is derived geometry. It is suitable for conservative
    intersection checks and report evidence, but it is not a source-spelling VMF
    patch and carries ``mutation_authorized=False`` unconditionally.
    """
    active_tolerances = tolerances or GeometryTolerances()
    if not _is_finite_vec(offset):
        return BrushTransformResult(
            status=GeometryTransformStatus.INVALID,
            brush=None,
            blockers=(
                GeometryBlocker(
                    code="TRANSFORM_NONFINITE_OFFSET",
                    message="Translation offset contains a non-finite coordinate.",
                ),
            ),
            tolerances=active_tolerances,
            operation="translation",
        )

    vertices = tuple(vertex + offset for vertex in brush.vertices)
    if not all(_is_finite_vec(vertex) for vertex in vertices):
        return BrushTransformResult(
            status=GeometryTransformStatus.INVALID,
            brush=None,
            blockers=(
                GeometryBlocker(
                    code="TRANSFORM_NONFINITE_RESULT",
                    message="Translation produced a non-finite brush coordinate.",
                ),
            ),
            tolerances=active_tolerances,
            operation="translation",
        )

    bounds_min, bounds_max = _bounds(vertices)
    bound_limit = active_tolerances.world_bound - active_tolerances.world_margin
    if any(max(abs(vertex.x), abs(vertex.y), abs(vertex.z)) > bound_limit for vertex in vertices):
        return BrushTransformResult(
            status=GeometryTransformStatus.INVALID,
            brush=None,
            blockers=(
                GeometryBlocker(
                    code="BRUSH_WORLD_BOUNDS_EXCEEDED",
                    message="Translated brush vertex exceeds configured world bounds.",
                ),
            ),
            tolerances=active_tolerances,
            operation="translation",
        )

    faces = tuple(_translate_face_for_analysis(face, offset) for face in brush.faces)
    return BrushTransformResult(
        status=GeometryTransformStatus.VALID,
        brush=ConvexBrush(
            vertices=vertices,
            faces=faces,
            volume=brush.volume,
            bounds_min=bounds_min,
            bounds_max=bounds_max,
        ),
        blockers=(),
        tolerances=active_tolerances,
        operation="translation",
    )


def _validate_expansion(expansion: float) -> None:
    if not math.isfinite(expansion):
        raise ValueError("Bounds expansion must be finite.")
    if expansion < 0.0:
        raise ValueError("Bounds expansion must not be negative.")


def _expanded_bounds(brush: ConvexBrush, expansion: float) -> tuple[Vec3, Vec3]:
    return (
        Vec3(
            brush.bounds_min.x - expansion,
            brush.bounds_min.y - expansion,
            brush.bounds_min.z - expansion,
        ),
        Vec3(
            brush.bounds_max.x + expansion,
            brush.bounds_max.y + expansion,
            brush.bounds_max.z + expansion,
        ),
    )


def _candidate_vertices(
    planes: tuple[Plane, ...], tolerances: GeometryTolerances
) -> tuple[Vec3, ...]:
    candidates: list[Vec3] = []
    for plane_a, plane_b, plane_c in combinations(planes, 3):
        intersection = _intersect_three_planes(
            plane_a,
            plane_b,
            plane_c,
            parallel_tolerance=tolerances.coplanar_normal,
        )
        if intersection is None:
            continue
        if all(
            plane.signed_distance(intersection) <= tolerances.plane_distance for plane in planes
        ):
            candidates.append(intersection)
    return tuple(candidates)


def _translate_face_for_analysis(face: FaceGeometry, offset: Vec3) -> FaceGeometry:
    translated_vertices = tuple(vertex + offset for vertex in face.vertices)
    translated_plane_points = (
        _derived_plane_point(face.plane.points[0].vector + offset),
        _derived_plane_point(face.plane.points[1].vector + offset),
        _derived_plane_point(face.plane.points[2].vector + offset),
    )
    translated_plane = Plane(
        normal=face.plane.normal,
        distance=face.plane.distance + face.plane.normal.dot(offset),
        points=translated_plane_points,
        raw=f"<generated:analysis-translation offset={_format_vec(offset)}>",
    )
    return FaceGeometry(
        side_id=face.side_id,
        plane=translated_plane,
        vertices=translated_vertices,
        area=face.area,
    )


def _derived_plane_point(point: Vec3) -> PlanePoint:
    return PlanePoint(
        NumericValue(raw=_format_float(point.x), value=point.x),
        NumericValue(raw=_format_float(point.y), value=point.y),
        NumericValue(raw=_format_float(point.z), value=point.z),
    )


def _format_vec(point: Vec3) -> str:
    return f"({_format_float(point.x)} {_format_float(point.y)} {_format_float(point.z)})"


def _format_float(value: float) -> str:
    return format(value, ".12g")


def _is_finite_vec(point: Vec3) -> bool:
    return all(math.isfinite(value) for value in point.as_tuple())


def _intersect_three_planes(
    plane_a: Plane,
    plane_b: Plane,
    plane_c: Plane,
    *,
    parallel_tolerance: float,
) -> Vec3 | None:
    cross_bc = plane_b.normal.cross(plane_c.normal)
    determinant = plane_a.normal.dot(cross_bc)
    if abs(determinant) <= parallel_tolerance:
        return None
    numerator = (
        cross_bc.scale(plane_a.distance)
        + plane_c.normal.cross(plane_a.normal).scale(plane_b.distance)
        + plane_a.normal.cross(plane_b.normal).scale(plane_c.distance)
    )
    point = numerator.scale(1.0 / determinant)
    if not all(math.isfinite(value) for value in point.as_tuple()):
        return None
    return point


def _deduplicate_vertices(vertices: tuple[Vec3, ...], tolerance: float) -> tuple[Vec3, ...]:
    unique: list[Vec3] = []
    for vertex in vertices:
        if not any(vertex.distance_to(existing) <= tolerance for existing in unique):
            unique.append(vertex)
    return tuple(sorted(unique, key=lambda point: (point.x, point.y, point.z)))


def _face_basis(normal: Vec3) -> tuple[Vec3, Vec3]:
    helper = Vec3(0.0, 0.0, 1.0)
    if abs(normal.dot(helper)) > 0.9:
        helper = Vec3(0.0, 1.0, 0.0)
    u_axis = helper.cross(normal).normalized()
    v_axis = normal.cross(u_axis).normalized()
    return u_axis, v_axis


def _sort_face_vertices(vertices: tuple[Vec3, ...], normal: Vec3) -> tuple[Vec3, ...]:
    centroid = _centroid(vertices)
    u_axis, v_axis = _face_basis(normal)
    return tuple(
        sorted(
            vertices,
            key=lambda vertex: math.atan2(
                (vertex - centroid).dot(v_axis),
                (vertex - centroid).dot(u_axis),
            ),
        )
    )


def _centroid(vertices: tuple[Vec3, ...]) -> Vec3:
    count = float(len(vertices))
    return Vec3(
        sum(vertex.x for vertex in vertices) / count,
        sum(vertex.y for vertex in vertices) / count,
        sum(vertex.z for vertex in vertices) / count,
    )


def _polygon_area(vertices: tuple[Vec3, ...], normal: Vec3) -> float:
    area_vector = Vec3(0.0, 0.0, 0.0)
    for first, second in zip(vertices, (*vertices[1:], vertices[0]), strict=True):
        area_vector += first.cross(second)
    return abs(area_vector.dot(normal)) * 0.5


def _edge_length_blocker(
    vertices: tuple[Vec3, ...],
    tolerances: GeometryTolerances,
    side_id: str | None,
) -> GeometryBlocker | None:
    for first, second in zip(vertices, (*vertices[1:], vertices[0]), strict=True):
        length = first.distance_to(second)
        if length < tolerances.min_edge_length:
            return GeometryBlocker(
                code="BRUSH_EDGE_TOO_SHORT",
                message=(
                    f"Face edge length {length:.6g} is below the configured minimum "
                    f"{tolerances.min_edge_length:.6g}."
                ),
                side_id=side_id,
            )
    return None


def _bounds(vertices: tuple[Vec3, ...]) -> tuple[Vec3, Vec3]:
    return (
        Vec3(
            min(vertex.x for vertex in vertices),
            min(vertex.y for vertex in vertices),
            min(vertex.z for vertex in vertices),
        ),
        Vec3(
            max(vertex.x for vertex in vertices),
            max(vertex.y for vertex in vertices),
            max(vertex.z for vertex in vertices),
        ),
    )


def _brush_volume(vertices: tuple[Vec3, ...], faces: list[FaceGeometry]) -> float:
    if not vertices or not faces:
        return 0.0
    center = _centroid(vertices)
    volume = 0.0
    for face in faces:
        distance_to_face = max(face.plane.distance - face.plane.normal.dot(center), 0.0)
        volume += face.area * distance_to_face / 3.0
    return volume


def _brush_contains_vertices(
    brush: ConvexBrush, vertices: tuple[Vec3, ...], *, tolerance: float
) -> bool:
    return all(
        face.plane.signed_distance(vertex) <= tolerance
        for face in brush.faces
        for vertex in vertices
    )


def _sat_overlap_depth(
    brush_a: ConvexBrush, brush_b: ConvexBrush, tolerances: GeometryTolerances
) -> float | None:
    minimum_overlap: float | None = None
    for axis in _separating_axes(brush_a, brush_b, tolerances):
        a_min, a_max = _project_vertices(brush_a.vertices, axis)
        b_min, b_max = _project_vertices(brush_b.vertices, axis)
        if a_max < b_min - tolerances.plane_distance:
            return None
        if b_max < a_min - tolerances.plane_distance:
            return None
        overlap = max(0.0, min(a_max, b_max) - max(a_min, b_min))
        minimum_overlap = overlap if minimum_overlap is None else min(minimum_overlap, overlap)
    return minimum_overlap


def _separating_axes(
    brush_a: ConvexBrush, brush_b: ConvexBrush, tolerances: GeometryTolerances
) -> tuple[Vec3, ...]:
    axes: list[Vec3] = []
    for face in (*brush_a.faces, *brush_b.faces):
        _append_axis(axes, face.plane.normal, tolerances)

    a_edges = _edge_directions(brush_a)
    b_edges = _edge_directions(brush_b)
    for edge_a in a_edges:
        for edge_b in b_edges:
            _append_axis(axes, edge_a.cross(edge_b), tolerances)
    return tuple(axes)


def _append_axis(axes: list[Vec3], axis: Vec3, tolerances: GeometryTolerances) -> None:
    length = axis.length()
    if length <= tolerances.coplanar_normal:
        return
    normalized = axis.scale(1.0 / length)
    if any(abs(normalized.dot(existing)) >= 1.0 - tolerances.coplanar_normal for existing in axes):
        return
    axes.append(normalized)


def _edge_directions(brush: ConvexBrush) -> tuple[Vec3, ...]:
    directions: list[Vec3] = []
    for face in brush.faces:
        for first, second in zip(
            face.vertices, (*face.vertices[1:], face.vertices[0]), strict=True
        ):
            directions.append(second - first)
    return tuple(directions)


def _project_vertices(vertices: tuple[Vec3, ...], axis: Vec3) -> tuple[float, float]:
    projections = tuple(vertex.dot(axis) for vertex in vertices)
    return min(projections), max(projections)


def _unique_blockers(blockers: list[GeometryBlocker]) -> tuple[GeometryBlocker, ...]:
    seen: set[tuple[str, str | None]] = set()
    unique: list[GeometryBlocker] = []
    for blocker in blockers:
        identity = (blocker.code, blocker.side_id)
        if identity in seen:
            continue
        seen.add(identity)
        unique.append(blocker)
    return tuple(unique)
