use crate::transform::Vec3;
use crate::vmf::{Document, Node};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredLandmark {
    pub targetname: String,
    pub origin: Option<Vec3>,
    pub entity_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LandmarkDuplicate {
    pub targetname: String,
    pub count: usize,
    pub valid_origins: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LandmarkDiscovery {
    pub entries: Vec<DiscoveredLandmark>,
    pub targetnames: Vec<String>,
    pub duplicates: Vec<LandmarkDuplicate>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LandmarkTargetStatus {
    Blank,
    Missing,
    Present { origin: Vec3 },
    InvalidOrigin { count: usize },
    Duplicate { count: usize, valid_origins: usize },
}

impl LandmarkDiscovery {
    pub fn status_for(&self, targetname: &str) -> LandmarkTargetStatus {
        let targetname = targetname.trim();
        if targetname.is_empty() {
            return LandmarkTargetStatus::Blank;
        }

        let mut count = 0;
        let mut first_origin = None;
        let mut valid_origins = 0;
        for landmark in self
            .entries
            .iter()
            .filter(|landmark| landmark.targetname == targetname)
        {
            count += 1;
            if let Some(origin) = landmark.origin {
                valid_origins += 1;
                first_origin.get_or_insert(origin);
            }
        }

        match (count, first_origin) {
            (0, _) => LandmarkTargetStatus::Missing,
            (1, Some(origin)) => LandmarkTargetStatus::Present { origin },
            (1, None) => LandmarkTargetStatus::InvalidOrigin { count },
            _ => LandmarkTargetStatus::Duplicate {
                count,
                valid_origins,
            },
        }
    }
}

pub fn discover_landmarks(document: &Document) -> LandmarkDiscovery {
    let mut entries = Vec::new();
    let mut targetnames = BTreeSet::new();
    let mut per_target: BTreeMap<String, (usize, usize)> = BTreeMap::new();
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

        if Node::get_property(body, "classname") != Some("info_landmark") {
            continue;
        }

        let Some(targetname) = Node::get_property(body, "targetname").map(str::trim) else {
            continue;
        };
        if targetname.is_empty() {
            continue;
        }

        let origin = Node::get_property(body, "origin").and_then(Vec3::parse);
        let targetname = targetname.to_string();
        targetnames.insert(targetname.clone());
        let counts = per_target.entry(targetname.clone()).or_insert((0, 0));
        counts.0 += 1;
        if origin.is_some() {
            counts.1 += 1;
        }
        entries.push(DiscoveredLandmark {
            targetname,
            origin,
            entity_index: current_entity_index,
        });
    }

    let duplicates = per_target
        .into_iter()
        .filter_map(|(targetname, (count, valid_origins))| {
            (count > 1).then_some(LandmarkDuplicate {
                targetname,
                count,
                valid_origins,
            })
        })
        .collect();

    LandmarkDiscovery {
        entries,
        targetnames: targetnames.into_iter().collect(),
        duplicates,
    }
}

pub fn landmark_status(document: &Document, targetname: &str) -> LandmarkTargetStatus {
    discover_landmarks(document).status_for(targetname)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmf::parse_document;

    #[test]
    fn discovers_info_landmark_targetnames_and_origins() {
        let document = parse_document(
            r#"
world { "id" "1" }
entity { "classname" "info_landmark" "targetname" "map_transition" "origin" "1 2 3" }
entity { "classname" "info_landmark" "targetname" "other" "origin" "4 5 6" }
entity { "classname" "prop_static" "targetname" "ignored" "origin" "7 8 9" }
entity { "classname" "info_landmark" "targetname" "" "origin" "10 11 12" }
"#,
        )
        .unwrap();

        let discovery = discover_landmarks(&document);

        assert_eq!(discovery.targetnames, vec!["map_transition", "other"]);
        assert_eq!(discovery.entries.len(), 2);
        assert_eq!(
            discovery.status_for("map_transition"),
            LandmarkTargetStatus::Present {
                origin: Vec3::new(1.0, 2.0, 3.0),
            }
        );
        assert_eq!(
            discovery.status_for("missing"),
            LandmarkTargetStatus::Missing
        );
        assert_eq!(discovery.status_for(""), LandmarkTargetStatus::Blank);
    }

    #[test]
    fn reports_duplicate_and_invalid_landmarks() {
        let document = parse_document(
            r#"
entity { "classname" "info_landmark" "targetname" "dupe" "origin" "0 0 0" }
entity { "classname" "info_landmark" "targetname" "dupe" "origin" "128 0 0" }
entity { "classname" "info_landmark" "targetname" "bad" "origin" "not a vector" }
"#,
        )
        .unwrap();

        let discovery = discover_landmarks(&document);

        assert_eq!(
            discovery.duplicates,
            vec![LandmarkDuplicate {
                targetname: "dupe".to_string(),
                count: 2,
                valid_origins: 2,
            }]
        );
        assert_eq!(
            discovery.status_for("dupe"),
            LandmarkTargetStatus::Duplicate {
                count: 2,
                valid_origins: 2,
            }
        );
        assert_eq!(
            discovery.status_for("bad"),
            LandmarkTargetStatus::InvalidOrigin { count: 1 }
        );
    }
}
