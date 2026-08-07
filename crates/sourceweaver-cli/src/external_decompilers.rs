use serde::Serialize;

#[derive(Debug, Clone, Copy)]
pub struct ExternalDecompilerPreset {
    pub id: &'static str,
    pub tool: &'static str,
    pub status: &'static str,
    pub maintenance: &'static str,
    pub license_summary: &'static str,
    pub source_url: &'static str,
    pub source_checked: &'static str,
    pub command_shape: &'static str,
    pub sourceweaver_workflow: &'static str,
    pub wrapper_example: &'static str,
    pub caveats: &'static [&'static str],
    pub bundle_policy: &'static str,
    pub real_tool_validation: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalDecompilerPresetSnapshot {
    pub id: String,
    pub tool: String,
    pub status: String,
    pub maintenance: String,
    pub license_summary: String,
    pub source_url: String,
    pub source_checked: String,
    pub command_shape: String,
    pub sourceweaver_workflow: String,
    pub wrapper_example: String,
    pub caveats: Vec<String>,
    pub bundle_policy: String,
    pub real_tool_validation: bool,
}

pub const EXTERNAL_DECOMPILER_PRESETS: &[ExternalDecompilerPreset] = &[
    ExternalDecompilerPreset {
        id: "bspsource-supported",
        tool: "BSPSource",
        status: "supported-local-path",
        maintenance: "Actively available upstream compared with legacy VMEX; Source Weaver supports launcher, jar, wrapper, managed manifest/checksum helpers, and argument presets.",
        license_summary: "Upstream BSPSource LICENSE.md lists BSPSource under the Unlicense, with Apache-2.0 and BSD-3-Clause dependencies; see docs/bspsource-managed-download.md.",
        source_url: "https://github.com/ata4/bspsrc",
        source_checked: "2026-08-06",
        command_shape: "bspsrc [tool-args] -o <out.vmf> <input.bsp>",
        sourceweaver_workflow: "Use sourceweaver bsp-import --bspsource <launcher> or --bspsource-jar <jar>; use --preset/--tool-arg as needed.",
        wrapper_example: "No wrapper required for normal BSPSource launcher/jar workflows.",
        caveats: &[
            "BSPSource remains external; Source Weaver does not bundle it.",
            "Decompiled VMFs are approximate and review-required.",
        ],
        bundle_policy: "do-not-bundle; local path or explicit managed cache helper only",
        real_tool_validation: false,
    },
    ExternalDecompilerPreset {
        id: "vmex-legacy-wrapper",
        tool: "VMEX",
        status: "legacy-documentation-only",
        maintenance: "Valve Developer Union marks VMEX obsolete, largely supplanted by BSPSource, no longer in active development, and archived for historical reasons.",
        license_summary: "VDU page says tools belong to their respective coders; no redistribution license for VMEX binaries was verified during this issue.",
        source_url: "https://valvedev.info/tools/vmex/",
        source_checked: "2026-08-06",
        command_shape: "vmex <input.bsp>; decompiled map names have _d appended according to VDU documentation.",
        sourceweaver_workflow: "Use Source Weaver generic wrapper mode only if the user supplies VMEX locally and a wrapper moves VMEX's _d output to Source Weaver's requested <out.vmf> path.",
        wrapper_example: "examples/wrappers/vmex-wrapper.sh",
        caveats: &[
            "No real VMEX run was performed for this issue.",
            "VMEX does not support post-Orange Box Source games according to the VDU page.",
            "VDU notes VMEX output may have broken complex solids, rounding issues, missing editor-only data, and unusable areaportals without post-decompile fixing.",
            "No managed download or redistribution is implemented because a redistribution license was not verified.",
        ],
        bundle_policy: "do-not-bundle; user-provided local tool only; documentation/wrapper example only",
        real_tool_validation: false,
    },
    ExternalDecompilerPreset {
        id: "unknown-wrapper-template",
        tool: "Generic BSP decompiler wrapper",
        status: "escape-hatch",
        maintenance: "Unknown; supplied by the user for local experiments or proprietary/internal tools.",
        license_summary: "User is responsible for tool licensing and redistribution rights. Source Weaver does not redistribute the tool.",
        source_url: "user-provided",
        source_checked: "not applicable",
        command_shape: "<wrapper> [tool-args] <input.bsp> <out.vmf>",
        sourceweaver_workflow: "Use sourceweaver bsp-import --tool <wrapper>; wrapper must write the requested output VMF path.",
        wrapper_example: "examples/wrappers/generic-bsp-decompiler-wrapper.sh",
        caveats: &[
            "Source Weaver cannot infer decompile quality for unknown tools beyond captured log/integrity parsing.",
            "Do not commit proprietary BSPs or generated VMFs without explicit redistribution review.",
        ],
        bundle_policy: "do-not-bundle; user-provided local wrapper only",
        real_tool_validation: false,
    },
];

pub fn preset_snapshots() -> Vec<ExternalDecompilerPresetSnapshot> {
    EXTERNAL_DECOMPILER_PRESETS.iter().map(snapshot).collect()
}

pub fn snapshot(preset: &ExternalDecompilerPreset) -> ExternalDecompilerPresetSnapshot {
    ExternalDecompilerPresetSnapshot {
        id: preset.id.to_string(),
        tool: preset.tool.to_string(),
        status: preset.status.to_string(),
        maintenance: preset.maintenance.to_string(),
        license_summary: preset.license_summary.to_string(),
        source_url: preset.source_url.to_string(),
        source_checked: preset.source_checked.to_string(),
        command_shape: preset.command_shape.to_string(),
        sourceweaver_workflow: preset.sourceweaver_workflow.to_string(),
        wrapper_example: preset.wrapper_example.to_string(),
        caveats: preset
            .caveats
            .iter()
            .map(|caveat| (*caveat).to_string())
            .collect(),
        bundle_policy: preset.bundle_policy.to_string(),
        real_tool_validation: preset.real_tool_validation,
    }
}
