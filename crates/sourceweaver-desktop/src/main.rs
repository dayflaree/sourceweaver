use eframe::egui;
use sourceweaver_core::{
    BrushRole, DeletionCriteria, DeletionReport, Document, EntityRecord, IntegrityReport,
    LandmarkDiscovery, LandmarkTargetStatus, MergeInput, MergeOptions, MergeReport, PreviewBounds,
    PreviewDocument, PreviewSolid, discover_landmarks, format_integrity_issue, inspect_entities,
    merge_maps, preview_document, prune_document, summarize_entity_types,
    validate_document_integrity,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Source Weaver")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Source Weaver",
        native_options,
        Box::new(|cc| Ok(Box::new(SourceWeaverApp::new(cc)))),
    )
}

struct SourceWeaverApp {
    maps: Vec<MapEntry>,
    selected_map: Option<usize>,
    base_index: usize,
    landmark: String,
    output_path: String,
    drop_classnames: String,
    drop_targetnames: String,
    role_options: Vec<RoleOption>,
    status: Vec<String>,
    active_table: TableMode,
    preview_scope: PreviewScope,
    merged_preview: Option<MergedPreview>,
    preview_view: PreviewView,
    preview_zoom: f32,
    preview_pan: egui::Vec2,
    preview_show_solids: bool,
    preview_show_entities: bool,
    preview_show_grid: bool,
    selected_entity_rows: BTreeSet<EntitySelectionKey>,
}

#[derive(Debug, Clone)]
struct MapEntry {
    path: PathBuf,
    analysis: Result<MapAnalysis, String>,
}

#[derive(Debug, Clone)]
struct MapAnalysis {
    entity_records: Vec<EntityRecord>,
    type_counts: BTreeMap<String, usize>,
    preview: PreviewDocument,
    landmarks: LandmarkDiscovery,
    integrity: IntegrityReport,
}

#[derive(Debug, Clone)]
struct MergedPreview {
    preview: PreviewDocument,
    summary: MergedPreviewSummary,
}

#[derive(Debug, Clone)]
struct MergedPreviewSummary {
    merged_maps: usize,
    appended_world_solids: usize,
    appended_entities: usize,
    removed_entities: usize,
    removed_world_solids: usize,
    offsets: Vec<String>,
}

#[derive(Debug, Clone)]
struct LandmarkOption {
    targetname: String,
    present_maps: usize,
    total_maps: usize,
    warning_maps: usize,
}

impl LandmarkOption {
    fn label(&self) -> String {
        let mut label = format!(
            "{} ({}/{})",
            self.targetname, self.present_maps, self.total_maps
        );
        if self.warning_maps > 0 {
            label.push_str(&format!(", {} warning(s)", self.warning_maps));
        }
        label
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewScope {
    SelectedMap,
    MergedResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TableMode {
    Preview,
    Entities,
    Classnames,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewView {
    Top,
    Front,
    Side,
}

#[derive(Debug, Clone)]
struct RoleOption {
    label: &'static str,
    role: BrushRole,
    selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EntitySelectionKey {
    map_path: String,
    record_index: usize,
    block_name: String,
    classname: Option<String>,
    targetname: Option<String>,
}

impl SourceWeaverApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            maps: Vec::new(),
            selected_map: None,
            base_index: 0,
            landmark: "map_transition".to_string(),
            output_path: String::new(),
            drop_classnames: String::new(),
            drop_targetnames: String::new(),
            role_options: vec![
                RoleOption::new("Triggers", BrushRole::Trigger),
                RoleOption::new("Clips", BrushRole::Clip),
                RoleOption::new("Areaportals", BrushRole::Areaportal),
                RoleOption::new("Skybox brushes", BrushRole::Skybox),
                RoleOption::new("Occluders", BrushRole::Occluder),
                RoleOption::new("Hint brushes", BrushRole::Hint),
                RoleOption::new("Skip brushes", BrushRole::Skip),
                RoleOption::new("Nodraw brushes", BrushRole::Nodraw),
                RoleOption::new("Water", BrushRole::Water),
                RoleOption::new("World brushes", BrushRole::WorldBrush),
                RoleOption::new("Brush entities", BrushRole::BrushEntity),
            ],
            status: vec!["Ready. Add VMF files to inspect or merge.".to_string()],
            active_table: TableMode::Preview,
            preview_scope: PreviewScope::SelectedMap,
            merged_preview: None,
            preview_view: PreviewView::Top,
            preview_zoom: 1.0,
            preview_pan: egui::Vec2::ZERO,
            preview_show_solids: true,
            preview_show_entities: true,
            preview_show_grid: true,
            selected_entity_rows: BTreeSet::new(),
        }
    }

    fn add_status(&mut self, message: impl Into<String>) {
        self.status.push(message.into());
        if self.status.len() > 12 {
            let overflow = self.status.len() - 12;
            self.status.drain(0..overflow);
        }
    }

    fn add_vmf_files(&mut self) {
        if let Some(files) = rfd::FileDialog::new()
            .set_title("Select Source VMF maps")
            .add_filter("Valve Map Format", &["vmf"])
            .pick_files()
        {
            let mut added = 0;
            for file in files {
                if self.maps.iter().any(|entry| entry.path == file) {
                    continue;
                }
                self.maps.push(MapEntry::load(file));
                added += 1;
            }
            if self.selected_map.is_none() && !self.maps.is_empty() {
                self.selected_map = Some(0);
            }
            self.base_index = self.base_index.min(self.maps.len().saturating_sub(1));
            self.retain_valid_entity_selections();
            self.clear_merged_preview();
            self.add_status(format!("Added {added} VMF file(s)."));
        }
    }

    fn rescan_maps(&mut self) {
        for map in &mut self.maps {
            let path = map.path.clone();
            *map = MapEntry::load(path);
        }
        self.retain_valid_entity_selections();
        self.clear_merged_preview();
        self.add_status("Re-scanned selected VMFs from disk.");
    }

    fn clear_maps(&mut self) {
        self.maps.clear();
        self.selected_map = None;
        self.base_index = 0;
        self.selected_entity_rows.clear();
        self.preview_pan = egui::Vec2::ZERO;
        self.preview_zoom = 1.0;
        self.clear_merged_preview();
        self.add_status("Cleared selected VMFs.");
    }

    fn remove_selected_map(&mut self) {
        let Some(index) = self.selected_map else {
            return;
        };
        if index < self.maps.len() {
            let removed = self.maps.remove(index);
            self.add_status(format!("Removed {}.", display_path(&removed.path)));
            if self.maps.is_empty() {
                self.selected_map = None;
                self.base_index = 0;
            } else {
                self.selected_map = Some(index.min(self.maps.len() - 1));
                self.base_index = self.base_index.min(self.maps.len() - 1);
            }
            self.retain_valid_entity_selections();
            self.clear_merged_preview();
        }
    }

    fn choose_output_path(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Save merged VMF")
            .add_filter("Valve Map Format", &["vmf"])
            .set_file_name("sourceweaver_merged.vmf")
            .save_file()
        {
            self.output_path = path.display().to_string();
        }
    }

    fn build_deletion_criteria(&self) -> DeletionCriteria {
        let mut criteria = DeletionCriteria::default();
        criteria
            .classnames
            .extend(split_csv(&self.drop_classnames).map(ToOwned::to_owned));
        criteria
            .targetnames
            .extend(split_csv(&self.drop_targetnames).map(ToOwned::to_owned));
        criteria.brush_roles.extend(
            self.role_options
                .iter()
                .filter(|option| option.selected)
                .map(|option| option.role.clone()),
        );
        criteria
    }

    fn clear_merged_preview(&mut self) {
        self.merged_preview = None;
        if self.preview_scope == PreviewScope::MergedResult {
            self.preview_scope = PreviewScope::SelectedMap;
        }
    }

    fn current_entity_selection_keys(&self) -> BTreeSet<EntitySelectionKey> {
        let mut keys = BTreeSet::new();
        for entry in &self.maps {
            let Ok(analysis) = &entry.analysis else {
                continue;
            };
            keys.extend(
                analysis
                    .entity_records
                    .iter()
                    .map(|record| entity_selection_key(&entry.path, record)),
            );
        }
        keys
    }

    fn retain_valid_entity_selections(&mut self) {
        let valid = self.current_entity_selection_keys();
        self.selected_entity_rows.retain(|key| valid.contains(key));
    }

    fn discovered_landmark_options(&self) -> Vec<LandmarkOption> {
        let total_maps = self.maps.len();
        let mut options: BTreeMap<String, LandmarkOption> = BTreeMap::new();

        for entry in &self.maps {
            let Ok(analysis) = &entry.analysis else {
                continue;
            };
            for targetname in &analysis.landmarks.targetnames {
                let status = analysis.landmarks.status_for(targetname);
                let option = options
                    .entry(targetname.clone())
                    .or_insert_with(|| LandmarkOption {
                        targetname: targetname.clone(),
                        present_maps: 0,
                        total_maps,
                        warning_maps: 0,
                    });
                option.present_maps += 1;
                if matches!(
                    status,
                    LandmarkTargetStatus::Duplicate { .. }
                        | LandmarkTargetStatus::InvalidOrigin { .. }
                ) {
                    option.warning_maps += 1;
                }
            }
        }

        options.into_values().collect()
    }

    fn landmark_warning_lines(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let selected = self.landmark.trim();

        for entry in &self.maps {
            let path = file_name_or_path(&entry.path);
            let Ok(analysis) = &entry.analysis else {
                warnings.push(format!(
                    "Warning: {path} could not be parsed; landmark status is unavailable."
                ));
                continue;
            };

            for duplicate in &analysis.landmarks.duplicates {
                warnings.push(format!(
                    "Warning: {path} has duplicate info_landmark `{}` ({} entries, {} valid origin(s)).",
                    duplicate.targetname, duplicate.count, duplicate.valid_origins
                ));
            }

            if selected.is_empty() {
                continue;
            }

            match analysis.landmarks.status_for(selected) {
                LandmarkTargetStatus::Blank | LandmarkTargetStatus::Present { .. } => {}
                LandmarkTargetStatus::Missing => warnings.push(format!(
                    "Warning: {path} is missing landmark `{selected}`; it will be left unshifted if merged."
                )),
                LandmarkTargetStatus::InvalidOrigin { .. } => warnings.push(format!(
                    "Warning: {path} has landmark `{selected}` with a missing or invalid origin; it will be left unshifted if merged."
                )),
                LandmarkTargetStatus::Duplicate {
                    count,
                    valid_origins,
                } => warnings.push(format!(
                    "Warning: {path} has duplicate landmark `{selected}` ({count} entries, {valid_origins} valid origin(s)); alignment is ambiguous."
                )),
            }
        }

        warnings
    }

    fn draw_landmark_status(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label("Landmark status");

            if self.maps.is_empty() {
                ui.weak("Add VMFs to discover info_landmark targetnames.");
                return;
            }

            let selected = self.landmark.trim();
            if selected.is_empty() {
                ui.weak("Blank landmark: selected maps will be appended without alignment.");
            } else {
                egui::Grid::new("landmark_status_grid")
                    .num_columns(2)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.strong("Map");
                        ui.strong(format!("`{selected}` status"));
                        ui.end_row();

                        for entry in &self.maps {
                            ui.label(file_name_or_path(&entry.path));
                            match &entry.analysis {
                                Ok(analysis) => {
                                    let (message, color) = landmark_status_label(
                                        &analysis.landmarks.status_for(selected),
                                    );
                                    ui.colored_label(color, message);
                                }
                                Err(error) => {
                                    ui.colored_label(
                                        egui::Color32::LIGHT_RED,
                                        format!("Parse failed: {error}"),
                                    );
                                }
                            }
                            ui.end_row();
                        }
                    });
            }

            let duplicate_lines = self
                .maps
                .iter()
                .filter_map(|entry| {
                    entry
                        .analysis
                        .as_ref()
                        .ok()
                        .map(|analysis| (entry, analysis))
                })
                .flat_map(|(entry, analysis)| {
                    analysis.landmarks.duplicates.iter().map(move |duplicate| {
                        format!(
                            "{} duplicates `{}` ({} entries, {} valid origin(s))",
                            file_name_or_path(&entry.path),
                            duplicate.targetname,
                            duplicate.count,
                            duplicate.valid_origins
                        )
                    })
                })
                .collect::<Vec<_>>();

            if !duplicate_lines.is_empty() {
                ui.separator();
                ui.colored_label(egui::Color32::YELLOW, "Duplicate landmark targetnames:");
                for line in duplicate_lines {
                    ui.small(line);
                }
            }
        });
    }

    fn integrity_status_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for entry in &self.maps {
            match &entry.analysis {
                Ok(analysis) => {
                    for issue in &analysis.integrity.issues {
                        lines.push(format!("Integrity {}", format_integrity_issue(issue)));
                    }
                }
                Err(error) => lines.push(format!(
                    "Integrity error: {}: parse/load failed: {error}",
                    display_path(&entry.path)
                )),
            }
        }
        lines
    }

    fn draw_integrity_status(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label("VMF integrity status");
            if self.maps.is_empty() {
                ui.weak("Add VMFs to run integrity checks.");
                return;
            }

            egui::Grid::new("integrity_status_grid")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    ui.strong("Map");
                    ui.strong("Status");
                    ui.end_row();

                    for entry in &self.maps {
                        ui.label(file_name_or_path(&entry.path));
                        match &entry.analysis {
                            Ok(analysis) => {
                                let errors = analysis.integrity.error_count();
                                let warnings = analysis.integrity.warning_count();
                                if errors > 0 {
                                    ui.colored_label(
                                        egui::Color32::LIGHT_RED,
                                        format!("{errors} error(s), {warnings} warning(s)"),
                                    );
                                } else if warnings > 0 {
                                    ui.colored_label(
                                        egui::Color32::YELLOW,
                                        format!("{warnings} warning(s)"),
                                    );
                                } else {
                                    ui.colored_label(egui::Color32::LIGHT_GREEN, "OK");
                                }
                            }
                            Err(error) => {
                                ui.colored_label(
                                    egui::Color32::LIGHT_RED,
                                    format!("Parse/load failed: {error}"),
                                );
                            }
                        }
                        ui.end_row();
                    }
                });

            let detail_lines = self.integrity_status_lines();
            if !detail_lines.is_empty() {
                ui.collapsing("Integrity details", |ui| {
                    for line in detail_lines {
                        ui.small(line);
                    }
                });
            }
        });
    }

    fn prepare_merge_inputs(&self) -> Result<(Vec<MergeInput>, DeletionReport), String> {
        if self.maps.len() < 2 {
            return Err("Merge preview needs at least two VMF files.".to_string());
        }
        if self.base_index >= self.maps.len() {
            return Err("Base map selection is invalid.".to_string());
        }

        let criteria = self.build_deletion_criteria();
        let mut ordered_indices = vec![self.base_index];
        ordered_indices.extend((0..self.maps.len()).filter(|index| *index != self.base_index));

        let mut merge_inputs = Vec::new();
        let mut removed_total = DeletionReport::default();
        for index in ordered_indices {
            let entry = &self.maps[index];
            let mut document = load_document(&entry.path)?;
            if !criteria.is_empty() {
                let report = prune_document(&mut document, &criteria);
                removed_total.removed_entities += report.removed_entities;
                removed_total.removed_world_solids += report.removed_world_solids;
            }
            merge_inputs.push(MergeInput {
                label: display_path(&entry.path),
                document,
            });
        }

        Ok((merge_inputs, removed_total))
    }

    fn build_merged_preview(&mut self) {
        let (merge_inputs, removed_total) = match self.prepare_merge_inputs() {
            Ok(prepared) => prepared,
            Err(error) => {
                self.add_status(error);
                return;
            }
        };

        for warning in self.landmark_warning_lines() {
            self.add_status(warning);
        }
        for line in self.integrity_status_lines() {
            self.add_status(line);
        }

        let landmark = blank_to_none(&self.landmark);
        match merge_maps(merge_inputs, &MergeOptions { landmark }) {
            Ok((document, report)) => {
                let preview = preview_document(&document);
                let summary = MergedPreviewSummary::from_reports(&report, &removed_total);
                self.merged_preview = Some(MergedPreview { preview, summary });
                self.preview_scope = PreviewScope::MergedResult;
                self.active_table = TableMode::Preview;
                self.preview_pan = egui::Vec2::ZERO;
                self.preview_zoom = 1.0;
                self.add_status(format!(
                    "Built merged preview for {} map(s): {} solids, {} entity origins.",
                    report.merged_maps,
                    self.merged_preview
                        .as_ref()
                        .map(|preview| preview.preview.solids.len())
                        .unwrap_or(0),
                    self.merged_preview
                        .as_ref()
                        .map(|preview| preview.preview.entities.len())
                        .unwrap_or(0)
                ));
                self.add_status(format!(
                    "Preview cleanup removed {} entities and {} world solids in memory; no VMF was written.",
                    removed_total.removed_entities, removed_total.removed_world_solids
                ));
            }
            Err(error) => self.add_status(format!("Merge preview failed: {error}")),
        }
    }

    fn preview_deletion(&mut self) {
        let criteria = self.build_deletion_criteria();
        if criteria.is_empty() {
            self.add_status("No deletion rules selected.");
            return;
        }

        let mut total = DeletionReport::default();
        let mut failures = 0;
        for entry in &self.maps {
            match load_document(&entry.path) {
                Ok(mut document) => {
                    let report = prune_document(&mut document, &criteria);
                    total.removed_entities += report.removed_entities;
                    total.removed_world_solids += report.removed_world_solids;
                }
                Err(_) => failures += 1,
            }
        }

        self.add_status(format!(
            "Preview: would remove {} entities and {} world solids across {} map(s).{}",
            total.removed_entities,
            total.removed_world_solids,
            self.maps.len().saturating_sub(failures),
            if failures == 0 {
                String::new()
            } else {
                format!(" {failures} map(s) could not be parsed.")
            }
        ));
    }

    fn save_cleaned_selected(&mut self) {
        let Some(index) = self.selected_map else {
            self.add_status("Select a VMF before writing a cleaned copy.");
            return;
        };
        let Some(entry) = self.maps.get(index) else {
            self.add_status("Selected VMF is no longer available.");
            return;
        };
        let criteria = self.build_deletion_criteria();
        if criteria.is_empty() {
            self.add_status("No deletion rules selected.");
            return;
        }

        let default_name = entry
            .path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| format!("{stem}_cleaned.vmf"))
            .unwrap_or_else(|| "cleaned.vmf".to_string());

        let Some(output_path) = rfd::FileDialog::new()
            .set_title("Save cleaned VMF")
            .add_filter("Valve Map Format", &["vmf"])
            .set_file_name(&default_name)
            .save_file()
        else {
            return;
        };

        match load_document(&entry.path) {
            Ok(mut document) => {
                let report = prune_document(&mut document, &criteria);
                let integrity = validate_document_integrity(&document, "cleaned output");
                for issue in integrity.warnings() {
                    self.add_status(format!("Integrity {}", format_integrity_issue(issue)));
                }
                if let Some(message) = integrity.error_message() {
                    self.add_status(format!("Cleaned output failed integrity checks: {message}"));
                    return;
                }
                match write_document(&output_path, &document) {
                    Ok(()) => self.add_status(format!(
                        "Wrote cleaned VMF: {}. Removed {} entities and {} world solids.",
                        display_path(&output_path),
                        report.removed_entities,
                        report.removed_world_solids
                    )),
                    Err(error) => self.add_status(error),
                }
            }
            Err(error) => self.add_status(error),
        }
    }

    fn merge_selected_maps(&mut self) {
        if self.output_path.trim().is_empty() {
            self.add_status("Choose an output VMF path before merging.");
            return;
        }

        let (merge_inputs, removed_total) = match self.prepare_merge_inputs() {
            Ok(prepared) => prepared,
            Err(error) => {
                self.add_status(error);
                return;
            }
        };

        for warning in self.landmark_warning_lines() {
            self.add_status(warning);
        }
        for line in self.integrity_status_lines() {
            self.add_status(line);
        }

        let landmark = blank_to_none(&self.landmark);
        match merge_maps(merge_inputs, &MergeOptions { landmark }) {
            Ok((document, report)) => {
                let integrity = validate_document_integrity(&document, "merged output");
                for issue in integrity.warnings() {
                    self.add_status(format!("Integrity {}", format_integrity_issue(issue)));
                }
                if let Some(message) = integrity.error_message() {
                    self.add_status(format!("Merged output failed integrity checks: {message}"));
                    return;
                }
                let output_path = PathBuf::from(self.output_path.trim());
                match write_document(&output_path, &document) {
                    Ok(()) => {
                        self.add_status(format!(
                            "Merged {} map(s) into {}.",
                            report.merged_maps,
                            display_path(&output_path)
                        ));
                        self.add_status(format!(
                            "Appended {} world solids and {} entities. Cleanup removed {} entities and {} world solids.",
                            report.appended_world_solids,
                            report.appended_entities,
                            removed_total.removed_entities,
                            removed_total.removed_world_solids
                        ));
                        for (label, offset) in report.applied_offsets {
                            self.add_status(format!("Offset {label}: {offset}"));
                        }
                    }
                    Err(error) => self.add_status(error),
                }
            }
            Err(error) => self.add_status(format!("Merge failed: {error}")),
        }
    }
}

impl eframe::App for SourceWeaverApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Source Weaver");
                ui.separator();
                if ui.button("Add VMFs...").clicked() {
                    self.add_vmf_files();
                }
                if ui.button("Re-scan").clicked() {
                    self.rescan_maps();
                }
                if ui.button("Remove selected").clicked() {
                    self.remove_selected_map();
                }
                if ui.button("Clear").clicked() {
                    self.clear_maps();
                }
            });
        });

        egui::SidePanel::left("maps")
            .resizable(true)
            .default_width(310.0)
            .show(ctx, |ui| {
                ui.heading("Selected VMFs");
                ui.label(
                    "First choose the maps. Then pick which one acts as the base output document.",
                );
                ui.separator();

                if self.maps.is_empty() {
                    ui.weak("No VMFs selected.");
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut next_base_index = self.base_index;
                    let mut next_selected_map = self.selected_map;
                    for (index, entry) in self.maps.iter().enumerate() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.radio_value(&mut next_base_index, index, "Base");
                                let selected = next_selected_map == Some(index);
                                if ui
                                    .selectable_label(selected, file_name_or_path(&entry.path))
                                    .on_hover_text(display_path(&entry.path))
                                    .clicked()
                                {
                                    next_selected_map = Some(index);
                                }
                            });
                            match &entry.analysis {
                                Ok(analysis) => {
                                    ui.small(format!(
                                        "{} records, {} classnames, {} landmarks, {} preview solids, {} integrity warning(s)",
                                        analysis.entity_records.len(),
                                        analysis.type_counts.len(),
                                        analysis.landmarks.targetnames.len(),
                                        analysis.preview.solids.len(),
                                        analysis.integrity.warning_count()
                                    ));
                                }
                                Err(error) => {
                                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                                }
                            }
                        });
                    }
                    self.base_index = next_base_index.min(self.maps.len().saturating_sub(1));
                    self.selected_map = next_selected_map.filter(|index| *index < self.maps.len());
                });
            });

        egui::TopBottomPanel::bottom("status")
            .resizable(true)
            .default_height(130.0)
            .show(ctx, |ui| {
                ui.heading("Status");
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in &self.status {
                            ui.label(line);
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.merge_panel(ui);
                ui.separator();
                self.cleanup_panel(ui);
                ui.separator();
                self.inspection_panel(ui);
            });
        });
    }
}

impl SourceWeaverApp {
    fn merge_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Merge setup");
        ui.horizontal(|ui| {
            ui.label("Base map:");
            let selected_text = self
                .maps
                .get(self.base_index)
                .map(|entry| file_name_or_path(&entry.path))
                .unwrap_or_else(|| "No VMF selected".to_string());
            egui::ComboBox::from_id_salt("base_map_combo")
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
                    for (index, entry) in self.maps.iter().enumerate() {
                        ui.selectable_value(
                            &mut self.base_index,
                            index,
                            file_name_or_path(&entry.path),
                        );
                    }
                });
        });

        let previous_landmark = self.landmark.clone();
        let landmark_options = self.discovered_landmark_options();
        ui.horizontal_wrapped(|ui| {
            ui.label("Landmark targetname:");
            egui::ComboBox::from_id_salt("landmark_targetname_combo")
                .width(280.0)
                .selected_text(if self.landmark.trim().is_empty() {
                    "Choose discovered landmark..."
                } else {
                    self.landmark.trim()
                })
                .show_ui(ui, |ui| {
                    if landmark_options.is_empty() {
                        ui.weak("No info_landmark targetnames discovered.");
                    } else {
                        for option in &landmark_options {
                            ui.selectable_value(
                                &mut self.landmark,
                                option.targetname.clone(),
                                option.label(),
                            );
                        }
                    }
                });
            ui.text_edit_singleline(&mut self.landmark)
                .on_hover_text("Manual entry remains available. Leave blank to append maps without landmark alignment.");
        });
        if self.landmark != previous_landmark {
            self.clear_merged_preview();
        }

        self.draw_landmark_status(ui);
        self.draw_integrity_status(ui);

        ui.horizontal(|ui| {
            ui.label("Output VMF:");
            ui.add(egui::TextEdit::singleline(&mut self.output_path).desired_width(f32::INFINITY));
            if ui.button("Browse...").clicked() {
                self.choose_output_path();
            }
        });

        ui.horizontal(|ui| {
            if ui.button("Preview selected merge").clicked() {
                self.build_merged_preview();
            }
            if ui.button("Merge selected VMFs").clicked() {
                self.merge_selected_maps();
            }
            ui.weak("Preview builds the same merge in memory without writing an output VMF.");
        });
        ui.weak("World solids, skybox brushes, point entities, and brush entities are appended from incoming maps.");
    }

    fn cleanup_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Bulk deletion rules");
        ui.label("These rules can preview removals, write a cleaned copy of the selected VMF, or be applied during merge.");

        egui::Grid::new("cleanup_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Drop classnames:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.drop_classnames)
                        .hint_text("prop_static, trigger_once, info_player_start"),
                );
                ui.end_row();

                ui.label("Drop targetnames:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.drop_targetnames)
                        .hint_text("cleanup_me, transition_trigger"),
                );
                ui.end_row();
            });

        ui.collapsing("Drop by brush role", |ui| {
            ui.columns(3, |columns| {
                for (index, option) in self.role_options.iter_mut().enumerate() {
                    columns[index % 3].checkbox(&mut option.selected, option.label);
                }
            });
        });

        ui.horizontal(|ui| {
            if ui.button("Preview deletion").clicked() {
                self.preview_deletion();
            }
            if ui.button("Save cleaned selected VMF...").clicked() {
                self.save_cleaned_selected();
            }
        });
    }

    fn inspection_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Map view and inspection");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_table, TableMode::Preview, "Preview");
            ui.selectable_value(&mut self.active_table, TableMode::Entities, "Entities");
            ui.selectable_value(&mut self.active_table, TableMode::Classnames, "Classnames");
        });

        let Some(index) = self.selected_map else {
            ui.weak("Select a VMF on the left to preview and inspect it.");
            return;
        };
        let Some(entry) = self.maps.get(index) else {
            ui.weak("Selected VMF is unavailable.");
            return;
        };

        let entry_path = entry.path.clone();
        let path = display_path(&entry_path);
        let analysis = entry.analysis.clone();
        match analysis {
            Ok(analysis) => match self.active_table {
                TableMode::Preview => {
                    self.draw_preview_scope_controls(ui, &path);
                    match self.preview_scope {
                        PreviewScope::SelectedMap => {
                            ui.label(format!("Selected VMF preview: {path}"));
                            self.draw_preview_panel(ui, &analysis.preview, None);
                        }
                        PreviewScope::MergedResult => {
                            if let Some(merged_preview) = self.merged_preview.clone() {
                                ui.label("Merged-output preview: current in-memory result");
                                self.draw_preview_panel(
                                    ui,
                                    &merged_preview.preview,
                                    Some(&merged_preview.summary),
                                );
                            } else {
                                ui.colored_label(
                                    egui::Color32::YELLOW,
                                    "No merged preview has been built yet. Click Preview selected merge.",
                                );
                                self.preview_scope = PreviewScope::SelectedMap;
                                self.draw_preview_panel(ui, &analysis.preview, None);
                            }
                        }
                    }
                }
                TableMode::Entities => {
                    ui.label(&path);
                    draw_entity_table(
                        ui,
                        &entry_path,
                        &analysis.entity_records,
                        &mut self.selected_entity_rows,
                    );
                }
                TableMode::Classnames => {
                    ui.label(&path);
                    draw_classname_table(ui, &analysis.type_counts);
                }
            },
            Err(error) => {
                ui.label(&path);
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
        }
    }

    fn draw_preview_scope_controls(&mut self, ui: &mut egui::Ui, selected_path: &str) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Preview source:");
            ui.radio_value(
                &mut self.preview_scope,
                PreviewScope::SelectedMap,
                "Selected VMF",
            );
            let merged_enabled = self.merged_preview.is_some();
            ui.add_enabled_ui(merged_enabled, |ui| {
                ui.radio_value(
                    &mut self.preview_scope,
                    PreviewScope::MergedResult,
                    "Merged result",
                );
            });
            if !merged_enabled {
                ui.weak("Click Preview selected merge to build a merged result.");
            }
            ui.separator();
            ui.weak(format!("Selected: {selected_path}"));
        });
    }

    fn draw_preview_panel(
        &mut self,
        ui: &mut egui::Ui,
        preview: &PreviewDocument,
        merged_summary: Option<&MergedPreviewSummary>,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.label("View:");
            ui.radio_value(&mut self.preview_view, PreviewView::Top, "Top X/Y");
            ui.radio_value(&mut self.preview_view, PreviewView::Front, "Front X/Z");
            ui.radio_value(&mut self.preview_view, PreviewView::Side, "Side Y/Z");
            ui.separator();
            ui.checkbox(&mut self.preview_show_grid, "Grid");
            ui.checkbox(&mut self.preview_show_solids, "Solids");
            ui.checkbox(&mut self.preview_show_entities, "Entities");
            if ui.button("Reset view").clicked() {
                self.preview_zoom = 1.0;
                self.preview_pan = egui::Vec2::ZERO;
            }
        });

        ui.horizontal_wrapped(|ui| {
            ui.label(format!("{} preview solids", preview.solids.len()));
            ui.separator();
            ui.label(format!("{} entity origins", preview.entities.len()));
            ui.separator();
            ui.add(egui::Slider::new(&mut self.preview_zoom, 0.1..=12.0).text("Zoom"));
            ui.weak("Mouse wheel zooms. Drag the preview to pan.");
        });

        if let Some(summary) = merged_summary {
            ui.group(|ui| {
                ui.label(format!(
                    "Merged preview: {} map(s), appended {} world solids and {} entities.",
                    summary.merged_maps, summary.appended_world_solids, summary.appended_entities
                ));
                ui.label(format!(
                    "Cleanup applied in memory: removed {} entities and {} world solids. No output VMF was written.",
                    summary.removed_entities, summary.removed_world_solids
                ));
                for offset in &summary.offsets {
                    ui.small(offset);
                }
            });
        }

        let desired_height = 560.0_f32.max(ui.available_height().min(720.0));
        let desired_size = egui::vec2(ui.available_width().max(360.0), desired_height);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::drag());
        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(21, 24, 28));
        draw_rect_outline(
            &painter,
            rect,
            egui::Stroke::new(1.0_f32, egui::Color32::DARK_GRAY),
        );

        if response.dragged() {
            self.preview_pan += ui.input(|input| input.pointer.delta());
        }
        if response.hovered() {
            let scroll_delta =
                ui.input(|input| input.smooth_scroll_delta.y + input.raw_scroll_delta.y);
            if scroll_delta.abs() > f32::EPSILON {
                let factor = (1.0 + scroll_delta * 0.0015).clamp(0.85, 1.18);
                self.preview_zoom = (self.preview_zoom * factor).clamp(0.1, 12.0);
            }
        }

        let Some(bounds) = preview.bounds else {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No brush planes or entity origins found in this VMF.",
                egui::FontId::proportional(16.0),
                egui::Color32::LIGHT_GRAY,
            );
            return;
        };

        let transform = PreviewTransform::new(
            rect,
            bounds,
            self.preview_view,
            self.preview_zoom,
            self.preview_pan,
        );
        if self.preview_show_grid {
            draw_preview_grid(&painter, rect, &transform);
        }
        draw_axes_label(&painter, rect, self.preview_view);

        if self.preview_show_solids {
            for solid in &preview.solids {
                draw_preview_solid(&painter, &transform, solid);
            }
        }

        if self.preview_show_entities {
            for entity in &preview.entities {
                let position = transform.world_to_screen(entity.origin);
                if rect.contains(position) {
                    painter.circle_filled(position, 4.5, egui::Color32::from_rgb(255, 232, 128));
                    painter.circle_stroke(
                        position,
                        6.5,
                        egui::Stroke::new(1.0_f32, egui::Color32::BLACK),
                    );
                    let label = entity
                        .targetname
                        .as_deref()
                        .or(entity.classname.as_deref())
                        .unwrap_or("entity");
                    painter.text(
                        position + egui::vec2(7.0, -7.0),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::monospace(10.0),
                        egui::Color32::from_rgb(255, 245, 180),
                    );
                }
            }
        }

        draw_preview_legend(&painter, rect);
    }
}

impl MergedPreviewSummary {
    fn from_reports(report: &MergeReport, deletion: &DeletionReport) -> Self {
        Self {
            merged_maps: report.merged_maps,
            appended_world_solids: report.appended_world_solids,
            appended_entities: report.appended_entities,
            removed_entities: deletion.removed_entities,
            removed_world_solids: deletion.removed_world_solids,
            offsets: report
                .applied_offsets
                .iter()
                .map(|(label, offset)| format!("Offset {label}: {offset}"))
                .collect(),
        }
    }
}

impl MapEntry {
    fn load(path: PathBuf) -> Self {
        let analysis = match load_document(&path) {
            Ok(document) => {
                let label = display_path(&path);
                Ok(MapAnalysis {
                    entity_records: inspect_entities(&document),
                    type_counts: summarize_entity_types(&document),
                    preview: preview_document(&document),
                    landmarks: discover_landmarks(&document),
                    integrity: validate_document_integrity(&document, &label),
                })
            }
            Err(error) => Err(error),
        };
        Self { path, analysis }
    }
}

impl RoleOption {
    fn new(label: &'static str, role: BrushRole) -> Self {
        Self {
            label,
            role,
            selected: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PreviewTransform {
    rect: egui::Rect,
    bounds: PreviewBounds,
    view: PreviewView,
    scale: f32,
    center_u: f64,
    center_v: f64,
    pan: egui::Vec2,
}

impl PreviewTransform {
    fn new(
        rect: egui::Rect,
        bounds: PreviewBounds,
        view: PreviewView,
        zoom: f32,
        pan: egui::Vec2,
    ) -> Self {
        let (min_u, min_v) = project_vec(bounds.min, view);
        let (max_u, max_v) = project_vec(bounds.max, view);
        let extent_u = (max_u - min_u).abs().max(1.0);
        let extent_v = (max_v - min_v).abs().max(1.0);
        let fit_scale = ((rect.width() * 0.88) as f64 / extent_u)
            .min((rect.height() * 0.88) as f64 / extent_v)
            .clamp(0.001, 128.0) as f32;
        Self {
            rect,
            bounds,
            view,
            scale: fit_scale * zoom,
            center_u: (min_u + max_u) * 0.5,
            center_v: (min_v + max_v) * 0.5,
            pan,
        }
    }

    fn world_to_screen(&self, point: sourceweaver_core::Vec3) -> egui::Pos2 {
        let (u, v) = project_vec(point, self.view);
        self.uv_to_screen(u, v)
    }

    fn uv_to_screen(&self, u: f64, v: f64) -> egui::Pos2 {
        self.rect.center()
            + self.pan
            + egui::vec2(
                ((u - self.center_u) as f32) * self.scale,
                -((v - self.center_v) as f32) * self.scale,
            )
    }
}

fn draw_entity_table(
    ui: &mut egui::Ui,
    map_path: &Path,
    records: &[EntityRecord],
    selected_rows: &mut BTreeSet<EntitySelectionKey>,
) {
    let row_keys = records
        .iter()
        .map(|record| entity_selection_key(map_path, record))
        .collect::<Vec<_>>();
    let current_selected = row_keys
        .iter()
        .filter(|key| selected_rows.contains(key))
        .count();

    ui.horizontal_wrapped(|ui| {
        ui.label(format!("{} world/entity records", records.len()));
        ui.separator();
        ui.label(format!(
            "{} selected in this map, {} selected total",
            current_selected,
            selected_rows.len()
        ));
        ui.separator();
        if ui.button("Select all rows").clicked() {
            selected_rows.extend(row_keys.iter().cloned());
        }
        if ui.button("Clear current map").clicked() {
            for key in &row_keys {
                selected_rows.remove(key);
            }
        }
        if ui.button("Clear all selections").clicked() {
            selected_rows.clear();
        }
    });

    ui.weak("Selections are tracked by VMF path, row index, block name, classname, and targetname so later deletion actions can target rows safely.");

    egui::ScrollArea::both().max_height(360.0).show(ui, |ui| {
        egui::Grid::new("entity_table")
            .striped(true)
            .num_columns(8)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.strong("Select");
                ui.strong("#");
                ui.strong("Block");
                ui.strong("Classname");
                ui.strong("Targetname");
                ui.strong("Origin");
                ui.strong("Solids");
                ui.strong("Roles");
                ui.end_row();

                for (record, key) in records.iter().zip(row_keys.iter()) {
                    let mut selected = selected_rows.contains(key);
                    if ui.checkbox(&mut selected, "").changed() {
                        if selected {
                            selected_rows.insert(key.clone());
                        } else {
                            selected_rows.remove(key);
                        }
                    }
                    ui.label(record.index.to_string());
                    ui.label(&record.block_name);
                    ui.label(record.classname.as_deref().unwrap_or("-"));
                    ui.label(record.targetname.as_deref().unwrap_or("-"));
                    ui.label(
                        record
                            .origin
                            .map(|origin| origin.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                    );
                    ui.label(record.solid_count.to_string());
                    ui.label(format_roles(&record.roles));
                    ui.end_row();
                }
            });
    });
}

fn entity_selection_key(map_path: &Path, record: &EntityRecord) -> EntitySelectionKey {
    EntitySelectionKey {
        map_path: display_path(map_path),
        record_index: record.index,
        block_name: record.block_name.clone(),
        classname: record.classname.clone(),
        targetname: record.targetname.clone(),
    }
}

fn draw_classname_table(ui: &mut egui::Ui, type_counts: &BTreeMap<String, usize>) {
    ui.label(format!("{} detected classnames", type_counts.len()));
    egui::ScrollArea::both().max_height(360.0).show(ui, |ui| {
        egui::Grid::new("classname_table")
            .striped(true)
            .num_columns(2)
            .spacing([24.0, 6.0])
            .show(ui, |ui| {
                ui.strong("Count");
                ui.strong("Classname");
                ui.end_row();
                for (classname, count) in type_counts {
                    ui.label(count.to_string());
                    ui.label(classname);
                    ui.end_row();
                }
            });
    });
}

fn draw_preview_grid(painter: &egui::Painter, rect: egui::Rect, transform: &PreviewTransform) {
    let (min_u, min_v) = project_vec(transform.bounds.min, transform.view);
    let (max_u, max_v) = project_vec(transform.bounds.max, transform.view);
    let pad_u = ((max_u - min_u).abs() * 0.25).max(512.0);
    let pad_v = ((max_v - min_v).abs() * 0.25).max(512.0);
    let step = nice_grid_step(transform.scale);
    let start_u = ((min_u - pad_u) / step).floor() as i64;
    let end_u = ((max_u + pad_u) / step).ceil() as i64;
    let start_v = ((min_v - pad_v) / step).floor() as i64;
    let end_v = ((max_v + pad_v) / step).ceil() as i64;
    let stroke = egui::Stroke::new(
        1.0_f32,
        egui::Color32::from_rgba_unmultiplied(85, 93, 105, 55),
    );
    let axis_stroke = egui::Stroke::new(
        1.5_f32,
        egui::Color32::from_rgba_unmultiplied(130, 148, 176, 120),
    );

    for i in start_u.max(-500)..=end_u.min(500) {
        let u = i as f64 * step;
        let a = transform.uv_to_screen(u, min_v - pad_v);
        let b = transform.uv_to_screen(u, max_v + pad_v);
        painter.line_segment([a, b], if i == 0 { axis_stroke } else { stroke });
    }
    for i in start_v.max(-500)..=end_v.min(500) {
        let v = i as f64 * step;
        let a = transform.uv_to_screen(min_u - pad_u, v);
        let b = transform.uv_to_screen(max_u + pad_u, v);
        painter.line_segment([a, b], if i == 0 { axis_stroke } else { stroke });
    }

    painter.text(
        rect.left_bottom() + egui::vec2(10.0, -12.0),
        egui::Align2::LEFT_BOTTOM,
        format!("grid {}u", step as i64),
        egui::FontId::monospace(10.0),
        egui::Color32::from_gray(150),
    );
}

fn draw_preview_solid(painter: &egui::Painter, transform: &PreviewTransform, solid: &PreviewSolid) {
    let (min_u, min_v) = project_vec(solid.bounds.min, transform.view);
    let (max_u, max_v) = project_vec(solid.bounds.max, transform.view);
    let a = transform.uv_to_screen(min_u, min_v);
    let b = transform.uv_to_screen(max_u, max_v);
    let rect = egui::Rect::from_two_pos(a, b);
    let color = solid_color(solid);

    if rect.intersects(transform.rect) {
        painter.rect_filled(rect, 0.0, color.gamma_multiply(0.18));
        draw_rect_outline(painter, rect, egui::Stroke::new(1.25_f32, color));
    }

    for chunk in solid.points.chunks(3) {
        if chunk.len() == 3 {
            let p0 = transform.world_to_screen(chunk[0]);
            let p1 = transform.world_to_screen(chunk[1]);
            let p2 = transform.world_to_screen(chunk[2]);
            painter.line_segment(
                [p0, p1],
                egui::Stroke::new(0.8_f32, color.gamma_multiply(0.7)),
            );
            painter.line_segment(
                [p1, p2],
                egui::Stroke::new(0.8_f32, color.gamma_multiply(0.7)),
            );
            painter.line_segment(
                [p2, p0],
                egui::Stroke::new(0.8_f32, color.gamma_multiply(0.7)),
            );
        }
    }
}

fn draw_preview_legend(painter: &egui::Painter, rect: egui::Rect) {
    let items = [
        ("world", egui::Color32::from_rgb(170, 190, 220)),
        ("skybox", egui::Color32::from_rgb(100, 170, 255)),
        ("trigger", egui::Color32::from_rgb(255, 170, 90)),
        ("clip", egui::Color32::from_rgb(230, 95, 95)),
        ("areaportal", egui::Color32::from_rgb(185, 115, 255)),
        ("water", egui::Color32::from_rgb(80, 210, 230)),
        ("entity origin", egui::Color32::from_rgb(255, 232, 128)),
    ];
    let mut cursor = rect.left_top() + egui::vec2(12.0, 12.0);
    for (label, color) in items {
        let swatch = egui::Rect::from_min_size(cursor, egui::vec2(10.0, 10.0));
        painter.rect_filled(swatch, 1.0, color);
        painter.text(
            cursor + egui::vec2(16.0, 5.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::monospace(10.0),
            egui::Color32::from_gray(220),
        );
        cursor.y += 16.0;
    }
}

fn draw_axes_label(painter: &egui::Painter, rect: egui::Rect, view: PreviewView) {
    let label = match view {
        PreviewView::Top => "Top view: X / Y",
        PreviewView::Front => "Front view: X / Z",
        PreviewView::Side => "Side view: Y / Z",
    };
    painter.text(
        rect.right_top() + egui::vec2(-12.0, 12.0),
        egui::Align2::RIGHT_TOP,
        label,
        egui::FontId::monospace(12.0),
        egui::Color32::from_gray(210),
    );
}

fn draw_rect_outline(painter: &egui::Painter, rect: egui::Rect, stroke: egui::Stroke) {
    painter.line_segment([rect.left_top(), rect.right_top()], stroke);
    painter.line_segment([rect.right_top(), rect.right_bottom()], stroke);
    painter.line_segment([rect.right_bottom(), rect.left_bottom()], stroke);
    painter.line_segment([rect.left_bottom(), rect.left_top()], stroke);
}

fn project_vec(point: sourceweaver_core::Vec3, view: PreviewView) -> (f64, f64) {
    match view {
        PreviewView::Top => (point.x, point.y),
        PreviewView::Front => (point.x, point.z),
        PreviewView::Side => (point.y, point.z),
    }
}

fn nice_grid_step(scale: f32) -> f64 {
    let target_world = (72.0_f64 / scale.max(0.001) as f64).max(1.0);
    let base = 10_f64.powf(target_world.log10().floor());
    for multiplier in [1.0, 2.0, 4.0, 8.0, 16.0] {
        let candidate = base * multiplier;
        if candidate >= target_world {
            return candidate;
        }
    }
    base * 32.0
}

fn solid_color(solid: &PreviewSolid) -> egui::Color32 {
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

fn load_document(path: impl AsRef<Path>) -> Result<Document, String> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read {}: {error}", display_path(path)))?;
    Document::parse(&text)
        .map_err(|error| format!("Failed to parse {}: {error}", display_path(path)))
}

fn write_document(path: impl AsRef<Path>, document: &Document) -> Result<(), String> {
    let path = path.as_ref();
    fs::write(path, document.to_vmf_string())
        .map_err(|error| format!("Failed to write {}: {error}", display_path(path)))
}

fn split_csv(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
}

fn blank_to_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn format_roles(roles: &[BrushRole]) -> String {
    if roles.is_empty() {
        return "-".to_string();
    }
    roles
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn landmark_status_label(status: &LandmarkTargetStatus) -> (String, egui::Color32) {
    match status {
        LandmarkTargetStatus::Blank => (
            "No alignment requested".to_string(),
            egui::Color32::LIGHT_GRAY,
        ),
        LandmarkTargetStatus::Missing => (
            "Missing; map will be unshifted".to_string(),
            egui::Color32::YELLOW,
        ),
        LandmarkTargetStatus::Present { origin } => {
            (format!("Present at {origin}"), egui::Color32::LIGHT_GREEN)
        }
        LandmarkTargetStatus::InvalidOrigin { .. } => (
            "Found, but origin is missing or invalid".to_string(),
            egui::Color32::YELLOW,
        ),
        LandmarkTargetStatus::Duplicate {
            count,
            valid_origins,
        } => (
            format!("Duplicate: {count} entries, {valid_origins} valid origin(s)"),
            egui::Color32::YELLOW,
        ),
    }
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn file_name_or_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| display_path(path))
}
