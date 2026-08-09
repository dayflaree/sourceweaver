//! Small status-label helpers.

use super::*;

pub(crate) fn landmark_status_label(status: &LandmarkTargetStatus) -> (String, egui::Color32) {
    match status {
        LandmarkTargetStatus::Blank => (
            "No alignment requested".to_string(),
            egui::Color32::LIGHT_GRAY,
        ),
        LandmarkTargetStatus::Missing => (
            "Missing; map will be unshifted".to_string(),
            egui::Color32::YELLOW,
        ),
        LandmarkTargetStatus::Present { origin } => {
            (format!("Present at {origin}"), egui::Color32::LIGHT_GREEN)
        }
        LandmarkTargetStatus::InvalidOrigin { .. } => (
            "Found, but origin is missing or invalid".to_string(),
            egui::Color32::YELLOW,
        ),
        LandmarkTargetStatus::Duplicate {
            count,
            valid_origins,
        } => (
            format!("Duplicate: {count} entries, {valid_origins} valid origin(s)"),
            egui::Color32::YELLOW,
        ),
    }
}
