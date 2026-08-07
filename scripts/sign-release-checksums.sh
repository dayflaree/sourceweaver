#!/usr/bin/env bash
set -euo pipefail

artifact_dir="${1:-target/release-artifacts}"
checksum_file="$artifact_dir/SHA256SUMS"
signature_file="$artifact_dir/SHA256SUMS.asc"

require_signatures="${SOURCEWEAVER_REQUIRE_RELEASE_SIGNATURES:-0}"
private_key_b64="${SOURCEWEAVER_GPG_PRIVATE_KEY_BASE64:-}"
passphrase="${SOURCEWEAVER_GPG_PASSPHRASE:-}"

if [[ ! -f "$checksum_file" ]]; then
  echo "checksum manifest does not exist: $checksum_file" >&2
  exit 1
fi

if [[ -z "$private_key_b64" ]]; then
  if [[ "$require_signatures" == "1" || "$require_signatures" == "true" || "$require_signatures" == "TRUE" ]]; then
    echo "SOURCEWEAVER_GPG_PRIVATE_KEY_BASE64 is required when SOURCEWEAVER_REQUIRE_RELEASE_SIGNATURES is set" >&2
    exit 1
  fi
  echo "SOURCEWEAVER_GPG_PRIVATE_KEY_BASE64 is not configured; skipped OpenPGP detached signature."
  exit 0
fi

if ! command -v gpg >/dev/null 2>&1; then
  echo "gpg was not found on PATH" >&2
  exit 1
fi

export GNUPGHOME
GNUPGHOME="$(mktemp -d)"
key_file="$(mktemp)"
cleanup() {
  rm -f "$key_file"
  rm -rf "$GNUPGHOME"
}
trap cleanup EXIT
chmod 700 "$GNUPGHOME"

printf '%s' "$private_key_b64" | base64 --decode > "$key_file"
gpg --batch --import "$key_file" >/dev/null

sign_args=(--batch --yes --armor --detach-sign --output "$signature_file")
if [[ -n "$passphrase" ]]; then
  sign_args+=(--pinentry-mode loopback --passphrase "$passphrase")
fi
sign_args+=("$checksum_file")

gpg "${sign_args[@]}"
gpg --batch --verify "$signature_file" "$checksum_file" >/dev/null

echo "$signature_file"
