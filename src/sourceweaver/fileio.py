"""Cross-platform, non-destructive file output helpers."""

from __future__ import annotations

import os
import tempfile
from pathlib import Path
from typing import Protocol

from sourceweaver.errors import UnsafeOutputError


class StatLike(Protocol):
    """Read-only fields used to compare file identity across platforms."""

    @property
    def st_dev(self) -> int: ...

    @property
    def st_ino(self) -> int: ...

    @property
    def st_size(self) -> int: ...

    @property
    def st_mtime_ns(self) -> int: ...

    @property
    def st_ctime_ns(self) -> int: ...


def stable_stat_identity(info: StatLike, *, platform_name: str | None = None) -> tuple[int, ...]:
    """Return metadata fields that are stable for one platform's stat APIs.

    Windows does not guarantee that file-index and creation-time fields exposed
    through ``fstat()`` and path ``stat()`` are represented identically on every
    filesystem. Size and last-write time are stable across both calls. POSIX
    platforms additionally use device, inode, and status-change time.
    """
    current_platform = os.name if platform_name is None else platform_name
    if current_platform == "nt":
        return (info.st_size, info.st_mtime_ns)
    return (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_ctime_ns)


def paths_refer_to_same_file(first: str | Path, second: str | Path) -> bool:
    """Return whether two paths identify the same file, including hard links."""
    left = Path(first)
    right = Path(second)
    try:
        if left.exists() and right.exists() and os.path.samefile(left, right):
            return True
    except OSError:
        pass
    left_key = os.path.normcase(os.path.abspath(os.path.realpath(left)))
    right_key = os.path.normcase(os.path.abspath(os.path.realpath(right)))
    return left_key == right_key


def _exclusive_direct_write(destination: Path, data: bytes) -> None:
    """Fallback for filesystems that cannot hard-link a prepared temp file."""
    descriptor = os.open(destination, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    created = True
    try:
        with os.fdopen(descriptor, "wb") as stream:
            descriptor = -1
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        created = False
    finally:
        if descriptor >= 0:
            os.close(descriptor)
        if created:
            destination.unlink(missing_ok=True)


def atomic_write_bytes(path: str | Path, data: bytes, *, overwrite: bool = False) -> Path:
    """Write bytes atomically while refusing accidental replacement by default."""
    destination = Path(path)
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.is_dir():
        raise UnsafeOutputError(f"Output path is a directory: {destination}")
    if destination.exists() and not overwrite:
        raise UnsafeOutputError(
            f"Output already exists: {destination}. Pass --force to replace generated output."
        )

    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb",
            prefix=f".{destination.name}.",
            suffix=".tmp",
            dir=destination.parent,
            delete=False,
        ) as stream:
            temporary = Path(stream.name)
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())

        if overwrite:
            os.replace(temporary, destination)
            temporary = None
            return destination

        try:
            # Both paths share a directory and filesystem. Linking publishes the
            # completed file atomically and fails without replacing an existing file.
            os.link(temporary, destination)
        except FileExistsError as exc:
            raise UnsafeOutputError(
                f"Output appeared during the write and was left untouched: {destination}"
            ) from exc
        except OSError:
            # FAT-like or restricted filesystems may not support hard links. An
            # exclusive direct write remains non-destructive, though less atomic.
            _exclusive_direct_write(destination, data)
        temporary.unlink()
        temporary = None
        return destination
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)
