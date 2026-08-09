#!/usr/bin/env bash
set -euo pipefail

mode="${SOURCEWEAVER_RELEASE_MODE:-preview}"
case "$mode" in
  preview)
    echo "preview release mode: production signing credentials are optional."
    exit 0
    ;;
  final)
    ;;
  *)
    echo "SOURCEWEAVER_RELEASE_MODE must be 'preview' or 'final'; got '$mode'." >&2
    exit 1
    ;;
esac

required_names=(
  SOURCEWEAVER_WINDOWS_SIGNING_PFX_BASE64
  SOURCEWEAVER_WINDOWS_SIGNING_PFX_PASSWORD
  SOURCEWEAVER_WINDOWS_TIMESTAMP_URL
  SOURCEWEAVER_WINDOWS_SIGNTOOL
  SOURCEWEAVER_GPG_PRIVATE_KEY_BASE64
  SOURCEWEAVER_UPDATE_SIGNING_KEY_BASE64
)

missing=()
for name in "${required_names[@]}"; do
  if [[ -z "${!name:-}" ]]; then
    missing+=("$name")
  fi
done

if (( ${#missing[@]} > 0 )); then
  printf 'final release mode requires configured signing secret/variable names; missing:' >&2
  printf ' %s' "${missing[@]}" >&2
  printf '\n' >&2
  exit 1
fi

echo "final release mode signing policy satisfied; secret values were not printed."
