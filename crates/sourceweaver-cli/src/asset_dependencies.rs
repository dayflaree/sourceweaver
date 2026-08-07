use serde::Serialize;
use sourceweaver_core::{Document, Node};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct AssetDependencyDiscovery {
    pub vmfs: Vec<String>,
    pub asset_roots: Vec<String>,
    pub references: Vec<AssetReferenceSnapshot>,
    pub assets: Vec<DiscoveredAssetSnapshot>,
    pub missing_assets: Vec<String>,
    pub ambiguous_assets: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssetReferenceSnapshot {
    pub internal_path: String,
    pub kind: String,
    pub source_vmf: String,
    pub source_key: String,
    pub source_value: String,
    pub context: String,
    pub derived_from: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredAssetSnapshot {
    pub internal_path: String,
    pub kind: String,
    pub selected_path: Option<String>,
    pub candidates: Vec<String>,
    pub status: String,
    pub sources: Vec<String>,
    pub derived_from: Option<String>,
}

#[derive(Debug, Clone)]
struct AssetReference {
    internal_path: String,
    kind: String,
    source_vmf: String,
    source_key: String,
    source_value: String,
    context: String,
    derived_from: Option<String>,
}

pub fn discover_vmf_dependencies(
    vmf_paths: &[PathBuf],
    asset_roots: &[PathBuf],
) -> Result<AssetDependencyDiscovery, String> {
    let mut references = Vec::new();
    let mut warnings = Vec::new();
    for vmf_path in vmf_paths {
        let text = fs::read_to_string(vmf_path)
            .map_err(|error| format!("failed to read VMF {}: {error}", vmf_path.display()))?;
        let document = Document::parse(&text)
            .map_err(|error| format!("failed to parse VMF {}: {error}", vmf_path.display()))?;
        collect_document_references(&document, vmf_path, &mut references, &mut warnings);
    }

    let discovered = build_asset_map(&references, asset_roots);
    let material_dependencies =
        discover_material_texture_dependencies(&discovered, asset_roots, &mut warnings);
    references.extend(material_dependencies);
    let mut discovered = build_asset_map(&references, asset_roots);
    let model_dependencies = discover_model_companions(&discovered, asset_roots);
    references.extend(model_dependencies);
    discovered = build_asset_map(&references, asset_roots);

    let mut missing_assets = Vec::new();
    let mut ambiguous_assets = Vec::new();
    let mut assets = Vec::new();
    for asset in discovered.into_values() {
        if asset.candidates.is_empty() {
            missing_assets.push(asset.internal_path.clone());
        } else if asset.candidates.len() > 1 {
            ambiguous_assets.push(asset.internal_path.clone());
        }
        assets.push(asset);
    }
    assets.sort_by(|left, right| left.internal_path.cmp(&right.internal_path));
    missing_assets.sort();
    ambiguous_assets.sort();
    for path in &missing_assets {
        warnings.push(format!(
            "discovered asset `{path}` was not found under any asset root"
        ));
    }
    for path in &ambiguous_assets {
        warnings.push(format!(
            "discovered asset `{path}` exists under more than one asset root; the first root wins"
        ));
    }
    warnings.sort();
    warnings.dedup();

    Ok(AssetDependencyDiscovery {
        vmfs: vmf_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        asset_roots: asset_roots
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        references: references
            .into_iter()
            .map(|reference| AssetReferenceSnapshot {
                internal_path: reference.internal_path,
                kind: reference.kind,
                source_vmf: reference.source_vmf,
                source_key: reference.source_key,
                source_value: reference.source_value,
                context: reference.context,
                derived_from: reference.derived_from,
            })
            .collect(),
        assets,
        missing_assets,
        ambiguous_assets,
        warnings,
    })
}

pub fn discovered_include_paths(discovery: &AssetDependencyDiscovery) -> Vec<String> {
    discovery
        .assets
        .iter()
        .map(|asset| asset.internal_path.clone())
        .collect()
}

fn collect_document_references(
    document: &Document,
    vmf_path: &Path,
    references: &mut Vec<AssetReference>,
    warnings: &mut Vec<String>,
) {
    for node in &document.nodes {
        collect_node_references(node, vmf_path, references, warnings, &mut Vec::new());
    }
}

fn collect_node_references(
    node: &Node,
    vmf_path: &Path,
    references: &mut Vec<AssetReference>,
    warnings: &mut Vec<String>,
    context: &mut Vec<String>,
) {
    match node {
        Node::Property { key, value } => {
            collect_property_reference(key, value, vmf_path, references, warnings, context);
        }
        Node::Block { name, body } => {
            context.push(name.clone());
            for child in body {
                collect_node_references(child, vmf_path, references, warnings, context);
            }
            context.pop();
        }
    }
}

fn collect_property_reference(
    key: &str,
    value: &str,
    vmf_path: &Path,
    references: &mut Vec<AssetReference>,
    warnings: &mut Vec<String>,
    context: &[String],
) {
    let key_lower = key.to_ascii_lowercase();
    let value_trimmed = value.trim();
    let value_lower = normalize_slashes(value_trimmed).to_ascii_lowercase();
    if value_trimmed.is_empty() || value_trimmed.starts_with('*') || value_trimmed.starts_with('!')
    {
        return;
    }
    let source = PropertyReferenceSource {
        vmf_path,
        key,
        value,
        context,
    };

    if (key_lower == "material" || key_lower.ends_with("material"))
        && let Some(path) = material_asset_path(value_trimmed)
    {
        push_reference(references, path, "material", &source, None);
    }

    if (key_lower == "model" || value_lower.ends_with(".mdl") || value_lower.starts_with("models/"))
        && let Some(path) = model_asset_path(value_trimmed)
    {
        push_reference(references, path, "model", &source, None);
    }

    if looks_like_sound_reference(&key_lower, &value_lower)
        && let Some(path) = sound_asset_path(value_trimmed)
    {
        push_reference(references, path, "sound", &source, None);
    }

    if looks_like_script_reference(&key_lower, &value_lower)
        && let Some(path) = script_asset_path(value_trimmed)
    {
        push_reference(references, path, "script", &source, None);
    }

    if (value_lower.starts_with("particles/") || value_lower.ends_with(".pcf"))
        && let Some(path) = rooted_or_prefixed_asset_path(value_trimmed, "particles", Some("pcf"))
    {
        push_reference(references, path, "particle", &source, None);
    } else if key_lower == "effect_name" || key_lower == "particle_system" {
        warnings.push(format!(
            "{} references particle system `{}` by name; Source Weaver cannot infer the owning PCF from VMF data alone",
            vmf_path.display(),
            value_trimmed
        ));
    }
}

struct PropertyReferenceSource<'a> {
    vmf_path: &'a Path,
    key: &'a str,
    value: &'a str,
    context: &'a [String],
}

fn push_reference(
    references: &mut Vec<AssetReference>,
    internal_path: String,
    kind: &str,
    source: &PropertyReferenceSource<'_>,
    derived_from: Option<String>,
) {
    references.push(AssetReference {
        internal_path,
        kind: kind.to_string(),
        source_vmf: source.vmf_path.display().to_string(),
        source_key: source.key.to_string(),
        source_value: source.value.to_string(),
        context: if source.context.is_empty() {
            "<root>".to_string()
        } else {
            source.context.join("/")
        },
        derived_from,
    });
}

fn build_asset_map(
    references: &[AssetReference],
    asset_roots: &[PathBuf],
) -> BTreeMap<String, DiscoveredAssetSnapshot> {
    let mut sources_by_path: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut kind_by_path: BTreeMap<String, String> = BTreeMap::new();
    let mut derived_by_path: BTreeMap<String, Option<String>> = BTreeMap::new();
    for reference in references {
        kind_by_path
            .entry(reference.internal_path.clone())
            .or_insert_with(|| reference.kind.clone());
        derived_by_path
            .entry(reference.internal_path.clone())
            .or_insert_with(|| reference.derived_from.clone());
        sources_by_path
            .entry(reference.internal_path.clone())
            .or_default()
            .insert(format!(
                "{}:{}:{}",
                reference.source_vmf, reference.source_key, reference.source_value
            ));
    }

    let mut assets = BTreeMap::new();
    for (internal_path, sources) in sources_by_path {
        let candidates = candidates_for(asset_roots, &internal_path);
        let status = match candidates.len() {
            0 => "missing",
            1 => "resolved",
            _ => "ambiguous",
        };
        assets.insert(
            internal_path.clone(),
            DiscoveredAssetSnapshot {
                internal_path: internal_path.clone(),
                kind: kind_by_path
                    .remove(&internal_path)
                    .unwrap_or_else(|| "asset".to_string()),
                selected_path: candidates.first().cloned(),
                candidates,
                status: status.to_string(),
                sources: sources.into_iter().collect(),
                derived_from: derived_by_path.remove(&internal_path).flatten(),
            },
        );
    }
    assets
}

fn discover_material_texture_dependencies(
    assets: &BTreeMap<String, DiscoveredAssetSnapshot>,
    asset_roots: &[PathBuf],
    warnings: &mut Vec<String>,
) -> Vec<AssetReference> {
    let mut dependencies = Vec::new();
    for asset in assets.values() {
        if asset.kind != "material" {
            continue;
        }
        let Some(selected_path) = asset.selected_path.as_ref() else {
            continue;
        };
        let Ok(text) = fs::read_to_string(selected_path) else {
            warnings.push(format!(
                "failed to read material `{selected_path}` for texture discovery"
            ));
            continue;
        };
        for texture in parse_vmt_texture_references(&text) {
            let Some(internal_path) = texture_asset_path(&texture) else {
                continue;
            };
            dependencies.push(AssetReference {
                internal_path,
                kind: "texture".to_string(),
                source_vmf: asset
                    .sources
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "<material>".to_string()),
                source_key: "vmt-texture".to_string(),
                source_value: texture,
                context: "material-dependency".to_string(),
                derived_from: Some(asset.internal_path.clone()),
            });
        }
        if asset.candidates.len() > 1 {
            warnings.push(format!(
                "material `{}` has multiple candidates; texture dependency scan used `{}`",
                asset.internal_path, selected_path
            ));
        }
    }
    let _ = asset_roots;
    dependencies
}

fn discover_model_companions(
    assets: &BTreeMap<String, DiscoveredAssetSnapshot>,
    asset_roots: &[PathBuf],
) -> Vec<AssetReference> {
    let mut dependencies = Vec::new();
    for asset in assets.values() {
        if asset.kind != "model" || !asset.internal_path.ends_with(".mdl") {
            continue;
        }
        let stem = asset.internal_path.trim_end_matches(".mdl");
        for suffix in [".vvd", ".dx80.vtx", ".dx90.vtx", ".sw.vtx", ".phy"] {
            let companion = format!("{stem}{suffix}");
            if candidates_for(asset_roots, &companion).is_empty() {
                continue;
            }
            dependencies.push(AssetReference {
                internal_path: companion,
                kind: "model-companion".to_string(),
                source_vmf: asset
                    .sources
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "<model>".to_string()),
                source_key: "model-companion".to_string(),
                source_value: asset.internal_path.clone(),
                context: "model-dependency".to_string(),
                derived_from: Some(asset.internal_path.clone()),
            });
        }
    }
    dependencies
}

fn candidates_for(asset_roots: &[PathBuf], internal_path: &str) -> Vec<String> {
    asset_roots
        .iter()
        .map(|root| root.join(Path::new(internal_path)))
        .filter(|path| path.is_file())
        .map(|path| path.display().to_string())
        .collect()
}

fn parse_vmt_texture_references(text: &str) -> Vec<String> {
    let texture_keys = [
        "$basetexture",
        "$basetexture2",
        "$bumpmap",
        "$bumpmap2",
        "$normalmap",
        "$detail",
        "$envmapmask",
        "$phongexponenttexture",
        "$lightwarptexture",
        "$selfillummask",
        "$blendmodulatetexture",
        "$flashlighttexture",
    ];
    let mut textures = BTreeSet::new();
    for raw_line in text.lines() {
        let line = raw_line.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        let key_offset = usize::from(lower.starts_with('"'));
        let comparable = lower.trim_start_matches('"');
        for key in texture_keys {
            if !comparable.starts_with(key) {
                continue;
            }
            let closing_quote_offset =
                usize::from(line.as_bytes().get(key_offset + key.len()) == Some(&b'"'));
            if let Some(value) =
                parse_vmt_value_after_key(line, key_offset + key.len() + closing_quote_offset)
            {
                let normalized = normalize_slashes(&value);
                let lower_value = normalized.to_ascii_lowercase();
                if lower_value == "env_cubemap" || lower_value.starts_with("_") {
                    continue;
                }
                textures.insert(normalized);
            }
        }
    }
    textures.into_iter().collect()
}

fn parse_vmt_value_after_key(line: &str, key_len: usize) -> Option<String> {
    let rest = line.get(key_len..)?.trim();
    if rest.is_empty() {
        return None;
    }
    if let Some(stripped) = rest.strip_prefix('"') {
        let value = stripped.split('"').next().unwrap_or("").trim();
        return (!value.is_empty()).then(|| value.to_string());
    }
    let value = rest
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches('"');
    (!value.is_empty()).then(|| value.to_string())
}

fn material_asset_path(value: &str) -> Option<String> {
    let value = normalize_slashes(value.trim())
        .trim_matches('"')
        .to_string();
    if value.is_empty() || value.starts_with('%') || value.starts_with('$') {
        return None;
    }
    let value = value.trim_start_matches('/');
    if value.to_ascii_lowercase().starts_with("materials/") {
        return ensure_extension(value, "vmt");
    }
    Some(format!("materials/{}", ensure_extension(value, "vmt")?))
}

fn texture_asset_path(value: &str) -> Option<String> {
    let value = normalize_slashes(value.trim())
        .trim_matches('"')
        .to_string();
    if value.is_empty() || value.starts_with('%') || value.starts_with('$') {
        return None;
    }
    let value = value.trim_start_matches('/');
    if value.to_ascii_lowercase().starts_with("materials/") {
        return ensure_extension(value, "vtf");
    }
    Some(format!("materials/{}", ensure_extension(value, "vtf")?))
}

fn model_asset_path(value: &str) -> Option<String> {
    rooted_or_prefixed_asset_path(value, "models", Some("mdl"))
}

fn sound_asset_path(value: &str) -> Option<String> {
    let value = normalize_slashes(value.trim())
        .trim_matches('"')
        .to_string();
    let lower = value.to_ascii_lowercase();
    if !(lower.ends_with(".wav") || lower.ends_with(".mp3") || lower.ends_with(".ogg")) {
        return None;
    }
    if lower.starts_with("sound/") {
        Some(value)
    } else {
        Some(format!("sound/{value}"))
    }
}

fn script_asset_path(value: &str) -> Option<String> {
    let value = normalize_slashes(value.trim())
        .trim_matches('"')
        .to_string();
    let lower = value.to_ascii_lowercase();
    if lower.starts_with("scripts/") || lower.starts_with("scenes/") || lower.starts_with("cfg/") {
        return Some(value);
    }
    if lower.ends_with(".nut") {
        return Some(format!("scripts/vscripts/{value}"));
    }
    if lower.ends_with(".vcd") {
        return Some(format!("scenes/{value}"));
    }
    if lower.ends_with(".txt") || lower.ends_with(".res") || lower.ends_with(".cfg") {
        return Some(format!("scripts/{value}"));
    }
    None
}

fn rooted_or_prefixed_asset_path(
    value: &str,
    root: &str,
    extension: Option<&str>,
) -> Option<String> {
    let value = normalize_slashes(value.trim())
        .trim_matches('"')
        .to_string();
    if value.is_empty() || value.starts_with('*') || value.starts_with('!') {
        return None;
    }
    let value = value.trim_start_matches('/');
    let with_extension = match extension {
        Some(extension) => ensure_extension(value, extension)?,
        None => value.to_string(),
    };
    if with_extension
        .to_ascii_lowercase()
        .starts_with(&format!("{root}/"))
    {
        Some(with_extension)
    } else {
        Some(format!("{root}/{with_extension}"))
    }
}

fn ensure_extension(value: &str, extension: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    if lower.ends_with(&format!(".{extension}")) {
        Some(value.to_string())
    } else if Path::new(value).extension().is_some() {
        None
    } else {
        Some(format!("{value}.{extension}"))
    }
}

fn looks_like_sound_reference(key_lower: &str, value_lower: &str) -> bool {
    key_lower.contains("sound")
        || key_lower.contains("noise")
        || key_lower == "message"
        || value_lower.ends_with(".wav")
        || value_lower.ends_with(".mp3")
        || value_lower.ends_with(".ogg")
}

fn looks_like_script_reference(key_lower: &str, value_lower: &str) -> bool {
    key_lower.contains("script")
        || key_lower.contains("vscript")
        || value_lower.starts_with("scripts/")
        || value_lower.starts_with("scenes/")
        || value_lower.ends_with(".nut")
        || value_lower.ends_with(".vcd")
        || value_lower.ends_with(".res")
}

fn normalize_slashes(value: &str) -> String {
    value.replace('\\', "/")
}
