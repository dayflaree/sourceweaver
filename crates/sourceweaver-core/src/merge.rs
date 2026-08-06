use crate::transform::{Vec3, find_landmark_origin, translate_block};
use crate::vmf::{Document, Node};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct MergeInput {
    pub label: String,
    pub document: Document,
}

#[derive(Debug, Clone, Default)]
pub struct MergeOptions {
    pub landmark: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MergeReport {
    pub merged_maps: usize,
    pub appended_world_solids: usize,
    pub appended_entities: usize,
    pub applied_offsets: Vec<(String, Vec3)>,
}

pub fn merge_maps(
    inputs: Vec<MergeInput>,
    options: &MergeOptions,
) -> Result<(Document, MergeReport), String> {
    if inputs.is_empty() {
        return Err("merge needs at least one VMF".to_string());
    }

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

        for child in &mut incoming_world_children {
            translate_block(child, offset);
            renumber_ids(child, &mut next_id, &mut used_ids);
        }
        for entity in &mut incoming_entities {
            translate_block(entity, offset);
            renumber_ids(entity, &mut next_id, &mut used_ids);
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

fn renumber_ids(node: &mut Node, next_id: &mut i64, used_ids: &mut BTreeSet<i64>) {
    match node {
        Node::Property { key, value } if key == "id" => {
            while used_ids.contains(next_id) {
                *next_id = next_id.saturating_add(1);
            }
            *value = next_id.to_string();
            used_ids.insert(*next_id);
            *next_id = next_id.saturating_add(1);
        }
        Node::Block { body, .. } => {
            for child in body {
                renumber_ids(child, next_id, used_ids);
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
            },
        )
        .unwrap();
        assert_eq!(report.appended_world_solids, 1);
        assert_eq!(report.appended_entities, 2);
        let vmf = merged.to_vmf_string();
        assert!(vmf.contains("\"origin\" \"28 0 0\""));
    }
}
