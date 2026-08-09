#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

expect_failure() {
  local label="$1"
  shift
  if "$@" >"$tmp_dir/${label}.out" 2>&1; then
    echo "expected failure but command succeeded: $label" >&2
    cat "$tmp_dir/${label}.out" >&2
    exit 1
  fi
}

expect_success() {
  local label="$1"
  shift
  if ! "$@" >"$tmp_dir/${label}.out" 2>&1; then
    echo "expected success but command failed: $label" >&2
    cat "$tmp_dir/${label}.out" >&2
    exit 1
  fi
}

expect_success preview-without-secrets env \
  -u SOURCEWEAVER_WINDOWS_SIGNING_PFX_BASE64 \
  -u SOURCEWEAVER_WINDOWS_SIGNING_PFX_PASSWORD \
  -u SOURCEWEAVER_WINDOWS_TIMESTAMP_URL \
  -u SOURCEWEAVER_WINDOWS_SIGNTOOL \
  -u SOURCEWEAVER_GPG_PRIVATE_KEY_BASE64 \
  -u SOURCEWEAVER_UPDATE_SIGNING_KEY_BASE64 \
  SOURCEWEAVER_RELEASE_MODE=preview \
  scripts/validate-final-release-environment.sh

expect_failure final-without-secrets env \
  -u SOURCEWEAVER_WINDOWS_SIGNING_PFX_BASE64 \
  -u SOURCEWEAVER_WINDOWS_SIGNING_PFX_PASSWORD \
  -u SOURCEWEAVER_WINDOWS_TIMESTAMP_URL \
  -u SOURCEWEAVER_WINDOWS_SIGNTOOL \
  -u SOURCEWEAVER_GPG_PRIVATE_KEY_BASE64 \
  -u SOURCEWEAVER_UPDATE_SIGNING_KEY_BASE64 \
  SOURCEWEAVER_RELEASE_MODE=final \
  scripts/validate-final-release-environment.sh

for required in \
  SOURCEWEAVER_WINDOWS_SIGNING_PFX_BASE64 \
  SOURCEWEAVER_WINDOWS_SIGNING_PFX_PASSWORD \
  SOURCEWEAVER_WINDOWS_TIMESTAMP_URL \
  SOURCEWEAVER_WINDOWS_SIGNTOOL \
  SOURCEWEAVER_GPG_PRIVATE_KEY_BASE64 \
  SOURCEWEAVER_UPDATE_SIGNING_KEY_BASE64; do
  grep -Fq "$required" "$tmp_dir/final-without-secrets.out"
done

expect_success final-with-dummy-secret-names env \
  SOURCEWEAVER_RELEASE_MODE=final \
  SOURCEWEAVER_WINDOWS_SIGNING_PFX_BASE64=dummy-pfx-base64 \
  SOURCEWEAVER_WINDOWS_SIGNING_PFX_PASSWORD=dummy-password \
  SOURCEWEAVER_WINDOWS_TIMESTAMP_URL=https://timestamp.example.invalid \
  SOURCEWEAVER_WINDOWS_SIGNTOOL=signtool.exe \
  SOURCEWEAVER_GPG_PRIVATE_KEY_BASE64=dummy-gpg-key \
  SOURCEWEAVER_UPDATE_SIGNING_KEY_BASE64=dummy-update-key \
  scripts/validate-final-release-environment.sh

printf 'release artifact\n' > "$tmp_dir/artifact.txt"
(
  cd "$tmp_dir"
  sha256sum artifact.txt > SHA256SUMS
)
expect_failure openpgp-required-without-key env \
  -u SOURCEWEAVER_GPG_PRIVATE_KEY_BASE64 \
  -u SOURCEWEAVER_GPG_PASSPHRASE \
  SOURCEWEAVER_REQUIRE_RELEASE_SIGNATURES=1 \
  scripts/sign-release-checksums.sh "$tmp_dir"
grep -Fq 'SOURCEWEAVER_GPG_PRIVATE_KEY_BASE64' "$tmp_dir/openpgp-required-without-key.out"

grep -Fq 'release_mode:' .github/workflows/desktop-builds.yml
grep -Fq 'SOURCEWEAVER_REQUIRE_WINDOWS_SIGNING: ${{ needs.release_policy.outputs.require_release_signatures }}' .github/workflows/desktop-builds.yml
grep -Fq -- '-RequireSigning' .github/workflows/desktop-builds.yml
grep -Fq 'SOURCEWEAVER_REQUIRE_RELEASE_SIGNATURES: ${{ needs.release_policy.outputs.require_release_signatures }}' .github/workflows/desktop-builds.yml
grep -Fq 'SOURCEWEAVER_UPDATE_SIGNING_KEY_BASE64: ${{ secrets.SOURCEWEAVER_UPDATE_SIGNING_KEY_BASE64 }}' .github/workflows/desktop-builds.yml
grep -Fq 'prerelease: ${{ needs.release_policy.outputs.prerelease }}' .github/workflows/desktop-builds.yml
grep -Fq 'make_latest: ${{ needs.release_policy.outputs.make_latest }}' .github/workflows/desktop-builds.yml

echo "final release policy validation passed"
