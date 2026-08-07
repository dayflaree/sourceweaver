use serde::{Deserialize, Serialize};
use sourceweaver_core::{
    BrushEntityDeletionMode, BrushRole, CampaignAdjacencyGraph, CampaignMapInput,
    CampaignOrderSuggestion, CampaignTransition, ChangelevelChange, ChangelevelPolicy,
    ChangelevelPolicyOptions, ChangelevelPolicyReport, ChangelevelPreserveRule,
    ChangelevelPreservedTransition, ChangelevelScope, DeletionCriteria, DeletionReport, Document,
    EntitySemanticsReport, IntegrityReport, MapComplexityReport, MergeInput, MergeOptions,
    MergeReport, RuleSetValidationReport, ValidationRuleSet, VmfToolValidationReport,
    discover_landmarks, discover_transitions, format_integrity_issue, inspect_entities, merge_maps,
    parse_compile_log, prune_document, suggest_campaign_order, summarize_entity_types,
    validate_document_integrity, validate_for_source_tools,
    validate_for_source_tools_with_rule_set, validation_rule_set_by_id,
    validation_rule_set_choices,
};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_EXTERNAL_TOOL_TIMEOUT_SECONDS: u64 = 900;

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("sourceweaver: {error}");
        process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        print_help();
        return Ok(());
    };

    match command {
        "inspect" => inspect_command(&args[1..]),
        "list-types" => list_types_command(&args[1..]),
        "prune" => prune_command(&args[1..]),
        "merge" => merge_command(&args[1..]),
        "validate" => validate_command(&args[1..]),
        "compile" => compile_command(&args[1..]),
        "compile-profile" | "profile" => compile_profile_command(&args[1..]),
        "model-inspect" => model_inspect_command(&args[1..]),
        "model-compile" => model_compile_command(&args[1..]),
        "bsp-import" | "decompile-bsp" => bsp_import_command(&args[1..]),
        "pack" | "pack-bsp" => pack_command(&args[1..]),
        "run" | "batch" | "job" => run_job_command(&args[1..]),
        "campaign-run" | "campaign-batch" | "campaign-plan" => campaign_run_command(&args[1..]),
        "job-template" => {
            print_job_template();
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("sourceweaver {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        other => Err(format!(
            "unknown command `{other}`. Run `sourceweaver help`."
        )),
    }
}

fn inspect_command(args: &[String]) -> Result<(), String> {
    if args.len() != 1 {
        return Err("usage: sourceweaver inspect <map.vmf>".to_string());
    }
    let document = load_document(&args[0])?;
    let records = inspect_entities(&document);
    let transitions = discover_transitions(&document);
    println!("entities: {}", records.len());
    println!("index\tblock\tclassname\ttargetname\torigin\tsolids\troles");
    for record in records {
        let classname = record.classname.unwrap_or_else(|| "-".to_string());
        let targetname = record.targetname.unwrap_or_else(|| "-".to_string());
        let origin = record
            .origin
            .map(|origin| origin.to_string())
            .unwrap_or_else(|| "-".to_string());
        let roles = format_roles(&record.roles);
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            record.index,
            record.block_name,
            classname,
            targetname,
            origin,
            record.solid_count,
            roles
        );
    }
    if !transitions.is_empty() {
        println!();
        println!("transitions: {}", transitions.len());
        println!("entity_index\ttargetname\ttarget_map\tlandmark\torigin\tsolids");
        for transition in transitions {
            println!(
                "{}\t{}\t{}\t{}\t{}\t{}",
                transition.entity_index,
                transition.targetname.as_deref().unwrap_or("-"),
                transition.target_map.as_deref().unwrap_or("-"),
                transition.landmark.as_deref().unwrap_or("-"),
                transition
                    .origin
                    .map(|origin| origin.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                transition.solid_count
            );
        }
    }
    Ok(())
}

fn list_types_command(args: &[String]) -> Result<(), String> {
    if args.len() != 1 {
        return Err("usage: sourceweaver list-types <map.vmf>".to_string());
    }
    let document = load_document(&args[0])?;
    for (classname, count) in summarize_entity_types(&document) {
        println!("{count}\t{classname}");
    }
    Ok(())
}

fn prune_command(args: &[String]) -> Result<(), String> {
    let mut output: Option<PathBuf> = None;
    let mut input: Option<PathBuf> = None;
    let mut criteria = DeletionCriteria::default();

    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "-o" | "--output" => {
                cursor += 1;
                let value = args.get(cursor).ok_or("--output needs a path")?;
                output = Some(PathBuf::from(value));
            }
            "--drop-classname" => {
                cursor += 1;
                let value = args.get(cursor).ok_or("--drop-classname needs a value")?;
                extend_csv(&mut criteria.classnames, value);
            }
            "--drop-targetname" => {
                cursor += 1;
                let value = args.get(cursor).ok_or("--drop-targetname needs a value")?;
                extend_csv(&mut criteria.targetnames, value);
            }
            "--drop-role" => {
                cursor += 1;
                let value = args.get(cursor).ok_or("--drop-role needs a value")?;
                add_roles(&mut criteria.brush_roles, value)?;
            }
            "--brush-entity-mode" => {
                cursor += 1;
                let value = args
                    .get(cursor)
                    .ok_or("--brush-entity-mode needs whole-entity or matching-solids")?;
                criteria.brush_entity_mode = BrushEntityDeletionMode::parse(value)
                    .ok_or_else(|| format!("unknown brush entity mode `{value}`"))?;
            }
            "--allow-critical-deletion" => {
                criteria.protect_critical_entities = false;
            }
            "--drop-all-entities" => {
                criteria.drop_all_entities = true;
            }
            value if value.starts_with('-') => return Err(format!("unknown prune flag `{value}`")),
            value => {
                if input.is_some() {
                    return Err("prune accepts one input VMF".to_string());
                }
                input = Some(PathBuf::from(value));
            }
        }
        cursor += 1;
    }

    let input = input.ok_or("usage: sourceweaver prune <map.vmf> -o <out.vmf> [--drop-classname name] [--drop-targetname name] [--drop-role role] [--drop-all-entities] [--brush-entity-mode whole-entity|matching-solids] [--allow-critical-deletion]")?;
    let output = output.ok_or("prune needs -o/--output")?;
    let mut document = load_document(&input)?;
    let report = prune_document(&mut document, &criteria);
    write_document(&output, &document)?;
    println!("removed entities: {}", report.removed_entities);
    println!("removed world solids: {}", report.removed_world_solids);
    println!(
        "removed brush-entity solids: {}",
        report.removed_brush_entity_solids
    );
    println!("wrote {}", output.display());
    Ok(())
}

fn merge_command(args: &[String]) -> Result<(), String> {
    let mut output: Option<PathBuf> = None;
    let mut landmark: Option<String> = None;
    let mut changelevel_policy = ChangelevelPolicy::Preserve;
    let mut changelevel_scope = ChangelevelScope::All;
    let mut preserve_external = Vec::new();
    let mut inputs = Vec::new();

    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "-o" | "--output" => {
                cursor += 1;
                let value = args.get(cursor).ok_or("--output needs a path")?;
                output = Some(PathBuf::from(value));
            }
            "--landmark" => {
                cursor += 1;
                let value = args.get(cursor).ok_or("--landmark needs a targetname")?;
                landmark = Some(value.clone());
            }
            "--changelevel-policy" => {
                cursor += 1;
                let value = args
                    .get(cursor)
                    .ok_or("--changelevel-policy needs a value")?;
                changelevel_policy = parse_changelevel_policy(value)?;
            }
            "--changelevel-scope" => {
                cursor += 1;
                let value = args
                    .get(cursor)
                    .ok_or("--changelevel-scope needs a value")?;
                changelevel_scope = parse_changelevel_scope(value)?;
            }
            "--preserve-external-map" => {
                cursor += 1;
                let value = args
                    .get(cursor)
                    .ok_or("--preserve-external-map needs a map name")?;
                preserve_external.push(ChangelevelPreserveRule {
                    map: Some(value.clone()),
                    landmark: None,
                    targetname: None,
                });
            }
            "--preserve-external-landmark" => {
                cursor += 1;
                let value = args
                    .get(cursor)
                    .ok_or("--preserve-external-landmark needs a landmark")?;
                preserve_external.push(ChangelevelPreserveRule {
                    map: None,
                    landmark: Some(value.clone()),
                    targetname: None,
                });
            }
            "--preserve-external-targetname" => {
                cursor += 1;
                let value = args
                    .get(cursor)
                    .ok_or("--preserve-external-targetname needs a targetname")?;
                preserve_external.push(ChangelevelPreserveRule {
                    map: None,
                    landmark: None,
                    targetname: Some(value.clone()),
                });
            }
            value if value.starts_with('-') => return Err(format!("unknown merge flag `{value}`")),
            value => inputs.push(PathBuf::from(value)),
        }
        cursor += 1;
    }

    if inputs.len() < 2 {
        return Err(
            "usage: sourceweaver merge -o <out.vmf> [--landmark name] [--changelevel-policy preserve|disable|delete|rewrite-internal] [--changelevel-scope all|internal-only] [--preserve-external-map map] [--preserve-external-landmark name] [--preserve-external-targetname name] <base.vmf> <add.vmf> [...]"
                .to_string(),
        );
    }
    let output = output.ok_or("merge needs -o/--output")?;

    let mut merge_inputs = Vec::new();
    for path in &inputs {
        let label = path.display().to_string();
        let document = load_document(path)?;
        let integrity = validate_document_integrity(&document, &label);
        for issue in integrity.warnings() {
            eprintln!("{}", format_integrity_issue(issue));
        }
        if let Some(message) = integrity.error_message() {
            return Err(message);
        }
        merge_inputs.push(MergeInput { label, document });
    }

    let output_map = output
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string());
    let stitched_maps = inputs
        .iter()
        .filter_map(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
        })
        .collect::<Vec<_>>();
    let (document, report) = merge_maps(
        merge_inputs,
        &MergeOptions {
            landmark,
            changelevel: ChangelevelPolicyOptions {
                policy: changelevel_policy,
                scope: changelevel_scope,
                output_map,
                stitched_maps,
                preserve_external,
            },
        },
    )?;
    write_document(&output, &document)?;
    println!("merged maps: {}", report.merged_maps);
    println!("appended world solids: {}", report.appended_world_solids);
    println!("appended entities: {}", report.appended_entities);
    for (label, offset) in &report.applied_offsets {
        println!("offset\t{}\t{}", label, offset);
    }
    println!("changelevel policy: {}", report.changelevel.policy);
    println!("changelevel scope: {}", report.changelevel.scope);
    println!(
        "changelevel changes: {}",
        report.changelevel.changed_count()
    );
    for warning in &report.changelevel.warnings {
        println!("changelevel warning\t{warning}");
    }
    for change in &report.changelevel.changed {
        println!(
            "changelevel\t{}\tentity[{}]\t{}",
            change.action, change.entity_index, change.rationale
        );
    }
    println!(
        "changelevel preserved: {}",
        report.changelevel.preserved.len()
    );
    for preserved in &report.changelevel.preserved {
        println!(
            "changelevel-preserved\tentity[{}]\t{}",
            preserved.entity_index, preserved.reason
        );
    }
    println!("wrote {}", output.display());
    Ok(())
}

fn validate_command(args: &[String]) -> Result<(), String> {
    let mut input: Option<PathBuf> = None;
    let mut compile_log: Option<PathBuf> = None;
    let mut vbsp: Option<PathBuf> = None;
    let mut game: Option<PathBuf> = None;
    let mut captured_log: Option<PathBuf> = None;
    let mut rule_set_id: Option<String> = None;
    let mut timeout_seconds = DEFAULT_EXTERNAL_TOOL_TIMEOUT_SECONDS;
    let mut json = false;

    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--compile-log" => {
                cursor += 1;
                compile_log = Some(PathBuf::from(
                    args.get(cursor).ok_or("--compile-log needs a path")?,
                ));
            }
            "--vbsp" => {
                cursor += 1;
                vbsp = Some(PathBuf::from(
                    args.get(cursor).ok_or("--vbsp needs a path")?,
                ));
            }
            "--game" => {
                cursor += 1;
                game = Some(PathBuf::from(
                    args.get(cursor).ok_or("--game needs a path")?,
                ));
            }
            "--capture-log" => {
                cursor += 1;
                captured_log = Some(PathBuf::from(
                    args.get(cursor).ok_or("--capture-log needs a path")?,
                ));
            }
            "--rule-set" | "--profile" | "--game-profile" => {
                cursor += 1;
                rule_set_id = Some(
                    args.get(cursor)
                        .ok_or("--rule-set needs a value")?
                        .to_string(),
                );
            }
            "--timeout-seconds" => {
                cursor += 1;
                timeout_seconds = parse_timeout_seconds(
                    args.get(cursor).ok_or("--timeout-seconds needs a value")?,
                )?;
            }
            "--json" => json = true,
            "--help" | "-h" => {
                print_validate_help();
                return Ok(());
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown validate flag `{value}`"));
            }
            value => {
                if input.is_some() {
                    return Err("validate accepts one input VMF".to_string());
                }
                input = Some(PathBuf::from(value));
            }
        }
        cursor += 1;
    }

    let input = input.ok_or("usage: sourceweaver validate <map.vmf> [--compile-log log.txt] [--rule-set none|hl2] [--vbsp path] [--game game-dir] [--capture-log log.txt] [--timeout-seconds seconds] [--json]")?;
    let rule_set = selected_validation_rule_set(rule_set_id.as_deref())?;
    let document = load_document(&input)?;
    let mut compile_log_text =
        match compile_log {
            Some(path) => Some(fs::read_to_string(&path).map_err(|error| {
                format!("failed to read compile log {}: {error}", path.display())
            })?),
            None => None,
        };

    let vbsp_status = if let Some(vbsp_path) = vbsp {
        let output = run_vbsp(
            &vbsp_path,
            game.as_deref(),
            &input,
            Duration::from_secs(timeout_seconds),
        )?;
        let log = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if let Some(path) = captured_log {
            fs::write(&path, &log).map_err(|error| {
                format!("failed to write captured log {}: {error}", path.display())
            })?;
        }
        compile_log_text = Some(log);
        Some(output.status.code().unwrap_or(-1))
    } else {
        None
    };

    let report = validate_for_source_tools_with_rule_set(
        &document,
        &input.display().to_string(),
        compile_log_text.as_deref(),
        rule_set,
    );
    let snapshot = ValidationSnapshot::from_report(&report, vbsp_status);

    if json {
        let text = serde_json::to_string_pretty(&snapshot)
            .map_err(|error| format!("failed to encode validation report: {error}"))?;
        println!("{text}");
    } else {
        print_validation_snapshot(&snapshot);
    }

    if !snapshot.ok {
        return Err("validation found errors".to_string());
    }
    Ok(())
}

fn run_vbsp(
    vbsp_path: &Path,
    game_dir: Option<&Path>,
    input: &Path,
    timeout: Duration,
) -> Result<Output, String> {
    let mut command = Command::new(vbsp_path);
    if let Some(game_dir) = game_dir {
        command.arg("-game").arg(game_dir);
    }
    command.arg(input);
    run_command_with_timeout(
        &mut command,
        &format!("VBSP command {}", vbsp_path.display()),
        timeout,
    )
}

fn tool_timeout_seconds(value: Option<u64>) -> u64 {
    value.unwrap_or(DEFAULT_EXTERNAL_TOOL_TIMEOUT_SECONDS)
}

fn parse_timeout_seconds(value: &str) -> Result<u64, String> {
    let seconds = value
        .parse::<u64>()
        .map_err(|error| format!("invalid timeout `{value}`: {error}"))?;
    if seconds == 0 {
        return Err("timeout must be at least 1 second".to_string());
    }
    Ok(seconds)
}

fn run_command_with_timeout(
    command: &mut Command,
    label: &str,
    timeout: Duration,
) -> Result<Output, String> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let pid = process::id();
    let stdout_path = env::temp_dir().join(format!("sourceweaver-{pid}-{nonce}-stdout.log"));
    let stderr_path = env::temp_dir().join(format!("sourceweaver-{pid}-{nonce}-stderr.log"));

    let stdout = fs::File::create(&stdout_path).map_err(|error| {
        format!(
            "failed to create temporary stdout file {}: {error}",
            stdout_path.display()
        )
    })?;
    let stderr = fs::File::create(&stderr_path).map_err(|error| {
        format!(
            "failed to create temporary stderr file {}: {error}",
            stderr_path.display()
        )
    })?;

    command
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to start {label}: {error}"))?;
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => return Err(format!("failed to wait for {label}: {error}")),
        }

        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = fs::remove_file(&stdout_path);
            let _ = fs::remove_file(&stderr_path);
            return Err(format!(
                "timed out after {} second(s) running {label}",
                timeout.as_secs()
            ));
        }

        thread::sleep(Duration::from_millis(100));
    };

    let stdout = fs::read(&stdout_path).map_err(|error| {
        format!(
            "failed to read temporary stdout file {}: {error}",
            stdout_path.display()
        )
    })?;
    let stderr = fs::read(&stderr_path).map_err(|error| {
        format!(
            "failed to read temporary stderr file {}: {error}",
            stderr_path.display()
        )
    })?;
    let _ = fs::remove_file(&stdout_path);
    let _ = fs::remove_file(&stderr_path);

    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn compile_command(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_compile_help();
        return Ok(());
    }
    let mut config = parse_compile_args(args)?;
    apply_compile_profile(&mut config)?;
    let input = config.input.clone().ok_or("usage: sourceweaver compile <map.vmf> [--profile profile.toml] [--vbsp path] [--vvis path] [--vrad path] [--game game-dir] [--steps vbsp,vvis,vrad] [--log-dir dir] [--timeout-seconds seconds] [--report report.json] [--json]")?;

    let document = load_document(&input)?;
    let validation = validate_for_source_tools(&document, &input.display().to_string(), None);
    let integrity = snapshot_integrity_report(&validation.integrity);
    if let Some(message) = validation.integrity.error_message() {
        return Err(message);
    }

    let steps = resolve_compile_steps(&config)?;
    let mut step_reports = Vec::new();
    for step in steps {
        step_reports.push(run_compile_pipeline_step(&step, &config, &input)?);
    }

    let ok = integrity.errors == 0 && step_reports.iter().all(|step| step.ok);
    let report = CompilePipelineReport {
        ok,
        map: input.display().to_string(),
        game: config.game.as_ref().map(|path| path.display().to_string()),
        log_dir: config
            .log_dir
            .as_ref()
            .map(|path| path.display().to_string()),
        integrity,
        steps: step_reports,
    };

    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode compile report: {error}"))?;
    if let Some(report_path) = &config.report {
        create_parent_dir(report_path, "report")?;
        fs::write(report_path, &json).map_err(|error| {
            format!(
                "failed to write compile report {}: {error}",
                report_path.display()
            )
        })?;
    }

    if config.json {
        println!("{json}");
    } else {
        print_compile_pipeline_report(&report);
    }

    if !report.ok {
        return Err("compile pipeline found errors".to_string());
    }
    Ok(())
}

fn parse_compile_args(args: &[String]) -> Result<CompileConfig, String> {
    let mut config = CompileConfig::default();
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--profile" => {
                cursor += 1;
                config.profile = Some(PathBuf::from(
                    args.get(cursor).ok_or("--profile needs a path")?,
                ));
            }
            "--vbsp" => {
                cursor += 1;
                config.vbsp = Some(PathBuf::from(
                    args.get(cursor).ok_or("--vbsp needs a path")?,
                ));
            }
            "--vvis" => {
                cursor += 1;
                config.vvis = Some(PathBuf::from(
                    args.get(cursor).ok_or("--vvis needs a path")?,
                ));
            }
            "--vrad" => {
                cursor += 1;
                config.vrad = Some(PathBuf::from(
                    args.get(cursor).ok_or("--vrad needs a path")?,
                ));
            }
            "--game" => {
                cursor += 1;
                config.game = Some(PathBuf::from(
                    args.get(cursor).ok_or("--game needs a path")?,
                ));
            }
            "--steps" => {
                cursor += 1;
                config.steps = Some(
                    args.get(cursor)
                        .ok_or("--steps needs a comma-separated list")?
                        .split(',')
                        .map(|step| step.trim().to_ascii_lowercase())
                        .filter(|step| !step.is_empty())
                        .collect(),
                );
            }
            "--log-dir" => {
                cursor += 1;
                config.log_dir = Some(PathBuf::from(
                    args.get(cursor).ok_or("--log-dir needs a path")?,
                ));
            }
            "--report" => {
                cursor += 1;
                config.report = Some(PathBuf::from(
                    args.get(cursor).ok_or("--report needs a path")?,
                ));
            }
            "--timeout-seconds" => {
                cursor += 1;
                config.timeout_seconds = Some(parse_timeout_seconds(
                    args.get(cursor).ok_or("--timeout-seconds needs a value")?,
                )?);
            }
            "--json" => config.json = true,
            "--help" | "-h" => {
                print_compile_help();
                return Err("".to_string());
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown compile flag `{value}`"));
            }
            value => {
                if config.input.is_some() {
                    return Err("compile accepts one input VMF".to_string());
                }
                config.input = Some(PathBuf::from(value));
            }
        }
        cursor += 1;
    }
    Ok(config)
}

fn apply_compile_profile(config: &mut CompileConfig) -> Result<(), String> {
    let Some(profile_path) = config.profile.clone() else {
        return Ok(());
    };
    let base_dir = profile_path.parent().unwrap_or_else(|| Path::new("."));
    let text = fs::read_to_string(&profile_path).map_err(|error| {
        format!(
            "failed to read compile profile {}: {error}",
            profile_path.display()
        )
    })?;
    let profile: CompileProfile = toml::from_str(&text).map_err(|error| {
        format!(
            "failed to parse compile profile {}: {error}",
            profile_path.display()
        )
    })?;
    if let Some(tools) = profile.tools {
        if config.vbsp.is_none() {
            config.vbsp = tools.vbsp.map(|path| resolve_job_path(base_dir, &path));
        }
        if config.vvis.is_none() {
            config.vvis = tools.vvis.map(|path| resolve_job_path(base_dir, &path));
        }
        if config.vrad.is_none() {
            config.vrad = tools.vrad.map(|path| resolve_job_path(base_dir, &path));
        }
        if config.game.is_none() {
            config.game = tools.game.map(|path| resolve_job_path(base_dir, &path));
        }
    }
    if let Some(settings) = profile.compile {
        if config.steps.is_none() {
            config.steps = settings.steps.map(|steps| {
                steps
                    .into_iter()
                    .map(|step| step.trim().to_ascii_lowercase())
                    .filter(|step| !step.is_empty())
                    .collect()
            });
        }
        if config.log_dir.is_none() {
            config.log_dir = settings
                .log_dir
                .map(|path| resolve_job_path(base_dir, &path));
        }
        if config.timeout_seconds.is_none() {
            config.timeout_seconds = settings.timeout_seconds;
        }
    }
    Ok(())
}

fn resolve_compile_steps(config: &CompileConfig) -> Result<Vec<String>, String> {
    let steps = match &config.steps {
        Some(steps) if !steps.is_empty() => steps.clone(),
        _ => {
            let mut steps = Vec::new();
            if config.vbsp.is_some() {
                steps.push("vbsp".to_string());
            }
            if config.vvis.is_some() {
                steps.push("vvis".to_string());
            }
            if config.vrad.is_some() {
                steps.push("vrad".to_string());
            }
            steps
        }
    };
    if steps.is_empty() {
        return Err("compile needs at least one configured step/tool".to_string());
    }
    for step in &steps {
        if !matches!(step.as_str(), "vbsp" | "vvis" | "vrad") {
            return Err(format!(
                "unknown compile step `{step}`; expected vbsp, vvis, or vrad"
            ));
        }
        if compile_tool_for_step(config, step).is_none() {
            return Err(format!(
                "compile step `{step}` needs --{step} or a profile tool path"
            ));
        }
    }
    Ok(steps)
}

fn compile_tool_for_step<'a>(config: &'a CompileConfig, step: &str) -> Option<&'a PathBuf> {
    match step {
        "vbsp" => config.vbsp.as_ref(),
        "vvis" => config.vvis.as_ref(),
        "vrad" => config.vrad.as_ref(),
        _ => None,
    }
}

fn compile_input_for_step(input: &Path, step: &str) -> PathBuf {
    match step {
        "vbsp" => input.to_path_buf(),
        "vvis" | "vrad" => input.with_extension("bsp"),
        _ => input.to_path_buf(),
    }
}

fn run_compile_pipeline_step(
    step: &str,
    config: &CompileConfig,
    map_input: &Path,
) -> Result<CompileStepReport, String> {
    let tool = compile_tool_for_step(config, step)
        .ok_or_else(|| format!("compile step `{step}` is missing a tool path"))?;
    let step_input = compile_input_for_step(map_input, step);
    let output = run_source_compile_tool(
        tool,
        config.game.as_deref(),
        &step_input,
        Duration::from_secs(tool_timeout_seconds(config.timeout_seconds)),
    )?;
    let log = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let log_path = if let Some(log_dir) = &config.log_dir {
        fs::create_dir_all(log_dir)
            .map_err(|error| format!("failed to create log dir {}: {error}", log_dir.display()))?;
        let path = log_dir.join(format!("{step}.log"));
        fs::write(&path, &log)
            .map_err(|error| format!("failed to write compile log {}: {error}", path.display()))?;
        Some(path)
    } else {
        None
    };
    let summary = parse_compile_log(&log);
    let exit_code = output.status.code();
    let ok = exit_code.map(|code| code == 0).unwrap_or(false) && summary.is_ok();
    Ok(CompileStepReport {
        step: step.to_string(),
        tool: tool.display().to_string(),
        input: step_input.display().to_string(),
        exit_code,
        ok,
        log_path: log_path.map(|path| path.display().to_string()),
        compile_log: CompileLogSnapshot {
            ok: summary.is_ok(),
            errors: summary.errors.len(),
            warnings: summary.warnings.len(),
            leak_detected: summary.leak_detected,
            error_lines: summary.errors,
            warning_lines: summary.warnings,
        },
    })
}

fn run_source_compile_tool(
    tool_path: &Path,
    game_dir: Option<&Path>,
    input: &Path,
    timeout: Duration,
) -> Result<Output, String> {
    let mut command = Command::new(tool_path);
    if let Some(game_dir) = game_dir {
        command.arg("-game").arg(game_dir);
    }
    command.arg(input);
    run_command_with_timeout(
        &mut command,
        &format!("Source compile tool {}", tool_path.display()),
        timeout,
    )
}

fn print_compile_pipeline_report(report: &CompilePipelineReport) {
    println!(
        "compile pipeline: {}",
        if report.ok { "ok" } else { "failed" }
    );
    println!("map: {}", report.map);
    if let Some(game) = &report.game {
        println!("game: {game}");
    }
    for step in &report.steps {
        println!(
            "step\t{}\t{}\texit={:?}\terrors={}\twarnings={}\tleak={}",
            step.step,
            if step.ok { "ok" } else { "failed" },
            step.exit_code,
            step.compile_log.errors,
            step.compile_log.warnings,
            step.compile_log.leak_detected
        );
        if let Some(log_path) = &step.log_path {
            println!("log\t{}\t{}", step.step, log_path);
        }
        for line in &step.compile_log.error_lines {
            println!("compile-error\t{}\t{}", step.step, line);
        }
        for line in &step.compile_log.warning_lines {
            println!("compile-warning\t{}\t{}", step.step, line);
        }
    }
}

fn compile_profile_command(args: &[String]) -> Result<(), String> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        print_compile_profile_help();
        return Ok(());
    };
    match subcommand {
        "create" => compile_profile_create_command(&args[1..]),
        "validate" | "check" => compile_profile_validate_command(&args[1..]),
        "discover" => compile_profile_discover_command(&args[1..]),
        "--help" | "-h" | "help" => {
            print_compile_profile_help();
            Ok(())
        }
        other => Err(format!(
            "unknown compile-profile subcommand `{other}`; expected create, validate, or discover"
        )),
    }
}

fn compile_profile_create_command(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_compile_profile_create_help();
        return Ok(());
    }
    let config = parse_compile_profile_create_args(args)?;
    let profile = profile_from_create_config(&config);
    let toml_text = toml::to_string_pretty(&profile)
        .map_err(|error| format!("failed to encode compile profile TOML: {error}"))?;
    if let Some(output) = &config.output {
        create_parent_dir(output, "compile profile")?;
        fs::write(output, &toml_text).map_err(|error| {
            format!(
                "failed to write compile profile {}: {error}",
                output.display()
            )
        })?;
    }
    if config.json || config.validate {
        let report = validate_compile_profile(
            &profile,
            config
                .output
                .as_deref()
                .unwrap_or_else(|| Path::new("<stdout>")),
        );
        let json = serde_json::to_string_pretty(&CompileProfileCreateReport {
            ok: !toml_text.trim().is_empty() && (!config.validate || report.ok),
            output: config
                .output
                .as_ref()
                .map(|path| path.display().to_string()),
            profile_toml: if config.output.is_some() {
                None
            } else {
                Some(toml_text.clone())
            },
            validation: Some(report),
        })
        .map_err(|error| format!("failed to encode compile profile create report: {error}"))?;
        println!("{json}");
    } else if config.output.is_none() {
        print!("{toml_text}");
    } else if let Some(output) = &config.output {
        println!("wrote compile profile: {}", output.display());
    }
    Ok(())
}

fn compile_profile_validate_command(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_compile_profile_validate_help();
        return Ok(());
    }
    let config = parse_compile_profile_validate_args(args)?;
    let profile_path = config
        .profile
        .as_ref()
        .ok_or("usage: sourceweaver compile-profile validate --profile profile.toml [--json]")?;
    let profile = read_compile_profile(profile_path)?;
    let report = validate_compile_profile(&profile, profile_path);
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode compile profile validation report: {error}"))?;
    if config.json {
        println!("{json}");
    } else {
        print_compile_profile_validation_report(&report);
    }
    if !report.ok {
        return Err("compile profile validation failed".to_string());
    }
    Ok(())
}

fn compile_profile_discover_command(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_compile_profile_discover_help();
        return Ok(());
    }
    let config = parse_compile_profile_discover_args(args)?;
    let report = discover_compile_tools(&config);
    if let Some(output) = &config.output {
        let profile = profile_from_discovery(&config, &report);
        let toml_text = toml::to_string_pretty(&profile).map_err(|error| {
            format!("failed to encode discovered compile profile TOML: {error}")
        })?;
        create_parent_dir(output, "compile profile")?;
        fs::write(output, toml_text).map_err(|error| {
            format!(
                "failed to write discovered compile profile {}: {error}",
                output.display()
            )
        })?;
    }
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode compile tool discovery report: {error}"))?;
    if config.json {
        println!("{json}");
    } else {
        print_compile_tool_discovery_report(&report);
        if let Some(output) = &config.output {
            println!("wrote compile profile: {}", output.display());
        }
    }
    if !report.ok {
        return Err("compile tool discovery found missing required tools".to_string());
    }
    Ok(())
}

fn parse_compile_profile_create_args(
    args: &[String],
) -> Result<CompileProfileCreateConfig, String> {
    let mut config = CompileProfileCreateConfig::default();
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--output" | "-o" => {
                cursor += 1;
                config.output = Some(PathBuf::from(
                    args.get(cursor).ok_or("--output needs a path")?,
                ));
            }
            "--vbsp" => {
                cursor += 1;
                config.vbsp = Some(PathBuf::from(
                    args.get(cursor).ok_or("--vbsp needs a path")?,
                ));
            }
            "--vvis" => {
                cursor += 1;
                config.vvis = Some(PathBuf::from(
                    args.get(cursor).ok_or("--vvis needs a path")?,
                ));
            }
            "--vrad" => {
                cursor += 1;
                config.vrad = Some(PathBuf::from(
                    args.get(cursor).ok_or("--vrad needs a path")?,
                ));
            }
            "--game" => {
                cursor += 1;
                config.game = Some(PathBuf::from(
                    args.get(cursor).ok_or("--game needs a path")?,
                ));
            }
            "--steps" => {
                cursor += 1;
                config.steps = Some(parse_compile_step_list(
                    args.get(cursor)
                        .ok_or("--steps needs a comma-separated list")?,
                )?);
            }
            "--log-dir" => {
                cursor += 1;
                config.log_dir = Some(PathBuf::from(
                    args.get(cursor).ok_or("--log-dir needs a path")?,
                ));
            }
            "--timeout-seconds" => {
                cursor += 1;
                config.timeout_seconds = Some(parse_timeout_seconds(
                    args.get(cursor).ok_or("--timeout-seconds needs a value")?,
                )?);
            }
            "--validate" => config.validate = true,
            "--json" => config.json = true,
            value if value.starts_with('-') => {
                return Err(format!("unknown compile-profile create flag `{value}`"));
            }
            value => {
                return Err(format!(
                    "unexpected compile-profile create argument `{value}`"
                ));
            }
        }
        cursor += 1;
    }
    Ok(config)
}

fn parse_compile_profile_validate_args(
    args: &[String],
) -> Result<CompileProfileValidateConfig, String> {
    let mut config = CompileProfileValidateConfig::default();
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--profile" => {
                cursor += 1;
                config.profile = Some(PathBuf::from(
                    args.get(cursor).ok_or("--profile needs a path")?,
                ));
            }
            "--json" => config.json = true,
            value if value.starts_with('-') => {
                return Err(format!("unknown compile-profile validate flag `{value}`"));
            }
            value => {
                if config.profile.is_some() {
                    return Err("compile-profile validate accepts one profile path".to_string());
                }
                config.profile = Some(PathBuf::from(value));
            }
        }
        cursor += 1;
    }
    Ok(config)
}

fn parse_compile_profile_discover_args(
    args: &[String],
) -> Result<CompileProfileDiscoverConfig, String> {
    let mut config = CompileProfileDiscoverConfig::default();
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--search-dir" => {
                cursor += 1;
                config.search_dirs.push(PathBuf::from(
                    args.get(cursor).ok_or("--search-dir needs a path")?,
                ));
            }
            "--output" | "-o" => {
                cursor += 1;
                config.output = Some(PathBuf::from(
                    args.get(cursor).ok_or("--output needs a path")?,
                ));
            }
            "--game" => {
                cursor += 1;
                config.game = Some(PathBuf::from(
                    args.get(cursor).ok_or("--game needs a path")?,
                ));
            }
            "--steps" => {
                cursor += 1;
                config.steps = Some(parse_compile_step_list(
                    args.get(cursor)
                        .ok_or("--steps needs a comma-separated list")?,
                )?);
            }
            "--log-dir" => {
                cursor += 1;
                config.log_dir = Some(PathBuf::from(
                    args.get(cursor).ok_or("--log-dir needs a path")?,
                ));
            }
            "--timeout-seconds" => {
                cursor += 1;
                config.timeout_seconds = Some(parse_timeout_seconds(
                    args.get(cursor).ok_or("--timeout-seconds needs a value")?,
                )?);
            }
            "--json" => config.json = true,
            value if value.starts_with('-') => {
                return Err(format!("unknown compile-profile discover flag `{value}`"));
            }
            value => {
                return Err(format!(
                    "unexpected compile-profile discover argument `{value}`"
                ));
            }
        }
        cursor += 1;
    }
    Ok(config)
}

fn parse_compile_step_list(value: &str) -> Result<Vec<String>, String> {
    let steps = value
        .split(',')
        .map(|step| step.trim().to_ascii_lowercase())
        .filter(|step| !step.is_empty())
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err("compile step list cannot be empty".to_string());
    }
    for step in &steps {
        if !matches!(step.as_str(), "vbsp" | "vvis" | "vrad") {
            return Err(format!(
                "unknown compile step `{step}`; expected vbsp, vvis, or vrad"
            ));
        }
    }
    Ok(steps)
}

fn profile_from_create_config(config: &CompileProfileCreateConfig) -> CompileProfile {
    CompileProfile {
        tools: Some(CompileProfileTools {
            vbsp: config.vbsp.clone(),
            vvis: config.vvis.clone(),
            vrad: config.vrad.clone(),
            game: config.game.clone(),
        }),
        compile: Some(CompileProfileSettings {
            steps: Some(config.steps.clone().unwrap_or_else(|| {
                let mut steps = Vec::new();
                if config.vbsp.is_some() {
                    steps.push("vbsp".to_string());
                }
                if config.vvis.is_some() {
                    steps.push("vvis".to_string());
                }
                if config.vrad.is_some() {
                    steps.push("vrad".to_string());
                }
                steps
            })),
            log_dir: config.log_dir.clone(),
            timeout_seconds: Some(
                config
                    .timeout_seconds
                    .unwrap_or(DEFAULT_EXTERNAL_TOOL_TIMEOUT_SECONDS),
            ),
        }),
    }
}

fn read_compile_profile(profile_path: &Path) -> Result<CompileProfile, String> {
    let text = fs::read_to_string(profile_path).map_err(|error| {
        format!(
            "failed to read compile profile {}: {error}",
            profile_path.display()
        )
    })?;
    toml::from_str(&text).map_err(|error| {
        format!(
            "failed to parse compile profile {}: {error}",
            profile_path.display()
        )
    })
}

fn validate_compile_profile(
    profile: &CompileProfile,
    profile_path: &Path,
) -> CompileProfileValidationReport {
    let base_dir = profile_path.parent().unwrap_or_else(|| Path::new("."));
    let tools = profile.tools.clone().unwrap_or_default();
    let settings = profile.compile.clone().unwrap_or_default();
    let steps = settings
        .steps
        .clone()
        .unwrap_or_else(|| inferred_compile_steps(&tools));
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    if steps.is_empty() {
        errors.push("profile must configure at least one compile step".to_string());
    }
    let mut tool_reports = Vec::new();
    for step in &steps {
        if !matches!(step.as_str(), "vbsp" | "vvis" | "vrad") {
            errors.push(format!(
                "unknown compile step `{step}`; expected vbsp, vvis, or vrad"
            ));
            continue;
        }
        let configured_path = compile_profile_tool_for_step(&tools, step);
        match configured_path {
            Some(path) => {
                let resolved = resolve_job_path(base_dir, path);
                let exists = resolved.exists();
                let is_file = resolved.is_file();
                let executable = is_executable_file(&resolved);
                if !exists {
                    errors.push(format!(
                        "{step} tool does not exist: {}",
                        resolved.display()
                    ));
                } else if !is_file {
                    errors.push(format!("{step} tool is not a file: {}", resolved.display()));
                } else if !executable {
                    warnings.push(format!(
                        "{step} tool may not be executable: {}",
                        resolved.display()
                    ));
                }
                tool_reports.push(CompileProfileToolCheck {
                    step: step.clone(),
                    path: resolved.display().to_string(),
                    exists,
                    is_file,
                    executable,
                    command_shape: compile_command_shape_for_step(step),
                });
            }
            None => {
                errors.push(format!("compile step `{step}` has no tool path in [tools]"));
                tool_reports.push(CompileProfileToolCheck {
                    step: step.clone(),
                    path: String::new(),
                    exists: false,
                    is_file: false,
                    executable: false,
                    command_shape: compile_command_shape_for_step(step),
                });
            }
        }
    }
    let game = tools
        .game
        .as_ref()
        .map(|path| resolve_job_path(base_dir, path));
    if let Some(game) = &game {
        if !game.exists() {
            errors.push(format!("game directory does not exist: {}", game.display()));
        } else if !game.is_dir() {
            errors.push(format!("game path is not a directory: {}", game.display()));
        }
    } else {
        warnings.push(
            "no [tools].game directory is configured; compile commands will omit -game".to_string(),
        );
    }
    let log_dir = settings
        .log_dir
        .as_ref()
        .map(|path| resolve_job_path(base_dir, path));
    if let Some(timeout) = settings.timeout_seconds {
        if timeout == 0 {
            errors.push("compile.timeout_seconds must be at least 1".to_string());
        }
    } else {
        warnings.push(format!(
            "compile.timeout_seconds is not set; sourceweaver compile defaults to {DEFAULT_EXTERNAL_TOOL_TIMEOUT_SECONDS} seconds"
        ));
    }
    CompileProfileValidationReport {
        ok: errors.is_empty(),
        profile: profile_path.display().to_string(),
        steps,
        tools: tool_reports,
        game: game.map(|path| path.display().to_string()),
        log_dir: log_dir.map(|path| path.display().to_string()),
        timeout_seconds: settings.timeout_seconds,
        errors,
        warnings,
    }
}

fn inferred_compile_steps(tools: &CompileProfileTools) -> Vec<String> {
    let mut steps = Vec::new();
    if tools.vbsp.is_some() {
        steps.push("vbsp".to_string());
    }
    if tools.vvis.is_some() {
        steps.push("vvis".to_string());
    }
    if tools.vrad.is_some() {
        steps.push("vrad".to_string());
    }
    steps
}

fn compile_profile_tool_for_step<'a>(
    tools: &'a CompileProfileTools,
    step: &str,
) -> Option<&'a PathBuf> {
    match step {
        "vbsp" => tools.vbsp.as_ref(),
        "vvis" => tools.vvis.as_ref(),
        "vrad" => tools.vrad.as_ref(),
        _ => None,
    }
}

fn compile_command_shape_for_step(step: &str) -> String {
    match step {
        "vbsp" => "<vbsp> [-game <game-dir>] <map.vmf>".to_string(),
        "vvis" => "<vvis> [-game <game-dir>] <map.bsp>".to_string(),
        "vrad" => "<vrad> [-game <game-dir>] <map.bsp>".to_string(),
        _ => "unknown".to_string(),
    }
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn discover_compile_tools(config: &CompileProfileDiscoverConfig) -> CompileToolDiscoveryReport {
    let mut search_dirs = config.search_dirs.clone();
    if let Some(paths) = env::var_os("PATH") {
        search_dirs.extend(env::split_paths(&paths));
    }
    search_dirs.sort();
    search_dirs.dedup();
    let requested_steps = config
        .steps
        .clone()
        .unwrap_or_else(|| vec!["vbsp".to_string(), "vvis".to_string(), "vrad".to_string()]);
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut tools = Vec::new();
    for step in &requested_steps {
        let candidates = discover_tool_candidates(step, &search_dirs);
        if candidates.is_empty() {
            errors.push(format!(
                "could not find `{step}` in --search-dir paths or PATH; pass an explicit --{step} path to compile-profile create"
            ));
        }
        tools.push(CompileToolDiscoveryCheck {
            step: step.clone(),
            selected: candidates.first().cloned(),
            candidates,
            command_shape: compile_command_shape_for_step(step),
        });
    }
    if let Some(game) = &config.game {
        if !game.exists() {
            errors.push(format!("game directory does not exist: {}", game.display()));
        } else if !game.is_dir() {
            errors.push(format!("game path is not a directory: {}", game.display()));
        }
    } else {
        warnings.push("no --game path was provided for the generated profile".to_string());
    }
    CompileToolDiscoveryReport {
        ok: errors.is_empty(),
        search_dirs: search_dirs
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        tools,
        game: config.game.as_ref().map(|path| path.display().to_string()),
        log_dir: config
            .log_dir
            .as_ref()
            .map(|path| path.display().to_string()),
        timeout_seconds: config.timeout_seconds,
        errors,
        warnings,
    }
}

fn discover_tool_candidates(step: &str, search_dirs: &[PathBuf]) -> Vec<String> {
    let names = tool_binary_names(step);
    let mut candidates = BTreeSet::new();
    for dir in search_dirs {
        for name in &names {
            let candidate = dir.join(name);
            if is_executable_file(&candidate) {
                candidates.insert(candidate.display().to_string());
            }
        }
    }
    candidates.into_iter().collect()
}

fn tool_binary_names(step: &str) -> Vec<String> {
    let mut names = vec![step.to_string()];
    if cfg!(windows) {
        names.push(format!("{step}.exe"));
    } else {
        names.push(format!("{step}.exe"));
        names.push(format!("{step}.sh"));
    }
    names
}

fn profile_from_discovery(
    config: &CompileProfileDiscoverConfig,
    report: &CompileToolDiscoveryReport,
) -> CompileProfile {
    let selected = |step: &str| -> Option<PathBuf> {
        report
            .tools
            .iter()
            .find(|tool| tool.step == step)
            .and_then(|tool| tool.selected.as_ref())
            .map(PathBuf::from)
    };
    CompileProfile {
        tools: Some(CompileProfileTools {
            vbsp: selected("vbsp"),
            vvis: selected("vvis"),
            vrad: selected("vrad"),
            game: config.game.clone(),
        }),
        compile: Some(CompileProfileSettings {
            steps: Some(config.steps.clone().unwrap_or_else(|| {
                vec!["vbsp".to_string(), "vvis".to_string(), "vrad".to_string()]
            })),
            log_dir: config.log_dir.clone(),
            timeout_seconds: Some(
                config
                    .timeout_seconds
                    .unwrap_or(DEFAULT_EXTERNAL_TOOL_TIMEOUT_SECONDS),
            ),
        }),
    }
}

fn print_compile_profile_validation_report(report: &CompileProfileValidationReport) {
    println!(
        "compile profile: {}",
        if report.ok { "ok" } else { "failed" }
    );
    println!("profile: {}", report.profile);
    if let Some(game) = &report.game {
        println!("game: {game}");
    }
    for tool in &report.tools {
        println!(
            "tool\t{}\t{}\texists={}\texecutable={}\t{}",
            tool.step, tool.path, tool.exists, tool.executable, tool.command_shape
        );
    }
    for warning in &report.warnings {
        println!("warning\t{warning}");
    }
    for error in &report.errors {
        println!("error\t{error}");
    }
}

fn print_compile_tool_discovery_report(report: &CompileToolDiscoveryReport) {
    println!(
        "compile tool discovery: {}",
        if report.ok { "ok" } else { "failed" }
    );
    for tool in &report.tools {
        println!(
            "tool\t{}\tselected={}\tcandidates={}",
            tool.step,
            tool.selected.as_deref().unwrap_or(""),
            tool.candidates.len()
        );
        for candidate in &tool.candidates {
            println!("candidate\t{}\t{}", tool.step, candidate);
        }
    }
    for warning in &report.warnings {
        println!("warning\t{warning}");
    }
    for error in &report.errors {
        println!("error\t{error}");
    }
}

fn model_inspect_command(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_model_inspect_help();
        return Ok(());
    }
    let mut input = None;
    let mut json = false;
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            value if value.starts_with('-') => {
                return Err(format!("unknown model-inspect flag `{value}`"));
            }
            value => {
                if input.is_some() {
                    return Err("model-inspect accepts one MDL path".to_string());
                }
                input = Some(PathBuf::from(value));
            }
        }
    }
    let input = input.ok_or("usage: sourceweaver model-inspect <model.mdl> [--json]")?;
    let report = inspect_mdl_header(&input)?;
    let encoded = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode model inspect report: {error}"))?;
    if json {
        println!("{encoded}");
    } else {
        println!("model inspect: {}", if report.ok { "ok" } else { "failed" });
        println!("path: {}", report.path);
        println!("size: {}", report.file_size);
        if let Some(header) = &report.header {
            println!("magic: {}", header.magic);
            println!("version: {}", header.version);
            println!("name: {}", header.name);
            println!("data length: {}", header.data_length);
        }
        for warning in &report.warnings {
            println!("warning\t{warning}");
        }
        for error in &report.errors {
            println!("error\t{error}");
        }
    }
    if !report.ok {
        return Err("model inspect failed".to_string());
    }
    Ok(())
}

fn model_compile_command(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_model_compile_help();
        return Ok(());
    }
    let config = parse_model_compile_args(args)?;
    let input_qc = config.input_qc.as_ref().ok_or("usage: sourceweaver model-compile <model.qc> --studiomdl <path> [--game game-dir] [--tool-arg arg] [--log log.txt] [--timeout-seconds seconds] [--report report.json] [--json]")?;
    let studiomdl = config
        .studiomdl
        .as_ref()
        .ok_or("model-compile needs --studiomdl <path>")?;
    if !input_qc.exists() {
        return Err(format!("QC file does not exist: {}", input_qc.display()));
    }
    let invocation = resolve_model_compile_invocation(&config, input_qc, studiomdl);
    let output = run_model_compile_tool(
        &invocation,
        Duration::from_secs(tool_timeout_seconds(config.timeout_seconds)),
    )?;
    let log_text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if let Some(log_path) = &config.log {
        create_parent_dir(log_path, "model compile log")?;
        fs::write(log_path, &log_text).map_err(|error| {
            format!(
                "failed to write model compile log {}: {error}",
                log_path.display()
            )
        })?;
    }
    let summary = parse_compile_log(&log_text);
    let log_snapshot = CompileLogSnapshot {
        ok: summary.errors.is_empty() && !summary.leak_detected,
        errors: summary.errors.len(),
        warnings: summary.warnings.len(),
        leak_detected: summary.leak_detected,
        error_lines: summary.errors,
        warning_lines: summary.warnings,
    };
    let ok = output.status.success() && log_snapshot.errors == 0 && !log_snapshot.leak_detected;
    let report = ModelCompileReport {
        ok,
        tool: invocation.executable.display().to_string(),
        command_shape: invocation.command_shape.to_string(),
        command_args: invocation.args,
        input_qc: input_qc.display().to_string(),
        game: config.game.as_ref().map(|path| path.display().to_string()),
        exit_code: output.status.code(),
        log_path: config.log.as_ref().map(|path| path.display().to_string()),
        log_summary: log_snapshot,
    };
    finish_model_compile_report(&config, report)
}

fn parse_model_compile_args(args: &[String]) -> Result<ModelCompileConfig, String> {
    let mut config = ModelCompileConfig::default();
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--studiomdl" => {
                cursor += 1;
                config.studiomdl = Some(PathBuf::from(
                    args.get(cursor).ok_or("--studiomdl needs a path")?,
                ));
            }
            "--game" => {
                cursor += 1;
                config.game = Some(PathBuf::from(
                    args.get(cursor).ok_or("--game needs a path")?,
                ));
            }
            "--tool-arg" => {
                cursor += 1;
                config
                    .tool_args
                    .push(args.get(cursor).ok_or("--tool-arg needs a value")?.clone());
            }
            "--log" => {
                cursor += 1;
                config.log = Some(PathBuf::from(args.get(cursor).ok_or("--log needs a path")?));
            }
            "--report" => {
                cursor += 1;
                config.report = Some(PathBuf::from(
                    args.get(cursor).ok_or("--report needs a path")?,
                ));
            }
            "--timeout-seconds" => {
                cursor += 1;
                config.timeout_seconds = Some(parse_timeout_seconds(
                    args.get(cursor).ok_or("--timeout-seconds needs a value")?,
                )?);
            }
            "--json" => config.json = true,
            value if value.starts_with('-') => {
                return Err(format!("unknown model-compile flag `{value}`"));
            }
            value => {
                if config.input_qc.is_some() {
                    return Err("model-compile accepts one QC path".to_string());
                }
                config.input_qc = Some(PathBuf::from(value));
            }
        }
        cursor += 1;
    }
    Ok(config)
}

fn inspect_mdl_header(path: &Path) -> Result<ModelInspectReport, String> {
    let data = fs::read(path)
        .map_err(|error| format!("failed to read model {}: {error}", path.display()))?;
    let file_size = data.len();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let header = if data.len() < 80 {
        errors
            .push("file is too small to contain a Source/GoldSource MDL header prefix".to_string());
        None
    } else {
        let magic = String::from_utf8_lossy(&data[0..4]).to_string();
        if !matches!(magic.as_str(), "IDST" | "IDSQ") {
            errors.push(format!(
                "unexpected MDL magic `{magic}`; expected IDST or IDSQ"
            ));
        }
        let version = read_i32_le(&data, 4);
        let checksum = read_i32_le(&data, 8);
        let name = trim_nul_utf8(&data[12..76]);
        let data_length = read_i32_le(&data, 76);
        if data_length <= 0 {
            warnings.push(format!("header data length is non-positive: {data_length}"));
        } else if data_length as usize > file_size {
            warnings.push(format!(
                "header data length {data_length} exceeds file size {file_size}"
            ));
        }
        Some(MdlHeaderSnapshot {
            magic,
            version,
            checksum,
            name,
            data_length,
            supported_magic: matches!(&data[0..4], b"IDST" | b"IDSQ"),
        })
    };
    Ok(ModelInspectReport {
        ok: errors.is_empty(),
        path: path.display().to_string(),
        file_size,
        header,
        warnings,
        errors,
    })
}

fn read_i32_le(data: &[u8], offset: usize) -> i32 {
    let mut bytes = [0_u8; 4];
    bytes.copy_from_slice(&data[offset..offset + 4]);
    i32::from_le_bytes(bytes)
}

fn trim_nul_utf8(data: &[u8]) -> String {
    let end = data
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(data.len());
    String::from_utf8_lossy(&data[..end]).trim().to_string()
}

fn resolve_model_compile_invocation(
    config: &ModelCompileConfig,
    input_qc: &Path,
    studiomdl: &Path,
) -> ModelCompileInvocation {
    let mut args = config.tool_args.clone();
    if let Some(game) = &config.game {
        args.push("-game".to_string());
        args.push(game.display().to_string());
    }
    args.push(input_qc.display().to_string());
    ModelCompileInvocation {
        executable: studiomdl.to_path_buf(),
        args,
        command_shape: "studiomdl [tool-args] [-game <game-dir>] <model.qc>",
    }
}

fn run_model_compile_tool(
    invocation: &ModelCompileInvocation,
    timeout: Duration,
) -> Result<Output, String> {
    let mut command = Command::new(&invocation.executable);
    command.args(&invocation.args);
    run_command_with_timeout(
        &mut command,
        &format!("StudioMDL tool {}", invocation.executable.display()),
        timeout,
    )
}

fn finish_model_compile_report(
    config: &ModelCompileConfig,
    report: ModelCompileReport,
) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode model compile report: {error}"))?;
    if let Some(report_path) = &config.report {
        create_parent_dir(report_path, "model compile report")?;
        fs::write(report_path, &json).map_err(|error| {
            format!(
                "failed to write model compile report {}: {error}",
                report_path.display()
            )
        })?;
    }
    if config.json {
        println!("{json}");
    } else {
        println!("model compile: {}", if report.ok { "ok" } else { "failed" });
        println!("tool: {}", report.tool);
        println!("input qc: {}", report.input_qc);
        if let Some(game) = &report.game {
            println!("game: {game}");
        }
        println!("exit code: {:?}", report.exit_code);
        println!("log errors: {}", report.log_summary.errors);
        println!("log warnings: {}", report.log_summary.warnings);
    }
    if !report.ok {
        return Err("model compile reported errors".to_string());
    }
    Ok(())
}

fn bsp_import_command(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_bsp_import_help();
        return Ok(());
    }
    let config = parse_bsp_import_args(args)?;
    let input = config.input.as_ref().ok_or("usage: sourceweaver bsp-import <map.bsp> (--bspsource <bspsrc> | --bspsource-jar <bspsrc.jar> | --tool <wrapper>) --output <out.vmf> [--java java] [--tool-arg arg] [--log log.txt] [--timeout-seconds seconds] [--report report.json] [--json]")?;
    let output_vmf = config
        .output
        .as_ref()
        .ok_or("bsp-import needs --output <out.vmf>")?;
    create_parent_dir(output_vmf, "output VMF")?;

    let invocation = resolve_bsp_decompiler_invocation(&config, input, output_vmf)?;
    let tool_version = probe_bsp_decompiler_version(&config);
    let tool_output = run_bsp_decompiler(
        &invocation,
        Duration::from_secs(tool_timeout_seconds(config.timeout_seconds)),
    )?;
    let log_text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&tool_output.stdout),
        String::from_utf8_lossy(&tool_output.stderr)
    );
    if let Some(log_path) = &config.log {
        create_parent_dir(log_path, "log")?;
        fs::write(log_path, &log_text).map_err(|error| {
            format!(
                "failed to write decompile log {}: {error}",
                log_path.display()
            )
        })?;
    }

    let summary = parse_compile_log(&log_text);
    let generated_vmf_exists = output_vmf.is_file();
    let mut integrity = None;
    let mut entity_count = None;
    let mut classname_count = None;
    if generated_vmf_exists {
        match load_document(output_vmf) {
            Ok(document) => {
                let report =
                    validate_document_integrity(&document, &output_vmf.display().to_string());
                entity_count = Some(inspect_entities(&document).len());
                classname_count = Some(summarize_entity_types(&document).len());
                integrity = Some(snapshot_integrity_report(&report));
            }
            Err(error) => {
                let compile_log = CompileLogSnapshot {
                    ok: false,
                    errors: summary.errors.len() + 1,
                    warnings: summary.warnings.len(),
                    leak_detected: summary.leak_detected,
                    error_lines: summary
                        .errors
                        .iter()
                        .cloned()
                        .chain(std::iter::once(error))
                        .collect(),
                    warning_lines: summary.warnings.clone(),
                };
                let report = BspImportReport {
                    ok: false,
                    tool: invocation.executable.display().to_string(),
                    tool_kind: invocation.kind.to_string(),
                    tool_version,
                    command_args: invocation.args.clone(),
                    command_shape: invocation.command_shape.to_string(),
                    input_bsp: input.display().to_string(),
                    output_vmf: output_vmf.display().to_string(),
                    exit_code: tool_output.status.code(),
                    log_path: config.log.as_ref().map(|path| path.display().to_string()),
                    generated_vmf_exists,
                    integrity,
                    entity_count,
                    classname_count,
                    log_summary: compile_log,
                };
                return finish_bsp_import_report(&config, report);
            }
        }
    }

    let log_snapshot = CompileLogSnapshot {
        ok: summary.is_ok() || summary.errors.is_empty(),
        errors: summary.errors.len(),
        warnings: summary.warnings.len(),
        leak_detected: summary.leak_detected,
        error_lines: summary.errors,
        warning_lines: summary.warnings,
    };
    let ok = tool_output.status.success()
        && generated_vmf_exists
        && integrity
            .as_ref()
            .map(|report| report.errors == 0)
            .unwrap_or(false);
    let report = BspImportReport {
        ok,
        tool: invocation.executable.display().to_string(),
        tool_kind: invocation.kind.to_string(),
        tool_version,
        command_args: invocation.args,
        command_shape: invocation.command_shape.to_string(),
        input_bsp: input.display().to_string(),
        output_vmf: output_vmf.display().to_string(),
        exit_code: tool_output.status.code(),
        log_path: config.log.as_ref().map(|path| path.display().to_string()),
        generated_vmf_exists,
        integrity,
        entity_count,
        classname_count,
        log_summary: log_snapshot,
    };
    finish_bsp_import_report(&config, report)
}

fn parse_bsp_import_args(args: &[String]) -> Result<BspImportConfig, String> {
    let mut config = BspImportConfig::default();
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--tool" => {
                cursor += 1;
                config.tool = Some(PathBuf::from(
                    args.get(cursor).ok_or("--tool needs a path")?,
                ));
            }
            "--bspsource" | "--bspsrc" => {
                cursor += 1;
                config.bspsource = Some(PathBuf::from(
                    args.get(cursor).ok_or("--bspsource needs a path")?,
                ));
            }
            "--bspsource-jar" | "--bspsrc-jar" => {
                cursor += 1;
                config.bspsource_jar = Some(PathBuf::from(
                    args.get(cursor).ok_or("--bspsource-jar needs a path")?,
                ));
            }
            "--java" => {
                cursor += 1;
                config.java = Some(PathBuf::from(
                    args.get(cursor).ok_or("--java needs a path")?,
                ));
            }
            "--output" | "-o" => {
                cursor += 1;
                config.output = Some(PathBuf::from(
                    args.get(cursor).ok_or("--output needs a path")?,
                ));
            }
            "--log" => {
                cursor += 1;
                config.log = Some(PathBuf::from(args.get(cursor).ok_or("--log needs a path")?));
            }
            "--report" => {
                cursor += 1;
                config.report = Some(PathBuf::from(
                    args.get(cursor).ok_or("--report needs a path")?,
                ));
            }
            "--timeout-seconds" => {
                cursor += 1;
                config.timeout_seconds = Some(parse_timeout_seconds(
                    args.get(cursor).ok_or("--timeout-seconds needs a value")?,
                )?);
            }
            "--tool-arg" => {
                cursor += 1;
                config
                    .tool_args
                    .push(args.get(cursor).ok_or("--tool-arg needs a value")?.clone());
            }
            "--json" => config.json = true,
            value if value.starts_with('-') => {
                return Err(format!("unknown bsp-import flag `{value}`"));
            }
            value => {
                if config.input.is_some() {
                    return Err("bsp-import accepts one input BSP".to_string());
                }
                config.input = Some(PathBuf::from(value));
            }
        }
        cursor += 1;
    }
    Ok(config)
}

fn resolve_bsp_decompiler_invocation(
    config: &BspImportConfig,
    input: &Path,
    output_vmf: &Path,
) -> Result<BspDecompilerInvocation, String> {
    let configured = usize::from(config.tool.is_some())
        + usize::from(config.bspsource.is_some())
        + usize::from(config.bspsource_jar.is_some());
    if configured != 1 {
        return Err(
            "bsp-import needs exactly one of --bspsource, --bspsource-jar, or --tool".to_string(),
        );
    }

    if let Some(tool) = &config.bspsource {
        let mut args = config.tool_args.clone();
        args.push("-o".to_string());
        args.push(output_vmf.display().to_string());
        args.push(input.display().to_string());
        return Ok(BspDecompilerInvocation {
            kind: "bspsource-cli",
            executable: tool.clone(),
            args,
            command_shape: "bspsrc [tool-args] -o <out.vmf> <input.bsp>",
        });
    }

    if let Some(jar) = &config.bspsource_jar {
        let mut args = vec!["-jar".to_string(), jar.display().to_string()];
        args.extend(config.tool_args.clone());
        args.push("-o".to_string());
        args.push(output_vmf.display().to_string());
        args.push(input.display().to_string());
        return Ok(BspDecompilerInvocation {
            kind: "bspsource-jar",
            executable: config.java.clone().unwrap_or_else(|| PathBuf::from("java")),
            args,
            command_shape: "java -jar <bspsrc.jar> [tool-args] -o <out.vmf> <input.bsp>",
        });
    }

    let tool = config
        .tool
        .as_ref()
        .expect("generic wrapper path exists when configured count is one");
    let mut args = config.tool_args.clone();
    args.push(input.display().to_string());
    args.push(output_vmf.display().to_string());
    Ok(BspDecompilerInvocation {
        kind: "generic-wrapper",
        executable: tool.clone(),
        args,
        command_shape: "<wrapper> [tool-args] <input.bsp> <out.vmf>",
    })
}

fn run_bsp_decompiler(
    invocation: &BspDecompilerInvocation,
    timeout: Duration,
) -> Result<Output, String> {
    let mut command = Command::new(&invocation.executable);
    command.args(&invocation.args);
    run_command_with_timeout(
        &mut command,
        &format!("BSP decompiler {}", invocation.executable.display()),
        timeout,
    )
}

fn probe_bsp_decompiler_version(config: &BspImportConfig) -> Option<String> {
    let (executable, args) = if let Some(tool) = &config.bspsource {
        (tool.clone(), vec!["--version".to_string()])
    } else if let Some(jar) = &config.bspsource_jar {
        (
            config.java.clone().unwrap_or_else(|| PathBuf::from("java")),
            vec![
                "-jar".to_string(),
                jar.display().to_string(),
                "--version".to_string(),
            ],
        )
    } else {
        return None;
    };

    let mut command = Command::new(&executable);
    command.args(args);
    match run_command_with_timeout(
        &mut command,
        "BSP decompiler version probe",
        Duration::from_secs(30),
    ) {
        Ok(output) => {
            let text = trimmed_tool_output(&output);
            if text.is_empty() { None } else { Some(text) }
        }
        Err(error) => Some(format!("version probe failed: {error}")),
    }
}

fn trimmed_tool_output(output: &Output) -> String {
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(20)
        .collect::<Vec<_>>()
        .join("\n")
}

fn finish_bsp_import_report(
    config: &BspImportConfig,
    report: BspImportReport,
) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode BSP import report: {error}"))?;
    if let Some(report_path) = &config.report {
        create_parent_dir(report_path, "report")?;
        fs::write(report_path, &json).map_err(|error| {
            format!(
                "failed to write BSP import report {}: {error}",
                report_path.display()
            )
        })?;
    }
    if config.json {
        println!("{json}");
    } else {
        println!("bsp import: {}", if report.ok { "ok" } else { "failed" });
        println!("tool kind: {}", report.tool_kind);
        println!("tool: {}", report.tool);
        if let Some(version) = &report.tool_version {
            println!("tool version: {version}");
        }
        println!("input bsp: {}", report.input_bsp);
        println!("output vmf: {}", report.output_vmf);
        println!("exit code: {:?}", report.exit_code);
        println!("generated vmf exists: {}", report.generated_vmf_exists);
        if let Some(integrity) = &report.integrity {
            println!("integrity errors: {}", integrity.errors);
            println!("integrity warnings: {}", integrity.warnings);
        }
        println!("log errors: {}", report.log_summary.errors);
        println!("log warnings: {}", report.log_summary.warnings);
    }
    if !report.ok {
        return Err("BSP import validation failed".to_string());
    }
    Ok(())
}

fn pack_command(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_pack_help();
        return Ok(());
    }
    let config = parse_pack_args(args)?;
    let input = config.input.as_ref().ok_or("usage: sourceweaver pack <map.bsp> --tool <bspzip> --output <out.bsp> (--filelist list.txt | --asset-root dir --include path) [--log log.txt] [--timeout-seconds seconds] [--report report.json] [--json]")?;
    let tool = config.tool.as_ref().ok_or("pack needs --tool <bspzip>")?;
    let output_bsp = config
        .output
        .as_ref()
        .ok_or("pack needs --output <out.bsp>")?;
    create_parent_dir(output_bsp, "output BSP")?;

    let list = prepare_pack_filelist(&config)?;
    let tool_version = probe_bsp_packer_version(tool);
    if !list.missing_files.is_empty() {
        let report = PackReport {
            ok: false,
            tool: tool.display().to_string(),
            tool_kind: "bspzip-addlist".to_string(),
            tool_version,
            command_shape: "bspzip -addlist <input.bsp> <filelist.txt> <output.bsp>".to_string(),
            command_args: Vec::new(),
            input_bsp: input.display().to_string(),
            output_bsp: output_bsp.display().to_string(),
            output_bsp_exists: output_bsp.exists(),
            filelist_path: list.filelist_path.display().to_string(),
            asset_roots: config
                .asset_roots
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            requested_files: list.assets,
            missing_files: list.missing_files,
            warnings: list.warnings,
            exit_code: None,
            log_path: config.log.as_ref().map(|path| path.display().to_string()),
            packed_file_count: None,
            log_summary: empty_compile_log_snapshot(),
        };
        return finish_pack_report(&config, report);
    }

    let invocation = BspPackInvocation {
        executable: tool.clone(),
        args: vec![
            "-addlist".to_string(),
            input.display().to_string(),
            list.filelist_path.display().to_string(),
            output_bsp.display().to_string(),
        ],
        command_shape: "bspzip -addlist <input.bsp> <filelist.txt> <output.bsp>",
    };
    let tool_output = run_bsp_packer(
        &invocation,
        Duration::from_secs(tool_timeout_seconds(config.timeout_seconds)),
    )?;
    let log_text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&tool_output.stdout),
        String::from_utf8_lossy(&tool_output.stderr)
    );
    if let Some(log_path) = &config.log {
        create_parent_dir(log_path, "log")?;
        fs::write(log_path, &log_text).map_err(|error| {
            format!(
                "failed to write BSP pack log {}: {error}",
                log_path.display()
            )
        })?;
    }

    let summary = parse_compile_log(&log_text);
    let log_snapshot = CompileLogSnapshot {
        ok: summary.is_ok() || summary.errors.is_empty(),
        errors: summary.errors.len(),
        warnings: summary.warnings.len(),
        leak_detected: summary.leak_detected,
        error_lines: summary.errors,
        warning_lines: summary.warnings,
    };
    let output_bsp_exists = output_bsp.exists();
    let ok = tool_output.status.success()
        && output_bsp_exists
        && log_snapshot.errors == 0
        && !log_snapshot.leak_detected;
    let report = PackReport {
        ok,
        tool: invocation.executable.display().to_string(),
        tool_kind: "bspzip-addlist".to_string(),
        tool_version,
        command_shape: invocation.command_shape.to_string(),
        command_args: invocation.args,
        input_bsp: input.display().to_string(),
        output_bsp: output_bsp.display().to_string(),
        output_bsp_exists,
        filelist_path: list.filelist_path.display().to_string(),
        asset_roots: config
            .asset_roots
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        requested_files: list.assets,
        missing_files: list.missing_files,
        warnings: list.warnings,
        exit_code: tool_output.status.code(),
        log_path: config.log.as_ref().map(|path| path.display().to_string()),
        packed_file_count: count_bspzip_added_files(&log_text),
        log_summary: log_snapshot,
    };
    finish_pack_report(&config, report)
}

fn parse_pack_args(args: &[String]) -> Result<PackConfig, String> {
    let mut config = PackConfig::default();
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--tool" => {
                cursor += 1;
                config.tool = Some(PathBuf::from(
                    args.get(cursor).ok_or("--tool needs a path")?,
                ));
            }
            "--output" | "-o" => {
                cursor += 1;
                config.output = Some(PathBuf::from(
                    args.get(cursor).ok_or("--output needs a path")?,
                ));
            }
            "--asset-root" => {
                cursor += 1;
                config.asset_roots.push(PathBuf::from(
                    args.get(cursor).ok_or("--asset-root needs a path")?,
                ));
            }
            "--include" => {
                cursor += 1;
                config.includes.push(
                    args.get(cursor)
                        .ok_or("--include needs a relative asset path")?
                        .clone(),
                );
            }
            "--filelist" => {
                cursor += 1;
                config.filelist = Some(PathBuf::from(
                    args.get(cursor).ok_or("--filelist needs a path")?,
                ));
            }
            "--log" => {
                cursor += 1;
                config.log = Some(PathBuf::from(args.get(cursor).ok_or("--log needs a path")?));
            }
            "--report" => {
                cursor += 1;
                config.report = Some(PathBuf::from(
                    args.get(cursor).ok_or("--report needs a path")?,
                ));
            }
            "--timeout-seconds" => {
                cursor += 1;
                config.timeout_seconds = Some(parse_timeout_seconds(
                    args.get(cursor).ok_or("--timeout-seconds needs a value")?,
                )?);
            }
            "--json" => config.json = true,
            value if value.starts_with('-') => return Err(format!("unknown pack flag `{value}`")),
            value => {
                if config.input.is_some() {
                    return Err("pack accepts one input BSP".to_string());
                }
                config.input = Some(PathBuf::from(value));
            }
        }
        cursor += 1;
    }
    Ok(config)
}

fn prepare_pack_filelist(config: &PackConfig) -> Result<PackFilelist, String> {
    if config.filelist.is_some() && (!config.includes.is_empty() || !config.asset_roots.is_empty())
    {
        return Err(
            "pack accepts either --filelist or --asset-root/--include generation, not both"
                .to_string(),
        );
    }
    if let Some(filelist) = &config.filelist {
        if !filelist.exists() {
            return Err(format!("filelist {} does not exist", filelist.display()));
        }
        return Ok(PackFilelist {
            filelist_path: filelist.clone(),
            assets: Vec::new(),
            missing_files: Vec::new(),
            warnings: Vec::new(),
        });
    }
    if config.includes.is_empty() {
        return Err("pack needs --filelist or at least one --include".to_string());
    }
    if config.asset_roots.is_empty() {
        return Err(
            "pack needs --asset-root when generating a file list from --include".to_string(),
        );
    }

    let mut assets = Vec::new();
    let mut missing_files = Vec::new();
    let mut warnings = Vec::new();
    let mut filelist_text = String::new();
    for include in &config.includes {
        let internal_path = normalize_source_asset_path(include)?;
        if !is_common_source_asset_path(&internal_path) {
            warnings.push(format!(
                "asset `{internal_path}` is outside common Source asset roots"
            ));
        }
        let external_path = config
            .asset_roots
            .iter()
            .map(|root| root.join(Path::new(&internal_path)))
            .find(|path| path.is_file());
        if let Some(external_path) = external_path {
            filelist_text.push_str(&internal_path);
            filelist_text.push('\n');
            filelist_text.push_str(&external_path.display().to_string());
            filelist_text.push('\n');
            assets.push(PackAssetReport {
                internal_path,
                external_path: Some(external_path.display().to_string()),
                exists: true,
            });
        } else {
            missing_files.push(internal_path.clone());
            assets.push(PackAssetReport {
                internal_path,
                external_path: None,
                exists: false,
            });
        }
    }

    let filelist_path = generated_pack_filelist_path();
    create_parent_dir(&filelist_path, "pack file list")?;
    fs::write(&filelist_path, filelist_text).map_err(|error| {
        format!(
            "failed to write generated BSP pack file list {}: {error}",
            filelist_path.display()
        )
    })?;

    Ok(PackFilelist {
        filelist_path,
        assets,
        missing_files,
        warnings,
    })
}

fn generated_pack_filelist_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    env::temp_dir().join(format!(
        "sourceweaver-{}-{nonce}-bspzip-filelist.txt",
        process::id()
    ))
}

fn normalize_source_asset_path(value: &str) -> Result<String, String> {
    let normalized = value.trim().replace('\\', "/");
    if normalized.is_empty() {
        return Err("empty asset include path".to_string());
    }
    if normalized.starts_with('/') || normalized.as_bytes().get(1) == Some(&b':') {
        return Err(format!("asset include `{value}` must be relative"));
    }
    let parts = normalized
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>();
    if parts.is_empty() || parts.contains(&"..") {
        return Err(format!("asset include `{value}` must not contain `..`"));
    }
    Ok(parts.join("/"))
}

fn is_common_source_asset_path(internal_path: &str) -> bool {
    let Some(root) = internal_path.split('/').next() else {
        return false;
    };
    matches!(
        root,
        "materials"
            | "models"
            | "sound"
            | "scripts"
            | "particles"
            | "resource"
            | "maps"
            | "cfg"
            | "media"
    )
}

fn run_bsp_packer(invocation: &BspPackInvocation, timeout: Duration) -> Result<Output, String> {
    let mut command = Command::new(&invocation.executable);
    command.args(&invocation.args);
    run_command_with_timeout(
        &mut command,
        &format!("BSP packer {}", invocation.executable.display()),
        timeout,
    )
}

fn probe_bsp_packer_version(tool: &Path) -> Option<String> {
    let mut command = Command::new(tool);
    command.arg("--version");
    match run_command_with_timeout(
        &mut command,
        "BSP packer version probe",
        Duration::from_secs(30),
    ) {
        Ok(output) if output.status.success() => {
            let text = trimmed_tool_output(&output);
            if text.is_empty() { None } else { Some(text) }
        }
        _ => None,
    }
}

fn count_bspzip_added_files(log: &str) -> Option<usize> {
    let count = log
        .lines()
        .filter(|line| {
            line.trim_start()
                .to_ascii_lowercase()
                .starts_with("adding file:")
        })
        .count();
    (count > 0).then_some(count)
}

fn empty_compile_log_snapshot() -> CompileLogSnapshot {
    CompileLogSnapshot {
        ok: false,
        errors: 0,
        warnings: 0,
        leak_detected: false,
        error_lines: Vec::new(),
        warning_lines: Vec::new(),
    }
}

fn finish_pack_report(config: &PackConfig, report: PackReport) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode BSP pack report: {error}"))?;
    if let Some(report_path) = &config.report {
        create_parent_dir(report_path, "report")?;
        fs::write(report_path, &json).map_err(|error| {
            format!(
                "failed to write BSP pack report {}: {error}",
                report_path.display()
            )
        })?;
    }
    if config.json {
        println!("{json}");
    } else {
        println!("bsp pack: {}", if report.ok { "ok" } else { "failed" });
        println!("tool: {}", report.tool);
        if let Some(version) = &report.tool_version {
            println!("tool version: {version}");
        }
        println!("input bsp: {}", report.input_bsp);
        println!("output bsp: {}", report.output_bsp);
        println!("filelist: {}", report.filelist_path);
        println!("exit code: {:?}", report.exit_code);
        println!("output bsp exists: {}", report.output_bsp_exists);
        println!("requested files: {}", report.requested_files.len());
        println!("missing files: {}", report.missing_files.len());
        if let Some(count) = report.packed_file_count {
            println!("packed file count: {count}");
        }
        println!("log errors: {}", report.log_summary.errors);
        println!("log warnings: {}", report.log_summary.warnings);
    }
    if !report.ok {
        return Err("BSP packing validation failed".to_string());
    }
    Ok(())
}

fn run_job_command(args: &[String]) -> Result<(), String> {
    let mut job_path: Option<PathBuf> = None;
    let mut report_override: Option<PathBuf> = None;
    let mut dry_run_override = false;

    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--job" | "--config" => {
                cursor += 1;
                let value = args.get(cursor).ok_or("--job needs a TOML path")?;
                job_path = Some(PathBuf::from(value));
            }
            "--report" => {
                cursor += 1;
                let value = args.get(cursor).ok_or("--report needs a JSON path")?;
                report_override = Some(PathBuf::from(value));
            }
            "--dry-run" => dry_run_override = true,
            "--help" | "-h" => {
                print_run_job_help();
                return Ok(());
            }
            value if value.starts_with('-') => return Err(format!("unknown run flag `{value}`")),
            value => {
                if job_path.is_some() {
                    return Err("run accepts one job TOML path".to_string());
                }
                job_path = Some(PathBuf::from(value));
            }
        }
        cursor += 1;
    }

    let job_path = job_path
        .ok_or("usage: sourceweaver run --job <job.toml> [--dry-run] [--report report.json]")?;
    let job_text = fs::read_to_string(&job_path)
        .map_err(|error| format!("failed to read {}: {error}", job_path.display()))?;
    let mut job: AutomationJob = toml::from_str(&job_text)
        .map_err(|error| format!("failed to parse {}: {error}", job_path.display()))?;
    if dry_run_override {
        job.dry_run = true;
    }
    if let Some(report_path) = report_override {
        job.report = Some(report_path);
    }

    let base_dir = job_path.parent().unwrap_or_else(|| Path::new("."));
    let report = execute_job(&job, base_dir)?;
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode JSON report: {error}"))?;

    if let Some(report_path) = &job.report {
        let report_path = resolve_job_path(base_dir, report_path);
        create_parent_dir(&report_path, "report")?;
        fs::write(&report_path, &json).map_err(|error| {
            format!("failed to write report {}: {error}", report_path.display())
        })?;
    }

    println!("{json}");
    Ok(())
}

fn campaign_run_command(args: &[String]) -> Result<(), String> {
    let mut plan_path: Option<PathBuf> = None;
    let mut report_override: Option<PathBuf> = None;
    let mut dry_run_override = false;

    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--plan" | "--campaign" | "--config" => {
                cursor += 1;
                let value = args.get(cursor).ok_or("--plan needs a TOML path")?;
                plan_path = Some(PathBuf::from(value));
            }
            "--report" => {
                cursor += 1;
                let value = args.get(cursor).ok_or("--report needs a JSON path")?;
                report_override = Some(PathBuf::from(value));
            }
            "--dry-run" => dry_run_override = true,
            "--help" | "-h" => {
                print_campaign_run_help();
                return Ok(());
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown campaign-run flag `{value}`"));
            }
            value => {
                if plan_path.is_some() {
                    return Err("campaign-run accepts one campaign plan TOML path".to_string());
                }
                plan_path = Some(PathBuf::from(value));
            }
        }
        cursor += 1;
    }

    let plan_path = plan_path.ok_or(
        "usage: sourceweaver campaign-run --plan <campaign.toml> [--dry-run] [--report summary.json]",
    )?;
    let plan_text = fs::read_to_string(&plan_path)
        .map_err(|error| format!("failed to read {}: {error}", plan_path.display()))?;
    let mut plan: CampaignPlan = toml::from_str(&plan_text)
        .map_err(|error| format!("failed to parse {}: {error}", plan_path.display()))?;
    if dry_run_override {
        plan.dry_run = true;
    }
    if let Some(report_path) = report_override {
        plan.report = Some(report_path);
    }

    let base_dir = plan_path.parent().unwrap_or_else(|| Path::new("."));
    let summary = execute_campaign_plan(&plan, base_dir)?;
    let json = serde_json::to_string_pretty(&summary)
        .map_err(|error| format!("failed to encode campaign JSON report: {error}"))?;

    if let Some(report_path) = &plan.report {
        let report_path = resolve_job_path(base_dir, report_path);
        create_parent_dir(&report_path, "campaign report")?;
        fs::write(&report_path, &json).map_err(|error| {
            format!(
                "failed to write campaign report {}: {error}",
                report_path.display()
            )
        })?;
    }

    println!("{json}");
    Ok(())
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignPlan {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    report: Option<PathBuf>,
    #[serde(default)]
    steps: Vec<CampaignPlanStep>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignPlanStep {
    name: String,
    base: PathBuf,
    #[serde(default)]
    inputs: Vec<PathBuf>,
    output: Option<PathBuf>,
    #[serde(default)]
    report: Option<PathBuf>,
    #[serde(default)]
    landmark: Option<String>,
    #[serde(default)]
    changelevel_policy: Option<String>,
    #[serde(default)]
    changelevel_scope: Option<String>,
    #[serde(default)]
    preserve_external_transition: Vec<ChangelevelPreserveRuleConfig>,
    #[serde(default)]
    delete_preset: Option<PathBuf>,
    #[serde(default)]
    delete: DeleteConfig,
}

#[derive(Debug, Clone, Serialize)]
struct CampaignPlanReport {
    name: Option<String>,
    dry_run: bool,
    step_count: usize,
    outputs_written: usize,
    summary_report: Option<String>,
    steps: Vec<CampaignPlanStepSummary>,
    step_reports: Vec<AutomationReport>,
}

#[derive(Debug, Clone, Serialize)]
struct CampaignPlanStepSummary {
    name: String,
    operation: String,
    base: String,
    inputs: Vec<String>,
    output: Option<String>,
    report: Option<String>,
    output_written: bool,
    integrity_errors: usize,
    integrity_warnings: usize,
    transition_count: usize,
    adjacency_edges: usize,
    changelevel_policy: String,
    changelevel_scope: String,
    changelevel_changed: usize,
    changelevel_preserved: usize,
}

fn execute_campaign_plan(
    plan: &CampaignPlan,
    base_dir: &Path,
) -> Result<CampaignPlanReport, String> {
    if plan.steps.is_empty() {
        return Err("campaign plan needs at least one [[steps]] entry".to_string());
    }

    let mut summaries = Vec::new();
    let mut step_reports = Vec::new();
    let mut outputs_written = 0;

    for step in &plan.steps {
        let mut job = AutomationJob {
            base: step.base.clone(),
            inputs: step.inputs.clone(),
            output: step.output.clone(),
            landmark: step.landmark.clone(),
            changelevel_policy: step.changelevel_policy.clone(),
            changelevel_scope: step.changelevel_scope.clone(),
            preserve_external_transition: step.preserve_external_transition.clone(),
            delete_preset: step.delete_preset.clone(),
            delete: step.delete.clone(),
            dry_run: plan.dry_run,
            report: step.report.clone(),
        };
        if plan.dry_run {
            job.dry_run = true;
        }
        let report = execute_job(&job, base_dir)?;
        if report.output_written {
            outputs_written += 1;
        }

        let step_report_path = job
            .report
            .as_ref()
            .map(|path| resolve_job_path(base_dir, path));
        if let Some(report_path) = &step_report_path {
            let json = serde_json::to_string_pretty(&report)
                .map_err(|error| format!("failed to encode step JSON report: {error}"))?;
            create_parent_dir(report_path, "step report")?;
            fs::write(report_path, json).map_err(|error| {
                format!(
                    "failed to write step report {}: {error}",
                    report_path.display()
                )
            })?;
        }

        summaries.push(CampaignPlanStepSummary {
            name: step.name.clone(),
            operation: report.operation.clone(),
            base: report.base.clone(),
            inputs: report.inputs.clone(),
            output: report.output.clone(),
            report: step_report_path.map(|path| path.display().to_string()),
            output_written: report.output_written,
            integrity_errors: report.integrity.errors,
            integrity_warnings: report.integrity.warnings,
            transition_count: report.transitions.len(),
            adjacency_edges: report.campaign_adjacency.edges.len(),
            changelevel_policy: report.changelevel.policy.clone(),
            changelevel_scope: report.changelevel.scope.clone(),
            changelevel_changed: report.changelevel.changed.len(),
            changelevel_preserved: report.changelevel.preserved.len(),
        });
        step_reports.push(report);
    }

    Ok(CampaignPlanReport {
        name: plan.name.clone(),
        dry_run: plan.dry_run,
        step_count: summaries.len(),
        outputs_written,
        summary_report: plan
            .report
            .as_ref()
            .map(|path| resolve_job_path(base_dir, path).display().to_string()),
        steps: summaries,
        step_reports,
    })
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AutomationJob {
    base: PathBuf,
    #[serde(default)]
    inputs: Vec<PathBuf>,
    output: Option<PathBuf>,
    #[serde(default)]
    landmark: Option<String>,
    #[serde(default)]
    changelevel_policy: Option<String>,
    #[serde(default)]
    changelevel_scope: Option<String>,
    #[serde(default)]
    preserve_external_transition: Vec<ChangelevelPreserveRuleConfig>,
    #[serde(default)]
    delete_preset: Option<PathBuf>,
    #[serde(default)]
    delete: DeleteConfig,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    report: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangelevelPreserveRuleConfig {
    #[serde(default)]
    map: Option<String>,
    #[serde(default)]
    landmark: Option<String>,
    #[serde(default)]
    targetname: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DeleteConfig {
    #[serde(default)]
    classnames: Vec<String>,
    #[serde(default)]
    targetnames: Vec<String>,
    #[serde(default)]
    roles: Vec<String>,
    #[serde(default)]
    all_entities: bool,
    #[serde(default)]
    brush_entity_mode: Option<String>,
    #[serde(default = "default_protect_critical_entities")]
    protect_critical_entities: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CustomDeletionPresetFile {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    delete: DeleteConfig,
}

#[derive(Debug, Clone, Serialize)]
struct AutomationReport {
    operation: String,
    dry_run: bool,
    output_written: bool,
    output: Option<String>,
    base: String,
    inputs: Vec<String>,
    landmark: Option<String>,
    deletion_preset: Option<String>,
    deletion: DeletionSnapshot,
    per_map: Vec<MapJobReport>,
    integrity: IntegritySnapshot,
    transitions: Vec<TransitionSnapshot>,
    campaign_order: CampaignOrderSnapshot,
    campaign_adjacency: CampaignAdjacencySnapshot,
    merge: Option<MergeSnapshot>,
    changelevel: ChangelevelPolicySnapshot,
    result_entity_types: BTreeMap<String, usize>,
    result_entity_records: usize,
}

#[derive(Debug, Clone, Serialize)]
struct IntegritySnapshot {
    errors: usize,
    warnings: usize,
    issues: Vec<IntegrityIssueSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
struct IntegrityIssueSnapshot {
    severity: String,
    map: String,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
struct DeletionSnapshot {
    classnames: Vec<String>,
    targetnames: Vec<String>,
    roles: Vec<String>,
    all_entities: bool,
    brush_entity_mode: String,
    protect_critical_entities: bool,
    removed_entities: usize,
    removed_world_solids: usize,
    removed_brush_entity_solids: usize,
}

#[derive(Debug, Clone, Serialize)]
struct MapJobReport {
    path: String,
    role: String,
    integrity_errors: usize,
    integrity_warnings: usize,
    entity_records_before: usize,
    entity_records_after: usize,
    entity_types_before: BTreeMap<String, usize>,
    entity_types_after: BTreeMap<String, usize>,
    transitions: Vec<TransitionSnapshot>,
    removed_entities: usize,
    removed_world_solids: usize,
    removed_brush_entity_solids: usize,
}

#[derive(Debug, Clone, Serialize)]
struct TransitionSnapshot {
    map: String,
    role: String,
    entity_index: usize,
    targetname: Option<String>,
    target_map: Option<String>,
    landmark: Option<String>,
    origin: Option<String>,
    solid_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ChangelevelPolicySnapshot {
    policy: String,
    scope: String,
    changed: Vec<ChangelevelChangeSnapshot>,
    preserved: Vec<ChangelevelPreservedSnapshot>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ChangelevelChangeSnapshot {
    entity_index: usize,
    targetname: Option<String>,
    action: String,
    old_map: Option<String>,
    new_map: Option<String>,
    landmark: Option<String>,
    rationale: String,
}

#[derive(Debug, Clone, Serialize)]
struct ChangelevelPreservedSnapshot {
    entity_index: usize,
    targetname: Option<String>,
    map: Option<String>,
    landmark: Option<String>,
    reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct CampaignOrderSnapshot {
    ordered_labels: Vec<String>,
    landmark_pairs: Vec<CampaignLandmarkPairSnapshot>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CampaignAdjacencySnapshot {
    edges: Vec<CampaignAdjacencyEdgeSnapshot>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CampaignAdjacencyEdgeSnapshot {
    from_map: String,
    to_map: String,
    evidence_kind: String,
    confidence: String,
    evidence: String,
}

#[derive(Debug, Clone, Serialize)]
struct CampaignLandmarkPairSnapshot {
    from_map: String,
    to_map: String,
    target_map: String,
    landmark: String,
    target_has_landmark: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ValidationSnapshot {
    ok: bool,
    map: String,
    integrity: IntegritySnapshot,
    entity_semantics: EntitySemanticsSnapshot,
    complexity: ComplexitySnapshot,
    rule_set: Option<RuleSetValidationSnapshot>,
    vbsp_exit_code: Option<i32>,
    compile_log: Option<CompileLogSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
struct EntitySemanticsSnapshot {
    errors: usize,
    warnings: usize,
    issues: Vec<EntitySemanticsIssueSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
struct EntitySemanticsIssueSnapshot {
    severity: String,
    map: String,
    category: String,
    rule_id: String,
    message: String,
    targetname: Option<String>,
    entity_index: Option<usize>,
    classname: Option<String>,
    key: Option<String>,
    value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ComplexitySnapshot {
    entities: usize,
    point_entities: usize,
    brush_entities: usize,
    brush_solids: usize,
    sides: usize,
    displacements: usize,
    overlays: usize,
    warnings: usize,
    risks: Vec<ComplexityRiskSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
struct ComplexityRiskSnapshot {
    severity: String,
    metric: String,
    count: usize,
    warn_at: usize,
    limit: usize,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
struct RuleSetValidationSnapshot {
    id: String,
    name: String,
    scope: String,
    errors: usize,
    warnings: usize,
    issues: Vec<RuleSetIssueSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
struct RuleSetIssueSnapshot {
    severity: String,
    map: String,
    rule_id: String,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
struct CompileLogSnapshot {
    ok: bool,
    errors: usize,
    warnings: usize,
    leak_detected: bool,
    error_lines: Vec<String>,
    warning_lines: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct CompileProfile {
    tools: Option<CompileProfileTools>,
    compile: Option<CompileProfileSettings>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct CompileProfileTools {
    vbsp: Option<PathBuf>,
    vvis: Option<PathBuf>,
    vrad: Option<PathBuf>,
    game: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct CompileProfileSettings {
    steps: Option<Vec<String>>,
    log_dir: Option<PathBuf>,
    timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Default)]
struct CompileProfileCreateConfig {
    output: Option<PathBuf>,
    vbsp: Option<PathBuf>,
    vvis: Option<PathBuf>,
    vrad: Option<PathBuf>,
    game: Option<PathBuf>,
    steps: Option<Vec<String>>,
    log_dir: Option<PathBuf>,
    timeout_seconds: Option<u64>,
    validate: bool,
    json: bool,
}

#[derive(Debug, Clone, Default)]
struct CompileProfileValidateConfig {
    profile: Option<PathBuf>,
    json: bool,
}

#[derive(Debug, Clone, Default)]
struct CompileProfileDiscoverConfig {
    search_dirs: Vec<PathBuf>,
    output: Option<PathBuf>,
    game: Option<PathBuf>,
    steps: Option<Vec<String>>,
    log_dir: Option<PathBuf>,
    timeout_seconds: Option<u64>,
    json: bool,
}

#[derive(Debug, Clone, Serialize)]
struct CompileProfileCreateReport {
    ok: bool,
    output: Option<String>,
    profile_toml: Option<String>,
    validation: Option<CompileProfileValidationReport>,
}

#[derive(Debug, Clone, Serialize)]
struct CompileProfileValidationReport {
    ok: bool,
    profile: String,
    steps: Vec<String>,
    tools: Vec<CompileProfileToolCheck>,
    game: Option<String>,
    log_dir: Option<String>,
    timeout_seconds: Option<u64>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CompileProfileToolCheck {
    step: String,
    path: String,
    exists: bool,
    is_file: bool,
    executable: bool,
    command_shape: String,
}

#[derive(Debug, Clone, Serialize)]
struct CompileToolDiscoveryReport {
    ok: bool,
    search_dirs: Vec<String>,
    tools: Vec<CompileToolDiscoveryCheck>,
    game: Option<String>,
    log_dir: Option<String>,
    timeout_seconds: Option<u64>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CompileToolDiscoveryCheck {
    step: String,
    selected: Option<String>,
    candidates: Vec<String>,
    command_shape: String,
}

#[derive(Debug, Clone, Default)]
struct CompileConfig {
    input: Option<PathBuf>,
    profile: Option<PathBuf>,
    vbsp: Option<PathBuf>,
    vvis: Option<PathBuf>,
    vrad: Option<PathBuf>,
    game: Option<PathBuf>,
    steps: Option<Vec<String>>,
    log_dir: Option<PathBuf>,
    report: Option<PathBuf>,
    timeout_seconds: Option<u64>,
    json: bool,
}

#[derive(Debug, Clone, Serialize)]
struct CompilePipelineReport {
    ok: bool,
    map: String,
    game: Option<String>,
    log_dir: Option<String>,
    integrity: IntegritySnapshot,
    steps: Vec<CompileStepReport>,
}

#[derive(Debug, Clone, Serialize)]
struct CompileStepReport {
    step: String,
    tool: String,
    input: String,
    exit_code: Option<i32>,
    ok: bool,
    log_path: Option<String>,
    compile_log: CompileLogSnapshot,
}

#[derive(Debug, Clone, Serialize)]
struct ModelInspectReport {
    ok: bool,
    path: String,
    file_size: usize,
    header: Option<MdlHeaderSnapshot>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct MdlHeaderSnapshot {
    magic: String,
    version: i32,
    checksum: i32,
    name: String,
    data_length: i32,
    supported_magic: bool,
}

#[derive(Debug, Clone, Default)]
struct ModelCompileConfig {
    input_qc: Option<PathBuf>,
    studiomdl: Option<PathBuf>,
    game: Option<PathBuf>,
    tool_args: Vec<String>,
    log: Option<PathBuf>,
    report: Option<PathBuf>,
    timeout_seconds: Option<u64>,
    json: bool,
}

#[derive(Debug, Clone)]
struct ModelCompileInvocation {
    executable: PathBuf,
    args: Vec<String>,
    command_shape: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct ModelCompileReport {
    ok: bool,
    tool: String,
    command_shape: String,
    command_args: Vec<String>,
    input_qc: String,
    game: Option<String>,
    exit_code: Option<i32>,
    log_path: Option<String>,
    log_summary: CompileLogSnapshot,
}

#[derive(Debug, Clone, Default)]
struct BspImportConfig {
    input: Option<PathBuf>,
    tool: Option<PathBuf>,
    bspsource: Option<PathBuf>,
    bspsource_jar: Option<PathBuf>,
    java: Option<PathBuf>,
    output: Option<PathBuf>,
    log: Option<PathBuf>,
    report: Option<PathBuf>,
    tool_args: Vec<String>,
    timeout_seconds: Option<u64>,
    json: bool,
}

#[derive(Debug, Clone)]
struct BspDecompilerInvocation {
    kind: &'static str,
    executable: PathBuf,
    args: Vec<String>,
    command_shape: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct BspImportReport {
    ok: bool,
    tool: String,
    tool_kind: String,
    tool_version: Option<String>,
    command_args: Vec<String>,
    command_shape: String,
    input_bsp: String,
    output_vmf: String,
    exit_code: Option<i32>,
    log_path: Option<String>,
    generated_vmf_exists: bool,
    integrity: Option<IntegritySnapshot>,
    entity_count: Option<usize>,
    classname_count: Option<usize>,
    log_summary: CompileLogSnapshot,
}

#[derive(Debug, Clone, Default)]
struct PackConfig {
    input: Option<PathBuf>,
    tool: Option<PathBuf>,
    output: Option<PathBuf>,
    asset_roots: Vec<PathBuf>,
    includes: Vec<String>,
    filelist: Option<PathBuf>,
    log: Option<PathBuf>,
    report: Option<PathBuf>,
    timeout_seconds: Option<u64>,
    json: bool,
}

#[derive(Debug, Clone)]
struct PackFilelist {
    filelist_path: PathBuf,
    assets: Vec<PackAssetReport>,
    missing_files: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct BspPackInvocation {
    executable: PathBuf,
    args: Vec<String>,
    command_shape: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct PackReport {
    ok: bool,
    tool: String,
    tool_kind: String,
    tool_version: Option<String>,
    command_shape: String,
    command_args: Vec<String>,
    input_bsp: String,
    output_bsp: String,
    output_bsp_exists: bool,
    filelist_path: String,
    asset_roots: Vec<String>,
    requested_files: Vec<PackAssetReport>,
    missing_files: Vec<String>,
    warnings: Vec<String>,
    exit_code: Option<i32>,
    log_path: Option<String>,
    packed_file_count: Option<usize>,
    log_summary: CompileLogSnapshot,
}

#[derive(Debug, Clone, Serialize)]
struct PackAssetReport {
    internal_path: String,
    external_path: Option<String>,
    exists: bool,
}

impl ValidationSnapshot {
    fn from_report(report: &VmfToolValidationReport, vbsp_exit_code: Option<i32>) -> Self {
        Self {
            ok: report.is_ok() && vbsp_exit_code.map(|code| code == 0).unwrap_or(true),
            map: report.map_label.clone(),
            integrity: snapshot_integrity_report(&report.integrity),
            entity_semantics: snapshot_entity_semantics_report(&report.entity_semantics),
            complexity: snapshot_complexity_report(&report.complexity),
            rule_set: report.rule_set.as_ref().map(snapshot_rule_set_report),
            vbsp_exit_code,
            compile_log: report.compile_log.as_ref().map(|log| CompileLogSnapshot {
                ok: log.is_ok(),
                errors: log.errors.len(),
                warnings: log.warnings.len(),
                leak_detected: log.leak_detected,
                error_lines: log.errors.clone(),
                warning_lines: log.warnings.clone(),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct MergeSnapshot {
    merged_maps: usize,
    appended_world_solids: usize,
    appended_entities: usize,
    applied_offsets: Vec<OffsetSnapshot>,
    changelevel: ChangelevelPolicySnapshot,
}

#[derive(Debug, Clone, Serialize)]
struct OffsetSnapshot {
    map: String,
    offset: String,
}

fn execute_job(job: &AutomationJob, base_dir: &Path) -> Result<AutomationReport, String> {
    let output_path = job
        .output
        .as_ref()
        .map(|path| resolve_job_path(base_dir, path));

    if output_path.is_none() && !job.dry_run {
        return Err("job needs `output` unless dry_run is true".to_string());
    }

    let (criteria, delete_preset_path) = criteria_from_job_delete_config(job, base_dir)?;
    let base_path = resolve_job_path(base_dir, &job.base);
    let mut map_paths = vec![base_path.clone()];
    map_paths.extend(
        job.inputs
            .iter()
            .map(|path| resolve_job_path(base_dir, path)),
    );

    let mut per_map = Vec::new();
    let mut prepared_documents = Vec::new();
    let mut removed_total = DeletionReport::default();
    let mut integrity_report = IntegrityReport::default();
    let mut transition_reports = Vec::new();
    let mut campaign_inputs = Vec::new();

    for (index, path) in map_paths.iter().enumerate() {
        let role = if index == 0 { "base" } else { "input" };
        let mut document = load_document(path)?;
        let label = path.display().to_string();
        let map_integrity = validate_document_integrity(&document, &label);
        for issue in map_integrity.warnings() {
            eprintln!("{}", format_integrity_issue(issue));
        }
        if let Some(message) = map_integrity.error_message() {
            return Err(message);
        }
        let integrity_errors = map_integrity.error_count();
        let integrity_warnings = map_integrity.warning_count();
        integrity_report.extend(map_integrity);

        let before_records = inspect_entities(&document).len();
        let before_types = summarize_entity_types(&document);
        let transitions = discover_transitions(&document);
        campaign_inputs.push(CampaignMapInput {
            label: label.clone(),
            transitions: transitions.clone(),
            landmarks: discover_landmarks(&document),
        });
        let map_transitions = transitions
            .iter()
            .map(|transition| snapshot_transition(&label, role, transition))
            .collect::<Vec<_>>();
        transition_reports.extend(map_transitions.iter().cloned());
        let deletion_report = prune_document(&mut document, &criteria);
        removed_total.removed_entities += deletion_report.removed_entities;
        removed_total.removed_world_solids += deletion_report.removed_world_solids;
        removed_total.removed_brush_entity_solids += deletion_report.removed_brush_entity_solids;
        let after_records = inspect_entities(&document).len();
        let after_types = summarize_entity_types(&document);

        per_map.push(MapJobReport {
            path: path.display().to_string(),
            role: role.to_string(),
            integrity_errors,
            integrity_warnings,
            entity_records_before: before_records,
            entity_records_after: after_records,
            entity_types_before: before_types,
            entity_types_after: after_types,
            transitions: map_transitions,
            removed_entities: deletion_report.removed_entities,
            removed_world_solids: deletion_report.removed_world_solids,
            removed_brush_entity_solids: deletion_report.removed_brush_entity_solids,
        });

        prepared_documents.push((label, document));
    }

    let (result_document, merge_snapshot, operation) = if prepared_documents.len() == 1 {
        let (_, document) = prepared_documents
            .into_iter()
            .next()
            .expect("one prepared document exists");
        (document, None, "clean".to_string())
    } else {
        let merge_inputs = prepared_documents
            .into_iter()
            .map(|(label, document)| MergeInput { label, document })
            .collect::<Vec<_>>();
        let changelevel_policy = selected_job_changelevel_policy(job)?;
        let changelevel_scope = selected_job_changelevel_scope(job)?;
        let preserve_external = job_preserve_external_rules(job)?;
        let output_map = output_path.as_ref().and_then(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
        });
        let stitched_maps = map_paths
            .iter()
            .filter_map(|path| {
                path.file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
            })
            .collect::<Vec<_>>();
        let (document, merge_report) = merge_maps(
            merge_inputs,
            &MergeOptions {
                landmark: job
                    .landmark
                    .clone()
                    .filter(|value| !value.trim().is_empty()),
                changelevel: ChangelevelPolicyOptions {
                    policy: changelevel_policy,
                    scope: changelevel_scope,
                    output_map,
                    stitched_maps,
                    preserve_external,
                },
            },
        )?;
        (
            document,
            Some(snapshot_merge_report(merge_report)),
            "merge".to_string(),
        )
    };

    let result_entity_types = summarize_entity_types(&result_document);
    let result_entity_records = inspect_entities(&result_document).len();
    let result_integrity = validate_document_integrity(&result_document, "result");
    for issue in result_integrity.warnings() {
        eprintln!("{}", format_integrity_issue(issue));
    }
    if let Some(message) = result_integrity.error_message() {
        return Err(message);
    }
    integrity_report.extend(result_integrity);

    let output_written = if job.dry_run {
        false
    } else {
        let output_path = output_path
            .as_ref()
            .expect("output path checked for non-dry-run jobs");
        write_document(output_path, &result_document)?;
        true
    };

    let campaign_order = suggest_campaign_order(&campaign_inputs);
    let campaign_adjacency = sourceweaver_core::build_campaign_adjacency_graph(&campaign_inputs);

    Ok(AutomationReport {
        operation,
        dry_run: job.dry_run,
        output_written,
        output: output_path.as_ref().map(|path| path.display().to_string()),
        base: base_path.display().to_string(),
        inputs: job
            .inputs
            .iter()
            .map(|path| resolve_job_path(base_dir, path).display().to_string())
            .collect(),
        landmark: job
            .landmark
            .clone()
            .filter(|value| !value.trim().is_empty()),
        deletion_preset: delete_preset_path
            .as_ref()
            .map(|path| path.display().to_string()),
        deletion: DeletionSnapshot {
            classnames: sorted_strings(criteria.classnames.iter()),
            targetnames: sorted_strings(criteria.targetnames.iter()),
            roles: criteria
                .brush_roles
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            all_entities: criteria.drop_all_entities,
            brush_entity_mode: criteria.brush_entity_mode.to_string(),
            protect_critical_entities: criteria.protect_critical_entities,
            removed_entities: removed_total.removed_entities,
            removed_world_solids: removed_total.removed_world_solids,
            removed_brush_entity_solids: removed_total.removed_brush_entity_solids,
        },
        per_map,
        integrity: snapshot_integrity_report(&integrity_report),
        transitions: transition_reports,
        campaign_order: snapshot_campaign_order(&campaign_order),
        campaign_adjacency: snapshot_campaign_adjacency(&campaign_adjacency),
        changelevel: merge_snapshot
            .as_ref()
            .map(|merge| merge.changelevel.clone())
            .unwrap_or_else(|| ChangelevelPolicySnapshot {
                policy: "preserve".to_string(),
                scope: "all".to_string(),
                changed: Vec::new(),
                preserved: Vec::new(),
                warnings: Vec::new(),
            }),
        merge: merge_snapshot,
        result_entity_types,
        result_entity_records,
    })
}

fn criteria_from_job_delete_config(
    job: &AutomationJob,
    base_dir: &Path,
) -> Result<(DeletionCriteria, Option<PathBuf>), String> {
    let Some(preset_path) = &job.delete_preset else {
        return Ok((criteria_from_delete_config(&job.delete)?, None));
    };
    let resolved = resolve_job_path(base_dir, preset_path);
    let text = fs::read_to_string(&resolved).map_err(|error| {
        format!(
            "failed to read deletion preset {}: {error}",
            resolved.display()
        )
    })?;
    let preset: CustomDeletionPresetFile = toml::from_str(&text).map_err(|error| {
        format!(
            "failed to parse deletion preset {}: {error}",
            resolved.display()
        )
    })?;
    let mut criteria = criteria_from_delete_config(&preset.delete)?;
    let extra = criteria_from_delete_config(&job.delete)?;
    criteria.classnames.extend(extra.classnames);
    criteria.targetnames.extend(extra.targetnames);
    criteria.brush_roles.extend(extra.brush_roles);
    criteria.drop_all_entities |= extra.drop_all_entities;
    criteria.protect_critical_entities = extra.protect_critical_entities;
    criteria.brush_entity_mode = extra.brush_entity_mode;
    Ok((criteria, Some(resolved)))
}

fn criteria_from_delete_config(delete: &DeleteConfig) -> Result<DeletionCriteria, String> {
    let mut criteria = DeletionCriteria {
        drop_all_entities: delete.all_entities,
        protect_critical_entities: delete.protect_critical_entities,
        ..DeletionCriteria::default()
    };
    if let Some(mode) = &delete.brush_entity_mode {
        criteria.brush_entity_mode = BrushEntityDeletionMode::parse(mode)
            .ok_or_else(|| format!("unknown delete.brush_entity_mode `{mode}`"))?;
    }
    criteria.classnames.extend(
        delete
            .classnames
            .iter()
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned),
    );
    criteria.targetnames.extend(
        delete
            .targetnames
            .iter()
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned),
    );
    for role in &delete.roles {
        let role = BrushRole::parse_filter(role.trim())
            .ok_or_else(|| format!("unknown brush role `{role}` in job delete.roles"))?;
        criteria.brush_roles.insert(role);
    }
    Ok(criteria)
}

fn default_protect_critical_entities() -> bool {
    true
}

fn snapshot_merge_report(report: MergeReport) -> MergeSnapshot {
    MergeSnapshot {
        merged_maps: report.merged_maps,
        appended_world_solids: report.appended_world_solids,
        appended_entities: report.appended_entities,
        applied_offsets: report
            .applied_offsets
            .into_iter()
            .map(|(map, offset)| OffsetSnapshot {
                map,
                offset: offset.to_string(),
            })
            .collect(),
        changelevel: snapshot_changelevel_policy_report(&report.changelevel),
    }
}

fn snapshot_changelevel_policy_report(
    report: &ChangelevelPolicyReport,
) -> ChangelevelPolicySnapshot {
    ChangelevelPolicySnapshot {
        policy: report.policy.to_string(),
        scope: report.scope.to_string(),
        changed: report
            .changed
            .iter()
            .map(snapshot_changelevel_change)
            .collect(),
        preserved: report
            .preserved
            .iter()
            .map(snapshot_changelevel_preserved)
            .collect(),
        warnings: report.warnings.clone(),
    }
}

fn snapshot_changelevel_preserved(
    preserved: &ChangelevelPreservedTransition,
) -> ChangelevelPreservedSnapshot {
    ChangelevelPreservedSnapshot {
        entity_index: preserved.entity_index,
        targetname: preserved.targetname.clone(),
        map: preserved.map.clone(),
        landmark: preserved.landmark.clone(),
        reason: preserved.reason.clone(),
    }
}

fn snapshot_changelevel_change(change: &ChangelevelChange) -> ChangelevelChangeSnapshot {
    ChangelevelChangeSnapshot {
        entity_index: change.entity_index,
        targetname: change.targetname.clone(),
        action: change.action.clone(),
        old_map: change.old_map.clone(),
        new_map: change.new_map.clone(),
        landmark: change.landmark.clone(),
        rationale: change.rationale.clone(),
    }
}

fn snapshot_integrity_report(report: &IntegrityReport) -> IntegritySnapshot {
    IntegritySnapshot {
        errors: report.error_count(),
        warnings: report.warning_count(),
        issues: report
            .issues
            .iter()
            .map(|issue| IntegrityIssueSnapshot {
                severity: issue.severity.to_string(),
                map: issue.label.clone(),
                message: issue.message.clone(),
            })
            .collect(),
    }
}

fn snapshot_entity_semantics_report(report: &EntitySemanticsReport) -> EntitySemanticsSnapshot {
    EntitySemanticsSnapshot {
        errors: report.error_count(),
        warnings: report.warning_count(),
        issues: report
            .issues
            .iter()
            .map(|issue| EntitySemanticsIssueSnapshot {
                severity: issue.severity.to_string(),
                map: issue.label.clone(),
                category: issue.category.clone(),
                rule_id: issue.rule_id.clone(),
                message: issue.message.clone(),
                targetname: issue.targetname.clone(),
                entity_index: issue.entity_index,
                classname: issue.classname.clone(),
                key: issue.key.clone(),
                value: issue.value.clone(),
            })
            .collect(),
    }
}

fn snapshot_complexity_report(report: &MapComplexityReport) -> ComplexitySnapshot {
    ComplexitySnapshot {
        entities: report.entity_count,
        point_entities: report.point_entity_count,
        brush_entities: report.brush_entity_count,
        brush_solids: report.brush_solid_count,
        sides: report.side_count,
        displacements: report.displacement_count,
        overlays: report.overlay_count,
        warnings: report.warning_count(),
        risks: report
            .risks
            .iter()
            .map(|risk| ComplexityRiskSnapshot {
                severity: risk.severity.to_string(),
                metric: risk.metric.to_string(),
                count: risk.count,
                warn_at: risk.warn_at,
                limit: risk.limit,
                message: risk.message.clone(),
            })
            .collect(),
    }
}

fn snapshot_rule_set_report(report: &RuleSetValidationReport) -> RuleSetValidationSnapshot {
    RuleSetValidationSnapshot {
        id: report.rule_set.id.to_string(),
        name: report.rule_set.name.to_string(),
        scope: report.rule_set.scope.to_string(),
        errors: report.error_count(),
        warnings: report.warning_count(),
        issues: report
            .issues
            .iter()
            .map(|issue| RuleSetIssueSnapshot {
                severity: issue.severity.to_string(),
                map: issue.label.clone(),
                rule_id: issue.rule_id.clone(),
                message: issue.message.clone(),
            })
            .collect(),
    }
}

fn snapshot_transition(
    map: &str,
    role: &str,
    transition: &CampaignTransition,
) -> TransitionSnapshot {
    TransitionSnapshot {
        map: map.to_string(),
        role: role.to_string(),
        entity_index: transition.entity_index,
        targetname: transition.targetname.clone(),
        target_map: transition.target_map.clone(),
        landmark: transition.landmark.clone(),
        origin: transition.origin.map(|origin| origin.to_string()),
        solid_count: transition.solid_count,
    }
}

fn snapshot_campaign_adjacency(graph: &CampaignAdjacencyGraph) -> CampaignAdjacencySnapshot {
    CampaignAdjacencySnapshot {
        edges: graph
            .edges
            .iter()
            .map(|edge| CampaignAdjacencyEdgeSnapshot {
                from_map: edge.from_map.clone(),
                to_map: edge.to_map.clone(),
                evidence_kind: edge.evidence_kind.clone(),
                confidence: edge.confidence.clone(),
                evidence: edge.evidence.clone(),
            })
            .collect(),
        warnings: graph.warnings.clone(),
    }
}

fn snapshot_campaign_order(suggestion: &CampaignOrderSuggestion) -> CampaignOrderSnapshot {
    CampaignOrderSnapshot {
        ordered_labels: suggestion.ordered_labels.clone(),
        landmark_pairs: suggestion
            .landmark_pairs
            .iter()
            .map(|pair| CampaignLandmarkPairSnapshot {
                from_map: pair.from_map.clone(),
                to_map: pair.to_map.clone(),
                target_map: pair.target_map.clone(),
                landmark: pair.landmark.clone(),
                target_has_landmark: pair.target_has_landmark,
            })
            .collect(),
        warnings: suggestion.warnings.clone(),
    }
}

fn print_validation_snapshot(snapshot: &ValidationSnapshot) {
    println!("validation: {}", if snapshot.ok { "ok" } else { "failed" });
    println!("map: {}", snapshot.map);
    println!("integrity errors: {}", snapshot.integrity.errors);
    println!("integrity warnings: {}", snapshot.integrity.warnings);
    for issue in &snapshot.integrity.issues {
        println!(
            "integrity\t{}\t{}\t{}",
            issue.severity, issue.map, issue.message
        );
    }
    println!(
        "entity semantics errors: {}",
        snapshot.entity_semantics.errors
    );
    println!(
        "entity semantics warnings: {}",
        snapshot.entity_semantics.warnings
    );
    for issue in &snapshot.entity_semantics.issues {
        println!(
            "entity-semantics\t{}\t{}\t{}\t{}",
            issue.severity, issue.map, issue.category, issue.message
        );
    }
    println!(
        "complexity: {} entities ({} point, {} brush), {} solids, {} sides, {} displacements, {} overlays",
        snapshot.complexity.entities,
        snapshot.complexity.point_entities,
        snapshot.complexity.brush_entities,
        snapshot.complexity.brush_solids,
        snapshot.complexity.sides,
        snapshot.complexity.displacements,
        snapshot.complexity.overlays
    );
    println!("complexity warnings: {}", snapshot.complexity.warnings);
    for risk in &snapshot.complexity.risks {
        println!(
            "complexity	{}	{}	{}	{}	{}",
            risk.severity, risk.metric, risk.count, risk.warn_at, risk.limit
        );
    }
    if let Some(rule_set) = &snapshot.rule_set {
        println!("rule set: {} ({})", rule_set.id, rule_set.name);
        println!("rule-set errors: {}", rule_set.errors);
        println!("rule-set warnings: {}", rule_set.warnings);
        for issue in &rule_set.issues {
            println!(
                "rule-set\t{}\t{}\t{}\t{}",
                issue.severity, issue.map, issue.rule_id, issue.message
            );
        }
    } else {
        println!("rule set: none");
    }
    if let Some(code) = snapshot.vbsp_exit_code {
        println!("vbsp exit code: {code}");
    }
    if let Some(log) = &snapshot.compile_log {
        println!("compile log ok: {}", log.ok);
        println!("compile errors: {}", log.errors);
        println!("compile warnings: {}", log.warnings);
        println!("leak detected: {}", log.leak_detected);
        for line in &log.error_lines {
            println!("compile-error\t{line}");
        }
        for line in &log.warning_lines {
            println!("compile-warning\t{line}");
        }
    } else {
        println!("compile log: not provided");
    }
}

fn parse_changelevel_scope(value: &str) -> Result<ChangelevelScope, String> {
    ChangelevelScope::parse(value).ok_or_else(|| {
        format!(
            "unknown changelevel scope `{value}`. available scopes: {}",
            ChangelevelScope::choices()
        )
    })
}

fn parse_changelevel_policy(value: &str) -> Result<ChangelevelPolicy, String> {
    ChangelevelPolicy::parse(value).ok_or_else(|| {
        format!(
            "unknown changelevel policy `{value}`. available policies: {}",
            ChangelevelPolicy::choices()
        )
    })
}

fn selected_job_changelevel_policy(job: &AutomationJob) -> Result<ChangelevelPolicy, String> {
    job.changelevel_policy
        .as_deref()
        .map(parse_changelevel_policy)
        .unwrap_or(Ok(ChangelevelPolicy::Preserve))
}

fn selected_job_changelevel_scope(job: &AutomationJob) -> Result<ChangelevelScope, String> {
    job.changelevel_scope
        .as_deref()
        .map(parse_changelevel_scope)
        .unwrap_or(Ok(ChangelevelScope::All))
}

fn job_preserve_external_rules(
    job: &AutomationJob,
) -> Result<Vec<ChangelevelPreserveRule>, String> {
    let mut rules = Vec::new();
    for rule in &job.preserve_external_transition {
        let rule = ChangelevelPreserveRule {
            map: rule.map.clone().filter(|value| !value.trim().is_empty()),
            landmark: rule
                .landmark
                .clone()
                .filter(|value| !value.trim().is_empty()),
            targetname: rule
                .targetname
                .clone()
                .filter(|value| !value.trim().is_empty()),
        };
        if rule.is_empty() {
            return Err("preserve_external_transition entries need at least one of map, landmark, or targetname".to_string());
        }
        rules.push(rule);
    }
    Ok(rules)
}

fn selected_validation_rule_set(
    value: Option<&str>,
) -> Result<Option<&'static ValidationRuleSet>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.trim().eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    validation_rule_set_by_id(value).map(Some).ok_or_else(|| {
        format!(
            "unknown validation rule set `{value}`. available rule sets: {}",
            validation_rule_set_choices()
        )
    })
}

fn resolve_job_path(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn load_document(path: impl AsRef<Path>) -> Result<Document, String> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    Document::parse(&text).map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn write_document(path: impl AsRef<Path>, document: &Document) -> Result<(), String> {
    let path = path.as_ref();
    create_parent_dir(path, "output")?;
    fs::write(path, document.to_vmf_string())
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
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

fn split_csv(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
}

fn extend_csv(set: &mut BTreeSet<String>, value: &str) {
    set.extend(split_csv(value).map(ToOwned::to_owned));
}

fn add_roles(set: &mut BTreeSet<BrushRole>, value: &str) -> Result<(), String> {
    for item in split_csv(value) {
        let role =
            BrushRole::parse_filter(item).ok_or_else(|| format!("unknown brush role `{item}`"))?;
        set.insert(role);
    }
    Ok(())
}

fn sorted_strings<'a>(values: impl Iterator<Item = &'a String>) -> Vec<String> {
    let mut values = values.cloned().collect::<Vec<_>>();
    values.sort();
    values
}

fn format_roles(roles: &[BrushRole]) -> String {
    if roles.is_empty() {
        return "-".to_string();
    }
    roles
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn print_help() {
    println!(
        r#"Source Weaver

Automatically inspect, prune, and merge Source VMF campaign maps.

Usage:
  sourceweaver inspect <map.vmf>
  sourceweaver list-types <map.vmf>
  sourceweaver prune <map.vmf> -o <out.vmf> [--drop-classname name] [--drop-targetname name] [--drop-role role] [--drop-all-entities] [--brush-entity-mode whole-entity|matching-solids] [--allow-critical-deletion]
  sourceweaver merge -o <out.vmf> [--landmark targetname] [--changelevel-policy preserve|disable|delete|rewrite-internal] [--changelevel-scope all|internal-only] [--preserve-external-map map] [--preserve-external-landmark name] [--preserve-external-targetname name] <base.vmf> <add.vmf> [...]
  sourceweaver validate <map.vmf> [--compile-log log.txt] [--rule-set none|hl2] [--vbsp path] [--game game-dir] [--capture-log log.txt] [--timeout-seconds seconds] [--json]
  sourceweaver compile <map.vmf> [--profile profile.toml] [--vbsp path] [--vvis path] [--vrad path] [--game game-dir] [--steps vbsp,vvis,vrad] [--log-dir dir] [--timeout-seconds seconds] [--report report.json] [--json]
  sourceweaver compile-profile create|validate|discover [options]
  sourceweaver model-inspect <model.mdl> [--json]
  sourceweaver model-compile <model.qc> --studiomdl <path> [--game game-dir] [--tool-arg arg] [--log log.txt] [--timeout-seconds seconds] [--report report.json] [--json]
  sourceweaver bsp-import <map.bsp> (--bspsource <bspsrc> | --bspsource-jar <bspsrc.jar> | --tool <wrapper>) --output <out.vmf> [--java java] [--tool-arg arg] [--log log.txt] [--timeout-seconds seconds] [--report report.json] [--json]
  sourceweaver pack <map.bsp> --tool <bspzip> --output <out.bsp> (--filelist list.txt | --asset-root dir --include path) [--log log.txt] [--timeout-seconds seconds] [--report report.json] [--json]
  sourceweaver run --job <job.toml> [--dry-run] [--report report.json]
  sourceweaver job-template

Deletion roles:
  trigger, clip, areaportal, skybox, occluder, hint, skip, nodraw, water, world-brush, brush-entity

Deletion safety:
  Job files can set delete.brush_entity_mode to "whole-entity" or "matching-solids".
  Critical entities are protected by default unless delete.protect_critical_entities is false.

Automation:
  Use `sourceweaver job-template > sourceweaver-job.toml`, edit paths/rules, then run
  `sourceweaver run --job sourceweaver-job.toml`. The run command is fully non-interactive
  and prints a JSON report, making it suitable for ChatGPT/Hermes-driven workflows.

Compiler validation:
  `sourceweaver validate` performs portable VMF readiness checks on Linux and can parse
  captured VBSP logs. When Source tooling is available, pass --vbsp and optional --game.

Merge behavior:
  - keeps the first VMF as the base map
  - appends world solids from each additional VMF, including skybox solids
  - appends all point and brush entities from each additional VMF
  - renumbers incoming VMF id keys to avoid collisions
  - when --landmark is supplied, aligns matching info_landmark targetnames to the base map
"#
    );
}

fn print_validate_help() {
    println!(
        r#"Usage:
  sourceweaver validate <map.vmf> [--compile-log log.txt] [--vbsp path] [--game game-dir] [--capture-log log.txt] [--timeout-seconds seconds] [--json]

Validates a VMF for Source tool readiness.

Linux-friendly workflow:
  sourceweaver validate merged.vmf --json
  sourceweaver validate merged.vmf --rule-set hl2 --json
  sourceweaver validate merged.vmf --compile-log captured-vbsp.log --json

Source SDK workflow when VBSP is installed:
  sourceweaver validate merged.vmf --vbsp /path/to/vbsp --game /path/to/game --capture-log vbsp.log --json

Rule sets are portable checks. Available rule sets: none, hl2.
Changelevel merge policies are portable VMF edits: preserve, disable, delete, rewrite-internal. Cleanup scope can be all or internal-only; external transitions can be preserved by map, landmark, or targetname.
External tool runs default to a 900 second timeout. Override with --timeout-seconds.
"#
    );
}

fn print_compile_help() {
    println!(
        r#"Usage:
  sourceweaver compile <map.vmf> [--profile profile.toml] [--vbsp path] [--vvis path] [--vrad path] [--game game-dir] [--steps vbsp,vvis,vrad] [--log-dir dir] [--timeout-seconds seconds] [--report report.json] [--json]

Runs an optional Source compile pipeline using user-provided tool paths.

Examples:
  sourceweaver compile stitched.vmf --vbsp /path/to/vbsp --log-dir target/compile-logs --json
  sourceweaver compile stitched.vmf --profile hl2-tools.toml --steps vbsp,vvis,vrad --report compile-report.json

Compile profile TOML:
  [tools]
  vbsp = "/path/to/vbsp"
  vvis = "/path/to/vvis"
  vrad = "/path/to/vrad"
  game = "/path/to/game-dir"

  [compile]
  steps = ["vbsp", "vvis", "vrad"]
  log_dir = "target/sourceweaver-compile-logs"
  timeout_seconds = 900

Each step captures stdout/stderr, writes step logs when --log-dir is set,
parses warnings/errors/leaks, and reports JSON when --json or --report is used.
External tool runs default to a 900 second timeout. Override with --timeout-seconds.
"#
    );
}

fn print_compile_profile_help() {
    println!(
        r#"Usage:
  sourceweaver compile-profile create [--output profile.toml] [--vbsp path] [--vvis path] [--vrad path] [--game game-dir] [--steps vbsp,vvis,vrad] [--log-dir dir] [--timeout-seconds seconds] [--validate] [--json]
  sourceweaver compile-profile validate --profile profile.toml [--json]
  sourceweaver compile-profile discover [--search-dir dir] [--output profile.toml] [--game game-dir] [--steps vbsp,vvis,vrad] [--log-dir dir] [--timeout-seconds seconds] [--json]

Creates, validates, or discovers Source compile profiles for user-provided external tools.

Source Weaver does not ship VBSP, VVIS, VRAD, game SDKs, or proprietary assets.
Use this command to make profile TOML without hand-editing it and to get actionable
missing-tool/game-path reports before running `sourceweaver compile`.
"#
    );
}

fn print_compile_profile_create_help() {
    println!(
        r#"Usage:
  sourceweaver compile-profile create [--output profile.toml] [--vbsp path] [--vvis path] [--vrad path] [--game game-dir] [--steps vbsp,vvis,vrad] [--log-dir dir] [--timeout-seconds seconds] [--validate] [--json]

Writes a compile profile TOML from explicit paths. Omit --output to print TOML.
Use --validate to check the generated profile paths in the same run.
"#
    );
}

fn print_compile_profile_validate_help() {
    println!(
        r#"Usage:
  sourceweaver compile-profile validate --profile profile.toml [--json]

Checks that the profile's selected compile steps have configured tool paths, that tool
paths exist, that the game directory exists when provided, and that timeout values are valid.
"#
    );
}

fn print_compile_profile_discover_help() {
    println!(
        r#"Usage:
  sourceweaver compile-profile discover [--search-dir dir] [--output profile.toml] [--game game-dir] [--steps vbsp,vvis,vrad] [--log-dir dir] [--timeout-seconds seconds] [--json]

Searches explicit --search-dir paths plus PATH for vbsp/vvis/vrad executables.
When --output is set, writes a profile using the first discovered candidate per step.
"#
    );
}

fn print_model_inspect_help() {
    println!(
        r#"Usage:
  sourceweaver model-inspect <model.mdl> [--json]

Reads a small Source/GoldSource MDL header prefix without decompiling assets.
This is a native metadata check only; it does not replace Crowbar or other model tools.
"#
    );
}

fn print_model_compile_help() {
    println!(
        r#"Usage:
  sourceweaver model-compile <model.qc> --studiomdl <path> [--game game-dir] [--tool-arg arg] [--log log.txt] [--timeout-seconds seconds] [--report report.json] [--json]

Runs a user-provided StudioMDL-compatible model compiler or wrapper.

Source Weaver does not bundle StudioMDL, Crowbar, game SDKs, models, or assets.
Command shape:
  studiomdl [--tool-arg values...] [-game <game-dir>] <model.qc>

Use --tool-arg once per additional StudioMDL option. External tool runs default to a 900 second timeout.
"#
    );
}

fn print_bsp_import_help() {
    println!(
        r#"Usage:
  sourceweaver bsp-import <map.bsp> (--bspsource <bspsrc> | --bspsource-jar <bspsrc.jar> | --tool <wrapper>) --output <out.vmf> [--java java] [--tool-arg arg] [--log log.txt] [--timeout-seconds seconds] [--report report.json] [--json]

Runs a user-provided BSP decompiler and validates the generated VMF.

Source Weaver remains VMF-first:
  - BSPSource/VMEX/game BSPs are not bundled.
  - BSPSource launchers and jar files are selected by the user.
  - The generated VMF is parsed, inspected, and integrity-checked before use.
  - JSON reports include tool kind, tool path, version probe, command arguments, input BSP, output VMF, exit code, log path, warnings/errors, and validation status.

BSPSource command shapes:
  bspsrc [--tool-arg values...] -o <out.vmf> <input.bsp>
  java -jar <bspsrc.jar> [--tool-arg values...] -o <out.vmf> <input.bsp>

Generic wrapper command shape:
  <wrapper> [--tool-arg values...] <input.bsp> <output.vmf>

Use --tool only as an escape hatch for unusual decompilers or argument orders.
External tool runs default to a 900 second timeout. Override with --timeout-seconds.
"#
    );
}

fn print_pack_help() {
    println!(
        r#"Usage:
  sourceweaver pack <map.bsp> --tool <bspzip> --output <out.bsp> (--filelist list.txt | --asset-root dir --include path) [--log log.txt] [--timeout-seconds seconds] [--report report.json] [--json]

Runs optional BSP content packing with a user-provided bspzip/BSPZIP++-compatible tool.

Source Weaver remains VMF-first:
  - BSP packers, compiled maps, and custom assets are not bundled.
  - Packing is optional and separate from VMF editing, merging, and compiling.
  - Generated file lists use BSPZIP's internal/external path-pair format.
  - JSON reports include tool path, best-effort version probe, command arguments, input/output BSP, file list, requested assets, missing files, detected packed file count, log path, warnings/errors, and exit status.

Generated file-list command shape:
  bspzip -addlist <input.bsp> <filelist.txt> <output.bsp>

Generate a file list from asset roots:
  sourceweaver pack map.bsp --tool /path/to/bspzip --output packed.bsp \
    --asset-root /path/to/game \
    --include materials/custom/wall01.vmt \
    --include materials/custom/wall01.vtf \
    --json

Or pass an existing BSPZIP file list:
  sourceweaver pack map.bsp --tool /path/to/bspzip --output packed.bsp --filelist pack-list.txt --json

External tool runs default to a 900 second timeout. Override with --timeout-seconds.
"#
    );
}

fn print_campaign_run_help() {
    println!("sourceweaver campaign-run --plan campaign.toml [--dry-run] [--report summary.json]");
    println!();
    println!(
        "Runs a multi-step campaign stitch plan. Each [[steps]] entry uses the same fields as a job: base, inputs, output, landmark, changelevel policy/scope, preserve_external_transition, and [steps.delete]."
    );
    println!("Dry-run mode validates and reports every step without writing merged VMFs.");
}

fn print_run_job_help() {
    println!(
        r#"Usage:
  sourceweaver run --job <job.toml> [--dry-run] [--report report.json]

Runs a complete non-interactive Source Weaver job from TOML.
Relative paths inside the job are resolved relative to the job file's directory.
The command prints a JSON report to stdout and optionally writes it to --report.
"#
    );
}

fn print_job_template() {
    println!(
        r#"# Source Weaver non-interactive job file.
# Relative paths are resolved from this TOML file's directory.

base = "base.vmf"
inputs = ["next.vmf", "another.vmf"]
output = "stitched.vmf"
landmark = "map_transition"
dry_run = false
report = "sourceweaver-report.json"

[delete]
classnames = []
targetnames = []
roles = ["trigger", "clip"]
all_entities = false
brush_entity_mode = "whole-entity"
protect_critical_entities = true
"#
    );
}
