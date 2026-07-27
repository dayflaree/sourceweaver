import importlib.util
from pathlib import Path
from types import ModuleType
from typing import Protocol, cast

import pytest


class _InstallerModule(Protocol):
    def tree_digest(self, path: Path) -> str: ...

    def install(
        self, source_root: Path, destination_root: Path, *, force: bool, dry_run: bool
    ) -> int: ...


def _load_installer() -> _InstallerModule:
    path = Path(__file__).parents[1] / "scripts" / "install_hermes_skills.py"
    spec = importlib.util.spec_from_file_location("sourceweaver_skill_installer", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load installer module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return cast(_InstallerModule, cast(ModuleType, module))


INSTALLER = _load_installer()


def _source_root(tmp_path: Path) -> Path:
    source_root = tmp_path / "source"
    skill = source_root / "sourceweaver-test"
    skill.mkdir(parents=True)
    (skill / "SKILL.md").write_text("---\nname: sourceweaver-test\n---\n", encoding="utf-8")
    return source_root


def test_tree_digest_is_stable(tmp_path: Path) -> None:
    source = _source_root(tmp_path) / "sourceweaver-test"
    assert INSTALLER.tree_digest(source) == INSTALLER.tree_digest(source)


def test_dry_run_does_not_create_destination(tmp_path: Path) -> None:
    source = _source_root(tmp_path)
    destination = tmp_path / "destination"
    assert INSTALLER.install(source, destination, force=False, dry_run=True) == 1
    assert not destination.exists()


def test_install_and_idempotent_reinstall(tmp_path: Path) -> None:
    source = _source_root(tmp_path)
    destination = tmp_path / "destination"
    assert INSTALLER.install(source, destination, force=False, dry_run=False) == 1
    installed = destination / "sourceweaver-test" / "SKILL.md"
    assert installed.is_file()
    assert INSTALLER.install(source, destination, force=False, dry_run=False) == 0


def test_changed_destination_requires_force(tmp_path: Path) -> None:
    source = _source_root(tmp_path)
    destination = tmp_path / "destination"
    INSTALLER.install(source, destination, force=False, dry_run=False)
    installed = destination / "sourceweaver-test" / "SKILL.md"
    installed.write_text("changed", encoding="utf-8")
    with pytest.raises(SystemExit, match="--force"):
        INSTALLER.install(source, destination, force=False, dry_run=False)
    assert installed.read_text(encoding="utf-8") == "changed"

    assert INSTALLER.install(source, destination, force=True, dry_run=False) == 1
    assert installed.read_text(encoding="utf-8").startswith("---")


def test_force_replaces_non_directory_destination(tmp_path: Path) -> None:
    source = _source_root(tmp_path)
    destination = tmp_path / "destination"
    destination.mkdir()
    conflicting = destination / "sourceweaver-test"
    conflicting.write_text("file", encoding="utf-8")
    assert INSTALLER.install(source, destination, force=True, dry_run=False) == 1
    assert (conflicting / "SKILL.md").is_file()


def test_failed_staged_replace_restores_previous_skill(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    source = _source_root(tmp_path)
    destination = tmp_path / "destination"
    destination.mkdir()
    installed = destination / "sourceweaver-test"
    installed.mkdir()
    original = installed / "SKILL.md"
    original.write_text("previous version", encoding="utf-8")
    original_replace = Path.replace

    def fail_staged_replace(self: Path, target: Path) -> Path:
        if self.name == "sourceweaver-test" and self.parent.name.endswith(".tmp"):
            raise OSError("simulated staged rename failure")
        return original_replace(self, target)

    monkeypatch.setattr(Path, "replace", fail_staged_replace)
    with pytest.raises(OSError, match="simulated staged rename failure"):
        INSTALLER.install(source, destination, force=True, dry_run=False)
    assert original.read_text(encoding="utf-8") == "previous version"
