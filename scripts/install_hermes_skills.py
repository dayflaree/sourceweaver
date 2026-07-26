#!/usr/bin/env python3
"""Install SourceWeaver project skills into a Hermes skill directory."""

from __future__ import annotations

import argparse
import hashlib
import shutil
from pathlib import Path


def tree_digest(path: Path) -> str:
    """Return a stable digest for all regular files in *path*."""
    digest = hashlib.sha256()
    for file_path in sorted(candidate for candidate in path.rglob("*") if candidate.is_file()):
        digest.update(file_path.relative_to(path).as_posix().encode("utf-8"))
        digest.update(b"\0")
        digest.update(file_path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def install(source_root: Path, destination_root: Path, *, force: bool, dry_run: bool) -> int:
    """Install all ``sourceweaver-*`` skill directories."""
    destination_root.mkdir(parents=True, exist_ok=True)
    installed = 0
    for source in sorted(source_root.glob("sourceweaver-*")):
        if not source.is_dir() or not (source / "SKILL.md").is_file():
            continue
        destination = destination_root / source.name
        source_hash = tree_digest(source)
        if destination.exists():
            destination_hash = tree_digest(destination)
            if destination_hash == source_hash:
                print(f"up to date: {source.name} ({source_hash[:12]})")
                continue
            if not force:
                raise SystemExit(
                    f"refusing to replace changed skill {destination}; rerun with --force"
                )
        print(f"install: {source.name} -> {destination} ({source_hash[:12]})")
        if dry_run:
            continue
        if destination.exists():
            shutil.rmtree(destination)
        shutil.copytree(source, destination)
        installed += 1
    return installed


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
    print(f"installed {count} skill directories")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
