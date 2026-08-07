#!/usr/bin/env bash
# Source Weaver BSPZIP-compatible explicit -game wrapper example.
#
# Use this only with a packer or wrapper that you have verified accepts
# `-game <dir>` before `-addlist`. Stock BSPZIP argument support varies by
# branch/tool build. Source Weaver keeps this as an opt-in context shape.

set -euo pipefail

: "${SOURCEWEAVER_BSPZIP_TOOL:?set SOURCEWEAVER_BSPZIP_TOOL to a BSPZIP-compatible executable}"
: "${SOURCEWEAVER_GAME_DIR:?set SOURCEWEAVER_GAME_DIR to the target game/content directory}"

exec "$SOURCEWEAVER_BSPZIP_TOOL" -game "$SOURCEWEAVER_GAME_DIR" "$@"
