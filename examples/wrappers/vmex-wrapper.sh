#!/usr/bin/env bash
# Example VMEX wrapper for Source Weaver generic BSP import mode.
#
# Usage from Source Weaver:
#   sourceweaver bsp-import --tool examples/wrappers/vmex-wrapper.sh input.bsp --output output.vmf
#
# Source Weaver invokes generic wrappers as:
#   <wrapper> [tool-args] <input.bsp> <out.vmf>
#
# VMEX is obsolete and user-provided. This example relies on the Valve Developer
# Union documentation that VMEX can be run as `vmex <map.bsp>` and writes a
# decompiled VMF with `_d` appended to the map name. This wrapper was not
# validated with a real VMEX binary in CI.

set -euo pipefail

if [[ "$#" -lt 2 ]]; then
  echo "usage: vmex-wrapper.sh [vmex args...] <input.bsp> <out.vmf>" >&2
  exit 64
fi

out_vmf="${@: -1}"
input_bsp="${@: -2:1}"
extra_args=("${@:1:$#-2}")
vmex_bin="${VMEX_BIN:-vmex}"

input_dir="$(dirname -- "$input_bsp")"
input_base="$(basename -- "$input_bsp")"
input_stem="${input_base%.*}"
expected_vmf="$input_dir/${input_stem}_d.vmf"

"$vmex_bin" "${extra_args[@]}" "$input_bsp"

if [[ ! -f "$expected_vmf" ]]; then
  echo "VMEX did not create expected output $expected_vmf" >&2
  exit 65
fi

mkdir -p "$(dirname -- "$out_vmf")"
mv -- "$expected_vmf" "$out_vmf"
