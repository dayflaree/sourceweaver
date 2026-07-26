import json
import tomllib
from pathlib import Path

ROOT = Path(__file__).parents[1]


def test_json_schemas_parse() -> None:
    schema_paths = sorted((ROOT / "schemas").glob("*.schema.json"))
    assert schema_paths
    for path in schema_paths:
        payload = json.loads(path.read_text(encoding="utf-8"))
        assert payload["$schema"].endswith("2020-12/schema")
        assert payload["type"] == "object"


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
