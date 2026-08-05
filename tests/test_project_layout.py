import json
import tomllib
from importlib import metadata
from pathlib import Path
from typing import cast

from jsonschema import Draft202012Validator

from sourceweaver import __version__
from sourceweaver.analysis import analyze_vmf

ROOT = Path(__file__).parents[1]
FIXTURE = ROOT / "tests" / "fixtures" / "minimal.vmf"


def _load_schema(name: str) -> dict[str, object]:
    payload = json.loads((ROOT / "schemas" / name).read_text(encoding="utf-8"))
    return cast(dict[str, object], payload)


def test_json_schemas_are_valid_draft_2020_12() -> None:
    schema_paths = sorted((ROOT / "schemas").glob("*.schema.json"))
    assert schema_paths
    for path in schema_paths:
        payload = json.loads(path.read_text(encoding="utf-8"))
        assert payload["$schema"].endswith("2020-12/schema")
        Draft202012Validator.check_schema(payload)


def test_analysis_report_matches_published_schema() -> None:
    payload = json.loads(analyze_vmf(FIXTURE).model_dump_json())
    Draft202012Validator(_load_schema("analysis-report.schema.json")).validate(payload)


def test_minimal_patch_manifest_matches_published_schema() -> None:
    payload = {
        "schema_version": "1.0",
        "run_id": "test-run",
        "source_files": [{"path": "minimal.vmf", "sha256": "0" * 64}],
        "profile": {"id": "test"},
        "policy_hash": "0" * 64,
        "transformations": [],
        "static_checks": [],
        "compile_runs": [],
        "runtime_runs": [],
        "metrics": {},
        "verdict": "blocked",
    }
    Draft202012Validator(_load_schema("patch-manifest.schema.json")).validate(payload)


def test_profiles_parse() -> None:
    profile_paths = sorted((ROOT / "profiles").glob("*.toml"))
    assert profile_paths
    for path in profile_paths:
        payload = tomllib.loads(path.read_text(encoding="utf-8"))
        assert payload["schema_version"] == "1.0"
        assert payload["id"]


def test_skill_layout() -> None:
    skill_paths = sorted((ROOT / "skills").glob("sourceweaver-*/SKILL.md"))
    assert len(skill_paths) >= 10
    names: set[str] = set()
    for path in skill_paths:
        text = path.read_text(encoding="utf-8")
        assert text.startswith("---\n")
        front_matter = text.split("---\n", 2)[1]
        fields = {
            key.strip(): value.strip()
            for line in front_matter.splitlines()
            if ":" in line
            for key, value in [line.split(":", 1)]
        }
        assert fields["name"] == path.parent.name
        assert fields["description"]
        assert fields["name"] not in names
        names.add(fields["name"])


def test_research_fingerprints_do_not_contain_absolute_home_paths() -> None:
    for path in (ROOT / "research" / "compiler-fingerprints").glob("*.json"):
        text = path.read_text(encoding="utf-8")
        assert "/home/" not in text
        assert "C:\\Users\\" not in text


def test_package_and_module_versions_match() -> None:
    assert metadata.version("sourceweaver") == __version__


def test_ci_covers_supported_os_and_python_versions() -> None:
    workflow = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
    assert "ubuntu-latest" in workflow
    assert "windows-latest" in workflow
    for version in ("3.11", "3.12", "3.13", "3.14"):
        assert f'"{version}"' in workflow
    assert "ruff format --check" in workflow
    assert "python -m build" in workflow
