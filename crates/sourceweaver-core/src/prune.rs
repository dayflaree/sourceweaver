use crate::classify::{BrushRole, block_roles, classify_solid_roles};
use crate::vmf::{Document, Node};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeletionCriteria {
    pub classnames: BTreeSet<String>,
    pub targetnames: BTreeSet<String>,
    pub brush_roles: BTreeSet<BrushRole>,
}

impl DeletionCriteria {
    pub fn is_empty(&self) -> bool {
        self.classnames.is_empty() && self.targetnames.is_empty() && self.brush_roles.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeletionReport {
    pub removed_entities: usize,
    pub removed_world_solids: usize,
}

pub fn prune_document(document: &mut Document, criteria: &DeletionCriteria) -> DeletionReport {
    let mut report = DeletionReport::default();
    if criteria.is_empty() {
        return report;
    }

    let mut retained = Vec::new();
    for mut node in document.nodes.drain(..) {
        match &mut node {
            Node::Block { name, .. } if name == "entity" => {
                if should_delete_entity(&node, criteria) {
                    report.removed_entities += 1;
                } else {
                    retained.push(node);
                }
            }
            Node::Block { name, body } if name == "world" => {
                report.removed_world_solids += prune_world_body(body, criteria);
                retained.push(node);
            }
            _ => retained.push(node),
        }
    }
    document.nodes = retained;
    report
}

fn should_delete_entity(node: &Node, criteria: &DeletionCriteria) -> bool {
    let Some(body) = node.as_body() else {
        return false;
    };
    if let Some(classname) = Node::get_property(body, "classname") {
        if criteria.classnames.contains(classname) {
            return true;
        }
    }
    if let Some(targetname) = Node::get_property(body, "targetname") {
        if criteria.targetnames.contains(targetname) {
            return true;
        }
    }
    if !criteria.brush_roles.is_empty() {
        let roles = block_roles(node);
        if roles.iter().any(|role| criteria.brush_roles.contains(role)) {
            return true;
        }
    }
    false
}

fn prune_world_body(body: &mut Vec<Node>, criteria: &DeletionCriteria) -> usize {
    let mut removed = 0;
    let mut retained = Vec::new();

    for mut node in body.drain(..) {
        match &mut node {
            Node::Block { name, .. } if name == "solid" => {
                let roles = classify_solid_roles(&node, None, true);
                if roles.iter().any(|role| criteria.brush_roles.contains(role)) {
                    removed += 1;
                } else {
                    retained.push(node);
                }
            }
            Node::Block {
                body: child_body, ..
            } => {
                removed += prune_world_body(child_body, criteria);
                retained.push(node);
            }
            _ => retained.push(node),
        }
    }

    *body = retained;
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmf::parse_document;

    #[test]
    fn removes_entities_by_classname() {
        let mut doc = parse_document(
            r#"
world { "id" "1" }
entity { "id" "2" "classname" "prop_static" }
entity { "id" "3" "classname" "info_player_start" }
"#,
        )
        .unwrap();
        let mut criteria = DeletionCriteria::default();
        criteria.classnames.insert("prop_static".into());
        let report = prune_document(&mut doc, &criteria);
        assert_eq!(report.removed_entities, 1);
        assert_eq!(doc.top_level_blocks("entity").count(), 1);
    }
}
