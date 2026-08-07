use serde::Serialize;

#[derive(Debug, Clone, Copy)]
pub struct BspSourceArgumentPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub args: &'static [&'static str],
    pub tradeoff: &'static str,
    pub research_note: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct BspSourceArgumentPresetSnapshot {
    pub id: String,
    pub label: String,
    pub args: Vec<String>,
    pub tradeoff: String,
    pub research_note: String,
}

pub const BSPSOURCE_ARGUMENT_PRESETS: &[BspSourceArgumentPreset] = &[
    BspSourceArgumentPreset {
        id: "default",
        label: "BSPSource defaults",
        args: &[],
        tradeoff: "Uses the upstream default decompile behavior for the installed BSPSource version.",
        research_note: "Upstream README documents normal CLI use as bspsrc -o <out.vmf> <input.bsp>; no extra flags are required for the default mode.",
    },
    BspSourceArgumentPreset {
        id: "extract-embedded",
        label: "Extract embedded assets",
        args: &["-unpack_embedded"],
        tradeoff: "Extracts BSP-embedded materials/models alongside decompilation so users can review dependencies. This can write many files and still requires users to manage game/content paths manually.",
        research_note: "BSPSource 1.4.0 release notes list CLI option -unpack_embedded; upstream README notes embedded materials/models often explain gray textures or error models.",
    },
    BspSourceArgumentPreset {
        id: "extract-embedded-all",
        label: "Extract embedded assets without smart filtering",
        args: &["-unpack_embedded", "-no_smart_unpack"],
        tradeoff: "Extracts embedded assets and disables BSPSource smart filtering. This may preserve more files for audit, but can include cubemap/generated/noisy content.",
        research_note: "BSPSource 1.4.0 release notes list -unpack_embedded and -no_smart_unpack. BSPSource 1.4.4 release notes mention smart extracting and cubemap-related files, so this preset is audit-oriented rather than minimal.",
    },
    BspSourceArgumentPreset {
        id: "manual-areaportal",
        label: "Force manual areaportal mapping",
        args: &["-force_manual_areaportal"],
        tradeoff: "May help areaportal reconstruction review for difficult maps, but can produce less automatic mapping and should be manually inspected.",
        research_note: "BSPSource 1.4.0 release notes list -force_manual_areaportal for areaportal entity mapping.",
    },
    BspSourceArgumentPreset {
        id: "disable-tool-texture-fix",
        label: "Disable tool texture fixing",
        args: &["--no_ttfix"],
        tradeoff: "Leaves BSPSource tool texture fixes disabled for users who prefer raw output or suspect the fixup is harmful for a map/game branch.",
        research_note: "BSPSource 1.4.7 release notes mention a CLI toggle for tooltexture fixing, and v1.4.8 release notes reference --no_ttfix.",
    },
    BspSourceArgumentPreset {
        id: "disable-cubemap-texture-fix",
        label: "Disable cubemap texture fixing",
        args: &["--no_cubemaptexfix"],
        tradeoff: "Leaves BSPSource cubemap texture fixup disabled for users auditing raw material references or diagnosing cubemap-related issues.",
        research_note: "BSPSource 1.4.7 release notes mention a CLI toggle for cubemap fixing, and v1.4.8 release notes reference --no_cubemaptexfix.",
    },
    BspSourceArgumentPreset {
        id: "audit-raw-output",
        label: "Audit raw-ish output",
        args: &["--no_ttfix", "--no_cubemaptexfix"],
        tradeoff: "Disables tool/cubemap texture fixups together. Useful for comparing BSPSource output to a default decompile, but may leave more broken or noisy texture references.",
        research_note: "Composes v1.4.8-documented --no_ttfix and --no_cubemaptexfix toggles; raw --tool-arg remains available for version-specific flags.",
    },
];

pub fn preset_by_id(id: &str) -> Option<&'static BspSourceArgumentPreset> {
    let normalized = normalize_preset_id(id);
    BSPSOURCE_ARGUMENT_PRESETS
        .iter()
        .find(|preset| normalize_preset_id(preset.id) == normalized)
}

pub fn preset_choices() -> String {
    BSPSOURCE_ARGUMENT_PRESETS
        .iter()
        .map(|preset| preset.id)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn preset_snapshot(preset: &BspSourceArgumentPreset) -> BspSourceArgumentPresetSnapshot {
    BspSourceArgumentPresetSnapshot {
        id: preset.id.to_string(),
        label: preset.label.to_string(),
        args: preset.args.iter().map(|arg| (*arg).to_string()).collect(),
        tradeoff: preset.tradeoff.to_string(),
        research_note: preset.research_note.to_string(),
    }
}

pub fn preset_args(ids: &[String]) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    for id in ids {
        let preset = preset_by_id(id).ok_or_else(|| {
            format!(
                "unknown BSPSource preset `{id}`. choices: {}",
                preset_choices()
            )
        })?;
        args.extend(preset.args.iter().map(|arg| (*arg).to_string()));
    }
    Ok(args)
}

pub fn preset_snapshots(ids: &[String]) -> Result<Vec<BspSourceArgumentPresetSnapshot>, String> {
    ids.iter()
        .map(|id| {
            preset_by_id(id).map(preset_snapshot).ok_or_else(|| {
                format!(
                    "unknown BSPSource preset `{id}`. choices: {}",
                    preset_choices()
                )
            })
        })
        .collect()
}

fn normalize_preset_id(id: &str) -> String {
    id.trim().to_ascii_lowercase().replace('_', "-")
}
