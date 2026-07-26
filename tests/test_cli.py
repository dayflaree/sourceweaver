import json
from pathlib import Path

from typer.testing import CliRunner

from sourceweaver.cli import app

RUNNER = CliRunner()
FIXTURE = Path(__file__).parent / "fixtures" / "minimal.vmf"


def test_version() -> None:
    result = RUNNER.invoke(app, ["--version"])
    assert result.exit_code == 0
    assert "0.1.0" in result.stdout


def test_inspect_json() -> None:
    result = RUNNER.invoke(app, ["inspect", str(FIXTURE), "--json"])
    assert result.exit_code == 0, result.stdout
    payload = json.loads(result.stdout)
    assert payload["metadata"]["lossless_roundtrip"] is True


def test_roundtrip_command(tmp_path: Path) -> None:
    output = tmp_path / "copy.vmf"
    result = RUNNER.invoke(app, ["roundtrip", str(FIXTURE), "--output", str(output)])
    assert result.exit_code == 0, result.stdout
    assert output.read_bytes() == FIXTURE.read_bytes()


def test_doctor_explicit_root(tmp_path: Path) -> None:
    root = tmp_path / "GarrysMod"
    compiler_dir = root / "bin" / "win64"
    compiler_dir.mkdir(parents=True)
    (compiler_dir / "vbspplusplus.exe").write_bytes(b"compiler")

    result = RUNNER.invoke(app, ["doctor", "--gmod-root", str(root), "--json"])
    assert result.exit_code == 0, result.stdout
    payload = json.loads(result.stdout)
    assert payload["compilers"]["vbsp"]["size"] == 8
    assert payload["compilers"]["vvis"] is None
