"""SourceWeaver command-line interface."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Annotated

import typer
from rich.console import Console
from rich.table import Table

from sourceweaver.analysis import analyze_vmf
from sourceweaver.compiler import discover_compilers, discover_gmod_root
from sourceweaver.errors import SourceWeaverError
from sourceweaver.version import __version__
from sourceweaver.vmf.document import VmfDocument

app = typer.Typer(
    name="sourceweaver",
    help="Analyze and transform VMFs through auditable, compiler-verified workflows.",
    no_args_is_help=True,
)
console = Console()


def _version(value: bool) -> None:
    if value:
        console.print(__version__)
        raise typer.Exit


@app.callback()
def main(
    version: Annotated[
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
    except SourceWeaverError as exc:
        console.print(f"[bold red]error:[/] {exc}")
        raise typer.Exit(2) from exc

    if json_output:
        console.print_json(report.model_dump_json(indent=2))
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
) -> None:
    """Parse and write the exact original VMF bytes as an integrity test."""
    document = VmfDocument.read(vmf)
    rendered = document.render_bytes()
    if rendered != document.raw_bytes:
        console.print("[bold red]Lossless invariant failed; output was not written.[/]")
        raise typer.Exit(3)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(rendered)
    console.print(f"Wrote byte-identical VMF: {output}")


@app.command()
def doctor(
    gmod_root: Annotated[
        Path | None,
        typer.Option("--gmod-root", help="Garry's Mod installation root."),
    ] = None,
    json_output: Annotated[bool, typer.Option("--json", help="Emit JSON.")] = False,
) -> None:
    """Discover and fingerprint the exact compiler toolchain."""
    root = discover_gmod_root(gmod_root)
    if root is None:
        console.print("[bold red]Garry's Mod installation was not found.[/]")
        raise typer.Exit(4)
    compilers = discover_compilers(root)
    payload = {
        "gmod_root": str(root),
        "compilers": {
            name: fingerprint.model_dump() if fingerprint else None
            for name, fingerprint in compilers.fingerprints().items()
        },
    }
    if json_output:
        console.print_json(json.dumps(payload))
        return
    table = Table(title="SourceWeaver compiler doctor")
    table.add_column("Compiler")
    table.add_column("Path")
    table.add_column("SHA-256")
    for name, fingerprint in compilers.fingerprints().items():
        table.add_row(
            name,
            fingerprint.path if fingerprint else "missing",
            fingerprint.sha256 if fingerprint else "-",
        )
    console.print(table)


if __name__ == "__main__":
    app()
