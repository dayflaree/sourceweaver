use crate::id_references::{is_suspected_id_reference_key, supported_id_reference_summary};
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
    validate_suspected_id_reference_fields(document, label, &mut report);
    validate_func_instance_preservation(document, label, &mut report);
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

fn validate_suspected_id_reference_fields(
    document: &Document,
    label: &str,
    report: &mut IntegrityReport,
) {
    validate_suspected_id_reference_fields_in_nodes(
        &document.nodes,
        label,
        report,
        &mut Vec::new(),
    );
}

fn validate_suspected_id_reference_fields_in_nodes(
    nodes: &[Node],
    label: &str,
    report: &mut IntegrityReport,
    context: &mut Vec<String>,
) {
    let mut block_counts: BTreeMap<&str, usize> = BTreeMap::new();

    for node in nodes {
        match node {
            Node::Property { key, value } => {
                if is_suspected_id_reference_key(key, value) {
                    let path = if context.is_empty() {
                        "<root>".to_string()
                    } else {
                        context.join("/")
                    };
                    report.push(IntegrityIssue::warning(
                        label,
                        format!(
                            "{path} property `{key}` has numeric value `{value}` and looks like an unsupported VMF ID-reference field; Source Weaver currently remaps {}; add a fixture before enabling automatic remap",
                            supported_id_reference_summary()
                        ),
                    ));
                }
            }
            Node::Block { name, body } => {
                let index = block_counts.entry(name.as_str()).or_insert(0);
                context.push(format!("{name}[{index}]"));
                validate_suspected_id_reference_fields_in_nodes(body, label, report, context);
                context.pop();
                *index += 1;
            }
        }
    }
}

fn validate_func_instance_preservation(
    document: &Document,
    label: &str,
    report: &mut IntegrityReport,
) {
    validate_func_instance_preservation_in_nodes(&document.nodes, label, report, &mut Vec::new());
}

fn validate_func_instance_preservation_in_nodes(
    nodes: &[Node],
    label: &str,
    report: &mut IntegrityReport,
    context: &mut Vec<String>,
) {
    let mut block_counts: BTreeMap<&str, usize> = BTreeMap::new();

    for node in nodes {
        let Node::Block { name, body } = node else {
            continue;
        };
        let index = block_counts.entry(name.as_str()).or_insert(0);
        context.push(format!("{name}[{index}]"));
        if name == "entity" && Node::get_property(body, "classname") == Some("func_instance") {
            validate_func_instance_entity(body, label, report, &context.join("/"));
        }
        validate_func_instance_preservation_in_nodes(body, label, report, context);
        context.pop();
        *index += 1;
    }
}

fn validate_func_instance_entity(
    body: &[Node],
    label: &str,
    report: &mut IntegrityReport,
    path: &str,
) {
    let targetname = Node::get_property(body, "targetname")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("<unnamed>");
    let file = Node::get_property(body, "file")
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match file {
        Some(file) => {
            report.push(IntegrityIssue::warning(
                label,
                format!(
                    "{path} func_instance `{targetname}` references instance file `{file}`; Source Weaver preserves the entity but does not resolve, inline, transform, apply fixups to, or expand nested instance VMFs",
                ),
            ));
            if instance_path_has_parent_or_absolute_segment(file) {
                report.push(IntegrityIssue::warning(
                    label,
                    format!(
                        "{path} func_instance `{targetname}` uses non-local instance path `{file}`; Source Weaver does not resolve instance search roots or normalize instance file paths",
                    ),
                ));
            }
        }
        None => report.push(IntegrityIssue::warning(
            label,
            format!(
                "{path} func_instance `{targetname}` has no non-empty `file` key; Source Weaver preserves the entity but cannot resolve or expand an instance VMF",
            ),
        )),
    }

    let replace_keys = body
        .iter()
        .filter_map(|node| match node {
            Node::Property { key, .. } if key.starts_with("replace") => Some(key.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if !replace_keys.is_empty() {
        report.push(IntegrityIssue::warning(
            label,
            format!(
                "{path} func_instance `{targetname}` has replacement parameter keys {}; Source Weaver preserves those keyvalues but does not apply parameter replacement",
                replace_keys.join(", ")
            ),
        ));
    }
}

fn instance_path_has_parent_or_absolute_segment(value: &str) -> bool {
    let lower = value.replace('\\', "/");
    lower.starts_with('/')
        || lower.starts_with("../")
        || lower.contains("/../")
        || lower.ends_with("/..")
        || lower.as_bytes().get(1).is_some_and(|byte| *byte == b':')
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
    fn warns_about_unknown_suspected_id_reference_fields() {
        let document = parse_document(include_str!(
            "../../../tests/fixtures/id_reference_suspected_unknown.vmf"
        ))
        .unwrap();
        let report = validate_document_integrity(&document, "id_reference_suspected_unknown.vmf");
        let messages = report
            .warnings()
            .map(|issue| issue.message.as_str())
            .collect::<Vec<_>>();

        assert!(
            messages
                .iter()
                .any(|message| message.contains("`targetid`"))
        );
        assert!(messages.iter().any(|message| message.contains("`nodeids`")));
        assert!(
            !messages
                .iter()
                .any(|message| message.contains("`hammerid`"))
        );
    }

    #[test]
    fn warns_about_func_instance_preservation_boundary() {
        let document = parse_document(include_str!(
            "../../../tests/fixtures/func_instance_preservation.vmf"
        ))
        .unwrap();
        let report = validate_document_integrity(&document, "func_instance_preservation.vmf");
        let messages = report
            .warnings()
            .map(|issue| issue.message.as_str())
            .collect::<Vec<_>>();

        assert!(messages.iter().any(|message| {
            message.contains("references instance file `instances/synthetic_room.vmf`")
                && message.contains("does not resolve, inline, transform")
        }));
        assert!(messages.iter().any(|message| {
            message.contains("replacement parameter keys replace01")
                && message.contains("does not apply parameter replacement")
        }));
        assert!(messages.iter().any(|message| {
            message.contains("missing_file_instance")
                && message.contains("has no non-empty `file` key")
        }));
    }

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
