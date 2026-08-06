use crate::integrity::IntegritySeverity;
use crate::vmf::{Document, Node};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntitySemanticsIssue {
    pub severity: IntegritySeverity,
    pub label: String,
    pub category: String,
    pub rule_id: String,
    pub message: String,
    pub targetname: Option<String>,
    pub entity_index: Option<usize>,
    pub classname: Option<String>,
    pub key: Option<String>,
    pub value: Option<String>,
}

impl EntitySemanticsIssue {
    fn duplicate_targetname(
        severity: IntegritySeverity,
        label: impl Into<String>,
        targetname: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let targetname = targetname.into();
        Self {
            severity,
            label: label.into(),
            category: "duplicate-targetname".to_string(),
            rule_id: "entity.duplicate_targetname".to_string(),
            message: message.into(),
            targetname: Some(targetname),
            entity_index: None,
            classname: None,
            key: None,
            value: None,
        }
    }

    fn missing_reference(
        label: impl Into<String>,
        entity_index: usize,
        classname: Option<String>,
        key: impl Into<String>,
        value: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let value = value.into();
        Self {
            severity: IntegritySeverity::Warning,
            label: label.into(),
            category: "missing-target-reference".to_string(),
            rule_id: "entity.missing_target_reference".to_string(),
            message: message.into(),
            targetname: Some(value.clone()),
            entity_index: Some(entity_index),
            classname,
            key: Some(key.into()),
            value: Some(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntitySemanticsReport {
    pub issues: Vec<EntitySemanticsIssue>,
}

impl EntitySemanticsReport {
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == IntegritySeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == IntegritySeverity::Warning)
            .count()
    }

    pub fn is_ok(&self) -> bool {
        self.error_count() == 0
    }
}

#[derive(Debug, Clone)]
struct EntitySummary<'a> {
    entity_index: usize,
    classname: Option<String>,
    targetname: Option<String>,
    body: &'a [Node],
}

pub fn format_entity_semantics_issue(issue: &EntitySemanticsIssue) -> String {
    format!(
        "{}: {}: {}: {}",
        issue.severity, issue.label, issue.rule_id, issue.message
    )
}

pub fn validate_entity_semantics(document: &Document, label: &str) -> EntitySemanticsReport {
    let entities = collect_entities(document);
    let mut report = EntitySemanticsReport::default();
    let targetnames = targetname_index(&entities);
    validate_duplicate_targetnames(label, &targetnames, &mut report);
    validate_missing_references(label, &entities, &targetnames, &mut report);
    report
}

fn collect_entities(document: &Document) -> Vec<EntitySummary<'_>> {
    let mut entities = Vec::new();
    for node in &document.nodes {
        let Node::Block { name, body } = node else {
            continue;
        };
        if name != "entity" {
            continue;
        }
        let entity_index = entities.len();
        entities.push(EntitySummary {
            entity_index,
            classname: non_empty_property(body, "classname"),
            targetname: non_empty_property(body, "targetname"),
            body,
        });
    }
    entities
}

fn targetname_index<'a>(
    entities: &'a [EntitySummary<'a>],
) -> BTreeMap<String, Vec<&'a EntitySummary<'a>>> {
    let mut targetnames: BTreeMap<String, Vec<&EntitySummary<'_>>> = BTreeMap::new();
    for entity in entities {
        if let Some(targetname) = &entity.targetname {
            targetnames
                .entry(targetname.to_string())
                .or_default()
                .push(entity);
        }
    }
    targetnames
}

fn validate_duplicate_targetnames(
    label: &str,
    targetnames: &BTreeMap<String, Vec<&EntitySummary<'_>>>,
    report: &mut EntitySemanticsReport,
) {
    for (targetname, entities) in targetnames {
        if entities.len() <= 1 {
            continue;
        }

        let classnames = entities
            .iter()
            .map(|entity| entity.classname.as_deref().unwrap_or("<unknown>"))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ");
        let indices = entities
            .iter()
            .map(|entity| format!("entity[{}]", entity.entity_index))
            .collect::<Vec<_>>()
            .join(", ");

        let unsafe_duplicate = entities.iter().any(|entity| {
            entity
                .classname
                .as_deref()
                .is_some_and(is_likely_unique_targetname_class)
        });
        let intent = if unsafe_duplicate {
            "likely unsafe because at least one entity class is expected to be uniquely addressable; review before merge/export"
        } else {
            "may be intentional because Source I/O can target groups; review before merge/export"
        };

        report.issues.push(EntitySemanticsIssue::duplicate_targetname(
            IntegritySeverity::Warning,
            label,
            targetname,
            format!(
                "targetname `{targetname}` appears on {} entities ({indices}; classes: {classnames}); {intent}.",
                entities.len()
            ),
        ));
    }
}

fn validate_missing_references(
    label: &str,
    entities: &[EntitySummary<'_>],
    targetnames: &BTreeMap<String, Vec<&EntitySummary<'_>>>,
    report: &mut EntitySemanticsReport,
) {
    for entity in entities {
        for node in entity.body {
            let Node::Property { key, value } = node else {
                continue;
            };
            let Some(reference) = reference_target_from_property(key, value) else {
                continue;
            };
            if targetnames.contains_key(reference) || should_skip_reference(reference) {
                continue;
            }
            report.issues.push(EntitySemanticsIssue::missing_reference(
                label,
                entity.entity_index,
                entity.classname.clone(),
                key,
                reference,
                format!(
                    "{} entity[{}] property `{key}` references missing targetname `{reference}`.",
                    entity.classname.as_deref().unwrap_or("<unknown>"),
                    entity.entity_index
                ),
            ));
        }
    }
}

fn reference_target_from_property<'a>(key: &str, value: &'a str) -> Option<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if is_source_output_key(key) {
        return trimmed
            .split_once(',')
            .map(|(target, _)| target.trim())
            .filter(|target| !target.is_empty());
    }

    if is_common_target_reference_key(key) {
        return Some(trimmed);
    }

    None
}

fn is_source_output_key(key: &str) -> bool {
    key.starts_with("On") && key.len() > 2
}

fn is_common_target_reference_key(key: &str) -> bool {
    matches!(
        key,
        "target" | "parentname" | "filtername" | "landmark" | "landmarkname"
    )
}

fn should_skip_reference(reference: &str) -> bool {
    let lower = reference.to_ascii_lowercase();
    lower.starts_with('!')
        || matches!(lower.as_str(), "player" | "worldspawn")
        || reference.contains('*')
        || reference.contains('?')
}

fn is_likely_unique_targetname_class(classname: &str) -> bool {
    matches!(
        classname,
        "info_landmark" | "path_track" | "path_corner" | "phys_constraint" | "logic_case"
    )
}

fn non_empty_property(body: &[Node], key: &str) -> Option<String> {
    Node::get_property(body, key)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmf::parse_document;

    #[test]
    fn reports_unsafe_duplicate_targetnames() {
        let document = parse_document(
            r#"
world { "id" "1" }
entity { "id" "2" "classname" "info_landmark" "targetname" "exit_a" }
entity { "id" "3" "classname" "info_landmark" "targetname" "exit_a" }
"#,
        )
        .unwrap();

        let report = validate_entity_semantics(&document, "dupe.vmf");

        assert_eq!(report.error_count(), 0);
        assert_eq!(report.warning_count(), 1);
        assert!(report.issues.iter().any(|issue| {
            issue.category == "duplicate-targetname"
                && issue.targetname.as_deref() == Some("exit_a")
        }));
    }

    #[test]
    fn reports_missing_output_and_target_field_references() {
        let document = parse_document(
            r#"
world { "id" "1" }
entity { "id" "2" "classname" "logic_relay" "targetname" "relay_a" "OnTrigger" "door_a,Open,,0,-1" }
entity { "id" "3" "classname" "trigger_multiple" "filtername" "filter_missing" "OnStartTouch" "!activator,Use,,0,-1" }
"#,
        )
        .unwrap();

        let report = validate_entity_semantics(&document, "refs.vmf");

        assert_eq!(report.warning_count(), 2);
        assert!(report.issues.iter().any(|issue| {
            issue.key.as_deref() == Some("OnTrigger")
                && issue.targetname.as_deref() == Some("door_a")
        }));
        assert!(report.issues.iter().any(|issue| {
            issue.key.as_deref() == Some("filtername")
                && issue.targetname.as_deref() == Some("filter_missing")
        }));
        assert!(
            !report
                .issues
                .iter()
                .any(|issue| issue.targetname.as_deref() == Some("!activator"))
        );
    }

    #[test]
    fn duplicate_group_targetnames_are_warnings() {
        let document = parse_document(
            r#"
world { "id" "1" }
entity { "id" "2" "classname" "prop_dynamic" "targetname" "group_a" }
entity { "id" "3" "classname" "prop_dynamic" "targetname" "group_a" }
entity { "id" "4" "classname" "logic_relay" "OnTrigger" "group_a,Kill,,0,-1" }
"#,
        )
        .unwrap();

        let report = validate_entity_semantics(&document, "group.vmf");

        assert_eq!(report.error_count(), 0);
        assert_eq!(report.warning_count(), 1);
        assert_eq!(report.issues[0].category, "duplicate-targetname");
    }
}
