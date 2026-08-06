use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityCategory {
    World,
    Point,
    Brush,
    Logic,
    Prop,
    Trigger,
    Npc,
    Spawn,
    Landmark,
    Transition,
    Overlay,
    Sound,
    Light,
    Other,
}

impl fmt::Display for EntityCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::World => f.write_str("world"),
            Self::Point => f.write_str("point"),
            Self::Brush => f.write_str("brush"),
            Self::Logic => f.write_str("logic"),
            Self::Prop => f.write_str("prop"),
            Self::Trigger => f.write_str("trigger"),
            Self::Npc => f.write_str("npc"),
            Self::Spawn => f.write_str("spawn"),
            Self::Landmark => f.write_str("landmark"),
            Self::Transition => f.write_str("transition"),
            Self::Overlay => f.write_str("overlay"),
            Self::Sound => f.write_str("sound"),
            Self::Light => f.write_str("light"),
            Self::Other => f.write_str("other"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityMetadata {
    pub classname: String,
    pub display_name: String,
    pub category: EntityCategory,
    pub description: Option<String>,
    pub source: EntityMetadataSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityMetadataSource {
    BuiltIn,
    Fgd(String),
    Inferred,
    Unknown,
}

impl fmt::Display for EntityMetadataSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BuiltIn => f.write_str("built-in"),
            Self::Fgd(path) => write!(f, "FGD: {path}"),
            Self::Inferred => f.write_str("inferred"),
            Self::Unknown => f.write_str("unknown"),
        }
    }
}

pub fn metadata_for_classname(classname: &str) -> EntityMetadata {
    builtin_metadata(classname).unwrap_or_else(|| inferred_metadata(classname))
}

pub fn metadata_for_classname_with_overrides(
    classname: &str,
    overrides: &BTreeMap<String, EntityMetadata>,
) -> EntityMetadata {
    overrides
        .get(classname)
        .cloned()
        .unwrap_or_else(|| metadata_for_classname(classname))
}

pub fn parse_fgd_metadata(text: &str, source_label: &str) -> Vec<EntityMetadata> {
    let mut entries = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('@') || !trimmed.contains('=') {
            continue;
        }
        let Some((annotation, rest)) = trimmed.split_once('=') else {
            continue;
        };
        let rest = rest.trim();
        let classname = rest
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches(':')
            .trim();
        if classname.is_empty() || !is_valid_classname(classname) {
            continue;
        }
        let description = rest
            .split_once(':')
            .and_then(|(_, description)| quoted_description(description));
        entries.push(EntityMetadata {
            classname: classname.to_string(),
            display_name: display_name_for(classname),
            category: category_from_fgd_annotation(annotation)
                .unwrap_or_else(|| infer_category(classname)),
            description,
            source: EntityMetadataSource::Fgd(source_label.to_string()),
        });
    }
    entries
}

fn builtin_metadata(classname: &str) -> Option<EntityMetadata> {
    let (category, description) = match classname {
        "worldspawn" => (EntityCategory::World, "Worldspawn/map root settings"),
        "info_landmark" => (
            EntityCategory::Landmark,
            "Landmark used to align map transitions",
        ),
        "trigger_changelevel" => (
            EntityCategory::Transition,
            "Brush trigger that transitions to another map",
        ),
        "info_player_start" => (EntityCategory::Spawn, "Single-player spawn point"),
        "info_player_deathmatch" => (EntityCategory::Spawn, "Deathmatch spawn point"),
        "prop_static" => (
            EntityCategory::Prop,
            "Static model prop compiled into the map",
        ),
        "prop_dynamic" => (
            EntityCategory::Prop,
            "Runtime animated or dynamic model prop",
        ),
        "prop_physics" => (EntityCategory::Prop, "Physics-simulated model prop"),
        "func_detail" => (EntityCategory::Brush, "Non-sealing brush detail geometry"),
        "func_brush" => (EntityCategory::Brush, "Generic brush entity"),
        "func_areaportal" => (EntityCategory::Brush, "Visibility portal brush"),
        "func_areaportalwindow" => (
            EntityCategory::Brush,
            "Distance-controlled visibility portal",
        ),
        "func_occluder" => (EntityCategory::Brush, "Visibility occluder brush"),
        "trigger_once" => (EntityCategory::Trigger, "Trigger that fires once"),
        "trigger_multiple" => (EntityCategory::Trigger, "Trigger that can fire repeatedly"),
        "trigger_teleport" => (EntityCategory::Trigger, "Trigger that teleports entities"),
        "logic_auto" => (
            EntityCategory::Logic,
            "Logic entity that fires outputs on map spawn",
        ),
        "logic_relay" => (EntityCategory::Logic, "Logic relay for grouping outputs"),
        "logic_timer" => (
            EntityCategory::Logic,
            "Logic entity that fires at intervals",
        ),
        "math_counter" => (EntityCategory::Logic, "Numeric logic counter"),
        "point_template" => (EntityCategory::Logic, "Template spawner for named entities"),
        "env_global" => (EntityCategory::Logic, "Global state entity"),
        "info_overlay" => (
            EntityCategory::Overlay,
            "Overlay decal projected onto brush sides",
        ),
        "npc_citizen" => (EntityCategory::Npc, "Citizen NPC"),
        "npc_combine_s" => (EntityCategory::Npc, "Combine soldier NPC"),
        "ambient_generic" => (EntityCategory::Sound, "Ambient or triggered sound source"),
        "env_soundscape" => (EntityCategory::Sound, "Soundscape controller"),
        "light" => (EntityCategory::Light, "Point light"),
        "light_spot" => (EntityCategory::Light, "Spot light"),
        "light_environment" => (EntityCategory::Light, "Environment/sun light"),
        _ => return None,
    };
    Some(EntityMetadata {
        classname: classname.to_string(),
        display_name: display_name_for(classname),
        category,
        description: Some(description.to_string()),
        source: EntityMetadataSource::BuiltIn,
    })
}

fn inferred_metadata(classname: &str) -> EntityMetadata {
    let category = infer_category(classname);
    EntityMetadata {
        classname: classname.to_string(),
        display_name: display_name_for(classname),
        category,
        description: if category == EntityCategory::Other {
            None
        } else {
            Some(format!("Inferred {category} entity from classname pattern"))
        },
        source: if category == EntityCategory::Other {
            EntityMetadataSource::Unknown
        } else {
            EntityMetadataSource::Inferred
        },
    }
}

fn infer_category(classname: &str) -> EntityCategory {
    let class = classname.to_ascii_lowercase();
    if class == "world" || class == "worldspawn" {
        EntityCategory::World
    } else if class == "info_landmark" {
        EntityCategory::Landmark
    } else if class == "trigger_changelevel" {
        EntityCategory::Transition
    } else if class.starts_with("trigger_") {
        EntityCategory::Trigger
    } else if class.starts_with("logic_")
        || class.starts_with("math_")
        || class.starts_with("point_")
        || class.starts_with("env_global")
    {
        EntityCategory::Logic
    } else if class.starts_with("prop_") {
        EntityCategory::Prop
    } else if class.starts_with("func_") {
        EntityCategory::Brush
    } else if class.starts_with("npc_") || class.starts_with("monster_") {
        EntityCategory::Npc
    } else if class.contains("player") && class.contains("start") {
        EntityCategory::Spawn
    } else if class.contains("overlay") || class.contains("decal") {
        EntityCategory::Overlay
    } else if class.contains("sound") || class.contains("ambient") {
        EntityCategory::Sound
    } else if class.starts_with("light") {
        EntityCategory::Light
    } else if class.starts_with("info_") {
        EntityCategory::Point
    } else {
        EntityCategory::Other
    }
}

fn category_from_fgd_annotation(annotation: &str) -> Option<EntityCategory> {
    let lower = annotation.to_ascii_lowercase();
    if lower.contains("pointclass") {
        Some(EntityCategory::Point)
    } else if lower.contains("solidclass") || lower.contains("baseclass") && lower.contains("brush")
    {
        Some(EntityCategory::Brush)
    } else if lower.contains("npcclass") {
        Some(EntityCategory::Npc)
    } else {
        None
    }
}

fn quoted_description(value: &str) -> Option<String> {
    let start = value.find('"')?;
    let rest = &value[start + 1..];
    let end = rest.find('"')?;
    let description = rest[..end].trim();
    (!description.is_empty()).then(|| description.to_string())
}

fn display_name_for(classname: &str) -> String {
    classname
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_valid_classname(value: &str) -> bool {
    value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_builtin_and_inferred_metadata() {
        let landmark = metadata_for_classname("info_landmark");
        assert_eq!(landmark.category, EntityCategory::Landmark);
        assert!(landmark.description.unwrap().contains("Landmark"));

        let unknown_trigger = metadata_for_classname("trigger_custom_script");
        assert_eq!(unknown_trigger.category, EntityCategory::Trigger);
        assert_eq!(unknown_trigger.source, EntityMetadataSource::Inferred);

        let unknown = metadata_for_classname("my_mod_weirdthing");
        assert_eq!(unknown.category, EntityCategory::Other);
        assert_eq!(unknown.source, EntityMetadataSource::Unknown);
        assert!(unknown.description.is_none());
    }

    #[test]
    fn parses_simple_fgd_class_metadata() {
        let entries = parse_fgd_metadata(
            r#"
@PointClass base(Targetname) iconsprite("editor/info_target.vmt") = info_target : "Target point"
[
]
@SolidClass base(Targetname) = func_ladder : "Climbable ladder brush"
[
]
"#,
            "test.fgd",
        );

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].classname, "info_target");
        assert_eq!(entries[0].category, EntityCategory::Point);
        assert_eq!(entries[0].description.as_deref(), Some("Target point"));
        assert_eq!(entries[1].classname, "func_ladder");
        assert_eq!(entries[1].category, EntityCategory::Brush);
    }
}
