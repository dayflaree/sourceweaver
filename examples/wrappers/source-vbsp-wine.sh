#!/usr/bin/env bash
set -euo pipefail
: "${SOURCE_SDK_BIN:?set SOURCE_SDK_BIN to the directory containing vbsp.exe}"
SOURCE_TOOL_EXE="$SOURCE_SDK_BIN/vbsp.exe" exec "$(dirname "$0")/wine-source-tool.sh" "$@"
