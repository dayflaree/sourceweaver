use crate::classify::{BrushRole, classify_solid_roles};
use crate::transform::Vec3;
use crate::vmf::{Document, Node};

#[derive(Debug, Clone, PartialEq)]
pub struct PreviewDocument {
    pub solids: Vec<PreviewSolid>,
    pub entities: Vec<PreviewEntityMarker>,
    pub landmarks: Vec<PreviewLandmarkMarker>,
    pub bounds: Option<PreviewBounds>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreviewSolid {
    pub owner_index: usize,
    pub owner_block: String,
    pub classname: Option<String>,
    pub targetname: Option<String>,
    pub source_index: Option<usize>,
    pub source_label: Option<String>,
    pub roles: Vec<BrushRole>,
    pub points: Vec<Vec3>,
    pub bounds: PreviewBounds,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreviewEntityMarker {
    pub owner_index: usize,
    pub classname: Option<String>,
    pub targetname: Option<String>,
    pub source_index: Option<usize>,
    pub source_label: Option<String>,
    pub origin: Vec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreviewLandmarkMarker {
    pub owner_index: usize,
    pub targetname: String,
    pub origin: Vec3,
    pub source_index: Option<usize>,
    pub source_label: Option<String>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PreviewBounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl PreviewBounds {
    pub fn from_point(point: Vec3) -> Self {
        Self {
            min: point,
            max: point,
        }
    }

    pub fn include(&mut self, point: Vec3) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.min.z = self.min.z.min(point.z);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
        self.max.z = self.max.z.max(point.z);
    }

    pub fn union(&mut self, other: PreviewBounds) {
        self.include(other.min);
        self.include(other.max);
    }

    pub fn center(&self) -> Vec3 {
        Vec3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }

    pub fn extent_x(&self) -> f64 {
        (self.max.x - self.min.x).abs()
    }

    pub fn extent_y(&self) -> f64 {
        (self.max.y - self.min.y).abs()
    }

    pub fn extent_z(&self) -> f64 {
        (self.max.z - self.min.z).abs()
    }
}

pub fn preview_document(document: &Document) -> PreviewDocument {
    preview_document_with_source(document, None, None)
}

pub fn preview_document_with_source(
    document: &Document,
    source_index: Option<usize>,
    source_label: Option<&str>,
) -> PreviewDocument {
    let mut solids = Vec::new();
    let mut entities = Vec::new();
    let mut landmarks = Vec::new();
    let mut bounds = BoundsBuilder::default();
    let mut owner_index = 0;

    for node in &document.nodes {
        let Node::Block { name, body } = node else {
            continue;
        };
        if name != "world" && name != "entity" {
            continue;
        }

        let classname = Node::get_property(body, "classname").map(ToOwned::to_owned);
        let targetname = Node::get_property(body, "targetname").map(ToOwned::to_owned);

        if name == "entity" {
            if let Some(origin) = Node::get_property(body, "origin").and_then(Vec3::parse) {
                bounds.include(origin);
                if classname.as_deref() == Some("info_landmark") {
                    if let Some(targetname) = Node::get_property(body, "targetname") {
                        let targetname = targetname.trim();
                        if !targetname.is_empty() {
                            landmarks.push(PreviewLandmarkMarker {
                                owner_index,
                                targetname: targetname.to_string(),
                                origin,
                                source_index,
                                source_label: source_label.map(ToOwned::to_owned),
                            });
                        }
                    }
                }
                entities.push(PreviewEntityMarker {
                    owner_index,
                    classname: classname.clone(),
                    targetname: targetname.clone(),
                    source_index,
                    source_label: source_label.map(ToOwned::to_owned),
                    origin,
                });
            }
        }

        collect_solids(
            body,
            OwnerContext {
                owner_index,
                owner_block: name,
                classname: classname.as_deref(),
                targetname: targetname.as_deref(),
                source_index,
                source_label,
            },
            &mut solids,
            &mut bounds,
        );
        owner_index += 1;
    }

    PreviewDocument {
        solids,
        entities,
        landmarks,
        bounds: bounds.finish(),
    }
}

pub fn translate_preview_document(preview: &mut PreviewDocument, offset: Vec3) {
    if offset == Vec3::ZERO {
        return;
    }

    let mut bounds = BoundsBuilder::default();
    for solid in &mut preview.solids {
        for point in &mut solid.points {
            *point = *point + offset;
        }
        solid.bounds.min = solid.bounds.min + offset;
        solid.bounds.max = solid.bounds.max + offset;
        bounds.include_bounds(solid.bounds);
    }
    for entity in &mut preview.entities {
        entity.origin = entity.origin + offset;
        bounds.include(entity.origin);
    }
    for landmark in &mut preview.landmarks {
        landmark.origin = landmark.origin + offset;
        bounds.include(landmark.origin);
    }
    preview.bounds = bounds.finish();
}

pub fn combine_preview_documents(previews: Vec<PreviewDocument>) -> PreviewDocument {
    let mut combined = PreviewDocument {
        solids: Vec::new(),
        entities: Vec::new(),
        landmarks: Vec::new(),
        bounds: None,
    };
    let mut bounds = BoundsBuilder::default();

    for preview in previews {
        for solid in preview.solids {
            bounds.include_bounds(solid.bounds);
            combined.solids.push(solid);
        }
        for entity in preview.entities {
            bounds.include(entity.origin);
            combined.entities.push(entity);
        }
        for landmark in preview.landmarks {
            bounds.include(landmark.origin);
            combined.landmarks.push(landmark);
        }
    }

    combined.bounds = bounds.finish();
    combined
}

#[derive(Debug, Copy, Clone)]
struct OwnerContext<'a> {
    owner_index: usize,
    owner_block: &'a str,
    classname: Option<&'a str>,
    targetname: Option<&'a str>,
    source_index: Option<usize>,
    source_label: Option<&'a str>,
}

fn collect_solids(
    body: &[Node],
    owner: OwnerContext<'_>,
    solids: &mut Vec<PreviewSolid>,
    document_bounds: &mut BoundsBuilder,
) {
    for node in body {
        let Node::Block {
            name,
            body: child_body,
        } = node
        else {
            continue;
        };

        if name == "solid" {
            let points = collect_plane_points(node);
            let mut solid_bounds = BoundsBuilder::default();
            for point in &points {
                solid_bounds.include(*point);
            }
            if let Some(bounds) = solid_bounds.finish() {
                document_bounds.include_bounds(bounds);
                let mut roles =
                    classify_solid_roles(node, owner.classname, owner.owner_block == "world");
                roles.sort();
                roles.dedup();
                solids.push(PreviewSolid {
                    owner_index: owner.owner_index,
                    owner_block: owner.owner_block.to_string(),
                    classname: owner.classname.map(ToOwned::to_owned),
                    targetname: owner.targetname.map(ToOwned::to_owned),
                    source_index: owner.source_index,
                    source_label: owner.source_label.map(ToOwned::to_owned),
                    roles,
                    points,
                    bounds,
                });
            }
        } else {
            collect_solids(child_body, owner, solids, document_bounds);
        }
    }
}

fn collect_plane_points(node: &Node) -> Vec<Vec3> {
    let mut points = Vec::new();
    collect_plane_points_from_node(node, &mut points);
    points
}

fn collect_plane_points_from_node(node: &Node, points: &mut Vec<Vec3>) {
    match node {
        Node::Property { key, value } if key == "plane" => {
            points.extend(parse_parenthesized_points(value));
        }
        Node::Block { body, .. } => {
            for child in body {
                collect_plane_points_from_node(child, points);
            }
        }
        _ => {}
    }
}

fn parse_parenthesized_points(value: &str) -> Vec<Vec3> {
    let mut points = Vec::new();
    let mut rest = value;
    while let Some(start) = rest.find('(') {
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find(')') else {
            break;
        };
        if let Some(point) = Vec3::parse(&after_start[..end]) {
            points.push(point);
        }
        rest = &after_start[end + 1..];
    }
    points
}

#[derive(Debug, Default)]
struct BoundsBuilder {
    bounds: Option<PreviewBounds>,
}

impl BoundsBuilder {
    fn include(&mut self, point: Vec3) {
        match &mut self.bounds {
            Some(bounds) => bounds.include(point),
            None => self.bounds = Some(PreviewBounds::from_point(point)),
        }
    }

    fn include_bounds(&mut self, bounds: PreviewBounds) {
        match &mut self.bounds {
            Some(existing) => existing.union(bounds),
            None => self.bounds = Some(bounds),
        }
    }

    fn finish(self) -> Option<PreviewBounds> {
        self.bounds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmf::parse_document;

    #[test]
    fn extracts_solids_entities_roles_and_bounds() {
        let document = parse_document(r#"
world { "id" "1" solid { side { "plane" "(0 0 0) (64 0 0) (64 64 0)" "material" "TOOLS/TOOLSSKYBOX" } } }
entity { "id" "2" "classname" "trigger_once" "targetname" "tr" "origin" "128 0 0" solid { side { "plane" "(100 0 0) (140 0 0) (140 20 0)" "material" "TOOLS/TOOLSTRIGGER" } } }
"#).unwrap();
        let preview = preview_document(&document);
        assert_eq!(preview.solids.len(), 2);
        assert_eq!(preview.entities.len(), 1);
        assert!(preview.solids[0].roles.contains(&BrushRole::Skybox));
        assert!(preview.solids[1].roles.contains(&BrushRole::Trigger));
        let bounds = preview.bounds.unwrap();
        assert_eq!(bounds.min.x, 0.0);
        assert_eq!(bounds.max.x, 140.0);
    }

    #[test]
    fn carries_source_metadata_and_combines_translated_previews() {
        let document = parse_document(
            r#"
world { "id" "1" solid { side { "plane" "(0 0 0) (32 0 0) (32 32 0)" "material" "BRICK/WALL001" } } }
entity { "id" "2" "classname" "prop_static" "origin" "64 0 0" }
entity { "id" "3" "classname" "info_landmark" "targetname" "map_transition" "origin" "80 0 0" }
"#,
        )
        .unwrap();

        let mut preview = preview_document_with_source(&document, Some(3), Some("incoming.vmf"));
        translate_preview_document(&mut preview, Vec3::new(100.0, 0.0, 0.0));

        assert_eq!(preview.solids[0].source_index, Some(3));
        assert_eq!(
            preview.solids[0].source_label.as_deref(),
            Some("incoming.vmf")
        );
        assert_eq!(preview.entities[0].source_index, Some(3));
        assert_eq!(preview.entities[0].origin, Vec3::new(164.0, 0.0, 0.0));
        assert_eq!(preview.landmarks.len(), 1);
        assert_eq!(preview.landmarks[0].targetname, "map_transition");
        assert_eq!(preview.landmarks[0].origin, Vec3::new(180.0, 0.0, 0.0));
        assert_eq!(preview.landmarks[0].source_index, Some(3));
        assert_eq!(preview.bounds.unwrap().max.x, 180.0);

        let combined = combine_preview_documents(vec![preview.clone(), preview]);
        assert_eq!(combined.solids.len(), 2);
        assert_eq!(combined.entities.len(), 4);
        assert_eq!(combined.landmarks.len(), 2);
        assert_eq!(combined.bounds.unwrap().max.x, 180.0);
    }
}
