use crate::vmf::{Document, Node};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChangelevelPolicy {
    #[default]
    Preserve,
    Disable,
    Delete,
    RewriteInternal,
}

impl ChangelevelPolicy {
    pub fn parse(value: &str) -> Option<Self> {
        match normalize_policy(value).as_str() {
            "preserve" | "keep" => Some(Self::Preserve),
            "disable" | "disabled" => Some(Self::Disable),
            "delete" | "remove" => Some(Self::Delete),
            "rewriteinternal" | "rewrite" => Some(Self::RewriteInternal),
            _ => None,
        }
    }

    pub fn choices() -> &'static str {
        "preserve, disable, delete, rewrite-internal"
    }
}

impl fmt::Display for ChangelevelPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preserve => f.write_str("preserve"),
            Self::Disable => f.write_str("disable"),
            Self::Delete => f.write_str("delete"),
            Self::RewriteInternal => f.write_str("rewrite-internal"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChangelevelScope {
    #[default]
    All,
    InternalOnly,
}

impl ChangelevelScope {
    pub fn parse(value: &str) -> Option<Self> {
        match normalize_policy(value).as_str() {
            "all" | "alltransitions" => Some(Self::All),
            "internalonly" | "internal" | "stitched" => Some(Self::InternalOnly),
            _ => None,
        }
    }

    pub fn choices() -> &'static str {
        "all, internal-only"
    }
}

impl fmt::Display for ChangelevelScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => f.write_str("all"),
            Self::InternalOnly => f.write_str("internal-only"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChangelevelPreserveRule {
    pub map: Option<String>,
    pub landmark: Option<String>,
    pub targetname: Option<String>,
}

impl ChangelevelPreserveRule {
    pub fn is_empty(&self) -> bool {
        self.map
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
            && self
                .landmark
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
            && self
                .targetname
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangelevelPolicyOptions {
    pub policy: ChangelevelPolicy,
    pub scope: ChangelevelScope,
    pub output_map: Option<String>,
    pub stitched_maps: Vec<String>,
    pub preserve_external: Vec<ChangelevelPreserveRule>,
}

impl Default for ChangelevelPolicyOptions {
    fn default() -> Self {
        Self {
            policy: ChangelevelPolicy::Preserve,
            scope: ChangelevelScope::All,
            output_map: None,
            stitched_maps: Vec::new(),
            preserve_external: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangelevelPolicyReport {
    pub policy: ChangelevelPolicy,
    pub scope: ChangelevelScope,
    pub changed: Vec<ChangelevelChange>,
    pub warnings: Vec<String>,
    pub preserved: Vec<ChangelevelPreservedTransition>,
}

impl ChangelevelPolicyReport {
    pub fn changed_count(&self) -> usize {
        self.changed.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangelevelPreservedTransition {
    pub entity_index: usize,
    pub targetname: Option<String>,
    pub map: Option<String>,
    pub landmark: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangelevelChange {
    pub entity_index: usize,
    pub targetname: Option<String>,
    pub action: String,
    pub old_map: Option<String>,
    pub new_map: Option<String>,
    pub landmark: Option<String>,
    pub rationale: String,
}

pub fn apply_changelevel_policy(
    document: &mut Document,
    options: &ChangelevelPolicyOptions,
) -> ChangelevelPolicyReport {
    let internal_maps = options
        .stitched_maps
        .iter()
        .map(|value| normalize_map_name(value))
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    let output_map = options
        .output_map
        .as_deref()
        .map(normalize_map_name)
        .filter(|value| !value.is_empty());
    let landmarks = collect_landmark_targetnames(document);

    let mut report = ChangelevelPolicyReport {
        policy: options.policy,
        scope: options.scope,
        changed: Vec::new(),
        warnings: Vec::new(),
        preserved: Vec::new(),
    };

    match options.policy {
        ChangelevelPolicy::Preserve => {}
        ChangelevelPolicy::Disable => disable_changelevels(
            document,
            &internal_maps,
            options.scope,
            &options.preserve_external,
            &landmarks,
            &mut report,
        ),
        ChangelevelPolicy::Delete => delete_changelevels(
            document,
            &internal_maps,
            options.scope,
            &options.preserve_external,
            &landmarks,
            &mut report,
        ),
        ChangelevelPolicy::RewriteInternal => rewrite_internal_changelevels(
            document,
            &internal_maps,
            output_map.as_deref(),
            &landmarks,
            &mut report,
        ),
    }

    report
}

fn disable_changelevels(
    document: &mut Document,
    internal_maps: &BTreeSet<String>,
    scope: ChangelevelScope,
    preserve_external: &[ChangelevelPreserveRule],
    landmarks: &BTreeSet<String>,
    report: &mut ChangelevelPolicyReport,
) {
    for (entity_index, body) in top_level_changelevel_bodies_mut(document) {
        let old_map = non_empty_property(body, "map");
        let landmark = transition_landmark(body);
        let targetname = non_empty_property(body, "targetname");
        let candidate = TransitionCleanupCandidate {
            entity_index,
            targetname: targetname.as_deref(),
            map: old_map.as_deref(),
            landmark: landmark.as_deref(),
        };
        if should_preserve_transition(&candidate, internal_maps, scope, preserve_external, report) {
            continue;
        }
        push_missing_landmark_warning(entity_index, landmark.as_deref(), landmarks, report);
        Node::set_property(body, "StartDisabled", "1");
        report.changed.push(ChangelevelChange {
            entity_index,
            targetname,
            action: "disable".to_string(),
            old_map,
            new_map: None,
            landmark,
            rationale: "policy `disable` sets StartDisabled=1 on trigger_changelevel while preserving destination metadata for manual review".to_string(),
        });
    }
}

fn delete_changelevels(
    document: &mut Document,
    internal_maps: &BTreeSet<String>,
    scope: ChangelevelScope,
    preserve_external: &[ChangelevelPreserveRule],
    landmarks: &BTreeSet<String>,
    report: &mut ChangelevelPolicyReport,
) {
    let mut entity_index = 0;
    document.nodes.retain(|node| {
        let remove = match node {
            Node::Block { name, body } if name == "entity" => {
                let current_index = entity_index;
                entity_index += 1;
                if is_changelevel_body(body) {
                    let old_map = non_empty_property(body, "map");
                    let landmark = transition_landmark(body);
                    let targetname = non_empty_property(body, "targetname");
                    let candidate = TransitionCleanupCandidate {
                        entity_index: current_index,
                        targetname: targetname.as_deref(),
                        map: old_map.as_deref(),
                        landmark: landmark.as_deref(),
                    };
                    if should_preserve_transition(
                        &candidate,
                        internal_maps,
                        scope,
                        preserve_external,
                        report,
                    ) {
                        return true;
                    }
                    push_missing_landmark_warning(current_index, landmark.as_deref(), landmarks, report);
                    report.changed.push(ChangelevelChange {
                        entity_index: current_index,
                        targetname,
                        action: "delete".to_string(),
                        old_map,
                        new_map: None,
                        landmark,
                        rationale: "policy `delete` removes trigger_changelevel entities from the stitched output".to_string(),
                    });
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        !remove
    });
}

fn rewrite_internal_changelevels(
    document: &mut Document,
    internal_maps: &BTreeSet<String>,
    output_map: Option<&str>,
    landmarks: &BTreeSet<String>,
    report: &mut ChangelevelPolicyReport,
) {
    let Some(output_map) = output_map else {
        report.warnings.push(
            "policy `rewrite-internal` needs an output map stem; no trigger_changelevel destinations were rewritten".to_string(),
        );
        return;
    };

    for (entity_index, body) in top_level_changelevel_bodies_mut(document) {
        let old_map = non_empty_property(body, "map");
        let landmark = transition_landmark(body);
        push_missing_landmark_warning(entity_index, landmark.as_deref(), landmarks, report);
        let Some(old_map_value) = old_map.as_deref() else {
            continue;
        };
        let normalized_old_map = normalize_map_name(old_map_value);
        if !internal_maps.contains(&normalized_old_map) || normalized_old_map == output_map {
            continue;
        }
        Node::set_property(body, "map", output_map.to_string());
        let rationale = format!(
            "policy `rewrite-internal` rewrites internal stitched-map destination `{old_map_value}` to output map `{output_map}` while leaving external destinations unchanged"
        );
        report.changed.push(ChangelevelChange {
            entity_index,
            targetname: non_empty_property(body, "targetname"),
            action: "rewrite-internal".to_string(),
            old_map,
            new_map: Some(output_map.to_string()),
            landmark,
            rationale,
        });
    }
}

struct TransitionCleanupCandidate<'a> {
    entity_index: usize,
    targetname: Option<&'a str>,
    map: Option<&'a str>,
    landmark: Option<&'a str>,
}

fn should_preserve_transition(
    candidate: &TransitionCleanupCandidate<'_>,
    internal_maps: &BTreeSet<String>,
    scope: ChangelevelScope,
    preserve_external: &[ChangelevelPreserveRule],
    report: &mut ChangelevelPolicyReport,
) -> bool {
    let is_internal = candidate
        .map
        .map(normalize_map_name)
        .is_some_and(|map| internal_maps.contains(&map));
    if scope == ChangelevelScope::InternalOnly && !is_internal {
        report.preserved.push(ChangelevelPreservedTransition {
            entity_index: candidate.entity_index,
            targetname: candidate.targetname.map(ToOwned::to_owned),
            map: candidate.map.map(ToOwned::to_owned),
            landmark: candidate.landmark.map(ToOwned::to_owned),
            reason: "scope `internal-only` preserves external transition".to_string(),
        });
        return true;
    }

    if !is_internal {
        for rule in preserve_external.iter().filter(|rule| !rule.is_empty()) {
            if preserve_rule_matches(
                rule,
                candidate.targetname,
                candidate.map,
                candidate.landmark,
            ) {
                report.preserved.push(ChangelevelPreservedTransition {
                    entity_index: candidate.entity_index,
                    targetname: candidate.targetname.map(ToOwned::to_owned),
                    map: candidate.map.map(ToOwned::to_owned),
                    landmark: candidate.landmark.map(ToOwned::to_owned),
                    reason: format!(
                        "external transition matched preserve rule map={:?} landmark={:?} targetname={:?}",
                        rule.map, rule.landmark, rule.targetname
                    ),
                });
                return true;
            }
        }
    }

    false
}

fn preserve_rule_matches(
    rule: &ChangelevelPreserveRule,
    targetname: Option<&str>,
    map: Option<&str>,
    landmark: Option<&str>,
) -> bool {
    rule.map
        .as_deref()
        .map(|rule_map| {
            map.is_some_and(|map| normalize_map_name(map) == normalize_map_name(rule_map))
        })
        .unwrap_or(true)
        && rule
            .landmark
            .as_deref()
            .map(|rule_landmark| landmark == Some(rule_landmark))
            .unwrap_or(true)
        && rule
            .targetname
            .as_deref()
            .map(|rule_targetname| targetname == Some(rule_targetname))
            .unwrap_or(true)
}

fn push_missing_landmark_warning(
    entity_index: usize,
    landmark: Option<&str>,
    landmarks: &BTreeSet<String>,
    report: &mut ChangelevelPolicyReport,
) {
    match landmark {
        Some(landmark) if !landmarks.contains(landmark) => {
            report.warnings.push(format!(
                "trigger_changelevel entity[{entity_index}] references missing info_landmark `{landmark}`; policy changes still use VMF text only"
            ));
        }
        _ => {}
    }
}

fn collect_landmark_targetnames(document: &Document) -> BTreeSet<String> {
    document
        .nodes
        .iter()
        .filter_map(|node| match node {
            Node::Block { name, body } if name == "entity" => Some(body.as_slice()),
            _ => None,
        })
        .filter(|body| Node::get_property(body, "classname") == Some("info_landmark"))
        .filter_map(|body| non_empty_property(body, "targetname"))
        .collect()
}

fn top_level_changelevel_bodies_mut(document: &mut Document) -> Vec<(usize, &mut Vec<Node>)> {
    let mut entity_index = 0;
    document
        .nodes
        .iter_mut()
        .filter_map(|node| match node {
            Node::Block { name, body } if name == "entity" => {
                let current_index = entity_index;
                entity_index += 1;
                if is_changelevel_body(body) {
                    Some((current_index, body))
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect()
}

fn is_changelevel_body(body: &[Node]) -> bool {
    Node::get_property(body, "classname") == Some("trigger_changelevel")
}

fn transition_landmark(body: &[Node]) -> Option<String> {
    non_empty_property(body, "landmark").or_else(|| non_empty_property(body, "landmarkname"))
}

fn non_empty_property(body: &[Node], key: &str) -> Option<String> {
    Node::get_property(body, key)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn normalize_map_name(value: &str) -> String {
    let value = value.trim().replace('\\', "/");
    let stem = value.rsplit('/').next().unwrap_or(value.as_str());
    stem.strip_suffix(".vmf")
        .or_else(|| stem.strip_suffix(".bsp"))
        .unwrap_or(stem)
        .to_ascii_lowercase()
}

fn normalize_policy(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| !matches!(ch, '-' | '_' | ' '))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmf::parse_document;

    #[test]
    fn disables_all_changelevel_entities_and_reports_missing_landmark() {
        let mut document = parse_document(include_str!(
            "../../../tests/fixtures/changelevel_policy_missing_landmark.vmf"
        ))
        .unwrap();
        let report = apply_changelevel_policy(
            &mut document,
            &ChangelevelPolicyOptions {
                policy: ChangelevelPolicy::Disable,
                output_map: Some("stitched_campaign".to_string()),
                stitched_maps: vec!["d1_a".to_string(), "d1_b".to_string()],
                ..ChangelevelPolicyOptions::default()
            },
        );

        assert_eq!(report.changed_count(), 1);
        assert_eq!(report.changed[0].action, "disable");
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("missing_lm"))
        );
        assert!(document.to_vmf_string().contains("\"StartDisabled\" \"1\""));
    }

    #[test]
    fn deletes_changelevel_entities() {
        let mut document = parse_document(include_str!(
            "../../../tests/fixtures/changelevel_policy_internal.vmf"
        ))
        .unwrap();
        let report = apply_changelevel_policy(
            &mut document,
            &ChangelevelPolicyOptions {
                policy: ChangelevelPolicy::Delete,
                output_map: Some("stitched_campaign".to_string()),
                stitched_maps: vec!["d1_a".to_string(), "d1_b".to_string()],
                ..ChangelevelPolicyOptions::default()
            },
        );

        assert_eq!(report.changed_count(), 1);
        assert!(!document.to_vmf_string().contains("trigger_changelevel"));
    }

    #[test]
    fn rewrites_internal_destinations_and_preserves_external_destinations() {
        let mut document = parse_document(include_str!(
            "../../../tests/fixtures/changelevel_policy_external.vmf"
        ))
        .unwrap();
        let report = apply_changelevel_policy(
            &mut document,
            &ChangelevelPolicyOptions {
                policy: ChangelevelPolicy::RewriteInternal,
                output_map: Some("stitched_campaign".to_string()),
                stitched_maps: vec!["d1_a".to_string(), "d1_b".to_string()],
                ..ChangelevelPolicyOptions::default()
            },
        );

        assert_eq!(report.changed_count(), 1);
        assert_eq!(report.changed[0].old_map.as_deref(), Some("d1_b"));
        assert_eq!(
            report.changed[0].new_map.as_deref(),
            Some("stitched_campaign")
        );
        let output = document.to_vmf_string();
        assert!(output.contains("\"map\" \"stitched_campaign\""));
        assert!(output.contains("\"map\" \"d1_c_external\""));
    }

    #[test]
    fn internal_only_delete_preserves_external_transitions() {
        let mut document = parse_document(include_str!(
            "../../../tests/fixtures/changelevel_policy_external.vmf"
        ))
        .unwrap();
        let report = apply_changelevel_policy(
            &mut document,
            &ChangelevelPolicyOptions {
                policy: ChangelevelPolicy::Delete,
                scope: ChangelevelScope::InternalOnly,
                output_map: Some("stitched_campaign".to_string()),
                stitched_maps: vec!["d1_a".to_string(), "d1_b".to_string()],
                preserve_external: Vec::new(),
            },
        );

        assert_eq!(report.changed_count(), 1);
        assert_eq!(report.preserved.len(), 1);
        let output = document.to_vmf_string();
        assert!(!output.contains("\"targetname\" \"to_internal\""));
        assert!(output.contains("\"targetname\" \"to_external\""));
    }

    #[test]
    fn external_preserve_rule_keeps_selected_transition() {
        let mut document = parse_document(include_str!(
            "../../../tests/fixtures/changelevel_policy_external.vmf"
        ))
        .unwrap();
        let report = apply_changelevel_policy(
            &mut document,
            &ChangelevelPolicyOptions {
                policy: ChangelevelPolicy::Delete,
                scope: ChangelevelScope::All,
                output_map: Some("stitched_campaign".to_string()),
                stitched_maps: vec!["d1_a".to_string(), "d1_b".to_string()],
                preserve_external: vec![ChangelevelPreserveRule {
                    map: Some("d1_c_external".to_string()),
                    landmark: Some("lm_exit".to_string()),
                    targetname: Some("to_external".to_string()),
                }],
            },
        );

        assert_eq!(report.changed_count(), 1);
        assert_eq!(report.preserved.len(), 1);
        let output = document.to_vmf_string();
        assert!(!output.contains("\"targetname\" \"to_internal\""));
        assert!(output.contains("\"targetname\" \"to_external\""));
    }

    #[test]
    fn parses_policy_aliases() {
        assert_eq!(
            ChangelevelPolicy::parse("rewrite-internal"),
            Some(ChangelevelPolicy::RewriteInternal)
        );
        assert_eq!(
            ChangelevelPolicy::parse("keep"),
            Some(ChangelevelPolicy::Preserve)
        );
        assert!(ChangelevelPolicy::parse("bogus").is_none());
    }
}
