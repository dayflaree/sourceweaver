"""SourceWeaver command-line interface."""

from __future__ import annotations

import json
import platform
from pathlib import Path
from typing import Annotated, NoReturn, cast

import typer
from rich.console import Console
from rich.table import Table

from sourceweaver.analysis import analyze_vmf
from sourceweaver.compiler import (
    discover_compilers,
    discover_gmod_root,
    executable_format,
    host_compatibility,
)
from sourceweaver.errors import SourceWeaverError, UnsafeOutputError
from sourceweaver.fileio import atomic_write_bytes, paths_refer_to_same_file
from sourceweaver.version import __version__
from sourceweaver.vmf.document import VmfDocument

app = typer.Typer(
    name="sourceweaver",
    help="Analyze and transform VMFs through auditable, compiler-verified workflows.",
    no_args_is_help=True,
)
console = Console()
error_console = Console(stderr=True)


def _version(value: bool) -> None:
    if value:
        typer.echo(__version__)
        raise typer.Exit


def _fail(message: str, *, code: int) -> NoReturn:
    error_console.print(f"[bold red]error:[/] {message}")
    raise typer.Exit(code)


@app.callback()
def main(
    _version_option: Annotated[
        bool,
        typer.Option("--version", callback=_version, is_eager=True, help="Show version and exit."),
    ] = False,
) -> None:
    """SourceWeaver CLI."""


@app.command()
def inspect(
    vmf: Annotated[Path, typer.Argument(exists=True, dir_okay=False, readable=True)],
    json_output: Annotated[
        bool, typer.Option("--json", help="Emit the stable JSON report.")
    ] = False,
) -> None:
    """Run non-mutating structural analysis on a VMF."""
    try:
        report = analyze_vmf(vmf)
    except (SourceWeaverError, OSError, UnicodeError) as exc:
        _fail(str(exc), code=2)

    if json_output:
        typer.echo(report.model_dump_json(indent=2))
        return

    table = Table(title=f"SourceWeaver inspection: {vmf.name}")
    table.add_column("Field")
    table.add_column("Value")
    table.add_row("SHA-256", report.source.sha256)
    table.add_row("Encoding", report.encoding)
    table.add_row("Newlines", report.newline)
    table.add_row("Lossless round-trip", str(report.metadata["lossless_roundtrip"]))
    table.add_row("Top-level blocks", ", ".join(report.top_level_blocks))
    table.add_row("Diagnostics", str(len(report.diagnostics)))
    console.print(table)
    for diagnostic in report.diagnostics:
        console.print(f"[{diagnostic.severity.value}] {diagnostic.code}: {diagnostic.message}")


@app.command()
def roundtrip(
    vmf: Annotated[Path, typer.Argument(exists=True, dir_okay=False, readable=True)],
    output: Annotated[Path, typer.Option("--output", "-o", help="Output path.")],
    force: Annotated[
        bool,
        typer.Option("--force", help="Replace an existing generated output, never the input VMF."),
    ] = False,
) -> None:
    """Parse and write the exact original VMF bytes as an integrity test."""
    try:
        if paths_refer_to_same_file(vmf, output):
            raise UnsafeOutputError(
                "The output path identifies the input VMF; source files are immutable."
            )
        document = VmfDocument.read(vmf)
        rendered = document.render_bytes()
        if rendered != document.raw_bytes:
            _fail("Lossless invariant failed; output was not written.", code=3)
        atomic_write_bytes(output, rendered, overwrite=force)
    except (SourceWeaverError, OSError, UnicodeError) as exc:
        _fail(str(exc), code=3)
    console.print(f"Wrote byte-identical VMF: {output}")


@app.command()
def doctor(
    gmod_root: Annotated[
        Path | None,
        typer.Option("--gmod-root", help="Garry's Mod installation root."),
    ] = None,
    json_output: Annotated[bool, typer.Option("--json", help="Emit stable JSON.")] = False,
    strict: Annotated[
        bool,
        typer.Option("--strict", help="Exit non-zero unless all four compilers are present."),
    ] = False,
) -> None:
    """Discover, fingerprint, and classify the exact compiler toolchain."""
    try:
        root = discover_gmod_root(gmod_root)
        if root is None:
            _fail("Garry's Mod installation was not found.", code=4)
        compilers = discover_compilers(root)
        fingerprints = compilers.fingerprints()
        compiler_payload: dict[str, object] = {}
        for name, path in compilers.items():
            if path is None:
                compiler_payload[name] = None
                continue
            fingerprint = fingerprints[name]
            if fingerprint is None:
                raise SourceWeaverError(f"Compiler disappeared before fingerprinting: {path}")
            kind = executable_format(path)
            compiler_payload[name] = {
                **fingerprint.model_dump(),
                "format": kind,
                "host_compatibility": host_compatibility(kind),
            }
    except (SourceWeaverError, OSError) as exc:
        _fail(str(exc), code=4)

    payload = {
        "gmod_root": str(root),
        "host": {"system": platform.system(), "machine": platform.machine()},
        "complete": compilers.complete,
        "compilers": compiler_payload,
    }
    if json_output:
        typer.echo(json.dumps(payload, indent=2))
    else:
        table = Table(title="SourceWeaver compiler doctor")
        table.add_column("Compiler")
        table.add_column("Path")
        table.add_column("Format")
        table.add_column("Host compatibility")
        table.add_column("SHA-256")
        for name, details in compiler_payload.items():
            if details is None:
                table.add_row(name, "missing", "-", "-", "-")
                continue
            details_map = cast(dict[str, object], details)
            table.add_row(
                name,
                str(details_map["path"]),
                str(details_map["format"]),
                str(details_map["host_compatibility"]),
                str(details_map["sha256"]),
            )
        console.print(table)

    if strict and not compilers.complete:
        raise typer.Exit(5)


if __name__ == "__main__":
    app()
