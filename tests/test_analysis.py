from pathlib import Path

from sourceweaver.analysis import analyze_vmf

FIXTURE = Path(__file__).parent / "fixtures/minimal.vmf"


CUBE_SIDES = """
        side { "id" "1" "plane" "(0 0 0) (0 1 0) (1 1 0)" }
        side { "id" "2" "plane" "(0 0 128) (128 0 128) (128 128 128)" }
        side { "id" "3" "plane" "(0 0 0) (128 0 0) (128 0 128)" }
        side { "id" "4" "plane" "(0 128 0) (0 128 128) (128 128 128)" }
        side { "id" "5" "plane" "(0 0 0) (0 0 128) (0 128 128)" }
        side { "id" "6" "plane" "(128 0 0) (128 128 0) (128 128 128)" }
"""


def _codes(path: Path) -> set[str]:
    return {diagnostic.code for diagnostic in analyze_vmf(path).diagnostics}


def _world_with_solid(sides: str) -> str:
    return f"""versioninfo {{}}
world
{{
    "id" "1"
    "classname" "worldspawn"
    solid
    {{
        "id" "20"
{sides}    }}
}}
"""


def test_analysis_reports_roundtrip() -> None:
    report = analyze_vmf(FIXTURE)
    assert report.metadata["lossless_roundtrip"] is True
    assert report.metadata["brush_source_count"] == 0
    assert report.metadata["valid_brush_count"] == 0
    assert report.metadata["geometry_blocker_count"] == 0
    assert report.source.sha256
    assert report.source.size == len(FIXTURE.read_bytes())
    assert report.newline == "LF"
    assert not [
        diagnostic for diagnostic in report.diagnostics if diagnostic.severity.value == "blocker"
    ]


def test_analysis_reports_missing_required_blocks(tmp_path: Path) -> None:
    path = tmp_path / "empty.vmf"
    path.write_text("cameras {}\n", encoding="utf-8")
    assert {"VMF001"} <= _codes(path)


def test_analysis_reports_duplicate_world(tmp_path: Path) -> None:
    path = tmp_path / "duplicate.vmf"
    path.write_text(
        'versioninfo {}\nworld { "classname" "worldspawn" }\nworld { "classname" "worldspawn" }\n',
        encoding="utf-8",
    )
    assert "VMF003" in _codes(path)


def test_analysis_reports_wrong_world_classname(tmp_path: Path) -> None:
    path = tmp_path / "wrong-world.vmf"
    path.write_text('versioninfo {}\nworld { "classname" "not_worldspawn" }\n', encoding="utf-8")
    assert "VMF004" in _codes(path)


def test_analysis_reports_root_pairs_and_missing_camera(tmp_path: Path) -> None:
    path = tmp_path / "root-pair.vmf"
    path.write_text(
        '"extension" "value"\nversioninfo {}\nworld { "classname" "worldspawn" }\n',
        encoding="utf-8",
    )
    assert {"VMF101", "VMF102"} <= _codes(path)


def test_analysis_reports_mixed_newlines(tmp_path: Path) -> None:
    path = tmp_path / "mixed.vmf"
    path.write_bytes(b'versioninfo {}\r\nworld {\n"classname" "worldspawn"\r\n}\n')
    assert analyze_vmf(path).newline == "MIXED"


def test_analysis_reports_valid_brush_geometry_summary(tmp_path: Path) -> None:
    path = tmp_path / "cube.vmf"
    path.write_text(_world_with_solid(CUBE_SIDES), encoding="utf-8")

    report = analyze_vmf(path)

    assert report.metadata["brush_source_count"] == 1
    assert report.metadata["valid_brush_count"] == 1
    assert report.metadata["geometry_blocker_count"] == 0
    assert "GEO001" not in {diagnostic.code for diagnostic in report.diagnostics}


def test_analysis_blocks_invalid_brush_geometry(tmp_path: Path) -> None:
    path = tmp_path / "open-brush.vmf"
    path.write_text(
        _world_with_solid(
            CUBE_SIDES.replace('        side { "id" "6"', '// removed\n        // side { "id" "6"')
        ),
        encoding="utf-8",
    )

    report = analyze_vmf(path)

    geometry_diagnostics = [
        diagnostic for diagnostic in report.diagnostics if diagnostic.code == "GEO001"
    ]
    assert report.metadata["brush_source_count"] == 1
    assert report.metadata["valid_brush_count"] == 0
    assert report.metadata["geometry_blocker_count"] >= 1
    assert len(geometry_diagnostics) == 1
    assert any("BRUSH_UNBOUNDED_OR_OPEN" in item for item in geometry_diagnostics[0].evidence)
