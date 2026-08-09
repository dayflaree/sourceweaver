//! Material preview scanning and color helpers.

use super::*;

pub(crate) fn scan_material_preview_roots(roots: &[PathBuf]) -> MaterialPreviewIndex {
    let mut index = MaterialPreviewIndex {
        roots: roots.to_vec(),
        ..Default::default()
    };
    for root in roots {
        scan_material_preview_root(root, root, &mut index, 0);
    }
    index
}

pub(crate) fn scan_material_preview_root(
    root: &Path,
    current: &Path,
    index: &mut MaterialPreviewIndex,
    depth: usize,
) {
    if depth > 12 {
        index
            .errors
            .push(format!("scan depth limit reached at {}", current.display()));
        return;
    }
    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(error) => {
            index
                .errors
                .push(format!("could not read {}: {error}", current.display()));
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_material_preview_root(root, &path, index, depth + 1);
            continue;
        }
        let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
            continue;
        };
        let extension = extension.to_ascii_lowercase();
        if !matches!(
            extension.as_str(),
            "vmt" | "vtf" | "png" | "jpg" | "jpeg" | "tga"
        ) {
            continue;
        }
        if let Some(material) = material_name_from_path(root, &path) {
            index.materials.insert(material.clone());
            if extension == "vmt" {
                index.vmt_files.insert(material.clone());
                collect_vmt_basetextures(&path, index);
            } else {
                index.texture_files.insert(material);
            }
        }
    }
}

pub(crate) fn collect_vmt_basetextures(path: &Path, index: &mut MaterialPreviewIndex) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    for line in text.lines() {
        let line = line.trim();
        if !line.to_ascii_lowercase().contains("$basetexture") {
            continue;
        }
        let tokens = quoted_tokens(line);
        if let Some(value) = tokens
            .iter()
            .rev()
            .find(|token| !token.to_ascii_lowercase().contains("$basetexture"))
        {
            index.materials.insert(normalize_material_name(value));
        }
    }
}

pub(crate) fn quoted_tokens(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    for ch in line.chars() {
        match ch {
            '"' => {
                if in_quote {
                    tokens.push(current.clone());
                    current.clear();
                }
                in_quote = !in_quote;
            }
            _ if in_quote => current.push(ch),
            _ => {}
        }
    }
    tokens
}

pub(crate) fn material_name_from_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let without_extension = relative.with_extension("");
    let mut parts = without_extension
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();
    if parts
        .first()
        .is_some_and(|part| part.eq_ignore_ascii_case("materials"))
    {
        parts.remove(0);
    }
    (!parts.is_empty()).then(|| normalize_material_name(&parts.join("/")))
}

pub(crate) fn normalize_material_name(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .replace('\\', "/")
        .to_ascii_lowercase()
        .trim_start_matches("materials/")
        .trim_start_matches('/')
        .to_string()
}

pub(crate) fn material_preview_color(
    material: &str,
    index: &MaterialPreviewIndex,
) -> egui::Color32 {
    let material = normalize_material_name(material);
    if material.starts_with("tools/") {
        return tool_material_color(&material);
    }
    if index.contains_material(&material) {
        return hashed_material_color(&material, 175, 230);
    }
    hashed_material_color(&material, 80, 130).gamma_multiply(0.85)
}

pub(crate) fn tool_material_color(material: &str) -> egui::Color32 {
    if material.contains("trigger") {
        egui::Color32::from_rgb(255, 170, 90)
    } else if material.contains("clip") {
        egui::Color32::from_rgb(230, 95, 95)
    } else if material.contains("skybox") || material.contains("sky") {
        egui::Color32::from_rgb(100, 170, 255)
    } else if material.contains("nodraw") {
        egui::Color32::from_rgb(100, 100, 115)
    } else if material.contains("hint") || material.contains("skip") {
        egui::Color32::from_rgb(185, 115, 255)
    } else {
        egui::Color32::from_rgb(150, 150, 150)
    }
}

pub(crate) fn hashed_material_color(material: &str, min: u8, max: u8) -> egui::Color32 {
    let mut hash = 1469598103934665603_u64;
    for byte in material.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    let span = u64::from(max.saturating_sub(min)).max(1);
    let channel = |shift: u32| -> u8 { min + (((hash >> shift) % span) as u8) };
    egui::Color32::from_rgb(channel(0), channel(16), channel(32))
}

pub(crate) fn solid_color(solid: &PreviewSolid) -> egui::Color32 {
    if solid.roles.contains(&BrushRole::Trigger) {
        egui::Color32::from_rgb(255, 170, 90)
    } else if solid.roles.contains(&BrushRole::Clip) {
        egui::Color32::from_rgb(230, 95, 95)
    } else if solid.roles.contains(&BrushRole::Areaportal) {
        egui::Color32::from_rgb(185, 115, 255)
    } else if solid.roles.contains(&BrushRole::Skybox) {
        egui::Color32::from_rgb(100, 170, 255)
    } else if solid.roles.contains(&BrushRole::Occluder) {
        egui::Color32::from_rgb(175, 175, 105)
    } else if solid.roles.contains(&BrushRole::Hint) || solid.roles.contains(&BrushRole::Skip) {
        egui::Color32::from_rgb(255, 225, 100)
    } else if solid.roles.contains(&BrushRole::Nodraw) {
        egui::Color32::from_rgb(150, 150, 150)
    } else if solid.roles.contains(&BrushRole::Water) {
        egui::Color32::from_rgb(80, 210, 230)
    } else if solid.roles.contains(&BrushRole::BrushEntity) {
        egui::Color32::from_rgb(130, 235, 145)
    } else {
        egui::Color32::from_rgb(170, 190, 220)
    }
}
