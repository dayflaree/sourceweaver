#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

required_fields=(
  "third_party_policy_review:"
  "name:"
  "category:"
  "upstream_url:"
  "version_or_commit:"
  "license:"
  "dependency_licenses:"
  "redistribution_allowed:"
  "attribution_required:"
  "provenance_source:"
  "sha256:"
  "size_bytes:"
  "update_policy:"
  "removal_policy:"
  "user_consent_text:"
  "validation_evidence:"
  "reviewer:"
  "decision:"
)

failures=0

error() {
  printf 'third-party policy review check: %s\n' "$*" >&2
  failures=$((failures + 1))
}

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    error "missing required file: $path"
  fi
}

require_grep() {
  local pattern="$1"
  local path="$2"
  local message="$3"
  if ! grep -Eq "$pattern" "$path"; then
    error "$message"
  fi
}

check_review_block() {
  local path="$1"
  local field
  for field in "${required_fields[@]}"; do
    if ! grep -Eq "^[[:space:]]*${field}" "$path"; then
      error "$path is missing review field: $field"
    fi
  done
}

require_file ".github/ISSUE_TEMPLATE/third_party_policy_review.yml"
require_file "docs/third-party-redistribution-policy.md"
require_file "docs/bspsource-managed-download.md"

check_review_block ".github/ISSUE_TEMPLATE/third_party_policy_review.yml"
check_review_block "docs/bspsource-managed-download.md"

require_grep 'Completed reviews live in either:' "docs/third-party-redistribution-policy.md" \
  "docs/third-party-redistribution-policy.md must tell maintainers where completed reviews live"
require_grep 'unknown license/provenance means `decision: deferred`' "docs/third-party-redistribution-policy.md" \
  "docs/third-party-redistribution-policy.md must keep the unknown-license default explicit"
require_grep '^\| BSPSource `v1\.4\.8` ZIP \| Managed download helper exists' "docs/third-party-redistribution-policy.md" \
  "BSPSource must remain the documented managed-download candidate"
require_grep 'decision: approved' "docs/bspsource-managed-download.md" \
  "BSPSource managed-download review must record an approved decision"
require_grep 'redistribution_allowed: yes' "docs/bspsource-managed-download.md" \
  "BSPSource managed-download review must record redistribution_allowed: yes for checksum-verified user download/cache"

if grep -E '^\| [^|]+ \| Managed download helper exists' "docs/third-party-redistribution-policy.md" | grep -Ev '^\| BSPSource `v1\.4\.8` ZIP \|' >/dev/null; then
  error "BSPSource must remain the only approved managed-download helper unless another completed review is added"
fi

while IFS= read -r -d '' path; do
  [[ "$path" == "docs/third-party-redistribution-policy.md" ]] && continue
  check_review_block "$path"
done < <(find docs -type f \( -name '*managed-download*.md' -o -name '*redistributable*.md' \) -print0)

if (( failures > 0 )); then
  exit 1
fi

printf 'third-party policy review check passed\n'
