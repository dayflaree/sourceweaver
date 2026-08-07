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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangelevelPolicyOptions {
    pub policy: ChangelevelPolicy,
    pub output_map: Option<String>,
    pub stitched_maps: Vec<String>,
}

impl Default for ChangelevelPolicyOptions {
    fn default() -> Self {
        Self {
            policy: ChangelevelPolicy::Preserve,
            output_map: None,
            stitched_maps: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangelevelPolicyReport {
    pub policy: ChangelevelPolicy,
    pub changed: Vec<ChangelevelChange>,
    pub warnings: Vec<String>,
}

impl ChangelevelPolicyReport {
    pub fn changed_count(&self) -> usize {
        self.changed.len()
    }
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
        changed: Vec::new(),
        warnings: Vec::new(),
    };

    match options.policy {
        ChangelevelPolicy::Preserve => {}
        ChangelevelPolicy::Disable => disable_changelevels(document, &landmarks, &mut report),
        ChangelevelPolicy::Delete => delete_changelevels(document, &landmarks, &mut report),
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
    landmarks: &BTreeSet<String>,
    report: &mut ChangelevelPolicyReport,
) {
    for (entity_index, body) in top_level_changelevel_bodies_mut(document) {
        let old_map = non_empty_property(body, "map");
        let landmark = transition_landmark(body);
        push_missing_landmark_warning(entity_index, landmark.as_deref(), landmarks, report);
        Node::set_property(body, "StartDisabled", "1");
        report.changed.push(ChangelevelChange {
            entity_index,
            targetname: non_empty_property(body, "targetname"),
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
                    let landmark = transition_landmark(body);
                    push_missing_landmark_warning(current_index, landmark.as_deref(), landmarks, report);
                    report.changed.push(ChangelevelChange {
                        entity_index: current_index,
                        targetname: non_empty_property(body, "targetname"),
                        action: "delete".to_string(),
                        old_map: non_empty_property(body, "map"),
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
