use crate::classify::{BrushRole, block_roles, classify_solid_roles};
use crate::vmf::{Document, Node};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletionCriteria {
    pub classnames: BTreeSet<String>,
    pub targetnames: BTreeSet<String>,
    pub brush_roles: BTreeSet<BrushRole>,
    pub brush_entity_mode: BrushEntityDeletionMode,
    pub protect_critical_entities: bool,
}

impl Default for DeletionCriteria {
    fn default() -> Self {
        Self {
            classnames: BTreeSet::new(),
            targetnames: BTreeSet::new(),
            brush_roles: BTreeSet::new(),
            brush_entity_mode: BrushEntityDeletionMode::WholeEntity,
            protect_critical_entities: true,
        }
    }
}

impl DeletionCriteria {
    pub fn is_empty(&self) -> bool {
        self.classnames.is_empty() && self.targetnames.is_empty() && self.brush_roles.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushEntityDeletionMode {
    WholeEntity,
    MatchingSolids,
}

impl Default for BrushEntityDeletionMode {
    fn default() -> Self {
        Self::WholeEntity
    }
}

impl BrushEntityDeletionMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "whole-entity" | "whole_entity" | "entity" | "delete-entity" => Some(Self::WholeEntity),
            "matching-solids" | "matching_solids" | "solids" | "contained-brushes" => {
                Some(Self::MatchingSolids)
            }
            _ => None,
        }
    }
}

impl std::fmt::Display for BrushEntityDeletionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WholeEntity => f.write_str("whole-entity"),
            Self::MatchingSolids => f.write_str("matching-solids"),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeletionReport {
    pub removed_entities: usize,
    pub removed_world_solids: usize,
    pub removed_brush_entity_solids: usize,
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
                    if criteria.brush_entity_mode == BrushEntityDeletionMode::MatchingSolids {
                        report.removed_brush_entity_solids +=
                            prune_brush_entity_body(&mut node, criteria);
                    }
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

    if criteria.protect_critical_entities && is_protected_entity_body(body) {
        return false;
    }

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
        if criteria.brush_entity_mode == BrushEntityDeletionMode::MatchingSolids {
            return false;
        }
        let roles = block_roles(node);
        if roles.iter().any(|role| criteria.brush_roles.contains(role)) {
            return true;
        }
    }
    false
}

fn prune_brush_entity_body(node: &mut Node, criteria: &DeletionCriteria) -> usize {
    let Some(body) = node.as_body() else {
        return 0;
    };
    if criteria.protect_critical_entities && is_protected_entity_body(body) {
        return 0;
    }
    let Some(body) = node.as_body_mut() else {
        return 0;
    };
    prune_contained_solids(body, criteria, false)
}

fn prune_world_body(body: &mut Vec<Node>, criteria: &DeletionCriteria) -> usize {
    prune_contained_solids(body, criteria, true)
}

fn prune_contained_solids(
    body: &mut Vec<Node>,
    criteria: &DeletionCriteria,
    is_world: bool,
) -> usize {
    let mut removed = 0;
    let mut retained = Vec::new();

    for mut node in body.drain(..) {
        match &mut node {
            Node::Block { name, .. } if name == "solid" => {
                let roles = classify_solid_roles(&node, None, is_world);
                let brush_entity_role_matches =
                    !is_world && criteria.brush_roles.contains(&BrushRole::BrushEntity);
                if brush_entity_role_matches
                    || roles.iter().any(|role| criteria.brush_roles.contains(role))
                {
                    removed += 1;
                } else {
                    retained.push(node);
                }
            }
            Node::Block {
                body: child_body, ..
            } => {
                removed += prune_contained_solids(child_body, criteria, is_world);
                retained.push(node);
            }
            _ => retained.push(node),
        }
    }

    *body = retained;
    removed
}

fn is_protected_entity_body(body: &[Node]) -> bool {
    Node::get_property(body, "classname")
        .map(|classname| {
            matches!(
                classname,
                "info_player_start"
                    | "info_player_deathmatch"
                    | "info_landmark"
                    | "trigger_changelevel"
                    | "logic_auto"
                    | "logic_relay"
                    | "env_global"
                    | "info_node"
                    | "info_node_hint"
            )
        })
        .unwrap_or(false)
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

    #[test]
    fn deletes_world_solids_by_role() {
        let mut doc = parse_document(
            r#"
world {
  "id" "1"
  solid { "id" "2" side { "id" "3" "material" "TOOLS/TOOLSCLIP" } }
  solid { "id" "4" side { "id" "5" "material" "BRICK/WALL001" } }
}
"#,
        )
        .unwrap();
        let mut criteria = DeletionCriteria::default();
        criteria.brush_roles.insert(BrushRole::Clip);

        let report = prune_document(&mut doc, &criteria);

        assert_eq!(report.removed_entities, 0);
        assert_eq!(report.removed_world_solids, 1);
        assert_eq!(report.removed_brush_entity_solids, 0);
        assert_eq!(doc.to_vmf_string().matches("solid").count(), 1);
    }

    #[test]
    fn whole_entity_mode_deletes_matching_brush_entities() {
        let mut doc = parse_document(
            r#"
world { "id" "1" }
entity { "id" "2" "classname" "trigger_once" solid { "id" "3" side { "id" "4" "material" "TOOLS/TOOLSTRIGGER" } } }
"#,
        )
        .unwrap();
        let mut criteria = DeletionCriteria::default();
        criteria.brush_roles.insert(BrushRole::Trigger);
        criteria.brush_entity_mode = BrushEntityDeletionMode::WholeEntity;

        let report = prune_document(&mut doc, &criteria);

        assert_eq!(report.removed_entities, 1);
        assert_eq!(report.removed_brush_entity_solids, 0);
        assert_eq!(doc.top_level_blocks("entity").count(), 0);
    }

    #[test]
    fn matching_solids_mode_keeps_brush_entity_and_deletes_matching_solids() {
        let mut doc = parse_document(
            r#"
world { "id" "1" }
entity {
  "id" "2" "classname" "func_detail"
  solid { "id" "3" side { "id" "4" "material" "TOOLS/TOOLSTRIGGER" } }
  solid { "id" "5" side { "id" "6" "material" "BRICK/WALL001" } }
}
"#,
        )
        .unwrap();
        let mut criteria = DeletionCriteria::default();
        criteria.brush_roles.insert(BrushRole::Trigger);
        criteria.brush_entity_mode = BrushEntityDeletionMode::MatchingSolids;

        let report = prune_document(&mut doc, &criteria);

        assert_eq!(report.removed_entities, 0);
        assert_eq!(report.removed_brush_entity_solids, 1);
        assert_eq!(doc.top_level_blocks("entity").count(), 1);
        assert_eq!(doc.to_vmf_string().matches("solid").count(), 1);
    }

    #[test]
    fn matching_solids_mode_with_brush_entity_role_removes_all_contained_solids() {
        let mut doc = parse_document(
            r#"
world { "id" "1" }
entity {
  "id" "2" "classname" "func_detail"
  solid { "id" "3" side { "id" "4" "material" "BRICK/WALL001" } }
  solid { "id" "5" side { "id" "6" "material" "BRICK/WALL002" } }
}
"#,
        )
        .unwrap();
        let mut criteria = DeletionCriteria::default();
        criteria.brush_roles.insert(BrushRole::BrushEntity);
        criteria.brush_entity_mode = BrushEntityDeletionMode::MatchingSolids;

        let report = prune_document(&mut doc, &criteria);

        assert_eq!(report.removed_entities, 0);
        assert_eq!(report.removed_brush_entity_solids, 2);
        assert_eq!(doc.top_level_blocks("entity").count(), 1);
        assert_eq!(doc.to_vmf_string().matches("solid").count(), 0);
    }

    #[test]
    fn protects_critical_entities_by_default() {
        let mut doc = parse_document(
            r#"
world { "id" "1" }
entity { "id" "2" "classname" "info_landmark" "targetname" "lm" }
"#,
        )
        .unwrap();
        let mut criteria = DeletionCriteria::default();
        criteria.classnames.insert("info_landmark".into());
        criteria.protect_critical_entities = true;

        let report = prune_document(&mut doc, &criteria);

        assert_eq!(report.removed_entities, 0);
        assert_eq!(doc.top_level_blocks("entity").count(), 1);
    }
}
