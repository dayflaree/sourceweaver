#!/usr/bin/env bash
set -euo pipefail

cargo audit \
  --deny warnings \
  --ignore RUSTSEC-2024-0436 \
  --ignore RUSTSEC-2026-0192
