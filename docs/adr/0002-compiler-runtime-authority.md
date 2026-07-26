# ADR-0002: Compilers and runtime are acceptance authorities

Status: accepted

Static analysis cannot reproduce every branch-specific compiler and game behavior. Transformations require exact compiler validation and, when gameplay or rendering can change, deterministic runtime scenarios.
