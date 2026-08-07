use crate::landmark::LandmarkDiscovery;
use crate::transition::CampaignTransition;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct CampaignMapInput {
    pub label: String,
    pub transitions: Vec<CampaignTransition>,
    pub landmarks: LandmarkDiscovery,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignLandmarkPairSuggestion {
    pub from_map: String,
    pub to_map: String,
    pub target_map: String,
    pub landmark: String,
    pub target_has_landmark: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CampaignOrderSuggestion {
    pub ordered_labels: Vec<String>,
    pub landmark_pairs: Vec<CampaignLandmarkPairSuggestion>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CampaignAdjacencyGraph {
    pub edges: Vec<CampaignAdjacencyEdge>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignAdjacencyEdge {
    pub from_map: String,
    pub to_map: String,
    pub evidence_kind: String,
    pub confidence: String,
    pub evidence: String,
}

pub fn build_campaign_adjacency_graph(maps: &[CampaignMapInput]) -> CampaignAdjacencyGraph {
    let mut graph = CampaignAdjacencyGraph::default();
    let mut keys_by_map_name = BTreeMap::new();
    for map in maps {
        for key in label_match_keys(&map.label) {
            keys_by_map_name.insert(key, map.label.clone());
        }
    }

    let mut explicit_pairs = BTreeSet::new();
    for map in maps {
        for transition in &map.transitions {
            let Some(target_map) = transition.target_map.as_deref() else {
                graph.warnings.push(format!(
                    "{} has trigger_changelevel #{} without a target map; no explicit adjacency edge was added.",
                    map.label, transition.entity_index
                ));
                continue;
            };
            let normalized_target = normalize_map_key(target_map);
            let Some(target_label) = keys_by_map_name.get(&normalized_target).cloned() else {
                graph.warnings.push(format!(
                    "{} trigger_changelevel #{} targets missing map `{target_map}`; explicit evidence remains separate from heuristics.",
                    map.label, transition.entity_index
                ));
                continue;
            };
            if target_label == map.label {
                graph.warnings.push(format!(
                    "{} trigger_changelevel #{} is a self-transition to `{target_map}`; no graph edge was added.",
                    map.label, transition.entity_index
                ));
                continue;
            }
            explicit_pairs.insert((map.label.clone(), target_label.clone()));
            graph.edges.push(CampaignAdjacencyEdge {
                from_map: map.label.clone(),
                to_map: target_label,
                evidence_kind: "trigger_changelevel".to_string(),
                confidence: "high".to_string(),
                evidence: format!(
                    "trigger_changelevel #{} targets map `{target_map}` with landmark {:?}",
                    transition.entity_index, transition.landmark
                ),
            });
        }
    }

    add_shared_landmark_edges(maps, &explicit_pairs, &mut graph);
    add_filename_sequence_edges(maps, &explicit_pairs, &mut graph);
    graph
}

fn add_shared_landmark_edges(
    maps: &[CampaignMapInput],
    explicit_pairs: &BTreeSet<(String, String)>,
    graph: &mut CampaignAdjacencyGraph,
) {
    let mut by_landmark: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for map in maps {
        for landmark in &map.landmarks.entries {
            by_landmark
                .entry(landmark.targetname.clone())
                .or_default()
                .push(map.label.clone());
        }
    }

    for (landmark, labels) in by_landmark {
        if labels.len() != 2 {
            continue;
        }
        let from = labels[0].clone();
        let to = labels[1].clone();
        if explicit_pairs.contains(&(from.clone(), to.clone()))
            || explicit_pairs.contains(&(to.clone(), from.clone()))
        {
            continue;
        }
        graph.edges.push(CampaignAdjacencyEdge {
            from_map: from.clone(),
            to_map: to.clone(),
            evidence_kind: "shared_landmark".to_string(),
            confidence: "medium".to_string(),
            evidence: format!(
                "both maps contain unique info_landmark targetname `{landmark}`; direction is heuristic"
            ),
        });
    }
}

fn add_filename_sequence_edges(
    maps: &[CampaignMapInput],
    explicit_pairs: &BTreeSet<(String, String)>,
    graph: &mut CampaignAdjacencyGraph,
) {
    let mut stems = maps
        .iter()
        .filter_map(|map| numbered_stem(&map.label).map(|numbered| (map.label.clone(), numbered)))
        .collect::<Vec<_>>();
    stems.sort_by(|left, right| left.1.cmp(&right.1));

    for pair in stems.windows(2) {
        let [(from_label, from), (to_label, to)] = pair else {
            continue;
        };
        if from.prefix != to.prefix || from.number + 1 != to.number {
            continue;
        }
        if explicit_pairs.contains(&(from_label.clone(), to_label.clone())) {
            continue;
        }
        graph.edges.push(CampaignAdjacencyEdge {
            from_map: from_label.clone(),
            to_map: to_label.clone(),
            evidence_kind: "filename_sequence".to_string(),
            confidence: "low".to_string(),
            evidence: format!(
                "map file stems share prefix `{}` and sequential numbers {} -> {}; direction follows sorted filename order",
                from.prefix, from.number, to.number
            ),
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct NumberedStem {
    prefix: String,
    number: u64,
}

fn numbered_stem(label: &str) -> Option<NumberedStem> {
    let stem = Path::new(label).file_stem()?.to_str()?.to_ascii_lowercase();
    let digits_start = stem
        .char_indices()
        .rev()
        .find(|(_, ch)| !ch.is_ascii_digit())
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    if digits_start == stem.len() {
        return None;
    }
    let (prefix, digits) = stem.split_at(digits_start);
    if prefix.is_empty() || digits.is_empty() {
        return None;
    }
    Some(NumberedStem {
        prefix: prefix.to_string(),
        number: digits.parse().ok()?,
    })
}

pub fn suggest_campaign_order(maps: &[CampaignMapInput]) -> CampaignOrderSuggestion {
    let mut suggestion = CampaignOrderSuggestion::default();
    let mut keys_by_map_name = BTreeMap::new();
    let mut labels = Vec::new();

    for map in maps {
        labels.push(map.label.clone());
        for key in label_match_keys(&map.label) {
            keys_by_map_name.insert(key, map.label.clone());
        }
    }

    let mut outgoing: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut indegree: BTreeMap<String, usize> =
        labels.iter().map(|label| (label.clone(), 0)).collect();

    for map in maps {
        for transition in &map.transitions {
            let Some(target_map) = transition.target_map.as_deref() else {
                suggestion.warnings.push(format!(
                    "{} has trigger_changelevel #{} without a target map.",
                    map.label, transition.entity_index
                ));
                continue;
            };
            let normalized_target = normalize_map_key(target_map);
            let Some(target_label) = keys_by_map_name.get(&normalized_target).cloned() else {
                suggestion.warnings.push(format!(
                    "{} points to missing map `{target_map}` via trigger_changelevel #{}.",
                    map.label, transition.entity_index
                ));
                continue;
            };
            if target_label == map.label {
                suggestion.warnings.push(format!(
                    "{} has a self-transition to `{target_map}` via trigger_changelevel #{}.",
                    map.label, transition.entity_index
                ));
                continue;
            }

            if outgoing
                .entry(map.label.clone())
                .or_default()
                .insert(target_label.clone())
            {
                *indegree.entry(target_label.clone()).or_default() += 1;
            }

            if let Some(landmark) = transition
                .landmark
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
            {
                let target_has_landmark = maps
                    .iter()
                    .find(|candidate| candidate.label == target_label)
                    .map(|candidate| candidate.landmarks.status_for(landmark))
                    .map(|status| {
                        matches!(
                            status,
                            crate::landmark::LandmarkTargetStatus::Present { .. }
                        )
                    })
                    .unwrap_or(false);
                if !target_has_landmark {
                    suggestion.warnings.push(format!(
                        "{} transition to {} references landmark `{landmark}`, but the target map does not have one usable matching landmark.",
                        map.label, target_label
                    ));
                }
                suggestion
                    .landmark_pairs
                    .push(CampaignLandmarkPairSuggestion {
                        from_map: map.label.clone(),
                        to_map: target_label.clone(),
                        target_map: target_map.to_string(),
                        landmark: landmark.to_string(),
                        target_has_landmark,
                    });
            }
        }
    }

    suggestion.ordered_labels = topological_order(&labels, &outgoing, &indegree);
    suggestion
}

fn topological_order(
    labels: &[String],
    outgoing: &BTreeMap<String, BTreeSet<String>>,
    indegree: &BTreeMap<String, usize>,
) -> Vec<String> {
    let mut indegree = indegree.clone();
    let mut queue = labels
        .iter()
        .filter(|label| indegree.get(*label).copied().unwrap_or(0) == 0)
        .cloned()
        .collect::<VecDeque<_>>();
    let mut ordered = Vec::new();
    let mut seen = BTreeSet::new();

    while let Some(label) = queue.pop_front() {
        if !seen.insert(label.clone()) {
            continue;
        }
        ordered.push(label.clone());
        if let Some(targets) = outgoing.get(&label) {
            for target in targets {
                let entry = indegree.entry(target.clone()).or_default();
                *entry = entry.saturating_sub(1);
                if *entry == 0 {
                    queue.push_back(target.clone());
                }
            }
        }
    }

    for label in labels {
        if seen.insert(label.clone()) {
            ordered.push(label.clone());
        }
    }

    ordered
}

fn label_match_keys(label: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    keys.insert(normalize_map_key(label));
    let path = Path::new(label);
    if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
        keys.insert(normalize_map_key(name));
    }
    if let Some(stem) = path.file_stem().and_then(|value| value.to_str()) {
        keys.insert(normalize_map_key(stem));
    }
    keys
}

fn normalize_map_key(value: &str) -> String {
    let value = value.trim().replace('\\', "/");
    let path = Path::new(&value);
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(value.as_str())
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{discover_landmarks, discover_transitions, parse_document};

    #[test]
    fn suggests_linear_order_and_landmark_pairs() {
        let first = parse_document(
            r#"
world { "id" "1" }
entity { "id" "2" "classname" "info_landmark" "targetname" "lm" "origin" "0 0 0" }
entity { "id" "3" "classname" "trigger_changelevel" "map" "second" "landmark" "lm" }
"#,
        )
        .unwrap();
        let second = parse_document(
            r#"
world { "id" "1" }
entity { "id" "2" "classname" "info_landmark" "targetname" "lm" "origin" "10 0 0" }
"#,
        )
        .unwrap();

        let suggestion = suggest_campaign_order(&[
            CampaignMapInput {
                label: "first.vmf".to_string(),
                transitions: discover_transitions(&first),
                landmarks: discover_landmarks(&first),
            },
            CampaignMapInput {
                label: "second.vmf".to_string(),
                transitions: discover_transitions(&second),
                landmarks: discover_landmarks(&second),
            },
        ]);

        assert_eq!(suggestion.ordered_labels, vec!["first.vmf", "second.vmf"]);
        assert_eq!(suggestion.landmark_pairs.len(), 1);
        assert!(suggestion.landmark_pairs[0].target_has_landmark);
        assert!(suggestion.warnings.is_empty());
    }

    #[test]
    fn builds_adjacency_graph_with_explicit_and_heuristic_edges() {
        let first = parse_document(
            r#"
world { "id" "1" }
entity { "id" "2" "classname" "info_landmark" "targetname" "shared_lm" "origin" "0 0 0" }
entity { "id" "3" "classname" "trigger_changelevel" "map" "d1_test_02" "landmark" "shared_lm" }
"#,
        )
        .unwrap();
        let second = parse_document(
            r#"
world { "id" "1" }
entity { "id" "2" "classname" "info_landmark" "targetname" "shared_lm" "origin" "10 0 0" }
"#,
        )
        .unwrap();
        let third = parse_document(
            r#"
world { "id" "1" }
entity { "id" "2" "classname" "info_landmark" "targetname" "solo_lm" "origin" "20 0 0" }
"#,
        )
        .unwrap();

        let graph = build_campaign_adjacency_graph(&[
            CampaignMapInput {
                label: "d1_test_01.vmf".to_string(),
                transitions: discover_transitions(&first),
                landmarks: discover_landmarks(&first),
            },
            CampaignMapInput {
                label: "d1_test_02.vmf".to_string(),
                transitions: discover_transitions(&second),
                landmarks: discover_landmarks(&second),
            },
            CampaignMapInput {
                label: "d1_test_03.vmf".to_string(),
                transitions: discover_transitions(&third),
                landmarks: discover_landmarks(&third),
            },
        ]);

        assert!(graph.edges.iter().any(|edge| {
            edge.evidence_kind == "trigger_changelevel"
                && edge.confidence == "high"
                && edge.from_map == "d1_test_01.vmf"
                && edge.to_map == "d1_test_02.vmf"
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.evidence_kind == "filename_sequence"
                && edge.confidence == "low"
                && edge.from_map == "d1_test_02.vmf"
                && edge.to_map == "d1_test_03.vmf"
        }));
        assert!(!graph.edges.iter().any(|edge| {
            edge.evidence_kind != "trigger_changelevel"
                && edge.from_map == "d1_test_01.vmf"
                && edge.to_map == "d1_test_02.vmf"
        }));
    }

    #[test]
    fn warns_about_missing_target_and_missing_landmark() {
        let first = parse_document(
            r#"
world { "id" "1" }
entity { "id" "3" "classname" "trigger_changelevel" "map" "missing" "landmark" "lm" }
entity { "id" "4" "classname" "trigger_changelevel" "map" "second" "landmark" "lm" }
"#,
        )
        .unwrap();
        let second = parse_document("world { \"id\" \"1\" }").unwrap();

        let suggestion = suggest_campaign_order(&[
            CampaignMapInput {
                label: "first.vmf".to_string(),
                transitions: discover_transitions(&first),
                landmarks: discover_landmarks(&first),
            },
            CampaignMapInput {
                label: "second.vmf".to_string(),
                transitions: discover_transitions(&second),
                landmarks: discover_landmarks(&second),
            },
        ]);

        assert_eq!(suggestion.ordered_labels, vec!["first.vmf", "second.vmf"]);
        assert_eq!(suggestion.landmark_pairs.len(), 1);
        assert!(!suggestion.landmark_pairs[0].target_has_landmark);
        assert!(
            suggestion
                .warnings
                .iter()
                .any(|warning| warning.contains("missing map"))
        );
        assert!(
            suggestion
                .warnings
                .iter()
                .any(|warning| warning.contains("target map does not have"))
        );
    }
}
