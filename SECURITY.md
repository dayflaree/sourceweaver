# Security policy

## Reporting

Use a private GitHub security advisory for vulnerabilities involving arbitrary file parsing, command execution, path traversal, unsafe compiler invocation, secret exposure, or generated runtime code.

## Untrusted inputs

Treat VMFs, FGDs, scripts, compiler logs, paths, archives, and asset files as untrusted. SourceWeaver must:

- avoid shell interpolation;
- reject path traversal outside configured roots;
- enforce input and output size limits;
- bound parser recursion and geometry workloads;
- isolate generated work directories;
- never execute map-provided commands during analysis;
- redact local paths and secrets in public reports.

## Supported versions

The project is pre-alpha. Security fixes are applied to the latest `main` branch until formal releases begin.
