#!/usr/bin/env python3
"""Install a built wheel into a clean environment and exercise public entry points."""

from __future__ import annotations

import argparse
import json
import os

# This CI script executes fixed argument arrays without a shell.
import subprocess  # nosec B404
import sys
import tempfile
from pathlib import Path


def _run(
    arguments: list[str], *, cwd: Path, env: dict[str, str]
) -> subprocess.CompletedProcess[str]:
    # Callers provide explicit trusted executable arrays.
    return subprocess.run(  # nosec B603
        arguments,
        cwd=cwd,
        env=env,
        check=True,
        capture_output=True,
        text=True,
    )


def smoke_wheel(wheel: Path) -> None:
    """Install and verify one wheel without importing from the repository."""
    repository = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory(prefix="sourceweaver-wheel-smoke-") as temporary_name:
        temporary = Path(temporary_name)
        environment = temporary / "venv"
        env = os.environ.copy()
        env.pop("PYTHONPATH", None)
        env["PYTHONNOUSERSITE"] = "1"
        _run(
            [sys.executable, "-m", "virtualenv", "--clear", str(environment)],
            cwd=temporary,
            env=env,
        )
        python = environment / ("Scripts/python.exe" if os.name == "nt" else "bin/python")

        _run(
            [
                str(python),
                "-m",
                "pip",
                "install",
                "--disable-pip-version-check",
                str(wheel.resolve()),
            ],
            cwd=temporary,
            env=env,
        )
        _run([str(python), "-m", "pip", "check"], cwd=temporary, env=env)

        version = _run([str(python), "-m", "sourceweaver", "--version"], cwd=temporary, env=env)
        if version.stdout.strip() != "0.1.0":
            raise RuntimeError(f"Unexpected installed version: {version.stdout!r}")

        location = _run(
            [
                str(python),
                "-c",
                "import pathlib, sourceweaver; "
                "print(pathlib.Path(sourceweaver.__file__).resolve())",
            ],
            cwd=temporary,
            env=env,
        )
        installed_path = Path(location.stdout.strip())
        if repository in installed_path.parents:
            raise RuntimeError(f"Smoke test imported the source checkout: {installed_path}")

        source = temporary / "minimal.vmf"
        source.write_bytes(b'versioninfo\n{\n}\nworld\n{\n"classname" "worldspawn"\n}\n')
        report_result = _run(
            [str(python), "-m", "sourceweaver", "inspect", str(source), "--json"],
            cwd=temporary,
            env=env,
        )
        report = json.loads(report_result.stdout)
        if report["metadata"]["lossless_roundtrip"] is not True:
            raise RuntimeError("Installed package failed the VMF round-trip invariant")

        output = temporary / "copy.vmf"
        _run(
            [
                str(python),
                "-m",
                "sourceweaver",
                "roundtrip",
                str(source),
                "--output",
                str(output),
            ],
            cwd=temporary,
            env=env,
        )
        if output.read_bytes() != source.read_bytes():
            raise RuntimeError("Installed package produced a non-identical round-trip file")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("wheel", type=Path)
    args = parser.parse_args()
    smoke_wheel(args.wheel)
    print(f"wheel smoke test passed: {args.wheel}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
