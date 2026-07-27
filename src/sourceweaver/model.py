"""Stable report and provenance models."""

from __future__ import annotations

import hashlib
import os
from datetime import UTC, datetime
from enum import StrEnum
from pathlib import Path
from typing import Any

from pydantic import BaseModel, ConfigDict, Field

from sourceweaver.errors import ArtifactChangedError


class Severity(StrEnum):
    INFO = "info"
    WARNING = "warning"
    ERROR = "error"
    BLOCKER = "blocker"


class Confidence(StrEnum):
    PROVEN = "proven"
    HIGH = "high"
    MEDIUM = "medium"
    LOW = "low"
    UNKNOWN = "unknown"


class Diagnostic(BaseModel):
    model_config = ConfigDict(frozen=True)

    code: str
    severity: Severity
    message: str
    evidence: list[str] = Field(default_factory=list)
    remediation: str | None = None
    confidence: Confidence = Confidence.PROVEN
    object_refs: list[str] = Field(default_factory=list)


class ArtifactFingerprint(BaseModel):
    model_config = ConfigDict(frozen=True)

    path: str
    size: int
    sha256: str

    @classmethod
    def from_bytes(cls, data: bytes, *, path: str | Path) -> ArtifactFingerprint:
        """Fingerprint the exact bytes already consumed by an analysis stage."""
        return cls(path=str(path), size=len(data), sha256=hashlib.sha256(data).hexdigest())

    @classmethod
    def from_path(cls, path: str | Path) -> ArtifactFingerprint:
        """Fingerprint one stable file and reject mid-read modifications."""
        source = Path(path)
        digest = hashlib.sha256()
        bytes_read = 0

        with source.open("rb") as stream:
            before = os.fstat(stream.fileno())
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                bytes_read += len(chunk)
                digest.update(chunk)
            after = os.fstat(stream.fileno())

        identity_before = (
            before.st_dev,
            before.st_ino,
            before.st_size,
            before.st_mtime_ns,
            before.st_ctime_ns,
        )
        identity_after = (
            after.st_dev,
            after.st_ino,
            after.st_size,
            after.st_mtime_ns,
            after.st_ctime_ns,
        )
        try:
            current = source.stat()
        except OSError as exc:
            raise ArtifactChangedError(
                f"Artifact path disappeared while being fingerprinted: {source}"
            ) from exc
        identity_current = (
            current.st_dev,
            current.st_ino,
            current.st_size,
            current.st_mtime_ns,
            current.st_ctime_ns,
        )
        if (
            identity_before != identity_after
            or identity_before != identity_current
            or bytes_read != before.st_size
        ):
            raise ArtifactChangedError(f"Artifact changed while being fingerprinted: {source}")

        return cls(path=str(source), size=bytes_read, sha256=digest.hexdigest())


class AnalysisReport(BaseModel):
    model_config = ConfigDict(frozen=True)

    schema_version: str = "1.0"
    generated_at: datetime = Field(default_factory=lambda: datetime.now(UTC))
    source: ArtifactFingerprint
    encoding: str
    newline: str
    top_level_blocks: list[str]
    diagnostics: list[Diagnostic] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)
