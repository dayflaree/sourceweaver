//! Signed-update IO and presentation helpers.

use super::*;
use std::io::Read;

pub(crate) fn desktop_read_update_text(location: &str) -> Result<String, String> {
    if desktop_is_https_url(location) {
        ureq::get(location)
            .set("Accept", "application/json")
            .call()
            .map_err(|error| format!("failed to fetch update manifest {location}: {error}"))?
            .into_string()
            .map_err(|error| format!("failed to read update manifest {location}: {error}"))
    } else if desktop_is_http_url(location) {
        Err(
            "refusing insecure HTTP update manifest URL; use HTTPS or a local file for tests"
                .to_string(),
        )
    } else {
        fs::read_to_string(location)
            .map_err(|error| format!("failed to read update manifest {location}: {error}"))
    }
}

pub(crate) fn desktop_read_update_bytes(location: &str) -> Result<Vec<u8>, String> {
    if desktop_is_https_url(location) {
        let response = ureq::get(location)
            .call()
            .map_err(|error| format!("failed to download update artifact {location}: {error}"))?;
        let mut bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|error| format!("failed to read update artifact {location}: {error}"))?;
        Ok(bytes)
    } else if desktop_is_http_url(location) {
        Err("refusing insecure HTTP update artifact URL; signed metadata still requires HTTPS downloads outside local tests".to_string())
    } else {
        fs::read(location)
            .map_err(|error| format!("failed to read update artifact {location}: {error}"))
    }
}

pub(crate) fn desktop_is_https_url(value: &str) -> bool {
    value.starts_with("https://")
}

pub(crate) fn desktop_is_http_url(value: &str) -> bool {
    value.starts_with("http://")
}

pub(crate) fn desktop_update_availability_label(availability: &UpdateAvailability) -> &'static str {
    match availability {
        UpdateAvailability::Current => "current",
        UpdateAvailability::UpdateAvailable => "update_available",
        UpdateAvailability::DowngradeBlocked => "downgrade_blocked",
        UpdateAvailability::ChannelMismatch => "channel_mismatch",
    }
}

pub(crate) fn desktop_safe_artifact_name(name: &str) -> String {
    Path::new(name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("sourceweaver-update-artifact")
        .to_string()
}

pub(crate) fn desktop_default_update_target() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows-x86_64"
    } else if cfg!(target_os = "linux") {
        "linux-x86_64"
    } else if cfg!(target_os = "macos") {
        "macos-x86_64"
    } else {
        "unknown"
    }
}
