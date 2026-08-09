//! Cleanup/deletion preset helpers.

use super::*;

pub(crate) fn deletion_presets() -> [DeletionPresetSpec; 6] {
    [
        DeletionPresetSpec {
            kind: DeletionPresetKind::RemoveTriggers,
            name: "Remove triggers",
            description: "Remove trigger brush content while leaving critical transition entities protected.",
        },
        DeletionPresetSpec {
            kind: DeletionPresetKind::RemoveClips,
            name: "Remove clips",
            description: "Remove clip/playerclip-style brush content from world and brush entities.",
        },
        DeletionPresetSpec {
            kind: DeletionPresetKind::RemoveAreaportals,
            name: "Remove areaportals",
            description: "Remove areaportal brush content that often needs rebuilding after stitching.",
        },
        DeletionPresetSpec {
            kind: DeletionPresetKind::RemoveGameplayLogic,
            name: "Remove gameplay logic",
            description: "Target common trigger and logic classnames; protected critical entities remain until protection is disabled.",
        },
        DeletionPresetSpec {
            kind: DeletionPresetKind::KeepWorldGeometry,
            name: "Keep only world geometry",
            description: "Remove non-protected entities and utility/tool world brushes, including skybox brushes.",
        },
        DeletionPresetSpec {
            kind: DeletionPresetKind::KeepWorldAndSkybox,
            name: "Keep world plus skybox",
            description: "Remove non-protected entities and utility/tool world brushes while preserving skybox brushes.",
        },
    ]
}

pub(crate) fn deletion_preset_criteria(kind: DeletionPresetKind) -> DeletionCriteria {
    let mut criteria = DeletionCriteria {
        protect_critical_entities: true,
        ..DeletionCriteria::default()
    };

    match kind {
        DeletionPresetKind::RemoveTriggers => {
            criteria.brush_roles.insert(BrushRole::Trigger);
            criteria.brush_entity_mode = BrushEntityDeletionMode::MatchingSolids;
        }
        DeletionPresetKind::RemoveClips => {
            criteria.brush_roles.insert(BrushRole::Clip);
            criteria.brush_entity_mode = BrushEntityDeletionMode::MatchingSolids;
        }
        DeletionPresetKind::RemoveAreaportals => {
            criteria.brush_roles.insert(BrushRole::Areaportal);
            criteria.brush_entity_mode = BrushEntityDeletionMode::MatchingSolids;
        }
        DeletionPresetKind::RemoveGameplayLogic => {
            criteria.classnames.extend([
                "trigger_once".to_string(),
                "trigger_multiple".to_string(),
                "logic_auto".to_string(),
                "logic_relay".to_string(),
                "logic_timer".to_string(),
                "math_counter".to_string(),
                "point_template".to_string(),
                "env_global".to_string(),
                "game_text".to_string(),
            ]);
            criteria.brush_roles.insert(BrushRole::Trigger);
            criteria.brush_entity_mode = BrushEntityDeletionMode::WholeEntity;
        }
        DeletionPresetKind::KeepWorldGeometry => {
            criteria.drop_all_entities = true;
            criteria.brush_roles.extend([
                BrushRole::Trigger,
                BrushRole::Clip,
                BrushRole::Areaportal,
                BrushRole::Skybox,
                BrushRole::Occluder,
                BrushRole::Hint,
                BrushRole::Skip,
                BrushRole::Nodraw,
                BrushRole::Water,
                BrushRole::BrushEntity,
            ]);
            criteria.brush_entity_mode = BrushEntityDeletionMode::MatchingSolids;
        }
        DeletionPresetKind::KeepWorldAndSkybox => {
            criteria.drop_all_entities = true;
            criteria.brush_roles.extend([
                BrushRole::Trigger,
                BrushRole::Clip,
                BrushRole::Areaportal,
                BrushRole::Occluder,
                BrushRole::Hint,
                BrushRole::Skip,
                BrushRole::Nodraw,
                BrushRole::Water,
                BrushRole::BrushEntity,
            ]);
            criteria.brush_entity_mode = BrushEntityDeletionMode::MatchingSolids;
        }
    }

    criteria
}

pub(crate) fn describe_deletion_criteria(criteria: &DeletionCriteria) -> String {
    let mut parts = Vec::new();
    if criteria.drop_all_entities {
        parts.push("all non-protected entities".to_string());
    }
    if !criteria.classnames.is_empty() {
        parts.push(format!(
            "classnames [{}]",
            criteria
                .classnames
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !criteria.targetnames.is_empty() {
        parts.push(format!(
            "targetnames [{}]",
            criteria
                .targetnames
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !criteria.brush_roles.is_empty() {
        parts.push(format!(
            "roles [{}]",
            criteria
                .brush_roles
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    parts.push(format!("brush entities: {}", criteria.brush_entity_mode));
    parts.push(format!(
        "critical protection: {}",
        if criteria.protect_critical_entities {
            "on"
        } else {
            "off"
        }
    ));
    parts.join("; ")
}
