use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct BspzipContextProfile {
    pub id: &'static str,
    pub label: &'static str,
    pub tool_family: &'static str,
    pub platforms: &'static [&'static str],
    pub summary: &'static str,
    pub requirements: &'static [&'static str],
    pub sourceweaver_support: &'static [&'static str],
    pub wrapper_examples: &'static [&'static str],
    pub sources_checked: &'static [&'static str],
    pub real_tool_validation: bool,
}

pub const BSPZIP_CONTEXT_PROFILES: &[BspzipContextProfile] = &[
    BspzipContextProfile {
        id: "stock-game-bin",
        label: "Stock Valve BSPZIP from a game bin directory",
        tool_family: "Valve BSPZIP",
        platforms: &["windows", "linux-toolchain-dependent"],
        summary: "Run the user-provided BSPZIP from the target game's bin/tool directory so vproject and sibling library auto-detection have the same context a mapper would use manually.",
        requirements: &[
            "User supplies a BSPZIP-compatible executable from an owned Source game or SDK install.",
            "Use --tool-cwd to run from the game's bin directory when the tool expects local DLLs or vproject-style auto-detection.",
            "Use explicit --asset-root values for custom content resolution; Source Weaver does not discover proprietary game assets by itself.",
        ],
        sourceweaver_support: &[
            "--tool-cwd sets the packer process working directory.",
            "--context-profile stock-game-bin records why that working directory was selected.",
            "Generated reports preserve the exact command arguments and context fields used for the run.",
        ],
        wrapper_examples: &["examples/wrappers/bspzip-windows-game-bin-wrapper.ps1"],
        sources_checked: &[
            "Valve Developer Union BSPZIP guide checked 2026-08-07: BSPZIP is found in the same game bin directory as Hammer/SDK tools and uses -addlist path-pair file lists.",
        ],
        real_tool_validation: false,
    },
    BspzipContextProfile {
        id: "linux-ld-library-path",
        label: "Linux wrapper with explicit Source library search path",
        tool_family: "Valve BSPZIP or compatible Linux wrapper",
        platforms: &["linux"],
        summary: "Use a wrapper or Source Weaver --library-path entries when a local packer needs Source/Steam runtime shared libraries beside the game/tool install.",
        requirements: &[
            "User supplies the local tool path and all library directories.",
            "Library directories are prepended to LD_LIBRARY_PATH for the packer process only.",
            "The profile records environment shaping only; it does not prove the real packer loaded successfully.",
        ],
        sourceweaver_support: &[
            "--library-path can be repeated and is joined into LD_LIBRARY_PATH for the packer invocation and version probe.",
            "--tool-cwd can be combined with --library-path for tools launched from a game bin directory.",
            "The JSON report records configured library paths and the environment key names applied.",
        ],
        wrapper_examples: &["examples/wrappers/bspzip-linux-ld-library-path-wrapper.sh"],
        sources_checked: &[
            "General Linux dynamic loader behavior checked 2026-08-07: LD_LIBRARY_PATH is a colon-separated library search path for a process.",
            "Valve Developer Union BSPZIP guide checked 2026-08-07 for game-bin tool context and -addlist file-list usage.",
        ],
        real_tool_validation: false,
    },
    BspzipContextProfile {
        id: "bspzipplusplus-sdk2013-x64",
        label: "BSPZIP++ SDK2013 x64 context",
        tool_family: "BSPZIP++",
        platforms: &["windows-x64", "source-sdk-2013-mp-x64"],
        summary: "Documented BSPZIP++ usage for 64-bit SDK2013-based games where users already have the tool installed in the supported game/tool context.",
        requirements: &[
            "Use only with a user-provided BSPZIP++ binary and supported 64-bit SDK2013 game/tool install.",
            "Hammer++ tools documentation lists examples such as Team Fortress 2, Counter-Strike: Source, Day of Defeat: Source, and Half-Life 2: Deathmatch, and states Garry's Mod is unsupported for BSPZIP++.",
            "Redistribution terms must be checked by the user before bundling any third-party tool.",
        ],
        sourceweaver_support: &[
            "--context-profile bspzipplusplus-sdk2013-x64 records the intended BSPZIP++ context in the JSON report.",
            "--tool-cwd can point at the x64/win64 game bin when local DLL/tool context is required.",
            "Source Weaver still invokes a user-selected executable with BSPZIP-compatible -addlist semantics.",
        ],
        wrapper_examples: &[
            "examples/wrappers/bspzip-windows-game-bin-wrapper.ps1",
            "examples/wrappers/bspzip-game-arg-wrapper.sh",
        ],
        sources_checked: &[
            "Hammer++ tools page checked 2026-08-07: BSPZIP++ is a rewrite of SDK2013 BSPZIP and supports 64-bit SDK2013-based games; Garry's Mod is documented as unsupported.",
        ],
        real_tool_validation: false,
    },
    BspzipContextProfile {
        id: "explicit-game-arg-wrapper",
        label: "Wrapper-compatible explicit -game context",
        tool_family: "BSPZIP-compatible wrapper",
        platforms: &["windows", "linux", "wrapper-dependent"],
        summary: "Forward -game <dir> only for wrappers or compatible packers where the user has verified that argument shape.",
        requirements: &[
            "Use --game-dir to record the game/content context.",
            "Use --pass-game-dir to insert -game <dir> before -addlist.",
            "Do not enable --pass-game-dir for stock tools unless that exact tool accepts the flag.",
        ],
        sourceweaver_support: &[
            "--game-dir records the selected game directory in the pack report.",
            "--pass-game-dir opt-in inserts -game <dir> into command_args before -addlist.",
            "Fake-wrapper tests verify command shaping without claiming stock BSPZIP support.",
        ],
        wrapper_examples: &["examples/wrappers/bspzip-game-arg-wrapper.sh"],
        sources_checked: &[
            "Source Weaver issue #107 acceptance criteria checked 2026-08-07 for wrapper-compatible -game profile support.",
            "Valve Developer Union BSPZIP guide checked 2026-08-07 for the baseline -addlist shape used after wrapper context arguments.",
        ],
        real_tool_validation: false,
    },
];

pub fn profile_by_id(id: &str) -> Option<&'static BspzipContextProfile> {
    BSPZIP_CONTEXT_PROFILES
        .iter()
        .find(|profile| profile.id == id)
}

pub fn profile_ids() -> String {
    BSPZIP_CONTEXT_PROFILES
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Debug, Clone, Serialize)]
struct BspzipContextProfilesReport {
    ok: bool,
    bundle_policy: &'static str,
    external_tool_boundary: &'static str,
    profiles: &'static [BspzipContextProfile],
}

pub fn command(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(());
    }
    let json = args.iter().any(|arg| arg == "--json");
    for arg in args {
        if arg != "--json" {
            return Err(format!("unknown bspzip-context-profiles flag `{arg}`"));
        }
    }
    let report = BspzipContextProfilesReport {
        ok: true,
        bundle_policy: "Source Weaver does not bundle Valve BSPZIP, BSPZIP++, game SDKs, Steam files, game content, or third-party packers.",
        external_tool_boundary: "This command reports documented context profiles only; it does not run BSPZIP, BSPZIP++, Hammer, VBSP, VVIS, VRAD, Steam, or a game runtime.",
        profiles: BSPZIP_CONTEXT_PROFILES,
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|error| format!("failed to encode BSPZIP context profiles: {error}"))?
        );
    } else {
        println!("BSPZIP context profiles:");
        for profile in BSPZIP_CONTEXT_PROFILES {
            println!("  {}\t{}", profile.id, profile.label);
        }
        println!("Run with --json for full profile details.");
    }
    Ok(())
}

fn print_help() {
    println!(
        r#"Usage:
  sourceweaver bspzip-context-profiles [--json]

Prints documented BSPZIP/BSPZIP++ context profiles and wrapper boundaries.
No external packer is launched.
"#
    );
}
