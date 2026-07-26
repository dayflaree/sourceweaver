from pathlib import Path

from sourceweaver.analysis import analyze_vmf

FIXTURE = Path(__file__).parent / "fixtures/minimal.vmf"


def test_analysis_reports_roundtrip() -> None:
    report = analyze_vmf(FIXTURE)
    assert report.metadata["lossless_roundtrip"] is True
    assert report.source.sha256
    assert not [
        diagnostic for diagnostic in report.diagnostics if diagnostic.severity.value == "blocker"
    ]
