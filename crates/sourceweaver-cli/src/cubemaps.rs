use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
struct CubemapProfile {
    id: &'static str,
    label: &'static str,
    source_url: &'static str,
    source_checked: &'static str,
    notes: &'static [&'static str],
    warnings: &'static [&'static str],
    launch_args: &'static [&'static str],
    commands: &'static [&'static str],
}

#[derive(Debug, Clone, Default)]
struct CubemapWorkflowConfig {
    input_bsp: Option<PathBuf>,
    profile_id: String,
    game_executable: Option<PathBuf>,
    steam_app_id: Option<String>,
    game_dir: Option<PathBuf>,
    write_cfg: Option<PathBuf>,
    report: Option<PathBuf>,
    json: bool,
}

#[derive(Debug, Clone, Serialize)]
struct CubemapWorkflowReport {
    ok: bool,
    map_bsp: String,
    map_name: String,
    profile: CubemapProfileSnapshot,
    command_shape: String,
    suggested_launch_args: Vec<String>,
    suggested_direct_command: Option<Vec<String>>,
    suggested_steam_command: Option<Vec<String>>,
    console_commands: Vec<String>,
    cfg_path: Option<String>,
    cfg_written: bool,
    game_executable: Option<String>,
    steam_app_id: Option<String>,
    game_dir: Option<String>,
    writes_bsp: bool,
    log_capture: String,
    runtime_launch_mode: String,
    real_game_runtime_validation: bool,
    external_tool_boundary: Vec<String>,
    warnings: Vec<String>,
    sources_checked: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CubemapProfileSnapshot {
    id: String,
    label: String,
    notes: Vec<String>,
}

const CUBEMAP_PROFILES: &[CubemapProfile] = &[
    CubemapProfile {
        id: "generic",
        label: "Generic Source buildcubemaps pass",
        source_url: "https://valvedev.info/guides/cubemaps-what-they-do-and-how-to-use-them/",
        source_checked: "2026-08-07",
        notes: &[
            "Loads the map, enables cheats for runtime-only helpers, disables specular reflections before building, runs buildcubemaps, then disconnects.",
            "Use for older Source games when a game-specific caveat is unknown.",
        ],
        warnings: &[
            "buildcubemaps runs inside a real game runtime and writes cubemap textures into the BSP.",
            "Back up the compiled BSP before running this workflow.",
            "Only build cubemaps after the final compile and before final asset packing/distribution.",
        ],
        launch_args: &["-dev", "-console", "-condebug", "-w", "1024", "-h", "768"],
        commands: &[
            "mat_specular 0",
            "map {map}",
            "sv_cheats 1",
            "buildcubemaps",
            "disconnect",
            "mat_specular 1",
            "map {map}",
        ],
    },
    CubemapProfile {
        id: "hl2-hdr",
        label: "Half-Life 2/Source HDR plus LDR cubemap passes",
        source_url: "https://valvedev.info/guides/cubemaps-what-they-do-and-how-to-use-them/",
        source_checked: "2026-08-07",
        notes: &[
            "VDU documents separate HDR/LDR passes using mat_hdr_level 0, reload/buildcubemaps, then mat_hdr_level 2 and reload.",
            "Use when the compiled BSP targets an HDR-capable Source branch and both lighting modes are required.",
        ],
        warnings: &[
            "Map reloads are part of the process; review console output and reload/restart the game before judging reflections.",
            "Cubemaps can break if the BSP is renamed after building.",
            "Game resolution must be higher than 512x512 during buildcubemaps according to VDU guidance.",
        ],
        launch_args: &["-dev", "-console", "-condebug", "-w", "1024", "-h", "768"],
        commands: &[
            "map {map}",
            "sv_cheats 1",
            "mat_hdr_level 0",
            "reload",
            "buildcubemaps",
            "mat_hdr_level 2",
            "reload",
            "buildcubemaps",
            "disconnect",
        ],
    },
    CubemapProfile {
        id: "tf2-source2013mp",
        label: "Team Fortress 2 / Source 2013 MP cubemap workaround",
        source_url: "https://valvedev.info/guides/cubemaps-what-they-do-and-how-to-use-them/",
        source_checked: "2026-08-07",
        notes: &[
            "VDU documents turning mat_specular off before buildcubemaps for TF2/Source 2013 MP so missing default cubemap reflections are not captured.",
            "Repeat with mat_hdr_level 2 if the target build requires an HDR pass.",
        ],
        warnings: &[
            "This is a game-runtime workflow and may need manual restart/reload review.",
            "Do not treat this as Source Weaver VMF validation or compiler validation.",
        ],
        launch_args: &["-dev", "-console", "-condebug", "-w", "1024", "-h", "768"],
        commands: &[
            "mat_specular 0",
            "map {map}",
            "sv_cheats 1",
            "buildcubemaps",
            "disconnect",
            "mat_specular 1",
            "map {map}",
        ],
    },
    CubemapProfile {
        id: "csgo",
        label: "Counter-Strike: Global Offensive cubemap pass",
        source_url: "https://valvedev.info/guides/cubemaps-what-they-do-and-how-to-use-them/",
        source_checked: "2026-08-07",
        notes: &[
            "VDU documents CS:GO as HDR-only for cubemap purposes.",
            "VDU documents starting CS:GO with -insecure for cubemap building.",
        ],
        warnings: &[
            "Use only with a local CS:GO legacy/tool install that supports the target BSP.",
            "VAC/online play context is outside Source Weaver; run this only in an offline mapping workflow.",
        ],
        launch_args: &[
            "-insecure",
            "-dev",
            "-console",
            "-condebug",
            "-w",
            "1024",
            "-h",
            "768",
        ],
        commands: &["map {map}", "sv_cheats 1", "buildcubemaps", "disconnect"],
    },
    CubemapProfile {
        id: "l4d",
        label: "Left 4 Dead / Left 4 Dead 2 cubemap pass",
        source_url: "https://valvedev.info/guides/cubemaps-what-they-do-and-how-to-use-them/",
        source_checked: "2026-08-07",
        notes: &[
            "VDU documents that L4D/L4D2 need a full game restart before newly built cubemaps render correctly.",
        ],
        warnings: &[
            "Plan a full runtime restart after buildcubemaps before checking the result.",
            "This command does not automate game restarts or visual inspection.",
        ],
        launch_args: &["-dev", "-console", "-condebug", "-w", "1024", "-h", "768"],
        commands: &["map {map}", "sv_cheats 1", "buildcubemaps", "disconnect"],
    },
    CubemapProfile {
        id: "portal2",
        label: "Portal 2 cubemap workaround plan",
        source_url: "https://valvedev.info/guides/cubemaps-what-they-do-and-how-to-use-them/",
        source_checked: "2026-08-07",
        notes: &[
            "VDU documents a Portal 2 crash caveat when building from portal2/maps and recommends building from portal2_dlc2/maps.",
        ],
        warnings: &[
            "Place the BSP under the correct Portal 2 DLC maps directory before running the generated commands.",
            "This command cannot verify the game install layout or move proprietary map/content files for you.",
        ],
        launch_args: &["-dev", "-console", "-condebug", "-w", "1024", "-h", "768"],
        commands: &["map {map}", "sv_cheats 1", "buildcubemaps", "disconnect"],
    },
];

pub fn command(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(());
    }
    let config = parse_args(args)?;
    let input_bsp = config.input_bsp.as_ref().ok_or("usage: sourceweaver cubemap-workflow <map.bsp> [--profile generic|hl2-hdr|tf2-source2013mp|csgo|l4d|portal2] [--game-executable path | --steam-app-id id] [--game-dir dir] [--write-cfg cfg] [--report report.json] [--json]")?;
    let profile = profile_by_id(if config.profile_id.is_empty() {
        "generic"
    } else {
        &config.profile_id
    })?;
    let report = build_report(&config, input_bsp, profile)?;
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode cubemap workflow report: {error}"))?;
    if let Some(report_path) = &config.report {
        create_parent_dir(report_path, "cubemap workflow report")?;
        fs::write(report_path, &json).map_err(|error| {
            format!(
                "failed to write cubemap workflow report {}: {error}",
                report_path.display()
            )
        })?;
    }
    if config.json {
        println!("{json}");
    } else {
        print_report(&report);
    }
    Ok(())
}

fn parse_args(args: &[String]) -> Result<CubemapWorkflowConfig, String> {
    let mut config = CubemapWorkflowConfig {
        profile_id: "generic".to_string(),
        ..CubemapWorkflowConfig::default()
    };
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--profile" => {
                cursor += 1;
                config.profile_id = args
                    .get(cursor)
                    .ok_or("--profile needs an id")?
                    .trim()
                    .to_ascii_lowercase();
            }
            "--game-executable" | "--game-bin" => {
                cursor += 1;
                config.game_executable = Some(PathBuf::from(
                    args.get(cursor).ok_or("--game-executable needs a path")?,
                ));
            }
            "--steam-app-id" | "--appid" => {
                cursor += 1;
                config.steam_app_id = Some(
                    args.get(cursor)
                        .ok_or("--steam-app-id needs a value")?
                        .clone(),
                );
            }
            "--game-dir" => {
                cursor += 1;
                config.game_dir = Some(PathBuf::from(
                    args.get(cursor).ok_or("--game-dir needs a path")?,
                ));
            }
            "--write-cfg" => {
                cursor += 1;
                config.write_cfg = Some(PathBuf::from(
                    args.get(cursor).ok_or("--write-cfg needs a path")?,
                ));
            }
            "--report" => {
                cursor += 1;
                config.report = Some(PathBuf::from(
                    args.get(cursor).ok_or("--report needs a path")?,
                ));
            }
            "--json" => config.json = true,
            value if value.starts_with('-') => {
                return Err(format!("unknown cubemap-workflow flag `{value}`"));
            }
            value => {
                if config.input_bsp.is_some() {
                    return Err("cubemap-workflow accepts one BSP path".to_string());
                }
                config.input_bsp = Some(PathBuf::from(value));
            }
        }
        cursor += 1;
    }
    if config.game_executable.is_some() && config.steam_app_id.is_some() {
        return Err("choose either --game-executable or --steam-app-id, not both".to_string());
    }
    Ok(config)
}

fn profile_by_id(id: &str) -> Result<&'static CubemapProfile, String> {
    CUBEMAP_PROFILES
        .iter()
        .find(|profile| profile.id == id)
        .ok_or_else(|| {
            format!(
                "unknown cubemap profile `{id}`; available profiles: {}",
                CUBEMAP_PROFILES
                    .iter()
                    .map(|profile| profile.id)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

fn build_report(
    config: &CubemapWorkflowConfig,
    input_bsp: &Path,
    profile: &CubemapProfile,
) -> Result<CubemapWorkflowReport, String> {
    if !input_bsp.exists() {
        return Err(format!("BSP does not exist: {}", input_bsp.display()));
    }
    if !input_bsp.is_file() {
        return Err(format!("BSP path is not a file: {}", input_bsp.display()));
    }
    let map_name = input_bsp
        .file_stem()
        .ok_or_else(|| format!("could not infer map name from {}", input_bsp.display()))?
        .to_string_lossy()
        .to_string();
    let console_commands = profile
        .commands
        .iter()
        .map(|command| command.replace("{map}", &map_name))
        .collect::<Vec<_>>();
    let launch_args = profile
        .launch_args
        .iter()
        .map(|arg| (*arg).to_string())
        .collect::<Vec<_>>();
    if let Some(cfg_path) = &config.write_cfg {
        let cfg_text = cubemap_cfg_text(profile, &console_commands);
        create_parent_dir(cfg_path, "cubemap cfg")?;
        fs::write(cfg_path, cfg_text).map_err(|error| {
            format!(
                "failed to write cubemap cfg {}: {error}",
                cfg_path.display()
            )
        })?;
    }
    let suggested_direct_command = config.game_executable.as_ref().map(|executable| {
        let mut command = vec![executable.display().to_string()];
        command.extend(launch_args.clone());
        command.push("+map".to_string());
        command.push(map_name.clone());
        command
    });
    let suggested_steam_command = config.steam_app_id.as_ref().map(|app_id| {
        let mut command = vec![
            "steam".to_string(),
            "-applaunch".to_string(),
            app_id.clone(),
        ];
        command.extend(launch_args.clone());
        command.push("+map".to_string());
        command.push(map_name.clone());
        command
    });
    let mut warnings = profile
        .warnings
        .iter()
        .map(|warning| (*warning).to_string())
        .collect::<Vec<_>>();
    if input_bsp
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase() != "bsp")
        .unwrap_or(true)
    {
        warnings.push("input path does not have a .bsp extension".to_string());
    }
    if config.game_executable.is_none() && config.steam_app_id.is_none() {
        warnings.push(
            "no --game-executable or --steam-app-id was supplied, so the report contains workflow steps only"
                .to_string(),
        );
    }
    if config.write_cfg.is_some() {
        warnings.push(
            "written cfg is a helper script; Source Weaver did not execute it or verify game-side command sequencing"
                .to_string(),
        );
    }

    Ok(CubemapWorkflowReport {
        ok: true,
        map_bsp: input_bsp.display().to_string(),
        map_name,
        profile: CubemapProfileSnapshot {
            id: profile.id.to_string(),
            label: profile.label.to_string(),
            notes: profile.notes.iter().map(|note| (*note).to_string()).collect(),
        },
        command_shape: "launch game with -dev -console -condebug +map <map>; run generated console/cfg commands inside the game runtime".to_string(),
        suggested_launch_args: launch_args,
        suggested_direct_command,
        suggested_steam_command,
        console_commands,
        cfg_path: config.write_cfg.as_ref().map(|path| path.display().to_string()),
        cfg_written: config.write_cfg.is_some(),
        game_executable: config
            .game_executable
            .as_ref()
            .map(|path| path.display().to_string()),
        steam_app_id: config.steam_app_id.clone(),
        game_dir: config.game_dir.as_ref().map(|path| path.display().to_string()),
        writes_bsp: true,
        log_capture: "Use -condebug to capture game console output to the engine console.log/qconsole.log location for the target game; attach that log to validation evidence after a real run.".to_string(),
        runtime_launch_mode: "plan-only".to_string(),
        real_game_runtime_validation: false,
        external_tool_boundary: vec![
            "Source Weaver generated this workflow report and optional cfg helper only.".to_string(),
            "No Steam client, Source game executable, game runtime, SDK tool, BSPZIP, VBSP, VVIS, or VRAD process was launched by this command.".to_string(),
            "A real cubemap validation row requires running the target game/runtime and preserving the console log plus before/after BSP evidence.".to_string(),
        ],
        warnings,
        sources_checked: vec![
            format!(
                "Valve Developer Union cubemap guide: {} checked {}",
                profile.source_url, profile.source_checked
            ),
            "Valve Developer Community command-line-options mirror checked 2026-08-07 for -buildcubemaps, -condebug, +map, and +sv_cheats launch-option behavior".to_string(),
        ],
    })
}

fn cubemap_cfg_text(profile: &CubemapProfile, commands: &[String]) -> String {
    let mut text = String::new();
    text.push_str("// Source Weaver cubemap workflow helper\n");
    text.push_str("// Profile: ");
    text.push_str(profile.id);
    text.push('\n');
    text.push_str("// Generated from documented workflow research. Review before running.\n");
    text.push_str("// buildcubemaps writes cubemap data into the loaded BSP. Keep a backup.\n");
    for command in commands {
        text.push_str(command);
        text.push('\n');
    }
    text
}

fn create_parent_dir(path: &Path, label: &str) -> Result<(), String> {
    let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create {label} directory {}: {error}",
            parent.display()
        )
    })
}

fn print_report(report: &CubemapWorkflowReport) {
    println!("cubemap workflow: ok");
    println!("map: {}", report.map_bsp);
    println!("map name: {}", report.map_name);
    println!("profile: {}", report.profile.id);
    println!("runtime launch mode: {}", report.runtime_launch_mode);
    println!("writes BSP: {}", report.writes_bsp);
    println!(
        "real game runtime validation: {}",
        report.real_game_runtime_validation
    );
    if let Some(command) = &report.suggested_direct_command {
        println!("direct command: {}", command.join(" "));
    }
    if let Some(command) = &report.suggested_steam_command {
        println!("steam command: {}", command.join(" "));
    }
    if let Some(cfg_path) = &report.cfg_path {
        println!("cfg: {cfg_path}");
    }
    println!("console commands:");
    for command in &report.console_commands {
        println!("  {command}");
    }
    for warning in &report.warnings {
        println!("warning\t{warning}");
    }
}

fn print_help() {
    println!(
        r#"Usage:
  sourceweaver cubemap-workflow <map.bsp> [--profile generic|hl2-hdr|tf2-source2013mp|csgo|l4d|portal2] [--game-executable path | --steam-app-id id] [--game-dir dir] [--write-cfg cfg] [--report report.json] [--json]

Creates a safe cubemap/buildcubemaps workflow report for a compiled BSP.

This command does not launch Steam or a game runtime. It records the expected
runtime boundary, suggested launch arguments, console commands, log-capture
notes, game-specific caveats, and optionally writes a cfg helper.

Examples:
  sourceweaver cubemap-workflow map.bsp --profile hl2-hdr --write-cfg cfg/sourceweaver_buildcubemaps.cfg --json
  sourceweaver cubemap-workflow map.bsp --profile csgo --steam-app-id 730 --report cubemap-report.json
"#
    );
}
