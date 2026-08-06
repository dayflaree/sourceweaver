use crate::vmf::{Document, Node};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegritySeverity {
    Error,
    Warning,
}

impl std::fmt::Display for IntegritySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegritySeverity::Error => f.write_str("error"),
            IntegritySeverity::Warning => f.write_str("warning"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrityIssue {
    pub severity: IntegritySeverity,
    pub label: String,
    pub message: String,
}

impl IntegrityIssue {
    pub fn error(label: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: IntegritySeverity::Error,
            label: label.into(),
            message: message.into(),
        }
    }

    pub fn warning(label: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: IntegritySeverity::Warning,
            label: label.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IntegrityReport {
    pub issues: Vec<IntegrityIssue>,
}

impl IntegrityReport {
    pub fn push(&mut self, issue: IntegrityIssue) {
        self.issues.push(issue);
    }

    pub fn extend(&mut self, other: IntegrityReport) {
        self.issues.extend(other.issues);
    }

    pub fn errors(&self) -> impl Iterator<Item = &IntegrityIssue> {
        self.issues
            .iter()
            .filter(|issue| issue.severity == IntegritySeverity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &IntegrityIssue> {
        self.issues
            .iter()
            .filter(|issue| issue.severity == IntegritySeverity::Warning)
    }

    pub fn error_count(&self) -> usize {
        self.errors().count()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings().count()
    }

    pub fn is_ok(&self) -> bool {
        self.error_count() == 0
    }

    pub fn error_message(&self) -> Option<String> {
        let errors = self
            .errors()
            .map(format_integrity_issue)
            .collect::<Vec<_>>();
        (!errors.is_empty()).then(|| errors.join("; "))
    }
}

pub fn format_integrity_issue(issue: &IntegrityIssue) -> String {
    format!("{}: {}: {}", issue.severity, issue.label, issue.message)
}

pub fn validate_document_integrity(document: &Document, label: &str) -> IntegrityReport {
    let mut report = IntegrityReport::default();
    validate_top_level_sections(document, label, &mut report);
    validate_ids(document, label, &mut report);
    report
}

pub fn validate_merge_inputs(inputs: &[(&str, &Document)]) -> IntegrityReport {
    let mut report = IntegrityReport::default();

    if inputs.is_empty() {
        report.push(IntegrityIssue::error(
            "merge inputs",
            "merge needs at least one VMF",
        ));
        return report;
    }

    for (label, document) in inputs {
        report.extend(validate_document_integrity(document, label));
    }

    report
}

fn validate_top_level_sections(document: &Document, label: &str, report: &mut IntegrityReport) {
    let world_count = document.top_level_blocks("world").count();
    match world_count {
        0 => report.push(IntegrityIssue::error(
            label,
            "missing required top-level `world` block",
        )),
        1 => {}
        count => report.push(IntegrityIssue::error(
            label,
            format!("expected exactly one top-level `world` block, found {count}"),
        )),
    }

    for section in ["versioninfo", "viewsettings"] {
        if document.top_level_blocks(section).count() == 0 {
            report.push(IntegrityIssue::warning(
                label,
                format!("missing common VMF section `{section}`"),
            ));
        }
    }
}

fn validate_ids(document: &Document, label: &str, report: &mut IntegrityReport) {
    let mut seen_ids: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    validate_child_ids(
        &document.nodes,
        label,
        report,
        &mut seen_ids,
        &mut Vec::new(),
    );

    for (id, paths) in seen_ids {
        if paths.len() > 1 {
            report.push(IntegrityIssue::warning(
                label,
                format!(
                    "duplicate numeric id `{id}` appears in {} places: {}",
                    paths.len(),
                    paths.join(", ")
                ),
            ));
        }
    }
}

fn validate_child_ids(
    nodes: &[Node],
    label: &str,
    report: &mut IntegrityReport,
    seen_ids: &mut BTreeMap<i64, Vec<String>>,
    context: &mut Vec<String>,
) {
    let mut block_counts: BTreeMap<&str, usize> = BTreeMap::new();

    for node in nodes {
        let Node::Block { name, .. } = node else {
            continue;
        };
        let index = block_counts.entry(name.as_str()).or_insert(0);
        validate_node_ids(node, *index, label, report, seen_ids, context);
        *index += 1;
    }
}

fn validate_node_ids(
    node: &Node,
    index: usize,
    label: &str,
    report: &mut IntegrityReport,
    seen_ids: &mut BTreeMap<i64, Vec<String>>,
    context: &mut Vec<String>,
) {
    let Node::Block { name, body } = node else {
        return;
    };

    context.push(format!("{name}[{index}]"));
    let path = context.join("/");

    if id_is_relevant(name) {
        let id_values = body
            .iter()
            .filter_map(|child| match child {
                Node::Property { key, value } if key == "id" => Some(value.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        match id_values.as_slice() {
            [] => report.push(IntegrityIssue::warning(
                label,
                format!("{path} is missing an `id` field"),
            )),
            [value] => match value.parse::<i64>() {
                Ok(id) => {
                    seen_ids.entry(id).or_default().push(path.clone());
                }
                Err(_) => report.push(IntegrityIssue::warning(
                    label,
                    format!("{path} has non-numeric id `{value}`"),
                )),
            },
            values => {
                let unique_values = values.iter().copied().collect::<BTreeSet<_>>();
                report.push(IntegrityIssue::warning(
                    label,
                    format!(
                        "{path} has multiple `id` fields: {}",
                        unique_values.into_iter().collect::<Vec<_>>().join(", ")
                    ),
                ));
            }
        }
    }

    validate_child_ids(body, label, report, seen_ids, context);
    context.pop();
}

fn id_is_relevant(block_name: &str) -> bool {
    matches!(
        block_name,
        "world" | "entity" | "solid" | "side" | "visgroup"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmf::parse_document;

    #[test]
    fn reports_missing_and_duplicate_world_blocks() {
        let missing = parse_document("entity { \"id\" \"1\" }").unwrap();
        let report = validate_document_integrity(&missing, "missing.vmf");
        assert!(
            report
                .errors()
                .any(|issue| issue.message.contains("missing required top-level `world`"))
        );

        let duplicate = parse_document("world { \"id\" \"1\" } world { \"id\" \"2\" }").unwrap();
        let report = validate_document_integrity(&duplicate, "dupe.vmf");
        assert!(report.errors().any(|issue| {
            issue
                .message
                .contains("expected exactly one top-level `world`")
        }));
    }

    #[test]
    fn warns_about_missing_common_sections() {
        let document = parse_document("world { \"id\" \"1\" }").unwrap();
        let report = validate_document_integrity(&document, "tiny.vmf");
        assert!(
            report
                .warnings()
                .any(|issue| issue.message.contains("`versioninfo`"))
        );
        assert!(
            report
                .warnings()
                .any(|issue| issue.message.contains("`viewsettings`"))
        );
    }

    #[test]
    fn warns_about_missing_duplicate_and_invalid_ids() {
        let document = parse_document(
            r#"
versioninfo { "editorversion" "400" }
viewsettings { "bSnapToGrid" "1" }
world { "id" "1" solid { side { "id" "bad" } } solid { "id" "1" side { "id" "2" } } }
entity { "classname" "prop_static" }
entity { "id" "3" "id" "4" "classname" "logic_auto" }
"#,
        )
        .unwrap();

        let report = validate_document_integrity(&document, "ids.vmf");

        assert!(report.warnings().any(|issue| {
            issue
                .message
                .contains("solid[0]/side[0] has non-numeric id `bad`")
        }));
        assert!(
            report
                .warnings()
                .any(|issue| issue.message.contains("duplicate numeric id `1`"))
        );
        assert!(
            report
                .warnings()
                .any(|issue| issue.message.contains("entity[0] is missing an `id`"))
        );
        assert!(
            report
                .warnings()
                .any(|issue| issue.message.contains("entity[1] has multiple `id` fields"))
        );
    }

    #[test]
    fn validates_empty_merge_inputs() {
        let report = validate_merge_inputs(&[]);
        assert_eq!(report.error_count(), 1);
        assert!(
            report
                .error_message()
                .unwrap()
                .contains("merge needs at least one VMF")
        );
    }
}
