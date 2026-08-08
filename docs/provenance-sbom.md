# Release provenance and SBOM

Source Weaver release provenance is a credential-free supply-chain layer for preview releases. It complements checksums, OpenPGP checksum signatures, Windows Authenticode signatures, and signed update manifests. It does not replace any of them.

## What the release workflow produces

`.github/workflows/desktop-builds.yml` now separates package building from provenance generation:

1. Linux and Windows packaging jobs build the release artifacts.
2. A provenance job downloads those package artifacts, generates `sourceweaver-sbom.cdx.json`, creates `SHA256SUMS`, uploads those files as a workflow artifact, and asks GitHub to attest every file in `target/release-artifacts/*`.
3. On `v*` tags, the release job downloads packages plus provenance files, optionally signs `SHA256SUMS`, optionally creates `sourceweaver-update-manifest.json`, attests the final release files, and publishes the GitHub Release.

Artifact attestations are generated for the files that exist in that run. This includes:

- Linux tarball;
- Linux AppImage;
- Windows portable zip;
- Windows NSIS setup executable;
- `SHA256SUMS`;
- `sourceweaver-sbom.cdx.json`;
- `SHA256SUMS.asc` when an OpenPGP key is configured;
- `sourceweaver-update-manifest.json` when an update-signing key is configured.

Unsigned update metadata is still refused. Artifact attestations do not make unsigned update metadata acceptable.

## SBOM format

`scripts/generate-release-sbom.py` writes a CycloneDX 1.5 JSON SBOM named `sourceweaver-sbom.cdx.json`. The generator uses:

```bash
cargo metadata --locked --format-version 1
```

The SBOM includes workspace crates, third-party Cargo packages, package versions, package URLs, license expressions when available, upstream repository/homepage/documentation references when available, and Cargo dependency edges.

Generate and validate it locally with:

```bash
scripts/generate-release-sbom.py --output /tmp/sourceweaver-sbom.cdx.json
scripts/generate-release-sbom.py --validate-only /tmp/sourceweaver-sbom.cdx.json
python3 -m json.tool /tmp/sourceweaver-sbom.cdx.json >/dev/null
```

## Attestation verification

GitHub artifact attestations bind artifact names and digests to build provenance. They are verified with GitHub CLI against the repository that produced them:

```bash
gh attestation verify sourceweaver-vX.Y.Z-linux-x86_64.tar.gz --repo dayflaree/sourceweaver
gh attestation verify sourceweaver-vX.Y.Z-linux-x86_64.AppImage --repo dayflaree/sourceweaver
gh attestation verify sourceweaver-vX.Y.Z-windows-x86_64.zip --repo dayflaree/sourceweaver
gh attestation verify sourceweaver-vX.Y.Z-windows-x86_64-setup.exe --repo dayflaree/sourceweaver
gh attestation verify SHA256SUMS --repo dayflaree/sourceweaver
gh attestation verify sourceweaver-sbom.cdx.json --repo dayflaree/sourceweaver
```

When optional files are present, verify them too:

```bash
gh attestation verify SHA256SUMS.asc --repo dayflaree/sourceweaver
gh attestation verify sourceweaver-update-manifest.json --repo dayflaree/sourceweaver
```

Verification proves the artifact matches a GitHub Actions attestation for this repository. It does not prove Windows publisher identity, OpenPGP key ownership, update-manifest signature validity, or game/editor/tool compatibility.

## Mechanism boundaries

| Mechanism | Proves | Does not prove |
| --- | --- | --- |
| `SHA256SUMS` | Downloaded bytes match the release checksum manifest. | Publisher identity if an attacker can change both artifact and manifest. |
| `SHA256SUMS.asc` | The checksum manifest was signed by the holder of the published OpenPGP key. | Windows publisher identity or update-manifest validity. |
| Windows Authenticode | The Windows executable/installer was signed by the certificate used in that release run. | Linux artifact integrity, SBOM completeness, or Source tool compatibility. |
| Signed update manifest | Update metadata and target artifact hashes were signed by the Ed25519 update key. | Automatic install/rollback, Authenticode signing, or OpenPGP signing. |
| GitHub artifact attestation | The file digest is associated with GitHub Actions provenance for `dayflaree/sourceweaver`. | Authenticode trust, OpenPGP trust, update-manifest signature validity, or malware-free status. |
| CycloneDX SBOM | The Rust workspace dependency inventory from `cargo metadata --locked` at build time. | Binary reproducibility, vulnerability absence, bundled non-Rust system libraries, or proprietary game-content review. |

## Release evidence checklist

For each release, record:

- which artifacts were attested;
- `gh attestation verify` command output for each published artifact when network access and repository permissions allow it;
- `sha256sum -c SHA256SUMS` output;
- `gpg --verify SHA256SUMS.asc SHA256SUMS` output when `SHA256SUMS.asc` exists;
- Authenticode verification output when Windows signing was configured;
- `sourceweaver update check --manifest sourceweaver-update-manifest.json --public-key <key>` output when update metadata exists;
- `scripts/generate-release-sbom.py --validate-only sourceweaver-sbom.cdx.json` output;
- the exact release workflow run URL and commit SHA.

Generated provenance files and SBOMs may be published as release artifacts because they contain repository/build metadata and Cargo package metadata, not proprietary game content. Review any future non-Rust SBOM expansion before publishing to ensure private paths and third-party redistribution boundaries remain respected.
