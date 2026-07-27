import json
import os
import subprocess
import sys
from pathlib import Path

from typer.testing import CliRunner

from sourceweaver.cli import app

RUNNER = CliRunner()
FIXTURE = Path(__file__).parent / "fixtures" / "minimal.vmf"


def _make_gmod_root(path: Path, *, complete: bool = False) -> Path:
    compiler_dir = path / "bin" / "win64"
    compiler_dir.mkdir(parents=True)
    (path / "garrysmod").mkdir()
    names = ["vbspplusplus.exe"]
    if complete:
        names.extend(["vvisplusplus.exe", "vradplusplus.exe", "bspzipplusplus.exe"])
    for name in names:
        (compiler_dir / name).write_bytes(b"MZcompiler")
    return path


def test_version() -> None:
    result = RUNNER.invoke(app, ["--version"])
    assert result.exit_code == 0
    assert result.stdout.strip() == "0.1.0"


def test_python_module_entry_point(tmp_path: Path) -> None:
    environment = os.environ.copy()
    for name in tuple(environment):
        if name.startswith("COV_CORE_") or name.startswith("COVERAGE_"):
            environment.pop(name)
    result = subprocess.run(
        [sys.executable, "-m", "sourceweaver", "--version"],
        cwd=tmp_path,
        env=environment,
        check=False,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0
    assert result.stdout.strip() == "0.1.0"


def test_inspect_json() -> None:
    result = RUNNER.invoke(app, ["inspect", str(FIXTURE), "--json"])
    assert result.exit_code == 0, result.stderr
    payload = json.loads(result.stdout)
    assert payload["metadata"]["lossless_roundtrip"] is True
    assert payload["newline"] == "LF"


def test_inspect_malformed_vmf_reports_clean_error(tmp_path: Path) -> None:
    malformed = tmp_path / "broken.vmf"
    malformed.write_text("world {", encoding="utf-8")
    result = RUNNER.invoke(app, ["inspect", str(malformed), "--json"])
    assert result.exit_code == 2
    assert "Missing closing brace" in result.stderr
    assert "Traceback" not in result.stderr


def test_roundtrip_command(tmp_path: Path) -> None:
    output = tmp_path / "nested" / "copy.vmf"
    result = RUNNER.invoke(app, ["roundtrip", str(FIXTURE), "--output", str(output)])
    assert result.exit_code == 0, result.stderr
    assert output.read_bytes() == FIXTURE.read_bytes()


def test_roundtrip_refuses_input_path_even_with_force(tmp_path: Path) -> None:
    source = tmp_path / "source.vmf"
    source.write_bytes(FIXTURE.read_bytes())
    original = source.read_bytes()
    result = RUNNER.invoke(app, ["roundtrip", str(source), "--output", str(source), "--force"])
    assert result.exit_code == 3
    assert "input VMF" in result.stderr
    assert source.read_bytes() == original


def test_roundtrip_refuses_existing_output_without_force(tmp_path: Path) -> None:
    output = tmp_path / "copy.vmf"
    output.write_bytes(b"keep me")
    result = RUNNER.invoke(app, ["roundtrip", str(FIXTURE), "--output", str(output)])
    assert result.exit_code == 3
    assert "already exists" in result.stderr
    assert output.read_bytes() == b"keep me"


def test_roundtrip_force_replaces_existing_output(tmp_path: Path) -> None:
    output = tmp_path / "copy.vmf"
    output.write_bytes(b"replace me")
    result = RUNNER.invoke(app, ["roundtrip", str(FIXTURE), "--output", str(output), "--force"])
    assert result.exit_code == 0, result.stderr
    assert output.read_bytes() == FIXTURE.read_bytes()


def test_doctor_explicit_root(tmp_path: Path) -> None:
    root = _make_gmod_root(tmp_path / "GarrysMod")
    result = RUNNER.invoke(app, ["doctor", "--gmod-root", str(root), "--json"])
    assert result.exit_code == 0, result.stderr
    payload = json.loads(result.stdout)
    assert payload["compilers"]["vbsp"]["size"] == 10
    assert payload["compilers"]["vbsp"]["format"] == "windows-pe"
    assert payload["compilers"]["vvis"] is None
    assert payload["complete"] is False


def test_doctor_strict_requires_complete_toolchain(tmp_path: Path) -> None:
    root = _make_gmod_root(tmp_path / "GarrysMod")
    result = RUNNER.invoke(app, ["doctor", "--gmod-root", str(root), "--json", "--strict"])
    assert result.exit_code == 5
    assert json.loads(result.stdout)["complete"] is False


def test_doctor_complete_toolchain_passes_strict(tmp_path: Path) -> None:
    root = _make_gmod_root(tmp_path / "GarrysMod", complete=True)
    result = RUNNER.invoke(app, ["doctor", "--gmod-root", str(root), "--json", "--strict"])
    assert result.exit_code == 0, result.stderr
    payload = json.loads(result.stdout)
    assert payload["complete"] is True
    assert set(payload["compilers"]) == {"vbsp", "vvis", "vrad", "bspzip"}
