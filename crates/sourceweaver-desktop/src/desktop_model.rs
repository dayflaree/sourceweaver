//! Small desktop data-model methods.

use super::*;

impl MapEntry {
    pub(crate) fn load(path: PathBuf, rule_set_id: Option<&str>) -> Self {
        let analysis = match load_document(&path) {
            Ok(document) => {
                let label = display_path(&path);
                let rule_set = rule_set_id
                    .and_then(validation_rule_set_by_id)
                    .map(|rule_set| validate_document_with_rule_set(&document, &label, rule_set));
                Ok(MapAnalysis {
                    entity_records: inspect_entities(&document),
                    type_counts: summarize_entity_types(&document),
                    preview: preview_document(&document),
                    landmarks: discover_landmarks(&document),
                    transitions: discover_transitions(&document),
                    integrity: validate_document_integrity(&document, &label),
                    entity_semantics: sourceweaver_core::validate_entity_semantics(
                        &document, &label,
                    ),
                    complexity: sourceweaver_core::analyze_map_complexity(&document),
                    rule_set,
                })
            }
            Err(error) => Err(error),
        };
        Self { path, analysis }
    }
}

impl RoleOption {
    pub(crate) fn new(label: &'static str, role: BrushRole) -> Self {
        Self {
            label,
            role,
            selected: false,
        }
    }
}
