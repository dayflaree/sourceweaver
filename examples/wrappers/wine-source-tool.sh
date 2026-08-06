#!/usr/bin/env bash
set -euo pipefail

# Generic Wine wrapper for Source compiler tools.
# Copy or symlink this script once per tool name, then set SOURCE_TOOL_EXE.
# Example:
#   SOURCE_TOOL_EXE="$HOME/.steam/steam/steamapps/common/Half-Life 2/bin/vbsp.exe" \
#     examples/wrappers/wine-source-tool.sh -game "$HOME/.steam/steam/steamapps/common/Half-Life 2/hl2" map.vmf

: "${SOURCE_TOOL_EXE:?set SOURCE_TOOL_EXE to vbsp.exe, vvis.exe, or vrad.exe}"
: "${WINE:=wine}"

exec "$WINE" "$SOURCE_TOOL_EXE" "$@"
