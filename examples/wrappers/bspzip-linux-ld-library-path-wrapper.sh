#!/usr/bin/env bash
# Source Weaver BSPZIP Linux LD_LIBRARY_PATH wrapper example.
#
# This is an example for users who already have a local BSPZIP-compatible tool
# and a legal Source game/SDK install. Source Weaver does not bundle or validate
# BSPZIP, BSPZIP++, Source SDK files, Steam files, or game content.

set -euo pipefail

: "${SOURCEWEAVER_BSPZIP_BIN:?set SOURCEWEAVER_BSPZIP_BIN to the directory containing bspzip or a compatible packer}"
: "${SOURCEWEAVER_BSPZIP_EXE:=bspzip}"

# Add every local Source/Steam runtime library directory needed by your packer.
# Repeat paths with ':' on Linux.
SOURCEWEAVER_BSPZIP_LIBS="${SOURCEWEAVER_BSPZIP_LIBS:-$SOURCEWEAVER_BSPZIP_BIN}"

if [[ -n "${LD_LIBRARY_PATH:-}" ]]; then
  export LD_LIBRARY_PATH="$SOURCEWEAVER_BSPZIP_LIBS:$LD_LIBRARY_PATH"
else
  export LD_LIBRARY_PATH="$SOURCEWEAVER_BSPZIP_LIBS"
fi

cd "$SOURCEWEAVER_BSPZIP_BIN"
exec "$SOURCEWEAVER_BSPZIP_BIN/$SOURCEWEAVER_BSPZIP_EXE" "$@"
