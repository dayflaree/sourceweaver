use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateManifest {
    pub payload: UpdateManifestPayload,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateManifestPayload {
    pub schema_version: u32,
    pub app_id: String,
    pub channel: String,
    pub version: String,
    pub published_at: String,
    pub release_notes_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_required_version: Option<String>,
    #[serde(default)]
    pub artifacts: Vec<UpdateArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateArtifact {
    pub target: String,
    pub kind: String,
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateAvailability {
    Current,
    UpdateAvailable,
    DowngradeBlocked,
    ChannelMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheckResult {
    pub availability: UpdateAvailability,
    pub manifest: UpdateManifestPayload,
    pub selected_artifact: Option<UpdateArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheckOptions {
    pub current_version: String,
    pub channel: String,
    pub target: String,
    pub allow_downgrade: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateVerificationError {
    InvalidJson(String),
    UnsupportedSchema(u32),
    WrongAppId(String),
    MissingSignature,
    InvalidPublicKey,
    InvalidSignatureEncoding,
    SignatureVerificationFailed,
    MissingArtifact { target: String },
    ChannelMismatch { expected: String, actual: String },
    DowngradeBlocked { current: String, candidate: String },
    InvalidSha256 { value: String },
    Sha256Mismatch { expected: String, actual: String },
    SizeMismatch { expected: u64, actual: u64 },
}

impl std::fmt::Display for UpdateVerificationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(error) => write!(formatter, "invalid update manifest JSON: {error}"),
            Self::UnsupportedSchema(version) => {
                write!(
                    formatter,
                    "unsupported update manifest schema version {version}"
                )
            }
            Self::WrongAppId(app_id) => write!(
                formatter,
                "manifest app_id is `{app_id}`, expected `sourceweaver`"
            ),
            Self::MissingSignature => write!(formatter, "update manifest has no signature"),
            Self::InvalidPublicKey => write!(formatter, "invalid update public key"),
            Self::InvalidSignatureEncoding => {
                write!(formatter, "invalid update manifest signature encoding")
            }
            Self::SignatureVerificationFailed => {
                write!(formatter, "update manifest signature verification failed")
            }
            Self::MissingArtifact { target } => {
                write!(formatter, "manifest has no artifact for target `{target}`")
            }
            Self::ChannelMismatch { expected, actual } => {
                write!(
                    formatter,
                    "manifest channel `{actual}` does not match requested channel `{expected}`"
                )
            }
            Self::DowngradeBlocked { current, candidate } => {
                write!(
                    formatter,
                    "candidate version `{candidate}` is older than current version `{current}`"
                )
            }
            Self::InvalidSha256 { value } => write!(formatter, "invalid SHA-256 digest `{value}`"),
            Self::Sha256Mismatch { expected, actual } => {
                write!(
                    formatter,
                    "artifact SHA-256 mismatch: expected {expected}, got {actual}"
                )
            }
            Self::SizeMismatch { expected, actual } => {
                write!(
                    formatter,
                    "artifact size mismatch: expected {expected} bytes, got {actual} bytes"
                )
            }
        }
    }
}

impl std::error::Error for UpdateVerificationError {}

pub fn canonical_update_payload_bytes(
    payload: &UpdateManifestPayload,
) -> Result<Vec<u8>, UpdateVerificationError> {
    serde_json::to_vec(payload)
        .map_err(|error| UpdateVerificationError::InvalidJson(error.to_string()))
}

pub fn parse_update_manifest(text: &str) -> Result<UpdateManifest, UpdateVerificationError> {
    serde_json::from_str(text)
        .map_err(|error| UpdateVerificationError::InvalidJson(error.to_string()))
}

pub fn sign_update_manifest_payload(
    payload: &UpdateManifestPayload,
    private_key_base64: &str,
) -> Result<String, UpdateVerificationError> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use ed25519_dalek::{Signer, SigningKey};

    let key_bytes = STANDARD
        .decode(private_key_base64.trim())
        .map_err(|_| UpdateVerificationError::InvalidPublicKey)?;
    let key_bytes: [u8; 32] = key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| UpdateVerificationError::InvalidPublicKey)?;
    let signing_key = SigningKey::from_bytes(&key_bytes);
    let payload_bytes = canonical_update_payload_bytes(payload)?;
    let signature = signing_key.sign(&payload_bytes);
    Ok(STANDARD.encode(signature.to_bytes()))
}

pub fn verify_update_manifest_signature(
    manifest: &UpdateManifest,
    public_key_base64: &str,
) -> Result<(), UpdateVerificationError> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    if manifest.signature.trim().is_empty() {
        return Err(UpdateVerificationError::MissingSignature);
    }
    let public_key_bytes = STANDARD
        .decode(public_key_base64.trim())
        .map_err(|_| UpdateVerificationError::InvalidPublicKey)?;
    let public_key_bytes: [u8; 32] = public_key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| UpdateVerificationError::InvalidPublicKey)?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|_| UpdateVerificationError::InvalidPublicKey)?;
    if verifying_key.is_weak() {
        return Err(UpdateVerificationError::InvalidPublicKey);
    }
    let signature_bytes = STANDARD
        .decode(manifest.signature.trim())
        .map_err(|_| UpdateVerificationError::InvalidSignatureEncoding)?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| UpdateVerificationError::InvalidSignatureEncoding)?;
    let payload_bytes = canonical_update_payload_bytes(&manifest.payload)?;
    verifying_key
        .verify(&payload_bytes, &signature)
        .map_err(|_| UpdateVerificationError::SignatureVerificationFailed)
}

pub fn verify_signed_update_manifest(
    text: &str,
    public_key_base64: &str,
) -> Result<UpdateManifest, UpdateVerificationError> {
    let manifest = parse_update_manifest(text)?;
    validate_manifest_payload(&manifest.payload)?;
    verify_update_manifest_signature(&manifest, public_key_base64)?;
    Ok(manifest)
}

pub fn validate_manifest_payload(
    payload: &UpdateManifestPayload,
) -> Result<(), UpdateVerificationError> {
    if payload.schema_version != 1 {
        return Err(UpdateVerificationError::UnsupportedSchema(
            payload.schema_version,
        ));
    }
    if payload.app_id != "sourceweaver" {
        return Err(UpdateVerificationError::WrongAppId(payload.app_id.clone()));
    }
    for artifact in &payload.artifacts {
        validate_sha256_hex(&artifact.sha256)?;
    }
    Ok(())
}

pub fn check_update_manifest(
    manifest: UpdateManifest,
    options: &UpdateCheckOptions,
) -> Result<UpdateCheckResult, UpdateVerificationError> {
    validate_manifest_payload(&manifest.payload)?;
    if manifest.payload.channel != options.channel {
        return Ok(UpdateCheckResult {
            availability: UpdateAvailability::ChannelMismatch,
            manifest: manifest.payload,
            selected_artifact: None,
        });
    }
    let artifact = manifest
        .payload
        .artifacts
        .iter()
        .find(|artifact| artifact.target == options.target)
        .cloned()
        .ok_or_else(|| UpdateVerificationError::MissingArtifact {
            target: options.target.clone(),
        })?;
    let comparison = compare_versions(&manifest.payload.version, &options.current_version);
    let availability = if comparison == std::cmp::Ordering::Equal {
        UpdateAvailability::Current
    } else if comparison == std::cmp::Ordering::Less && !options.allow_downgrade {
        UpdateAvailability::DowngradeBlocked
    } else {
        UpdateAvailability::UpdateAvailable
    };
    Ok(UpdateCheckResult {
        availability,
        manifest: manifest.payload,
        selected_artifact: Some(artifact),
    })
}

pub fn verify_artifact_bytes(
    artifact: &UpdateArtifact,
    bytes: &[u8],
) -> Result<String, UpdateVerificationError> {
    if artifact.size_bytes != bytes.len() as u64 {
        return Err(UpdateVerificationError::SizeMismatch {
            expected: artifact.size_bytes,
            actual: bytes.len() as u64,
        });
    }
    let actual = sha256_hex(bytes);
    if !actual.eq_ignore_ascii_case(&artifact.sha256) {
        return Err(UpdateVerificationError::Sha256Mismatch {
            expected: artifact.sha256.clone(),
            actual,
        });
    }
    Ok(actual)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn validate_sha256_hex(value: &str) -> Result<(), UpdateVerificationError> {
    let valid = value.len() == 64 && value.chars().all(|character| character.is_ascii_hexdigit());
    if valid {
        Ok(())
    } else {
        Err(UpdateVerificationError::InvalidSha256 {
            value: value.to_string(),
        })
    }
}

pub fn compare_versions(candidate: &str, current: &str) -> std::cmp::Ordering {
    let candidate = version_parts(candidate);
    let current = version_parts(current);
    candidate.cmp(&current)
}

fn version_parts(version: &str) -> Vec<u64> {
    let normalized = version.trim().trim_start_matches('v');
    let numeric = normalized
        .split_once('-')
        .map(|(prefix, _)| prefix)
        .unwrap_or(normalized);
    numeric
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use ed25519_dalek::{SigningKey, VerifyingKey};

    fn test_keys() -> (String, String) {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let verifying_key = VerifyingKey::from(&signing_key);
        (
            STANDARD.encode(signing_key.to_bytes()),
            STANDARD.encode(verifying_key.to_bytes()),
        )
    }

    fn payload_for(bytes: &[u8]) -> UpdateManifestPayload {
        UpdateManifestPayload {
            schema_version: 1,
            app_id: "sourceweaver".to_string(),
            channel: "stable".to_string(),
            version: "v0.2.0".to_string(),
            published_at: "2026-08-07T00:00:00Z".to_string(),
            release_notes_url: "https://github.com/dayflaree/sourceweaver/releases/tag/v0.2.0".to_string(),
            minimum_required_version: None,
            artifacts: vec![UpdateArtifact {
                target: "linux-x86_64".to_string(),
                kind: "appimage".to_string(),
                name: "sourceweaver-v0.2.0-linux-x86_64.AppImage".to_string(),
                url: "https://github.com/dayflaree/sourceweaver/releases/download/v0.2.0/sourceweaver-v0.2.0-linux-x86_64.AppImage".to_string(),
                sha256: sha256_hex(bytes),
                size_bytes: bytes.len() as u64,
                signature_url: None,
            }],
        }
    }

    fn signed_manifest(payload: UpdateManifestPayload) -> (String, String) {
        let (private_key, public_key) = test_keys();
        let signature = sign_update_manifest_payload(&payload, &private_key).unwrap();
        let manifest = UpdateManifest { payload, signature };
        (serde_json::to_string_pretty(&manifest).unwrap(), public_key)
    }

    #[test]
    fn verifies_signed_manifest_and_detects_update() {
        let (manifest_json, public_key) = signed_manifest(payload_for(b"artifact"));
        let manifest = verify_signed_update_manifest(&manifest_json, &public_key).unwrap();
        let result = check_update_manifest(
            manifest,
            &UpdateCheckOptions {
                current_version: "v0.1.0".to_string(),
                channel: "stable".to_string(),
                target: "linux-x86_64".to_string(),
                allow_downgrade: false,
            },
        )
        .unwrap();
        assert_eq!(result.availability, UpdateAvailability::UpdateAvailable);
        assert_eq!(result.selected_artifact.unwrap().kind, "appimage");
    }

    #[test]
    fn rejects_wrong_manifest_signature() {
        let (manifest_json, _public_key) = signed_manifest(payload_for(b"artifact"));
        let (_other_private_key, other_public_key) = {
            let signing_key = SigningKey::from_bytes(&[9u8; 32]);
            let verifying_key = VerifyingKey::from(&signing_key);
            (
                STANDARD.encode(signing_key.to_bytes()),
                STANDARD.encode(verifying_key.to_bytes()),
            )
        };
        let error = verify_signed_update_manifest(&manifest_json, &other_public_key).unwrap_err();
        assert_eq!(error, UpdateVerificationError::SignatureVerificationFailed);
    }

    #[test]
    fn rejects_corrupt_artifact_before_install_handoff() {
        let payload = payload_for(b"artifact");
        let artifact = payload.artifacts[0].clone();
        let error = verify_artifact_bytes(&artifact, b"corrupt").unwrap_err();
        assert!(matches!(
            error,
            UpdateVerificationError::SizeMismatch { .. }
                | UpdateVerificationError::Sha256Mismatch { .. }
        ));
        assert_eq!(
            verify_artifact_bytes(&artifact, b"artifact").unwrap(),
            artifact.sha256
        );
    }

    #[test]
    fn blocks_downgrades_unless_explicitly_allowed() {
        let (manifest_json, public_key) = signed_manifest(UpdateManifestPayload {
            version: "v0.1.0".to_string(),
            ..payload_for(b"artifact")
        });
        let manifest = verify_signed_update_manifest(&manifest_json, &public_key).unwrap();
        let result = check_update_manifest(
            manifest,
            &UpdateCheckOptions {
                current_version: "v0.2.0".to_string(),
                channel: "stable".to_string(),
                target: "linux-x86_64".to_string(),
                allow_downgrade: false,
            },
        )
        .unwrap();
        assert_eq!(result.availability, UpdateAvailability::DowngradeBlocked);
    }

    #[test]
    fn refuses_channel_switches_by_default() {
        let (manifest_json, public_key) = signed_manifest(UpdateManifestPayload {
            channel: "preview".to_string(),
            ..payload_for(b"artifact")
        });
        let manifest = verify_signed_update_manifest(&manifest_json, &public_key).unwrap();
        let result = check_update_manifest(
            manifest,
            &UpdateCheckOptions {
                current_version: "v0.1.0".to_string(),
                channel: "stable".to_string(),
                target: "linux-x86_64".to_string(),
                allow_downgrade: false,
            },
        )
        .unwrap();
        assert_eq!(result.availability, UpdateAvailability::ChannelMismatch);
        assert!(result.selected_artifact.is_none());
    }
}
