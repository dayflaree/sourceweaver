#!/usr/bin/env bash
# Source Weaver model decompile wrapper example.
#
# This template is for users who already have a legal local headless model
# decompiler or their own automation around a GUI/manual tool. Source Weaver
# does not bundle Crowbar, StudioMDL, model decompilers, SDK files, game models,
# or game content.

set -euo pipefail

: "${SOURCEWEAVER_MODEL_DECOMPILER:?set SOURCEWEAVER_MODEL_DECOMPILER to your headless model decompiler executable}"

input=""
output_dir=""
game_dir=""
extra_args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --input)
      input="${2:?--input needs a path}"
      shift 2
      ;;
    --output|--output-dir)
      output_dir="${2:?--output needs a directory}"
      shift 2
      ;;
    --game|--game-dir)
      game_dir="${2:?--game needs a directory}"
      shift 2
      ;;
    *)
      extra_args+=("$1")
      shift
      ;;
  esac
done

: "${input:?missing --input}"
: "${output_dir:?missing --output-dir}"
mkdir -p "$output_dir"

if [[ -n "$game_dir" ]]; then
  exec "$SOURCEWEAVER_MODEL_DECOMPILER" "${extra_args[@]}" --game "$game_dir" --input "$input" --output "$output_dir"
else
  exec "$SOURCEWEAVER_MODEL_DECOMPILER" "${extra_args[@]}" --input "$input" --output "$output_dir"
fi
