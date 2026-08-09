//! Desktop document, CSV, and validation-label helpers.

use super::*;

pub(crate) fn load_document(path: impl AsRef<Path>) -> Result<Document, String> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read {}: {error}", display_path(path)))?;
    Document::parse(&text)
        .map_err(|error| format!("Failed to parse {}: {error}", display_path(path)))
}

pub(crate) fn write_document(path: impl AsRef<Path>, document: &Document) -> Result<(), String> {
    let path = path.as_ref();
    fs::write(path, document.to_vmf_string())
        .map_err(|error| format!("Failed to write {}: {error}", display_path(path)))
}

pub(crate) fn split_csv(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
}

pub(crate) fn validation_rule_set_combo_label(value: &str) -> String {
    if value.trim().eq_ignore_ascii_case(NO_VALIDATION_RULE_SET_ID) {
        return "none: generic VMF integrity only".to_string();
    }
    validation_rule_set_by_id(value)
        .map(|rule_set| format!("{}: {}", rule_set.id, rule_set.name))
        .unwrap_or_else(|| format!("{}: unknown rule set", value.trim()))
}

pub(crate) fn blank_to_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(crate) fn format_roles(roles: &[BrushRole]) -> String {
    if roles.is_empty() {
        return "-".to_string();
    }
    roles
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}
