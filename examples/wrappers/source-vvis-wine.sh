#!/usr/bin/env bash
set -euo pipefail
: "${SOURCE_SDK_BIN:?set SOURCE_SDK_BIN to the directory containing vvis.exe}"
SOURCE_TOOL_EXE="$SOURCE_SDK_BIN/vvis.exe" exec "$(dirname "$0")/wine-source-tool.sh" "$@"
