use serde::{Deserialize, Serialize};
use sourceweaver_core::{
    BrushEntityDeletionMode, BrushRole, CampaignMapInput, CampaignOrderSuggestion,
    CampaignTransition, DeletionCriteria, DeletionReport, Document, IntegrityReport, MergeInput,
    MergeOptions, MergeReport, VmfToolValidationReport, discover_landmarks, discover_transitions,
    format_integrity_issue, inspect_entities, merge_maps, parse_compile_log, prune_document,
    suggest_campaign_order, summarize_entity_types, validate_document_integrity,
    validate_for_source_tools,
};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

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
        "bsp-import" | "decompile-bsp" => bsp_import_command(&args[1..]),
        "run" | "batch" | "job" => run_job_command(&args[1..]),
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
            value if value.starts_with('-') => return Err(format!("unknown merge flag `{value}`")),
            value => inputs.push(PathBuf::from(value)),
        }
        cursor += 1;
    }

    if inputs.len() < 2 {
        return Err(
            "usage: sourceweaver merge -o <out.vmf> [--landmark name] <base.vmf> <add.vmf> [...]",
        )?;
    }
    let output = output.ok_or("merge needs -o/--output")?;

    let mut merge_inputs = Vec::new();
    for path in inputs {
        let label = path.display().to_string();
        let document = load_document(&path)?;
        let integrity = validate_document_integrity(&document, &label);
        for issue in integrity.warnings() {
            eprintln!("{}", format_integrity_issue(issue));
        }
        if let Some(message) = integrity.error_message() {
            return Err(message);
        }
        merge_inputs.push(MergeInput { label, document });
    }

    let (document, report) = merge_maps(merge_inputs, &MergeOptions { landmark })?;
    write_document(&output, &document)?;
    println!("merged maps: {}", report.merged_maps);
    println!("appended world solids: {}", report.appended_world_solids);
    println!("appended entities: {}", report.appended_entities);
    for (label, offset) in report.applied_offsets {
        println!("offset\t{}\t{}", label, offset);
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

    let input = input.ok_or("usage: sourceweaver validate <map.vmf> [--compile-log log.txt] [--vbsp path] [--game game-dir] [--capture-log log.txt] [--json]")?;
    let document = load_document(&input)?;
    let mut compile_log_text =
        match compile_log {
            Some(path) => Some(fs::read_to_string(&path).map_err(|error| {
                format!("failed to read compile log {}: {error}", path.display())
            })?),
            None => None,
        };

    let vbsp_status = if let Some(vbsp_path) = vbsp {
        let output = run_vbsp(&vbsp_path, game.as_deref(), &input)?;
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

    let report = validate_for_source_tools(
        &document,
        &input.display().to_string(),
        compile_log_text.as_deref(),
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
) -> Result<std::process::Output, String> {
    let mut command = Command::new(vbsp_path);
    if let Some(game_dir) = game_dir {
        command.arg("-game").arg(game_dir);
    }
    command.arg(input);
    command.output().map_err(|error| {
        format!(
            "failed to run VBSP command {}: {error}",
            vbsp_path.display()
        )
    })
}

fn compile_command(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_compile_help();
        return Ok(());
    }
    let mut config = parse_compile_args(args)?;
    apply_compile_profile(&mut config)?;
    let input = config.input.clone().ok_or("usage: sourceweaver compile <map.vmf> [--profile profile.toml] [--vbsp path] [--vvis path] [--vrad path] [--game game-dir] [--steps vbsp,vvis,vrad] [--log-dir dir] [--report report.json] [--json]")?;

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
        if let Some(parent) = report_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "failed to create report directory {}: {error}",
                        parent.display()
                    )
                })?;
            }
        }
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
    let output = run_source_compile_tool(tool, config.game.as_deref(), &step_input)?;
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
) -> Result<std::process::Output, String> {
    let mut command = Command::new(tool_path);
    if let Some(game_dir) = game_dir {
        command.arg("-game").arg(game_dir);
    }
    command.arg(input);
    command.output().map_err(|error| {
        format!(
            "failed to run Source compile tool {}: {error}",
            tool_path.display()
        )
    })
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

fn bsp_import_command(args: &[String]) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_bsp_import_help();
        return Ok(());
    }
    let config = parse_bsp_import_args(args)?;
    let input = config.input.as_ref().ok_or("usage: sourceweaver bsp-import <map.bsp> --tool <decompiler> --output <out.vmf> [--tool-arg arg] [--log log.txt] [--report report.json] [--json]")?;
    let tool = config
        .tool
        .as_ref()
        .ok_or("bsp-import needs --tool <decompiler>")?;
    let output_vmf = config
        .output
        .as_ref()
        .ok_or("bsp-import needs --output <out.vmf>")?;

    let tool_output = run_bsp_decompiler(tool, &config.tool_args, input, output_vmf)?;
    let log_text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&tool_output.stdout),
        String::from_utf8_lossy(&tool_output.stderr)
    );
    if let Some(log_path) = &config.log {
        if let Some(parent) = log_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "failed to create log directory {}: {error}",
                        parent.display()
                    )
                })?;
            }
        }
        fs::write(log_path, &log_text).map_err(|error| {
            format!(
                "failed to write decompile log {}: {error}",
                log_path.display()
            )
        })?;
    }

    let summary = parse_compile_log(&log_text);
    let generated_vmf_exists = output_vmf.exists();
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
                    tool: tool.display().to_string(),
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
        && log_snapshot.errors == 0
        && !log_snapshot.leak_detected
        && integrity
            .as_ref()
            .map(|report| report.errors == 0)
            .unwrap_or(false);
    let report = BspImportReport {
        ok,
        tool: tool.display().to_string(),
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

fn run_bsp_decompiler(
    tool: &Path,
    tool_args: &[String],
    input: &Path,
    output_vmf: &Path,
) -> Result<std::process::Output, String> {
    let mut command = Command::new(tool);
    command.args(tool_args);
    command.arg(input);
    command.arg(output_vmf);
    command
        .output()
        .map_err(|error| format!("failed to run BSP decompiler {}: {error}", tool.display()))
}

fn finish_bsp_import_report(
    config: &BspImportConfig,
    report: BspImportReport,
) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode BSP import report: {error}"))?;
    if let Some(report_path) = &config.report {
        if let Some(parent) = report_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "failed to create report directory {}: {error}",
                        parent.display()
                    )
                })?;
            }
        }
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
        if let Some(parent) = report_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "failed to create report directory {}: {error}",
                        parent.display()
                    )
                })?;
            }
        }
        fs::write(&report_path, &json).map_err(|error| {
            format!("failed to write report {}: {error}", report_path.display())
        })?;
    }

    println!("{json}");
    Ok(())
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
    delete: DeleteConfig,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    report: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Deserialize)]
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

#[derive(Debug, Clone, Serialize)]
struct AutomationReport {
    operation: String,
    dry_run: bool,
    output_written: bool,
    output: Option<String>,
    base: String,
    inputs: Vec<String>,
    landmark: Option<String>,
    deletion: DeletionSnapshot,
    per_map: Vec<MapJobReport>,
    integrity: IntegritySnapshot,
    transitions: Vec<TransitionSnapshot>,
    campaign_order: CampaignOrderSnapshot,
    merge: Option<MergeSnapshot>,
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
struct CampaignOrderSnapshot {
    ordered_labels: Vec<String>,
    landmark_pairs: Vec<CampaignLandmarkPairSnapshot>,
    warnings: Vec<String>,
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
    vbsp_exit_code: Option<i32>,
    compile_log: Option<CompileLogSnapshot>,
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

#[derive(Debug, Clone, Deserialize, Default)]
struct CompileProfile {
    tools: Option<CompileProfileTools>,
    compile: Option<CompileProfileSettings>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct CompileProfileTools {
    vbsp: Option<PathBuf>,
    vvis: Option<PathBuf>,
    vrad: Option<PathBuf>,
    game: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct CompileProfileSettings {
    steps: Option<Vec<String>>,
    log_dir: Option<PathBuf>,
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

#[derive(Debug, Clone, Default)]
struct BspImportConfig {
    input: Option<PathBuf>,
    tool: Option<PathBuf>,
    output: Option<PathBuf>,
    log: Option<PathBuf>,
    report: Option<PathBuf>,
    tool_args: Vec<String>,
    json: bool,
}

#[derive(Debug, Clone, Serialize)]
struct BspImportReport {
    ok: bool,
    tool: String,
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

impl ValidationSnapshot {
    fn from_report(report: &VmfToolValidationReport, vbsp_exit_code: Option<i32>) -> Self {
        Self {
            ok: report.is_ok() && vbsp_exit_code.map(|code| code == 0).unwrap_or(true),
            map: report.map_label.clone(),
            integrity: snapshot_integrity_report(&report.integrity),
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

    let criteria = criteria_from_delete_config(&job.delete)?;
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
        let (document, merge_report) = merge_maps(
            merge_inputs,
            &MergeOptions {
                landmark: job
                    .landmark
                    .clone()
                    .filter(|value| !value.trim().is_empty()),
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
        merge: merge_snapshot,
        result_entity_types,
        result_entity_records,
    })
}

fn criteria_from_delete_config(delete: &DeleteConfig) -> Result<DeletionCriteria, String> {
    let mut criteria = DeletionCriteria::default();
    criteria.drop_all_entities = delete.all_entities;
    criteria.protect_critical_entities = delete.protect_critical_entities;
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
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
    }
    fs::write(path, document.to_vmf_string())
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
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
  sourceweaver merge -o <out.vmf> [--landmark targetname] <base.vmf> <add.vmf> [...]
  sourceweaver validate <map.vmf> [--compile-log log.txt] [--vbsp path] [--game game-dir] [--capture-log log.txt] [--json]
  sourceweaver compile <map.vmf> [--profile profile.toml] [--vbsp path] [--vvis path] [--vrad path] [--game game-dir] [--steps vbsp,vvis,vrad] [--log-dir dir] [--report report.json] [--json]
  sourceweaver bsp-import <map.bsp> --tool <decompiler> --output <out.vmf> [--tool-arg arg] [--log log.txt] [--report report.json] [--json]
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
  sourceweaver validate <map.vmf> [--compile-log log.txt] [--vbsp path] [--game game-dir] [--capture-log log.txt] [--json]

Validates a VMF for Source tool readiness.

Linux-friendly workflow:
  sourceweaver validate merged.vmf --json
  sourceweaver validate merged.vmf --compile-log captured-vbsp.log --json

Source SDK workflow when VBSP is installed:
  sourceweaver validate merged.vmf --vbsp /path/to/vbsp --game /path/to/game --capture-log vbsp.log --json
"#
    );
}

fn print_compile_help() {
    println!(
        r#"Usage:
  sourceweaver compile <map.vmf> [--profile profile.toml] [--vbsp path] [--vvis path] [--vrad path] [--game game-dir] [--steps vbsp,vvis,vrad] [--log-dir dir] [--report report.json] [--json]

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

Each step captures stdout/stderr, writes step logs when --log-dir is set,
parses warnings/errors/leaks, and reports JSON when --json or --report is used.
"#
    );
}

fn print_bsp_import_help() {
    println!(
        r#"Usage:
  sourceweaver bsp-import <map.bsp> --tool <decompiler> --output <out.vmf> [--tool-arg arg] [--log log.txt] [--report report.json] [--json]

Runs a user-provided external BSP decompiler and validates the generated VMF.

Source Weaver remains VMF-first:
  - BSPSource/VMEX/game BSPs are not bundled.
  - The decompiler path is supplied by the user.
  - The generated VMF is parsed, inspected, and integrity-checked before use.
  - JSON reports include tool path, input BSP, output VMF, exit code, log path, warnings/errors, and validation status.

Generic wrapper command shape:
  <decompiler> [--tool-arg values...] <input.bsp> <output.vmf>

If a decompiler needs a different argument order, create a small wrapper script and pass that script as --tool.
"#
    );
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
