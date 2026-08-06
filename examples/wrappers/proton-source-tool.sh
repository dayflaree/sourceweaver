#!/usr/bin/env bash
set -euo pipefail

# Generic Proton wrapper for Source compiler tools.
# Proton command-line use depends on the user's legal Steam install and chosen compatdata prefix.
# Set PROTON to the Proton executable/script, STEAM_COMPAT_DATA_PATH to a writable compatdata dir,
# and SOURCE_TOOL_EXE to vbsp.exe, vvis.exe, or vrad.exe.

: "${PROTON:?set PROTON to a Proton executable, for example .../Proton 9.0/proton}"
: "${STEAM_COMPAT_DATA_PATH:?set STEAM_COMPAT_DATA_PATH to the selected compatdata directory}"
: "${SOURCE_TOOL_EXE:?set SOURCE_TOOL_EXE to vbsp.exe, vvis.exe, or vrad.exe}"
: "${STEAM_COMPAT_CLIENT_INSTALL_PATH:=$HOME/.steam/steam}"

export STEAM_COMPAT_DATA_PATH
export STEAM_COMPAT_CLIENT_INSTALL_PATH

exec "$PROTON" run "$SOURCE_TOOL_EXE" "$@"
