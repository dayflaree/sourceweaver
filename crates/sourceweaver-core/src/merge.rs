use crate::changelevel::{
    ChangelevelPolicy, ChangelevelPolicyOptions, ChangelevelPolicyReport, apply_changelevel_policy,
    normalize_map_name,
};
use crate::id_references::{is_list_id_reference_key, is_single_id_reference_key};
use crate::integrity::validate_merge_inputs;
use crate::transform::{Vec3, find_landmark_origin, translate_block};
use crate::vmf::{Document, Node};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct MergeInput {
    pub label: String,
    pub document: Document,
}

#[derive(Debug, Clone, Default)]
pub struct MergeOptions {
    pub landmark: Option<String>,
    pub changelevel: ChangelevelPolicyOptions,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MergeReport {
    pub merged_maps: usize,
    pub appended_world_solids: usize,
    pub appended_entities: usize,
    pub applied_offsets: Vec<(String, Vec3)>,
    pub changelevel: ChangelevelPolicyReport,
}

pub fn merge_maps(
    inputs: Vec<MergeInput>,
    options: &MergeOptions,
) -> Result<(Document, MergeReport), String> {
    let input_refs = inputs
        .iter()
        .map(|input| (input.label.as_str(), &input.document))
        .collect::<Vec<_>>();
    let integrity = validate_merge_inputs(&input_refs);
    if let Some(message) = integrity.error_message() {
        return Err(message);
    }

    let all_labels = inputs
        .iter()
        .map(|input| input.label.clone())
        .collect::<Vec<_>>();
    let mut iter = inputs.into_iter();
    let first = iter.next().expect("checked non-empty");
    let base_label = first.label.clone();
    let mut base = first.document;
    ensure_world_exists(&mut base)?;

    let base_landmark = options
        .landmark
        .as_deref()
        .and_then(|name| find_landmark_origin(&base, name));

    let mut used_ids = collect_ids(&base);
    let mut next_id = used_ids
        .iter()
        .next_back()
        .copied()
        .unwrap_or(0)
        .saturating_add(1);

    let mut report = MergeReport {
        merged_maps: 1,
        appended_world_solids: 0,
        appended_entities: 0,
        applied_offsets: vec![(base_label, Vec3::ZERO)],
        changelevel: ChangelevelPolicyReport {
            policy: options.changelevel.policy,
            changed: Vec::new(),
            warnings: Vec::new(),
        },
    };

    for input in iter {
        let offset = compute_offset(&input.document, options.landmark.as_deref(), base_landmark);
        let mut incoming_world_children = Vec::new();
        let mut incoming_entities = Vec::new();

        for node in input.document.nodes {
            match node {
                Node::Block { name, body } if name == "world" => {
                    for child in body {
                        if child.block_name() == Some("solid") {
                            incoming_world_children.push(child);
                        }
                    }
                }
                Node::Block { ref name, .. } if name == "entity" => incoming_entities.push(node),
                _ => {}
            }
        }

        let mut id_remap = IdRemap::default();
        for child in &mut incoming_world_children {
            translate_block(child, offset);
            renumber_ids(child, &mut next_id, &mut used_ids, &mut id_remap);
        }
        for entity in &mut incoming_entities {
            translate_block(entity, offset);
            renumber_ids(entity, &mut next_id, &mut used_ids, &mut id_remap);
        }
        for child in &mut incoming_world_children {
            remap_id_references(child, &id_remap);
        }
        for entity in &mut incoming_entities {
            remap_id_references(entity, &id_remap);
        }

        let world = base
            .first_top_level_block_mut("world")
            .and_then(Node::as_body_mut)
            .ok_or_else(|| "base VMF has no editable world block".to_string())?;
        report.appended_world_solids += incoming_world_children.len();
        world.extend(incoming_world_children);

        report.appended_entities += incoming_entities.len();
        base.nodes.extend(incoming_entities);
        report.merged_maps += 1;
        report.applied_offsets.push((input.label, offset));
    }

    let mut changelevel_options = options.changelevel.clone();
    if changelevel_options.stitched_maps.is_empty() {
        changelevel_options.stitched_maps = all_labels
            .iter()
            .map(|label| normalize_map_name(label))
            .filter(|label| !label.is_empty())
            .collect();
    }
    if matches!(changelevel_options.policy, ChangelevelPolicy::Preserve) {
        report.changelevel = ChangelevelPolicyReport {
            policy: ChangelevelPolicy::Preserve,
            changed: Vec::new(),
            warnings: Vec::new(),
        };
    } else {
        report.changelevel = apply_changelevel_policy(&mut base, &changelevel_options);
    }

    Ok((base, report))
}

fn ensure_world_exists(document: &mut Document) -> Result<(), String> {
    if document.top_level_blocks("world").count() > 0 {
        return Ok(());
    }
    Err("base VMF does not contain a world block".to_string())
}

fn compute_offset(
    document: &Document,
    landmark: Option<&str>,
    base_landmark: Option<Vec3>,
) -> Vec3 {
    match (landmark, base_landmark) {
        (Some(name), Some(base_origin)) => find_landmark_origin(document, name)
            .map(|incoming_origin| base_origin - incoming_origin)
            .unwrap_or(Vec3::ZERO),
        _ => Vec3::ZERO,
    }
}

fn collect_ids(document: &Document) -> BTreeSet<i64> {
    let mut ids = BTreeSet::new();
    for node in &document.nodes {
        collect_ids_from_node(node, &mut ids);
    }
    ids
}

fn collect_ids_from_node(node: &Node, ids: &mut BTreeSet<i64>) {
    match node {
        Node::Property { key, value } if key == "id" => {
            if let Ok(id) = value.parse::<i64>() {
                ids.insert(id);
            }
        }
        Node::Block { body, .. } => {
            for child in body {
                collect_ids_from_node(child, ids);
            }
        }
        _ => {}
    }
}

#[derive(Debug, Default)]
struct IdRemap {
    ids: BTreeMap<i64, Vec<i64>>,
}

impl IdRemap {
    fn insert(&mut self, old_id: i64, new_id: i64) {
        self.ids.entry(old_id).or_default().push(new_id);
    }

    fn unique(&self, old_id: i64) -> Option<i64> {
        let new_ids = self.ids.get(&old_id)?;
        (new_ids.len() == 1).then_some(new_ids[0])
    }
}

fn renumber_ids(
    node: &mut Node,
    next_id: &mut i64,
    used_ids: &mut BTreeSet<i64>,
    id_remap: &mut IdRemap,
) {
    match node {
        Node::Property { key, value } if key == "id" => {
            let old_id = value.parse::<i64>().ok();
            while used_ids.contains(next_id) {
                *next_id = next_id.saturating_add(1);
            }
            let new_id = *next_id;
            *value = new_id.to_string();
            used_ids.insert(new_id);
            if let Some(old_id) = old_id {
                id_remap.insert(old_id, new_id);
            }
            *next_id = next_id.saturating_add(1);
        }
        Node::Block { body, .. } => {
            for child in body {
                renumber_ids(child, next_id, used_ids, id_remap);
            }
        }
        _ => {}
    }
}

fn remap_id_references(node: &mut Node, id_remap: &IdRemap) {
    match node {
        Node::Property { key, value } if is_single_id_reference_key(key) => {
            if let Some(new_id) = value
                .parse::<i64>()
                .ok()
                .and_then(|old_id| id_remap.unique(old_id))
            {
                *value = new_id.to_string();
            }
        }
        Node::Property { key, value } if is_list_id_reference_key(key) => {
            let mut changed = false;
            let remapped = value
                .split_whitespace()
                .map(|part| match part.parse::<i64>() {
                    Ok(old_id) => match id_remap.unique(old_id) {
                        Some(new_id) => {
                            changed = true;
                            new_id.to_string()
                        }
                        None => part.to_string(),
                    },
                    Err(_) => part.to_string(),
                })
                .collect::<Vec<_>>();
            if changed {
                *value = remapped.join(" ");
            }
        }
        Node::Block { body, .. } => {
            for child in body {
                remap_id_references(child, id_remap);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmf::parse_document;

    #[test]
    fn merges_world_solids_entities_and_aligns_landmark() {
        let base = parse_document(
            r#"
world { "id" "1" }
entity { "id" "2" "classname" "info_landmark" "targetname" "lm" "origin" "0 0 0" }
"#,
        )
        .unwrap();
        let add = parse_document(
            r#"
world { "id" "1" solid { "id" "2" side { "id" "3" "plane" "(0 0 0) (1 0 0) (1 1 0)" } } }
entity { "id" "4" "classname" "info_landmark" "targetname" "lm" "origin" "100 0 0" }
entity { "id" "5" "classname" "prop_static" "origin" "128 0 0" }
"#,
        )
        .unwrap();
        let (merged, report) = merge_maps(
            vec![
                MergeInput {
                    label: "base".into(),
                    document: base,
                },
                MergeInput {
                    label: "add".into(),
                    document: add,
                },
            ],
            &MergeOptions {
                landmark: Some("lm".into()),
                ..MergeOptions::default()
            },
        )
        .unwrap();
        assert_eq!(report.appended_world_solids, 1);
        assert_eq!(report.appended_entities, 2);
        let vmf = merged.to_vmf_string();
        assert!(vmf.contains("\"origin\" \"28 0 0\""));
    }

    #[test]
    fn remaps_known_id_references_after_renumbering() {
        let base = parse_document(r#"world { "id" "100" }"#).unwrap();
        let add = parse_document(include_str!(
            "../../../tests/fixtures/id_reference_remap_fields.vmf"
        ))
        .unwrap();

        let (merged, _) = merge_maps(
            vec![
                MergeInput {
                    label: "base".into(),
                    document: base,
                },
                MergeInput {
                    label: "add".into(),
                    document: add,
                },
            ],
            &MergeOptions::default(),
        )
        .unwrap();

        let vmf = merged.to_vmf_string();
        assert!(vmf.contains("\"id\" \"101\""));
        assert!(vmf.contains("\"id\" \"102\""));
        assert!(vmf.contains("\"id\" \"103\""));
        assert!(vmf.contains("\"id\" \"104\""));
        assert!(vmf.contains("\"id\" \"105\""));
        assert!(vmf.contains("\"id\" \"106\""));
        assert!(vmf.contains("\"groupid\" \"103\""));
        assert!(vmf.contains("\"parentid\" \"101\""));
        assert!(vmf.contains("\"sides\" \"102 105 999\""));
        assert!(vmf.contains("\"sideid\" \"102\""));
        assert!(vmf.contains("\"solidid\" \"101\""));
        assert!(vmf.contains("\"entityid\" \"103\""));
        assert!(vmf.contains("\"nodeid\" \"104\""));
        assert!(vmf.contains("\"visgroupid\" \"30\""));
    }

    #[test]
    fn leaves_ambiguous_duplicate_id_references_unmapped() {
        let base = parse_document(r#"world { "id" "100" }"#).unwrap();
        let add = parse_document(
            r#"
world {
  "id" "1"
  solid { "id" "10" side { "id" "11" "plane" "(0 0 0) (1 0 0) (1 1 0)" } }
  solid { "id" "12" side { "id" "11" "plane" "(0 0 2) (1 0 2) (1 1 2)" } }
}
entity { "id" "20" "classname" "info_overlay" "sides" "11" "sideid" "11" }
"#,
        )
        .unwrap();

        let (merged, _) = merge_maps(
            vec![
                MergeInput {
                    label: "base".into(),
                    document: base,
                },
                MergeInput {
                    label: "add".into(),
                    document: add,
                },
            ],
            &MergeOptions::default(),
        )
        .unwrap();

        let vmf = merged.to_vmf_string();
        assert!(vmf.contains("\"id\" \"102\""));
        assert!(vmf.contains("\"id\" \"104\""));
        assert!(vmf.contains("\"sides\" \"11\""));
        assert!(vmf.contains("\"sideid\" \"11\""));
    }

    #[test]
    fn preserves_base_editor_metadata_and_ignores_incoming_top_level_metadata() {
        let base = parse_document(
            r#"
versioninfo { "editorversion" "400" }
viewsettings { "bSnapToGrid" "1" }
visgroups { visgroup { "name" "base_visgroup" "visgroupid" "10" } }
cameras { "activecamera" "0" camera { "position" "[0 0 0]" } }
cordons { "active" "0" cordon { "name" "base_cordon" } }
world { "id" "100" editor { "color" "220 220 220" } }
"#,
        )
        .unwrap();
        let add = parse_document(
            r#"
versioninfo { "editorversion" "999" }
viewsettings { "bSnapToGrid" "0" }
visgroups { visgroup { "name" "incoming_visgroup" "visgroupid" "20" } }
cameras { camera { "position" "[999 999 999]" } }
cordons { cordon { "name" "incoming_cordon" } }
world {
  "id" "1"
  solid { "id" "2" side { "id" "3" "plane" "(0 0 0) (1 0 0) (1 1 0)" } editor { "color" "255 0 0" } }
}
entity { "id" "4" "classname" "prop_static" editor { "color" "0 255 0" } }
"#,
        )
        .unwrap();

        let (merged, report) = merge_maps(
            vec![
                MergeInput {
                    label: "base".into(),
                    document: base,
                },
                MergeInput {
                    label: "add".into(),
                    document: add,
                },
            ],
            &MergeOptions::default(),
        )
        .unwrap();

        assert_eq!(report.appended_world_solids, 1);
        assert_eq!(report.appended_entities, 1);
        let vmf = merged.to_vmf_string();
        assert!(vmf.contains("base_visgroup"));
        assert!(vmf.contains("base_cordon"));
        assert!(vmf.contains("\"bSnapToGrid\" \"1\""));
        assert!(vmf.contains("\"editorversion\" \"400\""));
        assert!(!vmf.contains("incoming_visgroup"));
        assert!(!vmf.contains("incoming_cordon"));
        assert!(!vmf.contains("\"editorversion\" \"999\""));
        assert!(vmf.contains("\"color\" \"255 0 0\""));
        assert!(vmf.contains("\"color\" \"0 255 0\""));
    }
}
