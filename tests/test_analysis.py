from pathlib import Path

from sourceweaver.analysis import analyze_vmf

FIXTURE = Path(__file__).parent / "fixtures/minimal.vmf"


def _codes(path: Path) -> set[str]:
    return {diagnostic.code for diagnostic in analyze_vmf(path).diagnostics}


def test_analysis_reports_roundtrip() -> None:
    report = analyze_vmf(FIXTURE)
    assert report.metadata["lossless_roundtrip"] is True
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
