# ADR-0001: Lossless CST is the write authority

Status: accepted

Semantic VMF exporters reconstruct known structures and may omit unknown editor extensions or formatting. SourceWeaver retains an exact concrete syntax tree and applies span-based edits. Semantic libraries remain analysis adapters.
