use crate::transform::Vec3;
use crate::vmf::{Document, Node};

#[derive(Debug, Clone, PartialEq)]
pub struct CampaignTransition {
    pub entity_index: usize,
    pub targetname: Option<String>,
    pub target_map: Option<String>,
    pub landmark: Option<String>,
    pub origin: Option<Vec3>,
    pub solid_count: usize,
}

pub fn discover_transitions(document: &Document) -> Vec<CampaignTransition> {
    let mut transitions = Vec::new();
    let mut entity_index = 0;

    for node in &document.nodes {
        let Node::Block { name, body } = node else {
            continue;
        };
        if name != "entity" {
            continue;
        }

        let current_entity_index = entity_index;
        entity_index += 1;

        if Node::get_property(body, "classname") != Some("trigger_changelevel") {
            continue;
        }

        transitions.push(CampaignTransition {
            entity_index: current_entity_index,
            targetname: non_empty_property(body, "targetname"),
            target_map: non_empty_property(body, "map"),
            landmark: non_empty_property(body, "landmark")
                .or_else(|| non_empty_property(body, "landmarkname")),
            origin: Node::get_property(body, "origin").and_then(Vec3::parse),
            solid_count: count_blocks(body, "solid"),
        });
    }

    transitions
}

fn non_empty_property(body: &[Node], key: &str) -> Option<String> {
    Node::get_property(body, key)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn count_blocks(body: &[Node], block_name: &str) -> usize {
    let mut count = 0;
    for node in body {
        let Node::Block { name, body } = node else {
            continue;
        };
        if name == block_name {
            count += 1;
        }
        count += count_blocks(body, block_name);
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmf::parse_document;

    #[test]
    fn detects_trigger_changelevel_properties() {
        let document = parse_document(
            r#"
world { "id" "1" }
entity {
  "id" "2"
  "classname" "trigger_changelevel"
  "targetname" "to_next"
  "map" "d1_trainstation_02"
  "landmark" "map_transition"
  "origin" "128 64 0"
  solid { "id" "3" side { "id" "4" "material" "TOOLS/TOOLSTRIGGER" } }
}
entity { "id" "5" "classname" "info_landmark" "targetname" "map_transition" }
"#,
        )
        .unwrap();

        let transitions = discover_transitions(&document);

        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].entity_index, 0);
        assert_eq!(transitions[0].targetname.as_deref(), Some("to_next"));
        assert_eq!(
            transitions[0].target_map.as_deref(),
            Some("d1_trainstation_02")
        );
        assert_eq!(transitions[0].landmark.as_deref(), Some("map_transition"));
        assert_eq!(transitions[0].origin, Some(Vec3::new(128.0, 64.0, 0.0)));
        assert_eq!(transitions[0].solid_count, 1);
    }

    #[test]
    fn accepts_landmarkname_alias() {
        let document = parse_document(
            r#"
world { "id" "1" }
entity {
  "id" "2"
  "classname" "trigger_changelevel"
  "map" "bm_c2a1b"
  "landmarkname" "lm_exit"
}
"#,
        )
        .unwrap();

        let transitions = discover_transitions(&document);

        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].landmark.as_deref(), Some("lm_exit"));
    }
}
