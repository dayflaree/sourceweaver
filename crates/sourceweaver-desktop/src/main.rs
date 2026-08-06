use eframe::egui;
use serde::{Deserialize, Serialize};
use sourceweaver_core::{
    BrushEntityDeletionMode, BrushRole, CampaignMapInput, CampaignOrderSuggestion,
    CampaignTransition, DeletionCriteria, DeletionReport, Document, EntityMetadata, EntityRecord,
    IntegrityReport, LandmarkDiscovery, LandmarkTargetStatus, MergeInput, MergeOptions,
    MergeReport, PreviewBounds, PreviewDocument, PreviewEntityMarker, PreviewSolid,
    combine_preview_documents, discover_landmarks, discover_transitions, format_integrity_issue,
    inspect_entities, is_critical_entity_classname, merge_maps,
    metadata_for_classname_with_overrides, parse_fgd_metadata, preview_document,
    preview_document_with_source, prune_document, suggest_campaign_order, summarize_entity_types,
    translate_preview_document, validate_document_integrity,
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
    drop_all_entities: bool,
    brush_entity_mode: BrushEntityDeletionMode,
    protect_critical_entities: bool,
    pending_deletion_review: Option<PendingDeletionReview>,
    cleanup_export_confirmed: bool,
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
    preview_deletion_mode: DeletionPreviewMode,
    selected_entity_rows: BTreeSet<EntitySelectionKey>,
    entity_search: String,
    entity_role_filter: Option<BrushRole>,
    entity_sort_column: EntitySortColumn,
    entity_sort_ascending: bool,
    classname_search: String,
    classname_sort_column: ClassnameSortColumn,
    classname_sort_ascending: bool,
    fgd_metadata: BTreeMap<String, EntityMetadata>,
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
    transitions: Vec<CampaignTransition>,
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
    removed_brush_entity_solids: usize,
    source_labels: Vec<String>,
    source_offsets: Vec<(String, sourceweaver_core::Vec3)>,
    offsets: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
struct PreviewDeletionCounts {
    solids: usize,
    entities: usize,
}

#[derive(Debug, Clone)]
struct PendingDeletionReview {
    criteria: DeletionCriteria,
    report: DeletionReport,
    maps_checked: usize,
    failures: usize,
    label: String,
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
    Transitions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewView {
    Top,
    Front,
    Side,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeletionPreviewMode {
    Off,
    HighlightRemoved,
    DimRemoved,
    HideRemoved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntitySortColumn {
    Index,
    Block,
    Category,
    Classname,
    Targetname,
    Origin,
    Solids,
    Roles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClassnameSortColumn {
    Classname,
    Count,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeletionPresetKind {
    RemoveTriggers,
    RemoveClips,
    RemoveAreaportals,
    RemoveGameplayLogic,
    KeepWorldGeometry,
    KeepWorldAndSkybox,
}

#[derive(Debug, Clone, Copy)]
struct DeletionPresetSpec {
    kind: DeletionPresetKind,
    name: &'static str,
    description: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectFile {
    base: String,
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    landmark: Option<String>,
    #[serde(default)]
    delete: ProjectDeleteConfig,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    report: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectDeleteConfig {
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
    #[serde(default = "default_project_protect_critical_entities")]
    protect_critical_entities: bool,
}

impl Default for ProjectDeleteConfig {
    fn default() -> Self {
        Self {
            classnames: Vec::new(),
            targetnames: Vec::new(),
            roles: Vec::new(),
            all_entities: false,
            brush_entity_mode: Some(BrushEntityDeletionMode::WholeEntity.to_string()),
            protect_critical_entities: true,
        }
    }
}

impl ProjectDeleteConfig {
    fn from_criteria(criteria: &DeletionCriteria) -> Self {
        Self {
            classnames: criteria.classnames.iter().cloned().collect(),
            targetnames: criteria.targetnames.iter().cloned().collect(),
            roles: criteria
                .brush_roles
                .iter()
                .map(ToString::to_string)
                .collect(),
            all_entities: criteria.drop_all_entities,
            brush_entity_mode: Some(criteria.brush_entity_mode.to_string()),
            protect_critical_entities: criteria.protect_critical_entities,
        }
    }

    fn to_criteria(&self) -> Result<DeletionCriteria, String> {
        let mut criteria = DeletionCriteria::default();
        criteria.classnames.extend(self.classnames.iter().cloned());
        criteria
            .targetnames
            .extend(self.targetnames.iter().cloned());
        criteria.drop_all_entities = self.all_entities;
        criteria.protect_critical_entities = self.protect_critical_entities;
        if let Some(mode) = &self.brush_entity_mode {
            criteria.brush_entity_mode = BrushEntityDeletionMode::parse(mode)
                .ok_or_else(|| format!("unknown delete.brush_entity_mode `{mode}`"))?;
        }
        for role in &self.roles {
            let parsed = BrushRole::parse_filter(role)
                .ok_or_else(|| format!("unknown delete role `{role}`"))?;
            criteria.brush_roles.insert(parsed);
        }
        Ok(criteria)
    }
}

fn default_project_protect_critical_entities() -> bool {
    true
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
            drop_all_entities: false,
            brush_entity_mode: BrushEntityDeletionMode::WholeEntity,
            protect_critical_entities: true,
            pending_deletion_review: None,
            cleanup_export_confirmed: false,
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
            preview_deletion_mode: DeletionPreviewMode::HighlightRemoved,
            selected_entity_rows: BTreeSet::new(),
            entity_search: String::new(),
            entity_role_filter: None,
            entity_sort_column: EntitySortColumn::Index,
            entity_sort_ascending: true,
            classname_search: String::new(),
            classname_sort_column: ClassnameSortColumn::Classname,
            classname_sort_ascending: true,
            fgd_metadata: BTreeMap::new(),
        }
    }

    fn add_status(&mut self, message: impl Into<String>) {
        self.status.push(message.into());
        if self.status.len() > 12 {
            let overflow = self.status.len() - 12;
            self.status.drain(0..overflow);
        }
    }

    fn load_fgd_dialog(&mut self) {
        let Some(paths) = rfd::FileDialog::new()
            .set_title("Load Hammer FGD metadata")
            .add_filter("Forge Game Data", &["fgd"])
            .pick_files()
        else {
            return;
        };

        let mut loaded_files = 0;
        let mut loaded_classes = 0;
        for path in paths {
            match fs::read_to_string(&path) {
                Ok(text) => {
                    let entries = parse_fgd_metadata(&text, &display_path(&path));
                    loaded_classes += entries.len();
                    for entry in entries {
                        self.fgd_metadata.insert(entry.classname.clone(), entry);
                    }
                    loaded_files += 1;
                }
                Err(error) => self.add_status(format!(
                    "Failed to load FGD {}: {error}",
                    display_path(&path)
                )),
            }
        }
        self.add_status(format!(
            "Loaded {loaded_classes} FGD class metadata record(s) from {loaded_files} file(s)."
        ));
    }

    fn save_project_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Save Source Weaver project")
            .add_filter("Source Weaver project", &["toml"])
            .set_file_name("project.sourceweaver.toml")
            .save_file()
        else {
            return;
        };

        match self.project_file_for_path(&path).and_then(|project| {
            toml::to_string_pretty(&project)
                .map_err(|error| format!("failed to serialize project: {error}"))
        }) {
            Ok(toml) => match fs::write(&path, toml) {
                Ok(()) => self.add_status(format!("Saved project {}.", display_path(&path))),
                Err(error) => self.add_status(format!(
                    "Failed to save project {}: {error}",
                    display_path(&path)
                )),
            },
            Err(error) => self.add_status(error),
        }
    }

    fn load_project_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Load Source Weaver project or job")
            .add_filter("TOML project/job", &["toml"])
            .pick_file()
        else {
            return;
        };

        match fs::read_to_string(&path)
            .map_err(|error| format!("failed to read project {}: {error}", display_path(&path)))
            .and_then(|text| {
                toml::from_str::<ProjectFile>(&text).map_err(|error| {
                    format!("failed to parse project {}: {error}", display_path(&path))
                })
            }) {
            Ok(project) => match self.load_project_file(&path, project) {
                Ok(()) => self.add_status(format!("Loaded project {}.", display_path(&path))),
                Err(error) => self.add_status(error),
            },
            Err(error) => self.add_status(error),
        }
    }

    fn project_file_for_path(&self, project_path: &Path) -> Result<ProjectFile, String> {
        let base_dir = project_path.parent().unwrap_or_else(|| Path::new("."));
        let base = self
            .maps
            .get(self.base_index)
            .ok_or("Add at least one VMF before saving a project.")?;
        let inputs = self
            .maps
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != self.base_index)
            .map(|(_, entry)| project_relative_path(&entry.path, base_dir))
            .collect::<Vec<_>>();
        let criteria = self.build_deletion_criteria();

        Ok(ProjectFile {
            base: project_relative_path(&base.path, base_dir),
            inputs,
            output: blank_to_none(&self.output_path)
                .map(|_| project_relative_path(&PathBuf::from(self.output_path.trim()), base_dir)),
            landmark: blank_to_none(&self.landmark),
            delete: ProjectDeleteConfig::from_criteria(&criteria),
            dry_run: false,
            report: None,
        })
    }

    fn load_project_file(
        &mut self,
        project_path: &Path,
        project: ProjectFile,
    ) -> Result<(), String> {
        let base_dir = project_path.parent().unwrap_or_else(|| Path::new("."));
        let mut paths = Vec::with_capacity(project.inputs.len() + 1);
        paths.push(resolve_project_path(base_dir, &project.base));
        paths.extend(
            project
                .inputs
                .iter()
                .map(|input| resolve_project_path(base_dir, input)),
        );

        let criteria = project.delete.to_criteria()?;
        self.maps = paths.into_iter().map(MapEntry::load).collect();
        self.selected_map = (!self.maps.is_empty()).then_some(0);
        self.base_index = 0;
        self.landmark = project.landmark.unwrap_or_default();
        self.output_path = project
            .output
            .map(|output| display_path(&resolve_project_path(base_dir, &output)))
            .unwrap_or_default();
        self.apply_deletion_criteria_to_controls(criteria);
        self.selected_entity_rows.clear();
        self.preview_pan = egui::Vec2::ZERO;
        self.preview_zoom = 1.0;
        self.active_table = TableMode::Preview;
        Ok(())
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
        criteria.drop_all_entities = self.drop_all_entities;
        criteria.brush_entity_mode = self.brush_entity_mode;
        criteria.protect_critical_entities = self.protect_critical_entities;
        criteria
    }

    fn apply_deletion_criteria_to_controls(&mut self, criteria: DeletionCriteria) {
        self.drop_classnames = criteria
            .classnames
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        self.drop_targetnames = criteria
            .targetnames
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        for option in &mut self.role_options {
            option.selected = criteria.brush_roles.contains(&option.role);
        }
        self.drop_all_entities = criteria.drop_all_entities;
        self.brush_entity_mode = criteria.brush_entity_mode;
        self.protect_critical_entities = criteria.protect_critical_entities;
        self.pending_deletion_review = None;
        self.cleanup_export_confirmed = false;
        self.clear_merged_preview();
    }

    fn require_cleanup_confirmation(&mut self, criteria: &DeletionCriteria) -> bool {
        if criteria.is_empty() {
            return true;
        }

        let Some(review) = &self.pending_deletion_review else {
            self.cleanup_export_confirmed = false;
            self.add_status(
                "Destructive cleanup requires Preview deletion, then Confirm cleanup export before writing.",
            );
            return false;
        };

        if review.criteria != *criteria {
            self.cleanup_export_confirmed = false;
            self.add_status(
                "Cleanup rules changed after the pending review. Preview deletion again before export.",
            );
            return false;
        }

        if !self.cleanup_export_confirmed {
            self.add_status(
                "Pending deletion review is ready. Click Confirm cleanup export before writing.",
            );
            return false;
        }

        true
    }

    fn clear_pending_cleanup_review(&mut self) {
        self.pending_deletion_review = None;
        self.cleanup_export_confirmed = false;
        self.add_status("Cleared pending cleanup review. No cleanup export is confirmed.");
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

    fn campaign_order_suggestion(&self) -> Option<CampaignOrderSuggestion> {
        let inputs = self
            .maps
            .iter()
            .filter_map(|entry| {
                let analysis = entry.analysis.as_ref().ok()?;
                Some(CampaignMapInput {
                    label: display_path(&entry.path),
                    transitions: analysis.transitions.clone(),
                    landmarks: analysis.landmarks.clone(),
                })
            })
            .collect::<Vec<_>>();
        (!inputs.is_empty()).then(|| suggest_campaign_order(&inputs))
    }

    fn apply_campaign_order(&mut self, ordered_labels: &[String]) {
        if ordered_labels.is_empty() {
            return;
        }

        let mut remaining = std::mem::take(&mut self.maps);
        let mut ordered = Vec::with_capacity(remaining.len());
        for label in ordered_labels {
            if let Some(position) = remaining
                .iter()
                .position(|entry| display_path(&entry.path) == *label)
            {
                ordered.push(remaining.remove(position));
            }
        }
        ordered.extend(remaining);
        self.maps = ordered;
        self.selected_map = (!self.maps.is_empty()).then_some(0);
        self.base_index = 0;
        self.clear_merged_preview();
        self.add_status(
            "Applied suggested campaign order. You can still override order/base manually.",
        );
    }

    fn draw_campaign_suggestions(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label("Campaign suggestions");
            ui.weak("Suggestions are advisory. You can override order with the selected-map list, base dropdown, and manual landmark field.");

            let Some(suggestion) = self.campaign_order_suggestion() else {
                ui.weak("Add parseable VMFs to inspect campaign transitions.");
                return;
            };

            if suggestion.ordered_labels.is_empty() {
                ui.weak("No map order suggestion is available yet.");
            } else {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Suggested order:");
                    ui.monospace(
                        suggestion
                            .ordered_labels
                            .iter()
                            .map(|label| file_label_for_legend(label))
                            .collect::<Vec<_>>()
                            .join(" → "),
                    );
                });
                if ui.button("Apply suggested order").clicked() {
                    self.apply_campaign_order(&suggestion.ordered_labels);
                }
            }

            if !suggestion.landmark_pairs.is_empty() {
                ui.separator();
                ui.label("Suggested landmark pairs:");
                egui::Grid::new("campaign_landmark_pairs_grid")
                    .striped(true)
                    .num_columns(5)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.strong("From");
                        ui.strong("To");
                        ui.strong("Target map");
                        ui.strong("Landmark");
                        ui.strong("Status");
                        ui.end_row();
                        for pair in &suggestion.landmark_pairs {
                            ui.label(file_label_for_legend(&pair.from_map));
                            ui.label(file_label_for_legend(&pair.to_map));
                            ui.label(&pair.target_map);
                            ui.monospace(&pair.landmark);
                            if pair.target_has_landmark {
                                ui.colored_label(egui::Color32::LIGHT_GREEN, "target has landmark");
                            } else {
                                ui.colored_label(egui::Color32::YELLOW, "target missing landmark");
                            }
                            ui.end_row();
                        }
                    });
                if let Some(first_pair) = suggestion.landmark_pairs.first() {
                    if ui.button("Use first suggested landmark").clicked() {
                        self.landmark = first_pair.landmark.clone();
                        self.clear_merged_preview();
                        self.add_status(format!(
                            "Using suggested landmark `{}`. You can still edit it manually.",
                            first_pair.landmark
                        ));
                    }
                }
            }

            if !suggestion.warnings.is_empty() {
                ui.separator();
                ui.colored_label(egui::Color32::YELLOW, "Campaign warnings:");
                for warning in &suggestion.warnings {
                    ui.small(warning);
                }
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
                removed_total.removed_brush_entity_solids += report.removed_brush_entity_solids;
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

        let preview_inputs = merge_inputs.clone();
        let landmark = blank_to_none(&self.landmark);
        match merge_maps(merge_inputs, &MergeOptions { landmark }) {
            Ok((_document, report)) => {
                let preview = build_source_colored_preview(&preview_inputs, &report);
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
                    "Preview cleanup removed {} entities, {} world solids, and {} brush-entity solids in memory; no VMF was written.",
                    removed_total.removed_entities,
                    removed_total.removed_world_solids,
                    removed_total.removed_brush_entity_solids
                ));
            }
            Err(error) => self.add_status(format!("Merge preview failed: {error}")),
        }
    }

    fn preview_deletion(&mut self) {
        let criteria = self.build_deletion_criteria();
        self.preview_deletion_with_criteria(criteria, "Preview");
    }

    fn preview_deletion_with_criteria(&mut self, criteria: DeletionCriteria, label: &str) {
        if criteria.is_empty() {
            self.pending_deletion_review = None;
            self.cleanup_export_confirmed = false;
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
                    total.removed_brush_entity_solids += report.removed_brush_entity_solids;
                }
                Err(_) => failures += 1,
            }
        }

        self.pending_deletion_review = Some(PendingDeletionReview {
            criteria: criteria.clone(),
            report: total.clone(),
            maps_checked: self.maps.len().saturating_sub(failures),
            failures,
            label: label.to_string(),
        });
        self.cleanup_export_confirmed = false;

        self.add_status(format!(
            "{label}: would remove {} entities, {} world solids, and {} brush-entity solids across {} map(s).{}",
            total.removed_entities,
            total.removed_world_solids,
            total.removed_brush_entity_solids,
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
        let entry_path = entry.path.clone();
        let criteria = self.build_deletion_criteria();
        if criteria.is_empty() {
            self.add_status("No deletion rules selected.");
            return;
        }
        if !self.require_cleanup_confirmation(&criteria) {
            return;
        }

        let default_name = entry_path
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

        match load_document(&entry_path) {
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
                    Ok(()) => {
                        self.add_status(format!(
                            "Wrote cleaned VMF: {}. Removed {} entities, {} world solids, and {} brush-entity solids.",
                            display_path(&output_path),
                            report.removed_entities,
                            report.removed_world_solids,
                            report.removed_brush_entity_solids
                        ));
                        self.pending_deletion_review = None;
                        self.cleanup_export_confirmed = false;
                    }
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

        let criteria = self.build_deletion_criteria();
        if !self.require_cleanup_confirmation(&criteria) {
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
                            "Appended {} world solids and {} entities. Cleanup removed {} entities, {} world solids, and {} brush-entity solids.",
                            report.appended_world_solids,
                            report.appended_entities,
                            removed_total.removed_entities,
                            removed_total.removed_world_solids,
                            removed_total.removed_brush_entity_solids
                        ));
                        for (label, offset) in report.applied_offsets {
                            self.add_status(format!("Offset {label}: {offset}"));
                        }
                        self.pending_deletion_review = None;
                        self.cleanup_export_confirmed = false;
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
                                        "{} records, {} classnames, {} landmarks, {} transitions, {} preview solids, {} integrity warning(s)",
                                        analysis.entity_records.len(),
                                        analysis.type_counts.len(),
                                        analysis.landmarks.targetnames.len(),
                                        analysis.transitions.len(),
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
        ui.horizontal_wrapped(|ui| {
            if ui.button("Load project/job...").clicked() {
                self.load_project_dialog();
            }
            if ui.button("Save project...").clicked() {
                self.save_project_dialog();
            }
            if ui.button("Load FGD metadata...").clicked() {
                self.load_fgd_dialog();
            }
            ui.weak(format!(
                "Project files use CLI job TOML where possible. {} FGD class record(s) loaded.",
                self.fgd_metadata.len()
            ));
        });
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

        self.draw_campaign_suggestions(ui);

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

        ui.collapsing("Deletion presets", |ui| {
            ui.weak("Presets are transparent: preview them before applying, then inspect the generated rules below.");
            for preset in deletion_presets() {
                let criteria = deletion_preset_criteria(preset.kind);
                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(preset.name);
                        if ui.button(format!("Preview##{}", preset.name)).clicked() {
                            self.preview_deletion_with_criteria(
                                criteria.clone(),
                                &format!("Preset `{}` preview", preset.name),
                            );
                        }
                        if ui.button(format!("Apply##{}", preset.name)).clicked() {
                            self.apply_deletion_criteria_to_controls(criteria.clone());
                            self.add_status(format!(
                                "Applied deletion preset `{}`. Preview deletion to verify counts before export.",
                                preset.name
                            ));
                        }
                    });
                    ui.label(preset.description);
                    ui.small(format!(
                        "Generated criteria: {}",
                        describe_deletion_criteria(&criteria)
                    ));
                });
            }
        });

        egui::Grid::new("cleanup_grid")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Drop all entities:");
                ui.checkbox(
                    &mut self.drop_all_entities,
                    "Remove all non-protected top-level entities",
                );
                ui.end_row();

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

        ui.collapsing("Deletion safety", |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Brush-entity role matches:");
                ui.radio_value(
                    &mut self.brush_entity_mode,
                    BrushEntityDeletionMode::WholeEntity,
                    "Delete whole entity",
                );
                ui.radio_value(
                    &mut self.brush_entity_mode,
                    BrushEntityDeletionMode::MatchingSolids,
                    "Delete matching contained solids",
                );
            });
            ui.checkbox(
                &mut self.protect_critical_entities,
                "Protect critical transition/player/logic entities",
            );
            ui.weak("Default safety preserves existing brush-role behavior by deleting whole matching brush entities, while protecting critical classnames unless this box is cleared.");
        });

        let current_criteria = self.build_deletion_criteria();
        ui.group(|ui| {
            ui.label("Pending cleanup review");
            if current_criteria.is_empty() {
                ui.weak("No cleanup rules are active.");
            } else if let Some(review) = self.pending_deletion_review.clone() {
                let stale = review.criteria != current_criteria;
                ui.label(format!(
                    "{}: would remove {} entities, {} world solids, and {} brush-entity solids across {} map(s).{}",
                    review.label,
                    review.report.removed_entities,
                    review.report.removed_world_solids,
                    review.report.removed_brush_entity_solids,
                    review.maps_checked,
                    if review.failures == 0 {
                        String::new()
                    } else {
                        format!(" {} map(s) failed to parse.", review.failures)
                    }
                ));
                if stale {
                    self.cleanup_export_confirmed = false;
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        "Cleanup rules changed after this review. Preview deletion again before export.",
                    );
                } else if self.cleanup_export_confirmed {
                    ui.colored_label(
                        egui::Color32::LIGHT_GREEN,
                        "Cleanup export confirmed. The next cleaned/merge export may write these removals.",
                    );
                } else {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        "Review these pending removals, then confirm before export.",
                    );
                }
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(!stale, egui::Button::new("Confirm cleanup export"))
                        .clicked()
                    {
                        self.cleanup_export_confirmed = true;
                        self.add_status("Confirmed pending cleanup export.");
                    }
                    if ui.button("Undo pending review").clicked() {
                        self.clear_pending_cleanup_review();
                    }
                });
            } else {
                ui.weak("Click Preview deletion to create a pending cleanup review before export.");
            }
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
            ui.selectable_value(
                &mut self.active_table,
                TableMode::Transitions,
                "Transitions",
            );
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
        let selected_landmark = self.landmark.trim().to_string();
        match analysis {
            Ok(analysis) => match self.active_table {
                TableMode::Preview => {
                    self.draw_preview_scope_controls(ui, &path);
                    match self.preview_scope {
                        PreviewScope::SelectedMap => {
                            ui.label(format!("Selected VMF preview: {path}"));
                            self.draw_preview_panel(
                                ui,
                                &analysis.preview,
                                None,
                                &selected_landmark,
                                Some((&entry_path, &analysis.entity_records)),
                            );
                        }
                        PreviewScope::MergedResult => {
                            if let Some(merged_preview) = self.merged_preview.clone() {
                                ui.label("Merged-output preview: current in-memory result");
                                self.draw_preview_panel(
                                    ui,
                                    &merged_preview.preview,
                                    Some(&merged_preview.summary),
                                    &selected_landmark,
                                    None,
                                );
                            } else {
                                ui.colored_label(
                                    egui::Color32::YELLOW,
                                    "No merged preview has been built yet. Click Preview selected merge.",
                                );
                                self.preview_scope = PreviewScope::SelectedMap;
                                self.draw_preview_panel(
                                    ui,
                                    &analysis.preview,
                                    None,
                                    &selected_landmark,
                                    Some((&entry_path, &analysis.entity_records)),
                                );
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
                        &mut self.entity_search,
                        &mut self.entity_role_filter,
                        &mut self.entity_sort_column,
                        &mut self.entity_sort_ascending,
                        &self.fgd_metadata,
                    );
                }
                TableMode::Classnames => {
                    ui.label(&path);
                    draw_classname_table(
                        ui,
                        &analysis.type_counts,
                        &mut self.classname_search,
                        &mut self.classname_sort_column,
                        &mut self.classname_sort_ascending,
                        &self.fgd_metadata,
                    );
                }
                TableMode::Transitions => {
                    ui.label(&path);
                    draw_transition_table(ui, &analysis.transitions);
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
        selected_landmark: &str,
        selection_context: Option<(&Path, &[EntityRecord])>,
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
            ui.label("Deletion preview:");
            egui::ComboBox::from_id_salt("deletion_preview_mode")
                .selected_text(deletion_preview_mode_label(self.preview_deletion_mode))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.preview_deletion_mode,
                        DeletionPreviewMode::Off,
                        deletion_preview_mode_label(DeletionPreviewMode::Off),
                    );
                    ui.selectable_value(
                        &mut self.preview_deletion_mode,
                        DeletionPreviewMode::HighlightRemoved,
                        deletion_preview_mode_label(DeletionPreviewMode::HighlightRemoved),
                    );
                    ui.selectable_value(
                        &mut self.preview_deletion_mode,
                        DeletionPreviewMode::DimRemoved,
                        deletion_preview_mode_label(DeletionPreviewMode::DimRemoved),
                    );
                    ui.selectable_value(
                        &mut self.preview_deletion_mode,
                        DeletionPreviewMode::HideRemoved,
                        deletion_preview_mode_label(DeletionPreviewMode::HideRemoved),
                    );
                });
            ui.weak("Selected-VMF previews update immediately from current cleanup rules.");
        });

        let deletion_criteria = self.build_deletion_criteria();
        let deletion_overlay_mode = if merged_summary.is_none() && !deletion_criteria.is_empty() {
            self.preview_deletion_mode
        } else {
            DeletionPreviewMode::Off
        };
        let preview_deletion_counts = count_preview_deletions(preview, &deletion_criteria);

        ui.horizontal_wrapped(|ui| {
            ui.label(format!("{} preview solids", preview.solids.len()));
            ui.separator();
            ui.label(format!("{} entity origins", preview.entities.len()));
            ui.separator();
            ui.label(format!("{} landmarks", preview.landmarks.len()));
            ui.separator();
            ui.add(egui::Slider::new(&mut self.preview_zoom, 0.1..=12.0).text("Zoom"));
            ui.weak("Mouse wheel zooms. Drag the preview to pan.");
        });

        if !deletion_criteria.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 150, 130),
                    format!(
                        "Deletion overlay matches {} preview solid(s) and {} entity marker(s).",
                        preview_deletion_counts.solids, preview_deletion_counts.entities
                    ),
                );
                if merged_summary.is_some() {
                    ui.weak("Merged preview is already built after applying cleanup rules; source VMF preview shows the overlay.");
                }
            });
        }

        if let Some(summary) = merged_summary {
            ui.group(|ui| {
                ui.label(format!(
                    "Merged preview: {} map(s), appended {} world solids and {} entities.",
                    summary.merged_maps, summary.appended_world_solids, summary.appended_entities
                ));
                ui.label(format!(
                    "Cleanup applied in memory: removed {} entities, {} world solids, and {} brush-entity solids. No output VMF was written.",
                    summary.removed_entities,
                    summary.removed_world_solids,
                    summary.removed_brush_entity_solids
                ));
                for offset in &summary.offsets {
                    ui.small(offset);
                }
            });
        }

        let desired_height = 560.0_f32.max(ui.available_height().min(720.0));
        let desired_size = egui::vec2(ui.available_width().max(360.0), desired_height);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
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
        let selected_owner_indices = selection_context
            .map(|(path, records)| {
                records
                    .iter()
                    .filter(|record| {
                        self.selected_entity_rows
                            .contains(&entity_selection_key(path, record))
                    })
                    .map(|record| record.index)
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();

        if response.clicked() {
            if let Some(click_position) = response.interact_pointer_pos() {
                if let Some((path, records)) = selection_context {
                    if let Some(owner_index) = preview_hit_owner_index(
                        preview,
                        &transform,
                        click_position,
                        &deletion_criteria,
                        deletion_overlay_mode,
                    ) {
                        if let Some(key) =
                            entity_selection_key_for_owner(path, records, owner_index)
                        {
                            let selected = if self.selected_entity_rows.contains(&key) {
                                self.selected_entity_rows.remove(&key);
                                false
                            } else {
                                self.selected_entity_rows.insert(key);
                                true
                            };
                            self.active_table = TableMode::Entities;
                            self.add_status(format!(
                                "{} preview owner row #{owner_index}.",
                                if selected { "Selected" } else { "Cleared" }
                            ));
                        }
                    }
                }
            }
        }
        if self.preview_show_grid {
            draw_preview_grid(&painter, rect, &transform);
        }
        draw_axes_label(&painter, rect, self.preview_view);

        if self.preview_show_solids {
            for solid in &preview.solids {
                let removed = preview_solid_removed(solid, &deletion_criteria);
                if deletion_overlay_mode == DeletionPreviewMode::HideRemoved && removed {
                    continue;
                }
                let selected = selected_owner_indices.contains(&solid.owner_index);
                draw_preview_solid(
                    &painter,
                    &transform,
                    solid,
                    deletion_overlay_mode,
                    removed,
                    selected,
                );
            }
        }

        if self.preview_show_entities {
            for entity in &preview.entities {
                let removed = preview_entity_removed(entity, &deletion_criteria);
                if deletion_overlay_mode == DeletionPreviewMode::HideRemoved && removed {
                    continue;
                }
                let position = transform.world_to_screen(entity.origin);
                if rect.contains(position) {
                    let entity_color = entity
                        .source_index
                        .map(source_color)
                        .unwrap_or_else(|| egui::Color32::from_rgb(255, 232, 128));
                    let entity_color =
                        deletion_entity_color(entity_color, deletion_overlay_mode, removed);
                    painter.circle_filled(position, if removed { 5.5 } else { 4.5 }, entity_color);
                    painter.circle_stroke(
                        position,
                        6.5,
                        egui::Stroke::new(
                            if selected_owner_indices.contains(&entity.owner_index) {
                                2.0_f32
                            } else {
                                1.0_f32
                            },
                            if selected_owner_indices.contains(&entity.owner_index) {
                                egui::Color32::YELLOW
                            } else {
                                egui::Color32::BLACK
                            },
                        ),
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

        draw_landmark_markers(&painter, &transform, preview, selected_landmark);
        if let Some(summary) = merged_summary {
            draw_offset_arrows(&painter, rect, &transform, summary);
        }

        draw_preview_legend(&painter, rect, merged_summary);
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
            removed_brush_entity_solids: deletion.removed_brush_entity_solids,
            source_labels: report
                .applied_offsets
                .iter()
                .map(|(label, _)| label.clone())
                .collect(),
            source_offsets: report.applied_offsets.clone(),
            offsets: report
                .applied_offsets
                .iter()
                .map(|(label, offset)| format!("Offset {label}: {offset}"))
                .collect(),
        }
    }
}

fn build_source_colored_preview(inputs: &[MergeInput], report: &MergeReport) -> PreviewDocument {
    let previews = inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            let mut preview = preview_document_with_source(
                &input.document,
                Some(index),
                Some(input.label.as_str()),
            );
            let offset = report
                .applied_offsets
                .iter()
                .find(|(label, _)| label == &input.label)
                .map(|(_, offset)| *offset)
                .unwrap_or(sourceweaver_core::Vec3::ZERO);
            translate_preview_document(&mut preview, offset);
            preview
        })
        .collect::<Vec<_>>();
    combine_preview_documents(previews)
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
                    transitions: discover_transitions(&document),
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
    search: &mut String,
    role_filter: &mut Option<BrushRole>,
    sort_column: &mut EntitySortColumn,
    sort_ascending: &mut bool,
    fgd_metadata: &BTreeMap<String, EntityMetadata>,
) {
    let row_keys = records
        .iter()
        .map(|record| entity_selection_key(map_path, record))
        .collect::<Vec<_>>();

    ui.horizontal_wrapped(|ui| {
        ui.label("Search entities:");
        ui.add(
            egui::TextEdit::singleline(search)
                .desired_width(260.0)
                .hint_text("classname, targetname, category, description, role"),
        );
        if !search.trim().is_empty() && ui.button("Clear search").clicked() {
            search.clear();
        }
        ui.separator();
        ui.label("Role:");
        egui::ComboBox::from_id_salt("entity_role_filter")
            .selected_text(
                role_filter
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "All roles".to_string()),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(role_filter, None, "All roles");
                for role in entity_role_filter_options() {
                    ui.selectable_value(role_filter, Some(role.clone()), role.to_string());
                }
            });
    });

    ui.horizontal_wrapped(|ui| {
        ui.label("Sort:");
        egui::ComboBox::from_id_salt("entity_sort_column")
            .selected_text(entity_sort_column_label(*sort_column))
            .show_ui(ui, |ui| {
                for column in [
                    EntitySortColumn::Index,
                    EntitySortColumn::Block,
                    EntitySortColumn::Category,
                    EntitySortColumn::Classname,
                    EntitySortColumn::Targetname,
                    EntitySortColumn::Origin,
                    EntitySortColumn::Solids,
                    EntitySortColumn::Roles,
                ] {
                    ui.selectable_value(sort_column, column, entity_sort_column_label(column));
                }
            });
        ui.checkbox(sort_ascending, "Ascending");
    });

    let mut rows = records
        .iter()
        .zip(row_keys.iter())
        .filter(|(record, _)| {
            entity_matches_filters(record, search, role_filter.as_ref(), fgd_metadata)
        })
        .collect::<Vec<_>>();
    sort_entity_rows(&mut rows, *sort_column, *sort_ascending, fgd_metadata);

    let filtered_keys = rows
        .iter()
        .map(|(_, key)| (*key).clone())
        .collect::<Vec<_>>();
    let current_selected = row_keys
        .iter()
        .filter(|key| selected_rows.contains(key))
        .count();
    let filtered_selected = filtered_keys
        .iter()
        .filter(|key| selected_rows.contains(key))
        .count();

    ui.horizontal_wrapped(|ui| {
        ui.label(format!(
            "Showing {} of {} world/entity records",
            rows.len(),
            records.len()
        ));
        ui.separator();
        ui.label(format!(
            "{} selected visible, {} selected in this map, {} selected total",
            filtered_selected,
            current_selected,
            selected_rows.len()
        ));
        ui.separator();
        if ui.button("Select visible rows").clicked() {
            selected_rows.extend(filtered_keys.iter().cloned());
        }
        if ui.button("Select all rows").clicked() {
            selected_rows.extend(row_keys.iter().cloned());
        }
        if ui.button("Clear visible").clicked() {
            for key in &filtered_keys {
                selected_rows.remove(key);
            }
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
            .num_columns(11)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.strong("Select");
                ui.strong("#");
                ui.strong("Block");
                ui.strong("Category");
                ui.strong("Friendly name");
                ui.strong("Classname");
                ui.strong("Targetname");
                ui.strong("Origin");
                ui.strong("Solids");
                ui.strong("Roles");
                ui.strong("Description");
                ui.end_row();

                for (record, key) in rows {
                    let metadata = record.classname.as_deref().map(|classname| {
                        metadata_for_classname_with_overrides(classname, fgd_metadata)
                    });
                    let mut selected = selected_rows.contains(key);
                    if ui.checkbox(&mut selected, "").changed() {
                        if selected {
                            selected_rows.insert(key.clone());
                        } else {
                            selected_rows.remove(key);
                        }
                    }
                    let text_color = if selected {
                        egui::Color32::YELLOW
                    } else {
                        ui.visuals().text_color()
                    };
                    ui.colored_label(text_color, record.index.to_string());
                    ui.colored_label(text_color, &record.block_name);
                    ui.colored_label(
                        text_color,
                        metadata
                            .as_ref()
                            .map(|metadata| metadata.category.to_string())
                            .unwrap_or_else(|| "world".to_string()),
                    );
                    ui.colored_label(
                        text_color,
                        metadata
                            .as_ref()
                            .map(|metadata| metadata.display_name.as_str())
                            .unwrap_or("World"),
                    );
                    ui.colored_label(text_color, record.classname.as_deref().unwrap_or("-"));
                    ui.colored_label(text_color, record.targetname.as_deref().unwrap_or("-"));
                    ui.colored_label(
                        text_color,
                        record
                            .origin
                            .map(|origin| origin.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                    );
                    ui.colored_label(text_color, record.solid_count.to_string());
                    ui.colored_label(text_color, format_roles(&record.roles));
                    ui.colored_label(
                        text_color,
                        metadata
                            .as_ref()
                            .and_then(|metadata| metadata.description.as_deref())
                            .unwrap_or("-"),
                    );
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

fn entity_selection_key_for_owner(
    map_path: &Path,
    records: &[EntityRecord],
    owner_index: usize,
) -> Option<EntitySelectionKey> {
    records
        .iter()
        .find(|record| record.index == owner_index)
        .map(|record| entity_selection_key(map_path, record))
}

fn draw_classname_table(
    ui: &mut egui::Ui,
    type_counts: &BTreeMap<String, usize>,
    search: &mut String,
    sort_column: &mut ClassnameSortColumn,
    sort_ascending: &mut bool,
    fgd_metadata: &BTreeMap<String, EntityMetadata>,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Search classnames:");
        ui.add(
            egui::TextEdit::singleline(search)
                .desired_width(260.0)
                .hint_text("classname, category, description"),
        );
        if !search.trim().is_empty() && ui.button("Clear search").clicked() {
            search.clear();
        }
        ui.separator();
        ui.label("Sort:");
        egui::ComboBox::from_id_salt("classname_sort_column")
            .selected_text(classname_sort_column_label(*sort_column))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    sort_column,
                    ClassnameSortColumn::Classname,
                    classname_sort_column_label(ClassnameSortColumn::Classname),
                );
                ui.selectable_value(
                    sort_column,
                    ClassnameSortColumn::Count,
                    classname_sort_column_label(ClassnameSortColumn::Count),
                );
            });
        ui.checkbox(sort_ascending, "Ascending");
    });

    let query = search.trim().to_ascii_lowercase();
    let mut rows = type_counts
        .iter()
        .filter(|(classname, _)| classname_matches_metadata_search(classname, &query, fgd_metadata))
        .collect::<Vec<_>>();
    sort_classname_rows(&mut rows, *sort_column, *sort_ascending);

    ui.label(format!(
        "Showing {} of {} detected classnames",
        rows.len(),
        type_counts.len()
    ));
    egui::ScrollArea::both().max_height(360.0).show(ui, |ui| {
        egui::Grid::new("classname_table")
            .striped(true)
            .num_columns(5)
            .spacing([24.0, 6.0])
            .show(ui, |ui| {
                ui.strong("Count");
                ui.strong("Classname");
                ui.strong("Category");
                ui.strong("Friendly name");
                ui.strong("Description");
                ui.end_row();
                for (classname, count) in rows {
                    let metadata = metadata_for_classname_with_overrides(classname, fgd_metadata);
                    ui.label(count.to_string());
                    ui.label(classname);
                    ui.label(metadata.category.to_string());
                    ui.label(metadata.display_name);
                    ui.label(metadata.description.as_deref().unwrap_or("-"));
                    ui.end_row();
                }
            });
    });
}

fn classname_matches_metadata_search(
    classname: &str,
    query: &str,
    fgd_metadata: &BTreeMap<String, EntityMetadata>,
) -> bool {
    if query.is_empty() {
        return true;
    }
    let metadata = metadata_for_classname_with_overrides(classname, fgd_metadata);
    classname.to_ascii_lowercase().contains(query)
        || metadata.category.to_string().contains(query)
        || metadata.display_name.to_ascii_lowercase().contains(query)
        || metadata
            .description
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains(query)
}

fn draw_transition_table(ui: &mut egui::Ui, transitions: &[CampaignTransition]) {
    if transitions.is_empty() {
        ui.weak("No trigger_changelevel entities detected in this VMF.");
        return;
    }

    ui.label(format!(
        "{} trigger_changelevel transition(s) detected",
        transitions.len()
    ));
    ui.weak("These target maps and landmarks can guide future stitching and landmark selection.");

    egui::ScrollArea::both().max_height(360.0).show(ui, |ui| {
        egui::Grid::new("transition_table")
            .striped(true)
            .num_columns(6)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                ui.strong("Entity #");
                ui.strong("Targetname");
                ui.strong("Target map");
                ui.strong("Landmark");
                ui.strong("Origin");
                ui.strong("Solids");
                ui.end_row();

                for transition in transitions {
                    ui.label(transition.entity_index.to_string());
                    ui.label(transition.targetname.as_deref().unwrap_or("-"));
                    ui.label(transition.target_map.as_deref().unwrap_or("-"));
                    ui.label(transition.landmark.as_deref().unwrap_or("-"));
                    ui.label(
                        transition
                            .origin
                            .map(|origin| origin.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                    );
                    ui.label(transition.solid_count.to_string());
                    ui.end_row();
                }
            });
    });
}

fn entity_matches_filters(
    record: &EntityRecord,
    search: &str,
    role_filter: Option<&BrushRole>,
    fgd_metadata: &BTreeMap<String, EntityMetadata>,
) -> bool {
    if let Some(role) = role_filter {
        if !record.roles.iter().any(|record_role| record_role == role) {
            return false;
        }
    }

    let query = search.trim().to_ascii_lowercase();
    if query.is_empty() {
        return true;
    }

    let roles = format_roles(&record.roles).to_ascii_lowercase();
    let metadata = record
        .classname
        .as_deref()
        .map(|classname| metadata_for_classname_with_overrides(classname, fgd_metadata));
    record.block_name.to_ascii_lowercase().contains(&query)
        || record
            .classname
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains(&query)
        || record
            .targetname
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains(&query)
        || metadata
            .as_ref()
            .map(|metadata| {
                metadata.category.to_string().contains(&query)
                    || metadata.display_name.to_ascii_lowercase().contains(&query)
                    || metadata
                        .description
                        .as_deref()
                        .unwrap_or("")
                        .to_ascii_lowercase()
                        .contains(&query)
            })
            .unwrap_or(false)
        || roles.contains(&query)
}

fn sort_entity_rows(
    rows: &mut Vec<(&EntityRecord, &EntitySelectionKey)>,
    column: EntitySortColumn,
    ascending: bool,
    fgd_metadata: &BTreeMap<String, EntityMetadata>,
) {
    rows.sort_by(|(left, _), (right, _)| {
        let ordering = match column {
            EntitySortColumn::Index => left.index.cmp(&right.index),
            EntitySortColumn::Block => left.block_name.cmp(&right.block_name),
            EntitySortColumn::Category => entity_category_sort_key(left, fgd_metadata)
                .cmp(&entity_category_sort_key(right, fgd_metadata)),
            EntitySortColumn::Classname => left.classname.cmp(&right.classname),
            EntitySortColumn::Targetname => left.targetname.cmp(&right.targetname),
            EntitySortColumn::Origin => left
                .origin
                .map(|origin| origin.to_string())
                .cmp(&right.origin.map(|origin| origin.to_string())),
            EntitySortColumn::Solids => left.solid_count.cmp(&right.solid_count),
            EntitySortColumn::Roles => format_roles(&left.roles).cmp(&format_roles(&right.roles)),
        };
        if ascending {
            ordering.then_with(|| left.index.cmp(&right.index))
        } else {
            ordering
                .reverse()
                .then_with(|| left.index.cmp(&right.index))
        }
    });
}

fn entity_category_sort_key(
    record: &EntityRecord,
    fgd_metadata: &BTreeMap<String, EntityMetadata>,
) -> String {
    record
        .classname
        .as_deref()
        .map(|classname| {
            metadata_for_classname_with_overrides(classname, fgd_metadata)
                .category
                .to_string()
        })
        .unwrap_or_else(|| "world".to_string())
}

fn sort_classname_rows(
    rows: &mut Vec<(&String, &usize)>,
    column: ClassnameSortColumn,
    ascending: bool,
) {
    rows.sort_by(|(left_name, left_count), (right_name, right_count)| {
        let ordering = match column {
            ClassnameSortColumn::Classname => left_name.cmp(right_name),
            ClassnameSortColumn::Count => left_count.cmp(right_count),
        };
        if ascending {
            ordering.then_with(|| left_name.cmp(right_name))
        } else {
            ordering.reverse().then_with(|| left_name.cmp(right_name))
        }
    });
}

fn entity_role_filter_options() -> Vec<BrushRole> {
    vec![
        BrushRole::Trigger,
        BrushRole::Clip,
        BrushRole::Areaportal,
        BrushRole::Skybox,
        BrushRole::Occluder,
        BrushRole::Hint,
        BrushRole::Skip,
        BrushRole::Nodraw,
        BrushRole::Water,
        BrushRole::WorldBrush,
        BrushRole::BrushEntity,
        BrushRole::Other,
    ]
}

fn entity_sort_column_label(column: EntitySortColumn) -> &'static str {
    match column {
        EntitySortColumn::Index => "Index",
        EntitySortColumn::Block => "Block",
        EntitySortColumn::Category => "Category",
        EntitySortColumn::Classname => "Classname",
        EntitySortColumn::Targetname => "Targetname",
        EntitySortColumn::Origin => "Origin",
        EntitySortColumn::Solids => "Solids",
        EntitySortColumn::Roles => "Roles",
    }
}

fn classname_sort_column_label(column: ClassnameSortColumn) -> &'static str {
    match column {
        ClassnameSortColumn::Classname => "Classname",
        ClassnameSortColumn::Count => "Count",
    }
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

fn draw_landmark_markers(
    painter: &egui::Painter,
    transform: &PreviewTransform,
    preview: &PreviewDocument,
    selected_landmark: &str,
) {
    let selected_landmark = selected_landmark.trim();
    for landmark in &preview.landmarks {
        let position = transform.world_to_screen(landmark.origin);
        if !transform.rect.contains(position) {
            continue;
        }

        let is_selected = !selected_landmark.is_empty() && landmark.targetname == selected_landmark;
        let base_color = landmark
            .source_index
            .map(source_color)
            .unwrap_or_else(|| egui::Color32::from_rgb(100, 255, 180));
        let color = if is_selected {
            egui::Color32::from_rgb(255, 255, 120)
        } else {
            base_color
        };
        let radius = if is_selected { 9.0 } else { 7.0 };
        let points = [
            position + egui::vec2(0.0, -radius),
            position + egui::vec2(radius, 0.0),
            position + egui::vec2(0.0, radius),
            position + egui::vec2(-radius, 0.0),
        ];

        painter.add(egui::Shape::convex_polygon(
            points.to_vec(),
            color.gamma_multiply(0.85),
            egui::Stroke::new(
                if is_selected { 2.0_f32 } else { 1.2_f32 },
                egui::Color32::BLACK,
            ),
        ));
        painter.text(
            position + egui::vec2(radius + 5.0, -radius - 2.0),
            egui::Align2::LEFT_CENTER,
            if is_selected {
                format!("★ {}", landmark.targetname)
            } else {
                landmark.targetname.clone()
            },
            egui::FontId::monospace(if is_selected { 11.0 } else { 10.0 }),
            if is_selected {
                egui::Color32::from_rgb(255, 255, 160)
            } else {
                egui::Color32::from_rgb(190, 255, 220)
            },
        );
    }
}

fn draw_offset_arrows(
    painter: &egui::Painter,
    rect: egui::Rect,
    transform: &PreviewTransform,
    summary: &MergedPreviewSummary,
) {
    let non_zero_offsets = summary
        .source_offsets
        .iter()
        .enumerate()
        .filter(|(_, (_, offset))| *offset != sourceweaver_core::Vec3::ZERO)
        .collect::<Vec<_>>();
    if non_zero_offsets.is_empty() {
        return;
    }

    let mut anchor = rect.right_bottom() + egui::vec2(-250.0, -28.0);
    painter.text(
        anchor + egui::vec2(0.0_f32, -16.0_f32),
        egui::Align2::LEFT_CENTER,
        "merge offsets",
        egui::FontId::monospace(10.0),
        egui::Color32::from_gray(230),
    );

    for (index, (label, offset)) in non_zero_offsets {
        let color = source_color(index);
        let direction = offset_vector_to_screen_delta(*offset, transform.view);
        let length = direction.length().clamp(24.0, 72.0);
        let unit = if direction.length() > f32::EPSILON {
            direction / direction.length()
        } else {
            egui::vec2(1.0, 0.0)
        };
        let start = anchor;
        let end = anchor + unit * length;
        painter.line_segment([start, end], egui::Stroke::new(2.0_f32, color));
        let left = end - unit * 8.0_f32 + egui::vec2(-unit.y, unit.x) * 4.0_f32;
        let right = end - unit * 8.0_f32 + egui::vec2(unit.y, -unit.x) * 4.0_f32;
        painter.line_segment([left, end], egui::Stroke::new(2.0_f32, color));
        painter.line_segment([right, end], egui::Stroke::new(2.0_f32, color));
        painter.text(
            start + egui::vec2(82.0_f32, 0.0_f32),
            egui::Align2::LEFT_CENTER,
            format!("{}: {offset}", file_label_for_legend(label)),
            egui::FontId::monospace(10.0),
            egui::Color32::from_gray(225),
        );
        anchor.y -= 20.0;
    }
}

fn offset_vector_to_screen_delta(offset: sourceweaver_core::Vec3, view: PreviewView) -> egui::Vec2 {
    let (u, v) = project_vec(offset, view);
    egui::vec2(u as f32, -(v as f32))
}

fn draw_preview_solid(
    painter: &egui::Painter,
    transform: &PreviewTransform,
    solid: &PreviewSolid,
    deletion_mode: DeletionPreviewMode,
    removed: bool,
    selected: bool,
) {
    let (min_u, min_v) = project_vec(solid.bounds.min, transform.view);
    let (max_u, max_v) = project_vec(solid.bounds.max, transform.view);
    let a = transform.uv_to_screen(min_u, min_v);
    let b = transform.uv_to_screen(max_u, max_v);
    let rect = egui::Rect::from_two_pos(a, b);
    let mut role_color = solid_color(solid);
    let mut fill_color = solid.source_index.map(source_color).unwrap_or(role_color);
    if removed {
        match deletion_mode {
            DeletionPreviewMode::Off | DeletionPreviewMode::HideRemoved => {}
            DeletionPreviewMode::HighlightRemoved => {
                role_color = egui::Color32::from_rgb(255, 40, 40);
                fill_color = egui::Color32::from_rgb(255, 40, 40);
            }
            DeletionPreviewMode::DimRemoved => {
                role_color = role_color.gamma_multiply(0.25);
                fill_color = fill_color.gamma_multiply(0.18);
            }
        }
    }

    if rect.intersects(transform.rect) {
        painter.rect_filled(
            rect,
            0.0,
            fill_color.gamma_multiply(if removed { 0.32 } else { 0.22 }),
        );
        draw_rect_outline(
            painter,
            rect,
            egui::Stroke::new(
                if selected {
                    2.5_f32
                } else if removed {
                    2.0_f32
                } else {
                    1.25_f32
                },
                if selected {
                    egui::Color32::YELLOW
                } else {
                    role_color
                },
            ),
        );
    }

    for chunk in solid.points.chunks(3) {
        if chunk.len() == 3 {
            let p0 = transform.world_to_screen(chunk[0]);
            let p1 = transform.world_to_screen(chunk[1]);
            let p2 = transform.world_to_screen(chunk[2]);
            painter.line_segment(
                [p0, p1],
                egui::Stroke::new(0.8_f32, role_color.gamma_multiply(0.7)),
            );
            painter.line_segment(
                [p1, p2],
                egui::Stroke::new(0.8_f32, role_color.gamma_multiply(0.7)),
            );
            painter.line_segment(
                [p2, p0],
                egui::Stroke::new(0.8_f32, role_color.gamma_multiply(0.7)),
            );
        }
    }
}

fn draw_preview_legend(
    painter: &egui::Painter,
    rect: egui::Rect,
    merged_summary: Option<&MergedPreviewSummary>,
) {
    let items = [
        ("world", egui::Color32::from_rgb(170, 190, 220)),
        ("skybox", egui::Color32::from_rgb(100, 170, 255)),
        ("trigger", egui::Color32::from_rgb(255, 170, 90)),
        ("clip", egui::Color32::from_rgb(230, 95, 95)),
        ("areaportal", egui::Color32::from_rgb(185, 115, 255)),
        ("water", egui::Color32::from_rgb(80, 210, 230)),
        ("entity origin", egui::Color32::from_rgb(255, 232, 128)),
        ("landmark", egui::Color32::from_rgb(100, 255, 180)),
        ("selected landmark", egui::Color32::from_rgb(255, 255, 120)),
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

    if let Some(summary) = merged_summary {
        if !summary.source_labels.is_empty() {
            cursor.y += 8.0;
            painter.text(
                cursor,
                egui::Align2::LEFT_TOP,
                "source maps",
                egui::FontId::monospace(10.0),
                egui::Color32::from_gray(235),
            );
            cursor.y += 16.0;
            for (index, label) in summary.source_labels.iter().enumerate() {
                let color = source_color(index);
                let swatch = egui::Rect::from_min_size(cursor, egui::vec2(10.0, 10.0));
                painter.rect_filled(swatch, 1.0, color);
                painter.text(
                    cursor + egui::vec2(16.0, 5.0),
                    egui::Align2::LEFT_CENTER,
                    file_label_for_legend(label),
                    egui::FontId::monospace(10.0),
                    egui::Color32::from_gray(220),
                );
                cursor.y += 16.0;
            }
        }
    }
}

fn source_color(index: usize) -> egui::Color32 {
    const PALETTE: [egui::Color32; 10] = [
        egui::Color32::from_rgb(80, 180, 255),
        egui::Color32::from_rgb(255, 160, 90),
        egui::Color32::from_rgb(145, 220, 115),
        egui::Color32::from_rgb(220, 140, 255),
        egui::Color32::from_rgb(255, 215, 95),
        egui::Color32::from_rgb(95, 220, 210),
        egui::Color32::from_rgb(255, 115, 150),
        egui::Color32::from_rgb(175, 175, 255),
        egui::Color32::from_rgb(210, 180, 120),
        egui::Color32::from_rgb(170, 230, 180),
    ];
    PALETTE[index % PALETTE.len()]
}

fn file_label_for_legend(label: &str) -> String {
    Path::new(label)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(label)
        .to_string()
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

fn preview_hit_owner_index(
    preview: &PreviewDocument,
    transform: &PreviewTransform,
    click_position: egui::Pos2,
    criteria: &DeletionCriteria,
    deletion_mode: DeletionPreviewMode,
) -> Option<usize> {
    let mut nearest_entity = None;
    let mut nearest_distance = f32::MAX;
    for entity in &preview.entities {
        if deletion_mode == DeletionPreviewMode::HideRemoved
            && preview_entity_removed(entity, criteria)
        {
            continue;
        }
        let position = transform.world_to_screen(entity.origin);
        let distance = position.distance(click_position);
        if distance < nearest_distance && distance <= 12.0 {
            nearest_distance = distance;
            nearest_entity = Some(entity.owner_index);
        }
    }
    if nearest_entity.is_some() {
        return nearest_entity;
    }

    for solid in preview.solids.iter().rev() {
        if deletion_mode == DeletionPreviewMode::HideRemoved
            && preview_solid_removed(solid, criteria)
        {
            continue;
        }
        if preview_solid_screen_rect(solid, transform).contains(click_position) {
            return Some(solid.owner_index);
        }
    }
    None
}

fn preview_solid_screen_rect(solid: &PreviewSolid, transform: &PreviewTransform) -> egui::Rect {
    let (min_u, min_v) = project_vec(solid.bounds.min, transform.view);
    let (max_u, max_v) = project_vec(solid.bounds.max, transform.view);
    egui::Rect::from_two_pos(
        transform.uv_to_screen(min_u, min_v),
        transform.uv_to_screen(max_u, max_v),
    )
}

fn deletion_preview_mode_label(mode: DeletionPreviewMode) -> &'static str {
    match mode {
        DeletionPreviewMode::Off => "Off",
        DeletionPreviewMode::HighlightRemoved => "Highlight removed red",
        DeletionPreviewMode::DimRemoved => "Dim removed",
        DeletionPreviewMode::HideRemoved => "Hide removed",
    }
}

fn count_preview_deletions(
    preview: &PreviewDocument,
    criteria: &DeletionCriteria,
) -> PreviewDeletionCounts {
    if criteria.is_empty() {
        return PreviewDeletionCounts::default();
    }
    PreviewDeletionCounts {
        solids: preview
            .solids
            .iter()
            .filter(|solid| preview_solid_removed(solid, criteria))
            .count(),
        entities: preview
            .entities
            .iter()
            .filter(|entity| preview_entity_removed(entity, criteria))
            .count(),
    }
}

fn preview_entity_removed(entity: &PreviewEntityMarker, criteria: &DeletionCriteria) -> bool {
    if criteria.is_empty() || preview_entity_protected(entity.classname.as_deref(), criteria) {
        return false;
    }
    if criteria.drop_all_entities {
        return true;
    }
    entity
        .classname
        .as_ref()
        .map(|classname| criteria.classnames.contains(classname))
        .unwrap_or(false)
        || entity
            .targetname
            .as_ref()
            .map(|targetname| criteria.targetnames.contains(targetname))
            .unwrap_or(false)
}

fn preview_solid_removed(solid: &PreviewSolid, criteria: &DeletionCriteria) -> bool {
    if criteria.is_empty() || preview_entity_protected(solid.classname.as_deref(), criteria) {
        return false;
    }

    let entity_level_match = solid.owner_block == "entity"
        && (criteria.drop_all_entities
            || solid
                .classname
                .as_ref()
                .map(|classname| criteria.classnames.contains(classname))
                .unwrap_or(false)
            || solid
                .targetname
                .as_ref()
                .map(|targetname| criteria.targetnames.contains(targetname))
                .unwrap_or(false));
    if entity_level_match {
        return true;
    }

    if criteria.brush_roles.is_empty() {
        return false;
    }

    let role_match = solid
        .roles
        .iter()
        .any(|role| criteria.brush_roles.contains(role));
    if solid.owner_block == "world" {
        return role_match;
    }

    match criteria.brush_entity_mode {
        BrushEntityDeletionMode::WholeEntity => role_match,
        BrushEntityDeletionMode::MatchingSolids => {
            role_match || criteria.brush_roles.contains(&BrushRole::BrushEntity)
        }
    }
}

fn preview_entity_protected(classname: Option<&str>, criteria: &DeletionCriteria) -> bool {
    criteria.protect_critical_entities
        && classname.map(is_critical_entity_classname).unwrap_or(false)
}

fn deletion_entity_color(
    color: egui::Color32,
    mode: DeletionPreviewMode,
    removed: bool,
) -> egui::Color32 {
    if !removed {
        return color;
    }
    match mode {
        DeletionPreviewMode::Off => color,
        DeletionPreviewMode::HighlightRemoved => egui::Color32::from_rgb(255, 70, 70),
        DeletionPreviewMode::DimRemoved => color.gamma_multiply(0.25),
        DeletionPreviewMode::HideRemoved => color,
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

fn deletion_presets() -> [DeletionPresetSpec; 6] {
    [
        DeletionPresetSpec {
            kind: DeletionPresetKind::RemoveTriggers,
            name: "Remove triggers",
            description: "Remove trigger brush content while leaving critical transition entities protected.",
        },
        DeletionPresetSpec {
            kind: DeletionPresetKind::RemoveClips,
            name: "Remove clips",
            description: "Remove clip/playerclip-style brush content from world and brush entities.",
        },
        DeletionPresetSpec {
            kind: DeletionPresetKind::RemoveAreaportals,
            name: "Remove areaportals",
            description: "Remove areaportal brush content that often needs rebuilding after stitching.",
        },
        DeletionPresetSpec {
            kind: DeletionPresetKind::RemoveGameplayLogic,
            name: "Remove gameplay logic",
            description: "Target common trigger and logic classnames; protected critical entities remain until protection is disabled.",
        },
        DeletionPresetSpec {
            kind: DeletionPresetKind::KeepWorldGeometry,
            name: "Keep only world geometry",
            description: "Remove non-protected entities and utility/tool world brushes, including skybox brushes.",
        },
        DeletionPresetSpec {
            kind: DeletionPresetKind::KeepWorldAndSkybox,
            name: "Keep world plus skybox",
            description: "Remove non-protected entities and utility/tool world brushes while preserving skybox brushes.",
        },
    ]
}

fn deletion_preset_criteria(kind: DeletionPresetKind) -> DeletionCriteria {
    let mut criteria = DeletionCriteria::default();
    criteria.protect_critical_entities = true;

    match kind {
        DeletionPresetKind::RemoveTriggers => {
            criteria.brush_roles.insert(BrushRole::Trigger);
            criteria.brush_entity_mode = BrushEntityDeletionMode::MatchingSolids;
        }
        DeletionPresetKind::RemoveClips => {
            criteria.brush_roles.insert(BrushRole::Clip);
            criteria.brush_entity_mode = BrushEntityDeletionMode::MatchingSolids;
        }
        DeletionPresetKind::RemoveAreaportals => {
            criteria.brush_roles.insert(BrushRole::Areaportal);
            criteria.brush_entity_mode = BrushEntityDeletionMode::MatchingSolids;
        }
        DeletionPresetKind::RemoveGameplayLogic => {
            criteria.classnames.extend([
                "trigger_once".to_string(),
                "trigger_multiple".to_string(),
                "logic_auto".to_string(),
                "logic_relay".to_string(),
                "logic_timer".to_string(),
                "math_counter".to_string(),
                "point_template".to_string(),
                "env_global".to_string(),
                "game_text".to_string(),
            ]);
            criteria.brush_roles.insert(BrushRole::Trigger);
            criteria.brush_entity_mode = BrushEntityDeletionMode::WholeEntity;
        }
        DeletionPresetKind::KeepWorldGeometry => {
            criteria.drop_all_entities = true;
            criteria.brush_roles.extend([
                BrushRole::Trigger,
                BrushRole::Clip,
                BrushRole::Areaportal,
                BrushRole::Skybox,
                BrushRole::Occluder,
                BrushRole::Hint,
                BrushRole::Skip,
                BrushRole::Nodraw,
                BrushRole::Water,
                BrushRole::BrushEntity,
            ]);
            criteria.brush_entity_mode = BrushEntityDeletionMode::MatchingSolids;
        }
        DeletionPresetKind::KeepWorldAndSkybox => {
            criteria.drop_all_entities = true;
            criteria.brush_roles.extend([
                BrushRole::Trigger,
                BrushRole::Clip,
                BrushRole::Areaportal,
                BrushRole::Occluder,
                BrushRole::Hint,
                BrushRole::Skip,
                BrushRole::Nodraw,
                BrushRole::Water,
                BrushRole::BrushEntity,
            ]);
            criteria.brush_entity_mode = BrushEntityDeletionMode::MatchingSolids;
        }
    }

    criteria
}

fn describe_deletion_criteria(criteria: &DeletionCriteria) -> String {
    let mut parts = Vec::new();
    if criteria.drop_all_entities {
        parts.push("all non-protected entities".to_string());
    }
    if !criteria.classnames.is_empty() {
        parts.push(format!(
            "classnames [{}]",
            criteria
                .classnames
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !criteria.targetnames.is_empty() {
        parts.push(format!(
            "targetnames [{}]",
            criteria
                .targetnames
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !criteria.brush_roles.is_empty() {
        parts.push(format!(
            "roles [{}]",
            criteria
                .brush_roles
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    parts.push(format!("brush entities: {}", criteria.brush_entity_mode));
    parts.push(format!(
        "critical protection: {}",
        if criteria.protect_critical_entities {
            "on"
        } else {
            "off"
        }
    ));
    parts.join("; ")
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

fn project_relative_path(path: &Path, base_dir: &Path) -> String {
    if path.is_absolute() {
        if let Ok(relative) = path.strip_prefix(base_dir) {
            if !relative.as_os_str().is_empty() {
                return display_path(relative);
            }
        }
    }
    display_path(path)
}

fn resolve_project_path(base_dir: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

fn file_name_or_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| display_path(path))
}
