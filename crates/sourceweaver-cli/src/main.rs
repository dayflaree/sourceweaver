use serde::{Deserialize, Serialize};
use sourceweaver_core::{
    BrushEntityDeletionMode, BrushRole, DeletionCriteria, DeletionReport, Document,
    IntegrityReport, MergeInput, MergeOptions, MergeReport, format_integrity_issue,
    inspect_entities, merge_maps, prune_document, summarize_entity_types,
    validate_document_integrity,
};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

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

    let input = input.ok_or("usage: sourceweaver prune <map.vmf> -o <out.vmf> [--drop-classname name] [--drop-targetname name] [--drop-role role] [--brush-entity-mode whole-entity|matching-solids] [--allow-critical-deletion]")?;
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
    removed_entities: usize,
    removed_world_solids: usize,
    removed_brush_entity_solids: usize,
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
            brush_entity_mode: criteria.brush_entity_mode.to_string(),
            protect_critical_entities: criteria.protect_critical_entities,
            removed_entities: removed_total.removed_entities,
            removed_world_solids: removed_total.removed_world_solids,
            removed_brush_entity_solids: removed_total.removed_brush_entity_solids,
        },
        per_map,
        integrity: snapshot_integrity_report(&integrity_report),
        merge: merge_snapshot,
        result_entity_types,
        result_entity_records,
    })
}

fn criteria_from_delete_config(delete: &DeleteConfig) -> Result<DeletionCriteria, String> {
    let mut criteria = DeletionCriteria::default();
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
  sourceweaver prune <map.vmf> -o <out.vmf> [--drop-classname name] [--drop-targetname name] [--drop-role role] [--brush-entity-mode whole-entity|matching-solids] [--allow-critical-deletion]
  sourceweaver merge -o <out.vmf> [--landmark targetname] <base.vmf> <add.vmf> [...]
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

Merge behavior:
  - keeps the first VMF as the base map
  - appends world solids from each additional VMF, including skybox solids
  - appends all point and brush entities from each additional VMF
  - renumbers incoming VMF id keys to avoid collisions
  - when --landmark is supplied, aligns matching info_landmark targetnames to the base map
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
brush_entity_mode = "whole-entity"
protect_critical_entities = true
"#
    );
}
