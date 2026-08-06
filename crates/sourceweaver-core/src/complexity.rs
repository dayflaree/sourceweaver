use crate::vmf::{Document, Node};

pub const SOURCE_MAX_MAP_ENTITIES: usize = 4096;
pub const SOURCE_MAX_MAP_BRUSHES: usize = 8192;
pub const SOURCE_MAX_MAP_BRUSHSIDES: usize = 65536;
pub const SOURCE_MAX_MAP_FACES: usize = 65536;
pub const SOURCE_MAX_MAP_DISPINFO: usize = 2048;
pub const SOURCE_MAX_MAP_OVERLAYS: usize = 512;
pub const SOURCE_COMPLEXITY_WARN_RATIO: f64 = 0.75;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapComplexityReport {
    pub entity_count: usize,
    pub point_entity_count: usize,
    pub brush_entity_count: usize,
    pub brush_solid_count: usize,
    pub side_count: usize,
    pub displacement_count: usize,
    pub overlay_count: usize,
    pub risks: Vec<MapComplexityRisk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapComplexityRisk {
    pub severity: &'static str,
    pub metric: &'static str,
    pub count: usize,
    pub warn_at: usize,
    pub limit: usize,
    pub message: String,
}

impl MapComplexityReport {
    pub fn warning_count(&self) -> usize {
        self.risks.len()
    }
}

pub fn analyze_map_complexity(document: &Document) -> MapComplexityReport {
    let mut counts = ComplexityCounts::default();
    count_document(document, &mut counts);

    let mut report = MapComplexityReport {
        entity_count: counts.entity_count,
        point_entity_count: counts.point_entity_count,
        brush_entity_count: counts.brush_entity_count,
        brush_solid_count: counts.solid_count,
        side_count: counts.side_count,
        displacement_count: counts.displacement_count,
        overlay_count: counts.overlay_count,
        risks: Vec::new(),
    };

    add_threshold_risk(
        &mut report.risks,
        "entities",
        report.entity_count,
        SOURCE_MAX_MAP_ENTITIES,
    );
    add_threshold_risk(
        &mut report.risks,
        "brush solids",
        report.brush_solid_count,
        SOURCE_MAX_MAP_BRUSHES,
    );
    add_threshold_risk(
        &mut report.risks,
        "brush sides",
        report.side_count,
        SOURCE_MAX_MAP_BRUSHSIDES,
    );
    add_threshold_risk(
        &mut report.risks,
        "faces",
        report.side_count,
        SOURCE_MAX_MAP_FACES,
    );
    add_threshold_risk(
        &mut report.risks,
        "displacements",
        report.displacement_count,
        SOURCE_MAX_MAP_DISPINFO,
    );
    add_threshold_risk(
        &mut report.risks,
        "overlays",
        report.overlay_count,
        SOURCE_MAX_MAP_OVERLAYS,
    );

    report
}

fn add_threshold_risk(
    risks: &mut Vec<MapComplexityRisk>,
    metric: &'static str,
    count: usize,
    limit: usize,
) {
    let warn_at = ((limit as f64) * SOURCE_COMPLEXITY_WARN_RATIO).ceil() as usize;
    if count < warn_at {
        return;
    }

    let state = if count >= limit {
        "at or above"
    } else {
        "approaching"
    };
    risks.push(MapComplexityRisk {
        severity: "warning",
        metric,
        count,
        warn_at,
        limit,
        message: format!(
            "{metric} count {count} is {state} the heuristic Source BSP limit {limit}; this is a VMF-only estimate and does not prove compile or runtime failure"
        ),
    });
}

#[derive(Debug, Clone, Copy, Default)]
struct ComplexityCounts {
    entity_count: usize,
    point_entity_count: usize,
    brush_entity_count: usize,
    solid_count: usize,
    side_count: usize,
    displacement_count: usize,
    overlay_count: usize,
}

fn count_document(document: &Document, counts: &mut ComplexityCounts) {
    for node in &document.nodes {
        let Node::Block { name, body } = node else {
            continue;
        };
        if name == "entity" {
            counts.entity_count += 1;
            let solid_count_before = counts.solid_count;
            count_nodes(body, counts);
            if Node::get_property(body, "classname") == Some("info_overlay") {
                counts.overlay_count += 1;
            }
            if counts.solid_count > solid_count_before {
                counts.brush_entity_count += 1;
            } else {
                counts.point_entity_count += 1;
            }
        } else {
            count_nodes(body, counts);
        }
    }
}

fn count_nodes(nodes: &[Node], counts: &mut ComplexityCounts) {
    for node in nodes {
        let Node::Block { name, body } = node else {
            continue;
        };
        match name.as_str() {
            "solid" => counts.solid_count += 1,
            "side" => counts.side_count += 1,
            "dispinfo" => counts.displacement_count += 1,
            _ => {}
        }
        count_nodes(body, counts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmf::parse_document;

    #[test]
    fn counts_entities_solids_sides_displacements_and_overlays() {
        let document = parse_document(
            r#"
world { "id" "1" solid { "id" "2" side { "id" "3" dispinfo { "power" "2" } } } }
entity { "id" "4" "classname" "func_detail" solid { "id" "5" side { "id" "6" } } }
entity { "id" "7" "classname" "info_overlay" "sides" "3 6" }
entity { "id" "8" "classname" "logic_relay" }
"#,
        )
        .unwrap();

        let report = analyze_map_complexity(&document);

        assert_eq!(report.entity_count, 3);
        assert_eq!(report.point_entity_count, 2);
        assert_eq!(report.brush_entity_count, 1);
        assert_eq!(report.brush_solid_count, 2);
        assert_eq!(report.side_count, 2);
        assert_eq!(report.displacement_count, 1);
        assert_eq!(report.overlay_count, 1);
        assert!(report.risks.is_empty());
    }

    #[test]
    fn warns_when_counts_approach_limits() {
        let mut document = parse_document("world { \"id\" \"1\" }").unwrap();
        for index in 0..SOURCE_MAX_MAP_OVERLAYS * 3 / 4 {
            document.nodes.push(Node::Block {
                name: "entity".to_string(),
                body: vec![
                    Node::Property {
                        key: "id".to_string(),
                        value: (index + 2).to_string(),
                    },
                    Node::Property {
                        key: "classname".to_string(),
                        value: "info_overlay".to_string(),
                    },
                ],
            });
        }

        let report = analyze_map_complexity(&document);

        assert!(report.risks.iter().any(|risk| risk.metric == "overlays"));
    }
}
