use crate::integrity::IntegritySeverity;
use crate::vmf::{Document, Node};
use std::collections::BTreeSet;

const HL2_ALIASES: &[&str] = &[
    "half-life-2",
    "halflife2",
    "hl2-sp",
    "half-life-2-sp",
    "source-2013-sp",
    "source2013sp",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationRuleSet {
    pub id: &'static str,
    pub name: &'static str,
    pub scope: &'static str,
    pub aliases: &'static [&'static str],
}

pub const NO_VALIDATION_RULE_SET_ID: &str = "none";

pub const BUILTIN_VALIDATION_RULE_SETS: &[ValidationRuleSet] = &[ValidationRuleSet {
    id: "hl2",
    name: "Half-Life 2 single-player",
    scope: "Portable structural checks for HL2/Source 2013 single-player VMFs. This profile does not run Hammer, VBSP, VVIS, VRAD, the game runtime, or require a game install.",
    aliases: HL2_ALIASES,
}];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleSetIssue {
    pub severity: IntegritySeverity,
    pub label: String,
    pub rule_set: String,
    pub rule_id: String,
    pub message: String,
}

impl RuleSetIssue {
    fn new(
        severity: IntegritySeverity,
        label: impl Into<String>,
        rule_set: impl Into<String>,
        rule_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            label: label.into(),
            rule_set: rule_set.into(),
            rule_id: rule_id.into(),
            message: message.into(),
        }
    }

    fn error(
        label: impl Into<String>,
        rule_set: impl Into<String>,
        rule_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(IntegritySeverity::Error, label, rule_set, rule_id, message)
    }

    fn warning(
        label: impl Into<String>,
        rule_set: impl Into<String>,
        rule_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            IntegritySeverity::Warning,
            label,
            rule_set,
            rule_id,
            message,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleSetValidationReport {
    pub rule_set: ValidationRuleSet,
    pub issues: Vec<RuleSetIssue>,
}

impl RuleSetValidationReport {
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

pub fn format_rule_set_issue(issue: &RuleSetIssue) -> String {
    format!(
        "{}: {}: {}: {}",
        issue.severity, issue.label, issue.rule_id, issue.message
    )
}

pub fn validation_rule_set_by_id(value: &str) -> Option<&'static ValidationRuleSet> {
    let normalized = normalize_rule_set_id(value);
    BUILTIN_VALIDATION_RULE_SETS.iter().find(|rule_set| {
        normalize_rule_set_id(rule_set.id) == normalized
            || rule_set
                .aliases
                .iter()
                .any(|alias| normalize_rule_set_id(alias) == normalized)
    })
}

pub fn validation_rule_set_choices() -> String {
    let mut ids = Vec::with_capacity(BUILTIN_VALIDATION_RULE_SETS.len() + 1);
    ids.push(NO_VALIDATION_RULE_SET_ID.to_string());
    ids.extend(
        BUILTIN_VALIDATION_RULE_SETS
            .iter()
            .map(|rule_set| rule_set.id.to_string()),
    );
    ids.join(", ")
}

pub fn validate_document_with_rule_set(
    document: &Document,
    label: &str,
    rule_set: &ValidationRuleSet,
) -> RuleSetValidationReport {
    let mut report = RuleSetValidationReport {
        rule_set: *rule_set,
        issues: Vec::new(),
    };

    if rule_set.id == "hl2" {
        validate_hl2_singleplayer(document, label, &mut report);
    }

    report
}

fn validate_hl2_singleplayer(
    document: &Document,
    label: &str,
    report: &mut RuleSetValidationReport,
) {
    let entities = top_level_entity_bodies(document);
    let has_player_start = entities.iter().any(|(_, body)| {
        get_property(body, "classname")
            .is_some_and(|value| value.eq_ignore_ascii_case("info_player_start"))
    });

    if !has_player_start {
        report.issues.push(RuleSetIssue::warning(
            label,
            report.rule_set.id,
            "hl2.player_start",
            "HL2 single-player maps normally need at least one `info_player_start`; Source Weaver did not find one.",
        ));
    }

    let landmarks = entities
        .iter()
        .filter_map(|(_, body)| {
            let classname = get_property(body, "classname")?;
            classname
                .eq_ignore_ascii_case("info_landmark")
                .then_some(body)
        })
        .filter_map(|body| get_property(body, "targetname"))
        .filter(|targetname| !targetname.trim().is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();

    for (entity_index, body) in entities {
        let Some(classname) = get_property(body, "classname") else {
            continue;
        };

        if classname.eq_ignore_ascii_case("info_landmark") {
            validate_hl2_landmark(label, entity_index, body, report);
        } else if classname.eq_ignore_ascii_case("trigger_changelevel") {
            validate_hl2_changelevel(label, entity_index, body, &landmarks, report);
        }
    }
}

fn validate_hl2_landmark(
    label: &str,
    entity_index: usize,
    body: &[Node],
    report: &mut RuleSetValidationReport,
) {
    let targetname = get_property(body, "targetname").unwrap_or_default().trim();
    if targetname.is_empty() {
        report.issues.push(RuleSetIssue::warning(
            label,
            report.rule_set.id,
            "hl2.landmark_targetname",
            format!("info_landmark entity[{entity_index}] has no targetname, so transition matching cannot reference it."),
        ));
    }

    match get_property(body, "origin") {
        Some(origin) if parse_origin(origin).is_some() => {}
        Some(origin) => report.issues.push(RuleSetIssue::warning(
            label,
            report.rule_set.id,
            "hl2.landmark_origin",
            format!("info_landmark entity[{entity_index}] has non-numeric origin `{origin}`."),
        )),
        None => report.issues.push(RuleSetIssue::warning(
            label,
            report.rule_set.id,
            "hl2.landmark_origin",
            format!("info_landmark entity[{entity_index}] has no origin."),
        )),
    }
}

fn validate_hl2_changelevel(
    label: &str,
    entity_index: usize,
    body: &[Node],
    landmarks: &BTreeSet<String>,
    report: &mut RuleSetValidationReport,
) {
    let map = get_property(body, "map").unwrap_or_default().trim();
    if map.is_empty() {
        report.issues.push(RuleSetIssue::error(
            label,
            report.rule_set.id,
            "hl2.changelevel_map",
            format!("trigger_changelevel entity[{entity_index}] has no `map` target."),
        ));
    }

    let landmark = get_property(body, "landmark").unwrap_or_default().trim();
    if landmark.is_empty() {
        report.issues.push(RuleSetIssue::warning(
            label,
            report.rule_set.id,
            "hl2.changelevel_landmark",
            format!("trigger_changelevel entity[{entity_index}] has no `landmark` key."),
        ));
    } else if !landmarks.contains(landmark) {
        report.issues.push(RuleSetIssue::warning(
            label,
            report.rule_set.id,
            "hl2.changelevel_landmark_reference",
            format!("trigger_changelevel entity[{entity_index}] references missing info_landmark targetname `{landmark}`."),
        ));
    }
}

fn top_level_entity_bodies(document: &Document) -> Vec<(usize, &[Node])> {
    document
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            Node::Block { name, body } if name == "entity" => Some((index, body.as_slice())),
            _ => None,
        })
        .collect()
}

fn get_property<'a>(body: &'a [Node], key: &str) -> Option<&'a str> {
    Node::get_property(body, key)
}

fn parse_origin(value: &str) -> Option<[f64; 3]> {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    let [x, y, z] = parts.as_slice() else {
        return None;
    };
    Some([x.parse().ok()?, y.parse().ok()?, z.parse().ok()?])
}

fn normalize_rule_set_id(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| !matches!(ch, '_' | ' '))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmf::parse_document;

    #[test]
    fn resolves_hl2_rule_set_aliases() {
        assert_eq!(
            validation_rule_set_by_id("hl2").map(|rule_set| rule_set.id),
            Some("hl2")
        );
        assert_eq!(
            validation_rule_set_by_id("Half-Life-2").map(|rule_set| rule_set.id),
            Some("hl2")
        );
        assert!(validation_rule_set_by_id("missing-profile").is_none());
    }

    #[test]
    fn hl2_profile_accepts_basic_singleplayer_fixture() {
        let document = parse_document(include_str!("../../../tests/fixtures/hl2_ruleset_ok.vmf"))
            .expect("fixture parses");
        let rule_set = validation_rule_set_by_id("hl2").expect("hl2 profile exists");
        let report = validate_document_with_rule_set(&document, "hl2_ruleset_ok.vmf", rule_set);

        assert!(report.is_ok());
        assert!(report.issues.is_empty());
    }

    #[test]
    fn hl2_profile_reports_separate_rule_set_issues() {
        let document = parse_document(
            r#"
versioninfo { "editorversion" "400" }
viewsettings { "bSnapToGrid" "1" }
world { "id" "1" }
entity { "id" "2" "classname" "info_landmark" "targetname" "exit_a" }
entity { "id" "3" "classname" "trigger_changelevel" "landmark" "missing_landmark" }
"#,
        )
        .expect("inline VMF parses");
        let rule_set = validation_rule_set_by_id("hl2").expect("hl2 profile exists");
        let report = validate_document_with_rule_set(&document, "broken_hl2.vmf", rule_set);

        assert_eq!(report.error_count(), 1);
        assert!(report.warning_count() >= 3);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.rule_id == "hl2.changelevel_map")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.rule_id == "hl2.changelevel_landmark_reference")
        );
    }
}
