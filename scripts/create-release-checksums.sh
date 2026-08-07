#!/usr/bin/env bash
set -euo pipefail

artifact_dir="${1:-target/release-artifacts}"
checksum_file="$artifact_dir/SHA256SUMS"

if [[ ! -d "$artifact_dir" ]]; then
  echo "artifact directory does not exist: $artifact_dir" >&2
  exit 1
fi

tmp_file="$(mktemp)"
trap 'rm -f "$tmp_file"' EXIT

(
  cd "$artifact_dir"
  find . -maxdepth 1 -type f \
    ! -name 'SHA256SUMS' \
    ! -name 'SHA256SUMS.asc' \
    ! -name '*.sig' \
    ! -name '*.asc' \
    -printf '%P\0' |
    sort -z |
    xargs -0r sha256sum
) > "$tmp_file"

if [[ ! -s "$tmp_file" ]]; then
  echo "no release artifacts found in $artifact_dir" >&2
  exit 1
fi

mv "$tmp_file" "$checksum_file"
trap - EXIT

echo "$checksum_file"
