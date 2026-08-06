use crate::transform::Vec3;
use crate::vmf::{Document, Node};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BrushRole {
    Trigger,
    Clip,
    Areaportal,
    Skybox,
    Occluder,
    Hint,
    Skip,
    Nodraw,
    Water,
    Tool(String),
    WorldBrush,
    BrushEntity,
    Other,
}

impl BrushRole {
    pub fn parse_filter(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "trigger" | "triggers" => Some(Self::Trigger),
            "clip" | "clips" => Some(Self::Clip),
            "areaportal" | "areaportals" => Some(Self::Areaportal),
            "skybox" | "skyboxes" => Some(Self::Skybox),
            "occluder" | "occluders" => Some(Self::Occluder),
            "hint" | "hints" => Some(Self::Hint),
            "skip" => Some(Self::Skip),
            "nodraw" => Some(Self::Nodraw),
            "water" => Some(Self::Water),
            "world" | "worldbrush" | "world_brush" => Some(Self::WorldBrush),
            "brushentity" | "brush_entity" | "brush-entity" => Some(Self::BrushEntity),
            "other" => Some(Self::Other),
            text if text.starts_with("tools/") => Some(Self::Tool(text.to_string())),
            _ => None,
        }
    }
}

impl fmt::Display for BrushRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrushRole::Trigger => write!(f, "trigger"),
            BrushRole::Clip => write!(f, "clip"),
            BrushRole::Areaportal => write!(f, "areaportal"),
            BrushRole::Skybox => write!(f, "skybox"),
            BrushRole::Occluder => write!(f, "occluder"),
            BrushRole::Hint => write!(f, "hint"),
            BrushRole::Skip => write!(f, "skip"),
            BrushRole::Nodraw => write!(f, "nodraw"),
            BrushRole::Water => write!(f, "water"),
            BrushRole::Tool(material) => write!(f, "{material}"),
            BrushRole::WorldBrush => write!(f, "world-brush"),
            BrushRole::BrushEntity => write!(f, "brush-entity"),
            BrushRole::Other => write!(f, "other"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityRecord {
    pub index: usize,
    pub block_name: String,
    pub classname: Option<String>,
    pub targetname: Option<String>,
    pub origin: Option<Vec3>,
    pub solid_count: usize,
    pub roles: Vec<BrushRole>,
}

impl EntityRecord {
    pub fn display_name(&self) -> String {
        self.classname
            .clone()
            .or_else(|| self.targetname.clone())
            .unwrap_or_else(|| self.block_name.clone())
    }
}

pub fn inspect_entities(document: &Document) -> Vec<EntityRecord> {
    let mut records = Vec::new();
    for node in &document.nodes {
        match node {
            Node::Block { name, body } if name == "world" || name == "entity" => {
                let classname = Node::get_property(body, "classname").map(ToOwned::to_owned);
                let targetname = Node::get_property(body, "targetname").map(ToOwned::to_owned);
                let origin = Node::get_property(body, "origin").and_then(Vec3::parse);
                let solid_count = count_blocks(body, "solid");
                let mut roles = classify_block_roles(name, classname.as_deref(), body);
                roles.sort();
                roles.dedup();
                records.push(EntityRecord {
                    index: records.len(),
                    block_name: name.clone(),
                    classname,
                    targetname,
                    origin,
                    solid_count,
                    roles,
                });
            }
            _ => {}
        }
    }
    records
}

pub fn summarize_entity_types(document: &Document) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for record in inspect_entities(document) {
        let key = record.classname.unwrap_or(record.block_name);
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

pub fn classify_solid_roles(
    solid: &Node,
    parent_classname: Option<&str>,
    parent_is_world: bool,
) -> Vec<BrushRole> {
    let mut roles = Vec::new();
    if parent_is_world {
        roles.push(BrushRole::WorldBrush);
    } else {
        roles.push(BrushRole::BrushEntity);
    }

    if let Some(classname) = parent_classname {
        classify_classname(classname, &mut roles);
    }

    let mut materials = Vec::new();
    collect_materials(solid, &mut materials);
    for material in materials {
        classify_material(&material, &mut roles);
    }

    roles.sort();
    roles.dedup();
    if roles.len() == 1 {
        roles.push(BrushRole::Other);
    }
    roles
}

pub fn block_roles(node: &Node) -> Vec<BrushRole> {
    match node {
        Node::Block { name, body } if name == "world" || name == "entity" => {
            let classname = Node::get_property(body, "classname");
            let mut roles = classify_block_roles(name, classname, body);
            roles.sort();
            roles.dedup();
            roles
        }
        Node::Block { name, .. } if name == "solid" => classify_solid_roles(node, None, true),
        _ => Vec::new(),
    }
}

fn classify_block_roles(
    block_name: &str,
    classname: Option<&str>,
    body: &[Node],
) -> Vec<BrushRole> {
    let mut roles = Vec::new();
    if let Some(classname) = classname {
        classify_classname(classname, &mut roles);
    }
    collect_solid_roles(body, classname, block_name == "world", &mut roles);
    roles.sort();
    roles.dedup();
    roles
}

fn classify_classname(classname: &str, roles: &mut Vec<BrushRole>) {
    let class = classname.to_ascii_lowercase();
    if class.starts_with("trigger_") || class == "trigger" {
        roles.push(BrushRole::Trigger);
    }
    if class == "func_areaportal" || class == "func_areaportalwindow" {
        roles.push(BrushRole::Areaportal);
    }
    if class == "func_occluder" {
        roles.push(BrushRole::Occluder);
    }
    if class == "func_clip_vphysics" || class.contains("clip") {
        roles.push(BrushRole::Clip);
    }
    if class == "water_lod_control" || class.contains("water") {
        roles.push(BrushRole::Water);
    }
}

fn classify_material(material: &str, roles: &mut Vec<BrushRole>) {
    let normalized = material.replace('\\', "/").to_ascii_lowercase();
    if normalized.contains("tools/toolstrigger") {
        roles.push(BrushRole::Trigger);
    }
    if normalized.contains("tools/toolsareaportal") {
        roles.push(BrushRole::Areaportal);
    }
    if normalized.contains("tools/toolsskybox") || normalized.contains("tools/toolsskybox2d") {
        roles.push(BrushRole::Skybox);
    }
    if normalized.contains("tools/toolsclip")
        || normalized.contains("tools/toolsplayerclip")
        || normalized.contains("tools/toolsnpcclip")
        || normalized.contains("tools/toolsblock")
    {
        roles.push(BrushRole::Clip);
    }
    if normalized.contains("tools/toolsoccluder") {
        roles.push(BrushRole::Occluder);
    }
    if normalized.contains("tools/toolshint") {
        roles.push(BrushRole::Hint);
    }
    if normalized.contains("tools/toolsskip") {
        roles.push(BrushRole::Skip);
    }
    if normalized.contains("tools/toolsnodraw") {
        roles.push(BrushRole::Nodraw);
    }
    if normalized.contains("water") {
        roles.push(BrushRole::Water);
    }
    if normalized.starts_with("tools/") {
        roles.push(BrushRole::Tool(normalized));
    }
}

fn collect_solid_roles(
    body: &[Node],
    classname: Option<&str>,
    parent_is_world: bool,
    roles: &mut Vec<BrushRole>,
) {
    for node in body {
        if let Node::Block {
            name,
            body: child_body,
        } = node
        {
            if name == "solid" {
                roles.extend(classify_solid_roles(node, classname, parent_is_world));
            } else {
                collect_solid_roles(child_body, classname, parent_is_world, roles);
            }
        }
    }
}

fn collect_materials(node: &Node, materials: &mut Vec<String>) {
    match node {
        Node::Property { key, value } if key == "material" => materials.push(value.clone()),
        Node::Block { body, .. } => {
            for child in body {
                collect_materials(child, materials);
            }
        }
        _ => {}
    }
}

fn count_blocks(body: &[Node], name: &str) -> usize {
    let mut count = 0;
    for node in body {
        if let Node::Block {
            name: child_name,
            body,
        } = node
        {
            if child_name == name {
                count += 1;
            }
            count += count_blocks(body, name);
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmf::parse_document;

    #[test]
    fn detects_entity_types_and_skybox() {
        let doc = parse_document(
            r#"
world { "id" "1" solid { side { "material" "TOOLS/TOOLSSKYBOX" } } }
entity { "id" "2" "classname" "trigger_once" solid { side { "material" "TOOLS/TOOLSTRIGGER" } } }
"#,
        )
        .unwrap();
        let records = inspect_entities(&doc);
        assert_eq!(records.len(), 2);
        assert!(records[0].roles.contains(&BrushRole::Skybox));
        assert!(records[1].roles.contains(&BrushRole::Trigger));
    }
}
