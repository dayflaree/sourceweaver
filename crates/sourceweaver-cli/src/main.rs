use sourceweaver_core::{
    BrushRole, DeletionCriteria, Document, MergeInput, MergeOptions, inspect_entities, merge_maps,
    prune_document, summarize_entity_types,
};
use std::collections::BTreeSet;
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
                for item in split_csv(value) {
                    let role = BrushRole::parse_filter(item)
                        .ok_or_else(|| format!("unknown brush role `{item}`"))?;
                    criteria.brush_roles.insert(role);
                }
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

    let input = input.ok_or("usage: sourceweaver prune <map.vmf> -o <out.vmf> [--drop-classname name] [--drop-targetname name] [--drop-role role]")?;
    let output = output.ok_or("prune needs -o/--output")?;
    let mut document = load_document(&input)?;
    let report = prune_document(&mut document, &criteria);
    write_document(&output, &document)?;
    println!("removed entities: {}", report.removed_entities);
    println!("removed world solids: {}", report.removed_world_solids);
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
        merge_inputs.push(MergeInput {
            label: path.display().to_string(),
            document: load_document(&path)?,
        });
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

fn load_document(path: impl AsRef<Path>) -> Result<Document, String> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    Document::parse(&text).map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn write_document(path: impl AsRef<Path>, document: &Document) -> Result<(), String> {
    let path = path.as_ref();
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
  sourceweaver prune <map.vmf> -o <out.vmf> [--drop-classname name] [--drop-targetname name] [--drop-role role]
  sourceweaver merge -o <out.vmf> [--landmark targetname] <base.vmf> <add.vmf> [...]

Deletion roles:
  trigger, clip, areaportal, skybox, occluder, hint, skip, nodraw, water, world-brush, brush-entity

Merge behavior in this first rebuild slice:
  - keeps the first VMF as the base map
  - appends world solids from each additional VMF, including skybox solids
  - appends all point and brush entities from each additional VMF
  - renumbers incoming VMF id keys to avoid collisions
  - when --landmark is supplied, aligns matching info_landmark targetnames to the base map
"#
    );
}
