use eframe::egui;
use sourceweaver_core::{
    BrushRole, DeletionCriteria, DeletionReport, Document, EntityRecord, MergeInput, MergeOptions,
    inspect_entities, merge_maps, prune_document, summarize_entity_types,
};
use std::collections::BTreeMap;
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TableMode {
    Entities,
    Classnames,
}

#[derive(Debug, Clone)]
struct RoleOption {
    label: &'static str,
    role: BrushRole,
    selected: bool,
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
            active_table: TableMode::Entities,
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
            self.add_status(format!("Added {added} VMF file(s)."));
        }
    }

    fn rescan_maps(&mut self) {
        for map in &mut self.maps {
            let path = map.path.clone();
            *map = MapEntry::load(path);
        }
        self.add_status("Re-scanned selected VMFs from disk.");
    }

    fn clear_maps(&mut self) {
        self.maps.clear();
        self.selected_map = None;
        self.base_index = 0;
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
        if self.maps.len() < 2 {
            self.add_status("Merge needs at least two VMF files.");
            return;
        }
        if self.output_path.trim().is_empty() {
            self.add_status("Choose an output VMF path before merging.");
            return;
        }
        if self.base_index >= self.maps.len() {
            self.add_status("Base map selection is invalid.");
            return;
        }

        let criteria = self.build_deletion_criteria();
        let mut ordered_indices = vec![self.base_index];
        ordered_indices.extend((0..self.maps.len()).filter(|index| *index != self.base_index));

        let mut merge_inputs = Vec::new();
        let mut removed_total = DeletionReport::default();
        for index in ordered_indices {
            let entry = &self.maps[index];
            match load_document(&entry.path) {
                Ok(mut document) => {
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
                Err(error) => {
                    self.add_status(error);
                    return;
                }
            }
        }

        let landmark = blank_to_none(&self.landmark);
        match merge_maps(merge_inputs, &MergeOptions { landmark }) {
            Ok((document, report)) => {
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
                                        "{} records, {} classnames",
                                        analysis.entity_records.len(),
                                        analysis.type_counts.len()
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

        ui.horizontal(|ui| {
            ui.label("Landmark targetname:");
            ui.text_edit_singleline(&mut self.landmark)
                .on_hover_text("Leave blank to append maps without landmark alignment.");
        });

        ui.horizontal(|ui| {
            ui.label("Output VMF:");
            ui.add(egui::TextEdit::singleline(&mut self.output_path).desired_width(f32::INFINITY));
            if ui.button("Browse...").clicked() {
                self.choose_output_path();
            }
        });

        ui.horizontal(|ui| {
            if ui.button("Merge selected VMFs").clicked() {
                self.merge_selected_maps();
            }
            ui.weak("World solids, skybox brushes, point entities, and brush entities are appended from incoming maps.");
        });
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
        ui.heading("Inspection");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_table, TableMode::Entities, "Entities");
            ui.selectable_value(&mut self.active_table, TableMode::Classnames, "Classnames");
        });

        let Some(index) = self.selected_map else {
            ui.weak("Select a VMF on the left to inspect entities and classnames.");
            return;
        };
        let Some(entry) = self.maps.get(index) else {
            ui.weak("Selected VMF is unavailable.");
            return;
        };

        ui.label(display_path(&entry.path));
        match &entry.analysis {
            Ok(analysis) => match self.active_table {
                TableMode::Entities => draw_entity_table(ui, &analysis.entity_records),
                TableMode::Classnames => draw_classname_table(ui, &analysis.type_counts),
            },
            Err(error) => {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
        }
    }
}

impl MapEntry {
    fn load(path: PathBuf) -> Self {
        let analysis = match load_document(&path) {
            Ok(document) => Ok(MapAnalysis {
                entity_records: inspect_entities(&document),
                type_counts: summarize_entity_types(&document),
            }),
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

fn draw_entity_table(ui: &mut egui::Ui, records: &[EntityRecord]) {
    ui.label(format!("{} world/entity records", records.len()));
    egui::ScrollArea::both().max_height(360.0).show(ui, |ui| {
        egui::Grid::new("entity_table")
            .striped(true)
            .num_columns(7)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.strong("#");
                ui.strong("Block");
                ui.strong("Classname");
                ui.strong("Targetname");
                ui.strong("Origin");
                ui.strong("Solids");
                ui.strong("Roles");
                ui.end_row();

                for record in records {
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

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn file_name_or_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| display_path(path))
}
