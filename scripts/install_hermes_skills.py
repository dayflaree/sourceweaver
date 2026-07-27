#!/usr/bin/env python3
"""Install SourceWeaver project skills into a Hermes skill directory."""

from __future__ import annotations

import argparse
import hashlib
import shutil
import tempfile
from pathlib import Path


def tree_digest(path: Path) -> str:
    """Return a stable digest for all regular files in *path*."""
    if not path.is_dir():
        raise ValueError(f"Skill path is not a directory: {path}")
    digest = hashlib.sha256()
    for file_path in sorted(candidate for candidate in path.rglob("*") if candidate.is_file()):
        digest.update(file_path.relative_to(path).as_posix().encode("utf-8"))
        digest.update(b"\0")
        digest.update(file_path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def _remove_destination(path: Path) -> None:
    if path.is_symlink() or path.is_file():
        path.unlink()
    elif path.exists():
        shutil.rmtree(path)


def install(source_root: Path, destination_root: Path, *, force: bool, dry_run: bool) -> int:
    """Install all ``sourceweaver-*`` skill directories."""
    if not source_root.is_dir():
        raise ValueError(f"Skill source root does not exist: {source_root}")
    if not dry_run:
        destination_root.mkdir(parents=True, exist_ok=True)

    changed = 0
    for source in sorted(source_root.glob("sourceweaver-*")):
        if not source.is_dir() or not (source / "SKILL.md").is_file():
            continue
        destination = destination_root / source.name
        source_hash = tree_digest(source)
        if destination.is_dir() and tree_digest(destination) == source_hash:
            print(f"up to date: {source.name} ({source_hash[:12]})")
            continue
        if (destination.exists() or destination.is_symlink()) and not force:
            raise SystemExit(f"refusing to replace changed skill {destination}; rerun with --force")

        action = "would install" if dry_run else "install"
        print(f"{action}: {source.name} -> {destination} ({source_hash[:12]})")
        changed += 1
        if dry_run:
            continue

        temporary_parent = destination_root
        temporary = Path(
            tempfile.mkdtemp(prefix=f".{source.name}.", suffix=".tmp", dir=temporary_parent)
        )
        backup = temporary / f"{source.name}.backup"
        destination_moved = False
        try:
            staged = temporary / source.name
            shutil.copytree(source, staged)
            if destination.exists() or destination.is_symlink():
                destination.replace(backup)
                destination_moved = True
            try:
                staged.replace(destination)
            except OSError:
                _remove_destination(destination)
                if destination_moved:
                    backup.replace(destination)
                    destination_moved = False
                raise
            if destination_moved:
                _remove_destination(backup)
                destination_moved = False
        finally:
            if destination_moved and not (destination.exists() or destination.is_symlink()):
                backup.replace(destination)
            shutil.rmtree(temporary, ignore_errors=True)
    return changed


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--destination",
        type=Path,
        default=Path.home() / ".hermes" / "skills",
        help="Hermes skill root (default: ~/.hermes/skills)",
    )
    parser.add_argument("--force", action="store_true", help="Replace changed installed skills")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    repository_root = Path(__file__).resolve().parents[1]
    source_root = repository_root / "skills"
    count = install(
        source_root, args.destination.expanduser(), force=args.force, dry_run=args.dry_run
    )
    verb = "would install" if args.dry_run else "installed"
    print(f"{verb} {count} skill directories")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
