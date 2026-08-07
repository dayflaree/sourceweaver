use eframe::egui;
use serde::{Deserialize, Serialize};
use sourceweaver_core::{
    BUILTIN_VALIDATION_RULE_SETS, BrushEntityDeletionMode, BrushRole, CampaignMapInput,
    CampaignOrderSuggestion, CampaignTransition, ChangelevelPolicy, ChangelevelPolicyOptions,
    ChangelevelPreserveRule, ChangelevelScope, DeletionCriteria, DeletionReport, Document,
    EntityMetadata, EntityRecord, EntitySemanticsReport, IntegrityReport, LandmarkDiscovery,
    LandmarkTargetStatus, MapComplexityReport, MergeInput, MergeOptions, MergeReport,
    NO_VALIDATION_RULE_SET_ID, PreviewBounds, PreviewDocument, PreviewEntityMarker, PreviewSolid,
    RuleSetValidationReport, combine_preview_documents, discover_landmarks, discover_transitions,
    format_entity_semantics_issue, format_integrity_issue, format_rule_set_issue, inspect_entities,
    is_critical_entity_classname, merge_maps, metadata_for_classname_with_overrides,
    parse_fgd_metadata, preview_document, preview_document_with_source, prune_document,
    suggest_campaign_order, summarize_entity_types, translate_preview_document,
    validate_document_integrity, validate_document_with_rule_set, validation_rule_set_by_id,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;

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
    changelevel_policy: ChangelevelPolicy,
    changelevel_scope: ChangelevelScope,
    preserve_external_map: String,
    preserve_external_landmark: String,
    preserve_external_targetname: String,
    drop_classnames: String,
    drop_targetnames: String,
    role_options: Vec<RoleOption>,
    drop_all_entities: bool,
    brush_entity_mode: BrushEntityDeletionMode,
    protect_critical_entities: bool,
    custom_delete_preset_name: String,
    custom_delete_preset_description: String,
    custom_delete_preset_path: String,
    pending_deletion_review: Option<PendingDeletionReview>,
    cleanup_export_confirmed: bool,
    status: Vec<String>,
    active_table: TableMode,
    preview_scope: PreviewScope,
    merged_preview: Option<MergedPreview>,
    preview_view: PreviewView,
    preview_zoom: f32,
    preview_pan: egui::Vec2,
    preview_3d_yaw: f32,
    preview_3d_pitch: f32,
    preview_show_solids: bool,
    preview_show_entities: bool,
    preview_show_grid: bool,
    preview_detail_mode: PreviewDetailMode,
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
    validation_rule_set: String,
    bsp_derived_vmfs: BTreeSet<String>,
    bsp_decompile_bsp_path: String,
    bsp_decompile_output_vmf: String,
    bsp_decompile_bspsource_path: String,
    bsp_decompile_jar_path: String,
    bsp_decompile_java_path: String,
    bsp_decompile_wrapper_path: String,
    bsp_decompile_tool_args: String,
    bsp_decompile_log_path: String,
    bsp_decompile_report_path: String,
    bsp_decompile_timeout_seconds: String,
    bsp_decompile_status: DesktopBspDecompileStatus,
    bsp_decompile_receiver: Option<Receiver<DesktopBspDecompileMessage>>,
    recent_vmfs: Vec<PathBuf>,
    recent_projects: Vec<PathBuf>,
    compile_profile_path: String,
    profile_wizard_output_path: String,
    profile_wizard_vbsp_path: String,
    profile_wizard_vvis_path: String,
    profile_wizard_vrad_path: String,
    profile_wizard_game_path: String,
    profile_wizard_log_dir: String,
    profile_wizard_steps: String,
    profile_wizard_timeout_seconds: String,
    profile_wizard_discover_dir: String,
    profile_wizard_status: DesktopProfileWizardStatus,
    profile_wizard_receiver: Option<Receiver<DesktopProfileWizardMessage>>,
    compile_steps: String,
    compile_log_dir: String,
    compile_report_path: String,
    compile_timeout_seconds: String,
    compile_run_after_merge: bool,
    compile_status: DesktopCompileStatus,
    compile_receiver: Option<Receiver<DesktopCompileMessage>>,
    bsp_pack_tool_path: String,
    bsp_pack_input_bsp: String,
    bsp_pack_output_bsp: String,
    bsp_pack_asset_roots: String,
    bsp_pack_includes: String,
    bsp_pack_filelist_path: String,
    bsp_pack_log_path: String,
    bsp_pack_report_path: String,
    bsp_pack_timeout_seconds: String,
    bsp_pack_after_compile: bool,
    bsp_pack_status: DesktopBspPackStatus,
    bsp_pack_receiver: Option<Receiver<DesktopBspPackMessage>>,
    model_inspect_mdl_path: String,
    model_inspect_status: DesktopModelInspectStatus,
    model_inspect_receiver: Option<Receiver<DesktopModelInspectMessage>>,
    model_compile_qc_path: String,
    model_compile_studiomdl_path: String,
    model_compile_game_path: String,
    model_compile_tool_args: String,
    model_compile_log_path: String,
    model_compile_report_path: String,
    model_compile_timeout_seconds: String,
    model_compile_status: DesktopModelCompileStatus,
    model_compile_receiver: Option<Receiver<DesktopModelCompileMessage>>,
    last_error_dialog: Option<String>,
    use_dark_theme: bool,
    preview_panel_height: f32,
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
    entity_semantics: EntitySemanticsReport,
    complexity: MapComplexityReport,
    rule_set: Option<RuleSetValidationReport>,
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

#[derive(Debug, Clone, Default)]
struct DesktopProfileWizardStatus {
    running: bool,
    summary: String,
    command: Vec<String>,
    report_json: Option<String>,
    stdout_tail: Vec<String>,
    stderr_tail: Vec<String>,
}

#[derive(Debug, Clone)]
struct DesktopProfileWizardRequest {
    cli_path: PathBuf,
    action: DesktopProfileWizardAction,
    profile_path: PathBuf,
    vbsp: Option<PathBuf>,
    vvis: Option<PathBuf>,
    vrad: Option<PathBuf>,
    game: Option<PathBuf>,
    log_dir: Option<PathBuf>,
    steps: Option<String>,
    timeout_seconds: Option<u64>,
    search_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
enum DesktopProfileWizardAction {
    CreateValidate,
    Validate,
    Discover,
}

#[derive(Debug, Clone)]
struct DesktopProfileWizardMessage {
    ok: bool,
    summary: String,
    command: Vec<String>,
    report_json: Option<String>,
    stdout_tail: Vec<String>,
    stderr_tail: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct DesktopCompileStatus {
    running: bool,
    summary: String,
    command: Vec<String>,
    report_json: Option<String>,
    stdout_tail: Vec<String>,
    stderr_tail: Vec<String>,
}

#[derive(Debug, Clone)]
struct DesktopCompileRequest {
    cli_path: PathBuf,
    map_path: PathBuf,
    profile_path: PathBuf,
    steps: Option<String>,
    log_dir: Option<PathBuf>,
    report_path: PathBuf,
    timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone)]
struct DesktopCompileMessage {
    ok: bool,
    summary: String,
    command: Vec<String>,
    report_json: Option<String>,
    stdout_tail: Vec<String>,
    stderr_tail: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct DesktopBspPackStatus {
    running: bool,
    summary: String,
    command: Vec<String>,
    report_json: Option<String>,
    stdout_tail: Vec<String>,
    stderr_tail: Vec<String>,
    missing_files: usize,
    packed_file_count: Option<u64>,
}

#[derive(Debug, Clone)]
struct DesktopBspPackRequest {
    cli_path: PathBuf,
    tool_path: PathBuf,
    input_bsp: PathBuf,
    output_bsp: PathBuf,
    asset_roots: Vec<PathBuf>,
    includes: Vec<String>,
    filelist_path: Option<PathBuf>,
    log_path: Option<PathBuf>,
    report_path: PathBuf,
    timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone)]
struct DesktopBspPackMessage {
    ok: bool,
    summary: String,
    command: Vec<String>,
    report_json: Option<String>,
    stdout_tail: Vec<String>,
    stderr_tail: Vec<String>,
    missing_files: usize,
    packed_file_count: Option<u64>,
}

#[derive(Debug, Clone)]
struct DesktopModelInspectStatus {
    running: bool,
    summary: String,
    command: Vec<String>,
    report_json: Option<String>,
    stdout_tail: Vec<String>,
    stderr_tail: Vec<String>,
}

impl Default for DesktopModelInspectStatus {
    fn default() -> Self {
        Self {
            running: false,
            summary: "Model inspect idle. Select an MDL to inspect metadata.".to_string(),
            command: Vec::new(),
            report_json: None,
            stdout_tail: Vec::new(),
            stderr_tail: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct DesktopModelInspectRequest {
    cli_path: PathBuf,
    mdl_path: PathBuf,
}

#[derive(Debug, Clone)]
struct DesktopModelInspectMessage {
    ok: bool,
    summary: String,
    command: Vec<String>,
    report_json: Option<String>,
    stdout_tail: Vec<String>,
    stderr_tail: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct DesktopModelCompileStatus {
    running: bool,
    summary: String,
    command: Vec<String>,
    report_json: Option<String>,
    stdout_tail: Vec<String>,
    stderr_tail: Vec<String>,
}

#[derive(Debug, Clone)]
struct DesktopModelCompileRequest {
    cli_path: PathBuf,
    qc_path: PathBuf,
    studiomdl_path: PathBuf,
    game_path: Option<PathBuf>,
    tool_args: Vec<String>,
    log_path: Option<PathBuf>,
    report_path: PathBuf,
    timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone)]
struct DesktopModelCompileMessage {
    ok: bool,
    summary: String,
    command: Vec<String>,
    report_json: Option<String>,
    stdout_tail: Vec<String>,
    stderr_tail: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct DesktopBspDecompileStatus {
    running: bool,
    summary: String,
    command: Vec<String>,
    output_vmf: Option<PathBuf>,
    report_json: Option<String>,
    stdout_tail: Vec<String>,
    stderr_tail: Vec<String>,
}

#[derive(Debug, Clone)]
struct DesktopBspDecompileRequest {
    cli_path: PathBuf,
    input_bsp: PathBuf,
    output_vmf: PathBuf,
    bspsource: Option<PathBuf>,
    bspsource_jar: Option<PathBuf>,
    java: Option<PathBuf>,
    wrapper: Option<PathBuf>,
    tool_args: Vec<String>,
    log_path: Option<PathBuf>,
    report_path: PathBuf,
    timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone)]
struct DesktopBspDecompileMessage {
    ok: bool,
    summary: String,
    command: Vec<String>,
    output_vmf: Option<PathBuf>,
    report_json: Option<String>,
    stdout_tail: Vec<String>,
    stderr_tail: Vec<String>,
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
    ThreeD,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewDetailMode {
    Fast,
    Auto,
    Full,
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
    changelevel_policy: Option<String>,
    #[serde(default)]
    changelevel_scope: Option<String>,
    #[serde(default)]
    preserve_external_map: Option<String>,
    #[serde(default)]
    preserve_external_landmark: Option<String>,
    #[serde(default)]
    preserve_external_targetname: Option<String>,
    #[serde(default)]
    delete: ProjectDeleteConfig,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    report: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectDeletionPresetFile {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    delete: ProjectDeleteConfig,
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
            changelevel_policy: ChangelevelPolicy::Preserve,
            changelevel_scope: ChangelevelScope::All,
            preserve_external_map: String::new(),
            preserve_external_landmark: String::new(),
            preserve_external_targetname: String::new(),
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
            custom_delete_preset_name: String::new(),
            custom_delete_preset_description: String::new(),
            custom_delete_preset_path: String::new(),
            pending_deletion_review: None,
            cleanup_export_confirmed: false,
            status: vec!["Ready. Add VMF files to inspect or merge.".to_string()],
            active_table: TableMode::Preview,
            preview_scope: PreviewScope::SelectedMap,
            merged_preview: None,
            preview_view: PreviewView::Top,
            preview_zoom: 1.0,
            preview_pan: egui::Vec2::ZERO,
            preview_3d_yaw: 45.0,
            preview_3d_pitch: 35.264,
            preview_show_solids: true,
            preview_show_entities: true,
            preview_show_grid: true,
            preview_detail_mode: PreviewDetailMode::Auto,
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
            validation_rule_set: NO_VALIDATION_RULE_SET_ID.to_string(),
            bsp_derived_vmfs: BTreeSet::new(),
            bsp_decompile_bsp_path: String::new(),
            bsp_decompile_output_vmf: String::new(),
            bsp_decompile_bspsource_path: String::new(),
            bsp_decompile_jar_path: String::new(),
            bsp_decompile_java_path: String::new(),
            bsp_decompile_wrapper_path: String::new(),
            bsp_decompile_tool_args: String::new(),
            bsp_decompile_log_path: String::new(),
            bsp_decompile_report_path: String::new(),
            bsp_decompile_timeout_seconds: "900".to_string(),
            bsp_decompile_status: DesktopBspDecompileStatus {
                summary:
                    "BSP decompile runner idle. Select a user-provided BSPSource launcher or jar."
                        .to_string(),
                ..Default::default()
            },
            bsp_decompile_receiver: None,
            recent_vmfs: Vec::new(),
            recent_projects: Vec::new(),
            compile_profile_path: String::new(),
            profile_wizard_output_path: String::new(),
            profile_wizard_vbsp_path: String::new(),
            profile_wizard_vvis_path: String::new(),
            profile_wizard_vrad_path: String::new(),
            profile_wizard_game_path: String::new(),
            profile_wizard_log_dir: String::new(),
            profile_wizard_steps: "vbsp,vvis,vrad".to_string(),
            profile_wizard_timeout_seconds: "900".to_string(),
            profile_wizard_discover_dir: String::new(),
            profile_wizard_status: DesktopProfileWizardStatus {
                summary: "Compile profile wizard idle. Create, validate, or discover user-provided tools.".to_string(),
                ..Default::default()
            },
            profile_wizard_receiver: None,
            compile_steps: "vbsp,vvis,vrad".to_string(),
            compile_log_dir: String::new(),
            compile_report_path: String::new(),
            compile_timeout_seconds: "900".to_string(),
            compile_run_after_merge: false,
            compile_status: DesktopCompileStatus {
                summary: "Compile runner idle. External Source tools are required.".to_string(),
                ..Default::default()
            },
            compile_receiver: None,
            bsp_pack_tool_path: String::new(),
            bsp_pack_input_bsp: String::new(),
            bsp_pack_output_bsp: String::new(),
            bsp_pack_asset_roots: String::new(),
            bsp_pack_includes: String::new(),
            bsp_pack_filelist_path: String::new(),
            bsp_pack_log_path: String::new(),
            bsp_pack_report_path: String::new(),
            bsp_pack_timeout_seconds: "900".to_string(),
            bsp_pack_after_compile: false,
            bsp_pack_status: DesktopBspPackStatus {
                summary: "BSP packing idle. A user-provided BSPZIP-compatible tool is required."
                    .to_string(),
                ..Default::default()
            },
            bsp_pack_receiver: None,
            model_inspect_mdl_path: String::new(),
            model_inspect_status: DesktopModelInspectStatus::default(),
            model_inspect_receiver: None,
            model_compile_qc_path: String::new(),
            model_compile_studiomdl_path: String::new(),
            model_compile_game_path: String::new(),
            model_compile_tool_args: String::new(),
            model_compile_log_path: String::new(),
            model_compile_report_path: String::new(),
            model_compile_timeout_seconds: "900".to_string(),
            model_compile_status: DesktopModelCompileStatus {
                summary: "Model compile idle. StudioMDL-compatible tools and model assets are user-provided.".to_string(),
                ..Default::default()
            },
            model_compile_receiver: None,
            last_error_dialog: None,
            use_dark_theme: true,
            preview_panel_height: 560.0,
        }
    }

    fn add_status(&mut self, message: impl Into<String>) {
        let message = message.into();
        let lowered = message.to_ascii_lowercase();
        if lowered.contains("failed")
            || lowered.contains("error")
            || lowered.contains("invalid")
            || lowered.contains("parse/load")
        {
            self.last_error_dialog = Some(message.clone());
        }
        self.status.push(message);
        if self.status.len() > 12 {
            let overflow = self.status.len() - 12;
            self.status.drain(0..overflow);
        }
    }

    fn remember_recent_vmf(&mut self, path: PathBuf) {
        remember_recent_path(&mut self.recent_vmfs, path);
    }

    fn remember_recent_project(&mut self, path: PathBuf) {
        remember_recent_path(&mut self.recent_projects, path);
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|input| input.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }

        let mut vmfs = Vec::new();
        let mut projects = Vec::new();
        let mut fgds = Vec::new();
        for file in dropped {
            let Some(path) = file.path else {
                continue;
            };
            match path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.to_ascii_lowercase())
                .as_deref()
            {
                Some("vmf") => vmfs.push(path),
                Some("toml") => projects.push(path),
                Some("fgd") => fgds.push(path),
                _ => self.add_status(format!(
                    "Ignored dropped file with unsupported extension: {}",
                    display_path(&path)
                )),
            }
        }

        if !vmfs.is_empty() {
            self.add_vmf_paths(vmfs);
        }
        for project in projects {
            self.load_project_path(project);
        }
        if !fgds.is_empty() {
            self.load_fgd_paths(fgds);
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

        self.load_fgd_paths(paths);
    }

    fn load_fgd_paths(&mut self, paths: Vec<PathBuf>) {
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
                Ok(()) => {
                    self.remember_recent_project(path.clone());
                    self.add_status(format!("Saved project {}.", display_path(&path)));
                }
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

        self.load_project_path(path);
    }

    fn load_project_path(&mut self, path: PathBuf) {
        self.remember_recent_project(path.clone());

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
            changelevel_policy: Some(self.changelevel_policy.to_string()),
            changelevel_scope: Some(self.changelevel_scope.to_string()),
            preserve_external_map: blank_to_none(&self.preserve_external_map),
            preserve_external_landmark: blank_to_none(&self.preserve_external_landmark),
            preserve_external_targetname: blank_to_none(&self.preserve_external_targetname),
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
        let rule_set_id = self
            .selected_validation_rule_set_id()
            .map(ToOwned::to_owned);
        self.maps = paths
            .into_iter()
            .map(|path| MapEntry::load(path, rule_set_id.as_deref()))
            .collect();
        self.selected_map = (!self.maps.is_empty()).then_some(0);
        self.base_index = 0;
        self.landmark = project.landmark.unwrap_or_default();
        self.changelevel_policy = project
            .changelevel_policy
            .as_deref()
            .and_then(ChangelevelPolicy::parse)
            .unwrap_or(ChangelevelPolicy::Preserve);
        self.changelevel_scope = project
            .changelevel_scope
            .as_deref()
            .and_then(ChangelevelScope::parse)
            .unwrap_or(ChangelevelScope::All);
        self.preserve_external_map = project.preserve_external_map.unwrap_or_default();
        self.preserve_external_landmark = project.preserve_external_landmark.unwrap_or_default();
        self.preserve_external_targetname =
            project.preserve_external_targetname.unwrap_or_default();
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
            self.add_vmf_paths(files);
        }
    }

    fn add_bsp_derived_vmf_dialog(&mut self) {
        if let Some(files) = rfd::FileDialog::new()
            .set_title("Import BSP-derived VMF")
            .add_filter("Valve Map Format", &["vmf"])
            .pick_files()
        {
            for file in &files {
                self.bsp_derived_vmfs.insert(display_path(file));
            }
            self.add_vmf_paths(files);
            self.add_status("Imported BSP-derived VMF(s). Review parse/integrity warnings before merge; decompiled VMFs can have broken solids, areaportals, materials, overlays, or missing editor metadata.");
        }
    }

    fn add_vmf_paths(&mut self, files: Vec<PathBuf>) {
        let mut added = 0;
        for file in files {
            self.remember_recent_vmf(file.clone());
            if self.maps.iter().any(|entry| entry.path == file) {
                continue;
            }
            let rule_set_id = self
                .selected_validation_rule_set_id()
                .map(ToOwned::to_owned);
            self.maps.push(MapEntry::load(file, rule_set_id.as_deref()));
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

    fn rescan_maps(&mut self) {
        let rule_set_id = self
            .selected_validation_rule_set_id()
            .map(ToOwned::to_owned);
        for map in &mut self.maps {
            let path = map.path.clone();
            *map = MapEntry::load(path, rule_set_id.as_deref());
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
        self.bsp_derived_vmfs.clear();
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

    fn save_custom_deletion_preset(&mut self) {
        let Some(path) = blank_to_none(&self.custom_delete_preset_path).map(PathBuf::from) else {
            self.add_status("Set a deletion preset path before saving/exporting.");
            return;
        };
        let Some(name) = blank_to_none(&self.custom_delete_preset_name) else {
            self.add_status("Set a deletion preset name before saving/exporting.");
            return;
        };
        let preset = ProjectDeletionPresetFile {
            name,
            description: blank_to_none(&self.custom_delete_preset_description),
            delete: ProjectDeleteConfig::from_criteria(&self.build_deletion_criteria()),
        };
        let text = match toml::to_string_pretty(&preset) {
            Ok(text) => text,
            Err(error) => {
                self.add_status(format!("Failed to encode deletion preset TOML: {error}"));
                return;
            }
        };
        if let Some(parent) = path.parent() {
            match fs::create_dir_all(parent) {
                Ok(()) => {}
                Err(error) => {
                    self.add_status(format!(
                        "Failed to create deletion preset directory {}: {error}",
                        parent.display()
                    ));
                    return;
                }
            }
        }
        match fs::write(&path, text) {
            Ok(()) => self.add_status(format!(
                "Saved deletion preset `{}` to {}.",
                preset.name,
                path.display()
            )),
            Err(error) => self.add_status(format!(
                "Failed to write deletion preset {}: {error}",
                path.display()
            )),
        }
    }

    fn load_custom_deletion_preset(&mut self) {
        let Some(path) = blank_to_none(&self.custom_delete_preset_path).map(PathBuf::from) else {
            self.add_status("Set a deletion preset path before loading/importing.");
            return;
        };
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                self.add_status(format!(
                    "Failed to read deletion preset {}: {error}",
                    path.display()
                ));
                return;
            }
        };
        let preset = match toml::from_str::<ProjectDeletionPresetFile>(&text) {
            Ok(preset) => preset,
            Err(error) => {
                self.add_status(format!(
                    "Failed to parse deletion preset {}: {error}",
                    path.display()
                ));
                return;
            }
        };
        let criteria = match preset.delete.to_criteria() {
            Ok(criteria) => criteria,
            Err(error) => {
                self.add_status(format!(
                    "Deletion preset `{}` has invalid criteria: {error}",
                    preset.name
                ));
                return;
            }
        };
        self.custom_delete_preset_name = preset.name.clone();
        self.custom_delete_preset_description = preset.description.clone().unwrap_or_default();
        self.apply_deletion_criteria_to_controls(criteria);
        self.add_status(format!(
            "Loaded deletion preset `{}` from {}. Preview deletion to verify counts before export.",
            preset.name,
            path.display()
        ));
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

    fn selected_validation_rule_set_id(&self) -> Option<&str> {
        let value = self.validation_rule_set.trim();
        if value.is_empty() || value.eq_ignore_ascii_case(NO_VALIDATION_RULE_SET_ID) {
            None
        } else {
            Some(value)
        }
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

    fn complexity_status_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for entry in &self.maps {
            if let Ok(analysis) = &entry.analysis {
                for risk in &analysis.complexity.risks {
                    lines.push(format!("Complexity warning: {}", risk.message));
                }
            }
        }
        lines
    }

    fn entity_semantics_status_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for entry in &self.maps {
            if let Ok(analysis) = &entry.analysis {
                for issue in &analysis.entity_semantics.issues {
                    lines.push(format!(
                        "Entity semantics {}",
                        format_entity_semantics_issue(issue)
                    ));
                }
            }
        }
        lines
    }

    fn rule_set_status_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for entry in &self.maps {
            if let Some(report) = entry
                .analysis
                .as_ref()
                .ok()
                .and_then(|analysis| analysis.rule_set.as_ref())
            {
                for issue in &report.issues {
                    lines.push(format!("Rule set {}", format_rule_set_issue(issue)));
                }
            }
        }
        lines
    }

    fn draw_integrity_status(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label("VMF integrity status");
            let mut selected_rule_set = self.validation_rule_set.clone();
            ui.horizontal(|ui| {
                ui.label("Rule set");
                egui::ComboBox::from_id_salt("validation_rule_set_combo")
                    .selected_text(validation_rule_set_combo_label(&selected_rule_set))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut selected_rule_set,
                            NO_VALIDATION_RULE_SET_ID.to_string(),
                            "none: generic VMF integrity only",
                        );
                        for rule_set in BUILTIN_VALIDATION_RULE_SETS {
                            ui.selectable_value(
                                &mut selected_rule_set,
                                rule_set.id.to_string(),
                                format!("{}: {}", rule_set.id, rule_set.name),
                            );
                        }
                    });
            });
            if selected_rule_set != self.validation_rule_set {
                self.validation_rule_set = selected_rule_set;
                self.rescan_maps();
                self.add_status(format!(
                    "Selected validation rule set `{}`.",
                    self.validation_rule_set
                ));
            }
            ui.weak("Rule sets are portable Source Weaver checks; they do not run Hammer, VBSP, VVIS, VRAD, or a game runtime.");

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
                                let semantic_errors = analysis.entity_semantics.error_count();
                                let semantic_warnings = analysis.entity_semantics.warning_count();
                                let complexity_warnings = analysis.complexity.warning_count();
                                let rule_errors = analysis
                                    .rule_set
                                    .as_ref()
                                    .map(RuleSetValidationReport::error_count)
                                    .unwrap_or(0);
                                let rule_warnings = analysis
                                    .rule_set
                                    .as_ref()
                                    .map(RuleSetValidationReport::warning_count)
                                    .unwrap_or(0);
                                let summary = format!(
                                    "{errors} integrity error(s), {warnings} integrity warning(s), {semantic_errors} semantic error(s), {semantic_warnings} semantic warning(s), {complexity_warnings} complexity warning(s), {rule_errors} rule error(s), {rule_warnings} rule warning(s)"
                                );
                                if errors > 0 || semantic_errors > 0 || rule_errors > 0 {
                                    ui.colored_label(egui::Color32::LIGHT_RED, summary);
                                } else if warnings > 0
                                    || semantic_warnings > 0
                                    || complexity_warnings > 0
                                    || rule_warnings > 0
                                {
                                    ui.colored_label(egui::Color32::YELLOW, summary);
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
            let complexity_lines = self.complexity_status_lines();
            if !complexity_lines.is_empty() {
                ui.collapsing("Complexity risk details", |ui| {
                    for line in complexity_lines {
                        ui.small(line);
                    }
                });
            }
            let semantic_lines = self.entity_semantics_status_lines();
            if !semantic_lines.is_empty() {
                ui.collapsing("Entity semantic details", |ui| {
                    for line in semantic_lines {
                        ui.small(line);
                    }
                });
            }
            let rule_lines = self.rule_set_status_lines();
            if !rule_lines.is_empty() {
                ui.collapsing("Rule-set details", |ui| {
                    for line in rule_lines {
                        ui.small(line);
                    }
                });
            }
        });
    }

    fn draw_scan_progress(&self, ui: &mut egui::Ui) {
        let total = self.maps.len();
        if total == 0 {
            return;
        }
        let loaded = self
            .maps
            .iter()
            .filter(|entry| entry.analysis.is_ok())
            .count();
        let failed = total.saturating_sub(loaded);
        let fraction = loaded as f32 / total as f32;
        ui.add(
            egui::ProgressBar::new(fraction)
                .show_percentage()
                .text(format!("Parsed {loaded}/{total} selected VMF(s)")),
        );
        if failed > 0 {
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                format!("{failed} VMF(s) need attention."),
            );
        }
    }

    fn draw_recent_paths(&mut self, ui: &mut egui::Ui) {
        if self.recent_vmfs.is_empty() && self.recent_projects.is_empty() {
            return;
        }

        ui.collapsing("Recent files", |ui| {
            if !self.recent_projects.is_empty() {
                ui.label("Projects/jobs");
                for path in self.recent_projects.clone() {
                    ui.horizontal(|ui| {
                        if ui.button("Open").clicked() {
                            self.load_project_path(path.clone());
                        }
                        ui.label(file_label_for_legend(&display_path(&path)))
                            .on_hover_text(display_path(&path));
                    });
                }
            }
            if !self.recent_vmfs.is_empty() {
                ui.label("VMFs");
                for path in self.recent_vmfs.clone() {
                    ui.horizontal(|ui| {
                        if ui.button("Add").clicked() {
                            self.add_vmf_paths(vec![path.clone()]);
                        }
                        ui.label(file_label_for_legend(&display_path(&path)))
                            .on_hover_text(display_path(&path));
                    });
                }
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
                if !suggestion.landmark_pairs.is_empty()
                    && ui.button("Use first suggested landmark").clicked()
                {
                    let first_pair = &suggestion.landmark_pairs[0];
                    self.landmark = first_pair.landmark.clone();
                    self.clear_merged_preview();
                    self.add_status(format!(
                        "Using suggested landmark `{}`. You can still edit it manually.",
                        first_pair.landmark
                    ));
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

    fn current_changelevel_options(&self) -> ChangelevelPolicyOptions {
        let output_path = PathBuf::from(self.output_path.trim());
        let output_map = output_path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string());
        let stitched_maps = self
            .maps
            .iter()
            .filter_map(|entry| {
                entry
                    .path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
            })
            .collect::<Vec<_>>();
        let mut preserve_external = Vec::new();
        if let Some(map) = blank_to_none(&self.preserve_external_map) {
            preserve_external.push(ChangelevelPreserveRule {
                map: Some(map),
                landmark: None,
                targetname: None,
            });
        }
        if let Some(landmark) = blank_to_none(&self.preserve_external_landmark) {
            preserve_external.push(ChangelevelPreserveRule {
                map: None,
                landmark: Some(landmark),
                targetname: None,
            });
        }
        if let Some(targetname) = blank_to_none(&self.preserve_external_targetname) {
            preserve_external.push(ChangelevelPreserveRule {
                map: None,
                landmark: None,
                targetname: Some(targetname),
            });
        }
        ChangelevelPolicyOptions {
            policy: self.changelevel_policy,
            scope: self.changelevel_scope,
            output_map,
            stitched_maps,
            preserve_external,
        }
    }

    fn add_changelevel_report_status(
        &mut self,
        report: &sourceweaver_core::ChangelevelPolicyReport,
    ) {
        self.add_status(format!(
            "Changelevel policy `{}` with scope `{}` changed {} transition entity/entities and preserved {}.",
            report.policy,
            report.scope,
            report.changed_count(),
            report.preserved.len()
        ));
        for warning in &report.warnings {
            self.add_status(format!("Changelevel warning: {warning}"));
        }
        for change in &report.changed {
            self.add_status(format!(
                "Changelevel {} entity[{}]: {}",
                change.action, change.entity_index, change.rationale
            ));
        }
        for preserved in &report.preserved {
            self.add_status(format!(
                "Changelevel preserved entity[{}]: {}",
                preserved.entity_index, preserved.reason
            ));
        }
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
        match merge_maps(
            merge_inputs,
            &MergeOptions {
                landmark,
                changelevel: self.current_changelevel_options(),
            },
        ) {
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
                self.add_changelevel_report_status(&report.changelevel);
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
        match merge_maps(
            merge_inputs,
            &MergeOptions {
                landmark,
                changelevel: self.current_changelevel_options(),
            },
        ) {
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
                        for (label, offset) in &report.applied_offsets {
                            self.add_status(format!("Offset {label}: {offset}"));
                        }
                        self.add_changelevel_report_status(&report.changelevel);
                        if self.compile_run_after_merge {
                            self.launch_compile_for_path(output_path.clone());
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
    fn choose_bsp_decompile_input(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select BSP to decompile")
            .add_filter("Source BSP", &["bsp"])
            .pick_file()
        {
            self.bsp_decompile_bsp_path = display_path(&path);
            if self.bsp_decompile_output_vmf.trim().is_empty() {
                self.bsp_decompile_output_vmf =
                    display_path(&default_bsp_decompile_output_path(&path));
            }
            if self.bsp_decompile_report_path.trim().is_empty() {
                self.bsp_decompile_report_path =
                    display_path(&default_bsp_decompile_report_path(&path));
            }
            if self.bsp_decompile_log_path.trim().is_empty() {
                self.bsp_decompile_log_path = display_path(&default_bsp_decompile_log_path(&path));
            }
        }
    }

    fn choose_bsp_decompile_output(&mut self) {
        let default_name = blank_to_none(&self.bsp_decompile_bsp_path)
            .map(|path| default_bsp_decompile_output_path(&PathBuf::from(path)))
            .and_then(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_else(|| "decompiled_map.vmf".to_string());
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Save decompiled VMF")
            .add_filter("Valve Map Format", &["vmf"])
            .set_file_name(&default_name)
            .save_file()
        {
            self.bsp_decompile_output_vmf = display_path(&path);
        }
    }

    fn choose_bsp_decompile_bspsource(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select BSPSource launcher")
            .pick_file()
        {
            self.bsp_decompile_bspsource_path = display_path(&path);
            self.bsp_decompile_jar_path.clear();
            self.bsp_decompile_wrapper_path.clear();
        }
    }

    fn choose_bsp_decompile_jar(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select BSPSource jar")
            .add_filter("Java archive", &["jar"])
            .pick_file()
        {
            self.bsp_decompile_jar_path = display_path(&path);
            self.bsp_decompile_bspsource_path.clear();
            self.bsp_decompile_wrapper_path.clear();
        }
    }

    fn choose_bsp_decompile_java(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select Java executable")
            .pick_file()
        {
            self.bsp_decompile_java_path = display_path(&path);
        }
    }

    fn choose_bsp_decompile_wrapper(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select generic BSP decompiler wrapper")
            .pick_file()
        {
            self.bsp_decompile_wrapper_path = display_path(&path);
            self.bsp_decompile_bspsource_path.clear();
            self.bsp_decompile_jar_path.clear();
        }
    }

    fn choose_bsp_decompile_log(&mut self) {
        let default_name = blank_to_none(&self.bsp_decompile_bsp_path)
            .map(|path| default_bsp_decompile_log_path(&PathBuf::from(path)))
            .and_then(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_else(|| "bsp-decompile.log".to_string());
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Save BSP decompile log")
            .add_filter("Log", &["log", "txt"])
            .set_file_name(&default_name)
            .save_file()
        {
            self.bsp_decompile_log_path = display_path(&path);
        }
    }

    fn choose_bsp_decompile_report(&mut self) {
        let default_name = blank_to_none(&self.bsp_decompile_bsp_path)
            .map(|path| default_bsp_decompile_report_path(&PathBuf::from(path)))
            .and_then(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_else(|| "bsp-import-report.json".to_string());
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Save BSP decompile report")
            .add_filter("JSON", &["json"])
            .set_file_name(&default_name)
            .save_file()
        {
            self.bsp_decompile_report_path = display_path(&path);
        }
    }

    fn launch_bsp_decompile(&mut self) {
        if self.bsp_decompile_status.running {
            self.add_status("A BSP decompile run is already in progress.");
            return;
        }
        let Some(input_bsp) = blank_to_none(&self.bsp_decompile_bsp_path).map(PathBuf::from) else {
            self.add_status("Select a BSP before launching decompile.");
            return;
        };
        if !input_bsp.exists() {
            self.add_status(format!(
                "Input BSP does not exist: {}",
                display_path(&input_bsp)
            ));
            return;
        }
        let Some(output_vmf) = blank_to_none(&self.bsp_decompile_output_vmf).map(PathBuf::from)
        else {
            self.add_status("Choose an output VMF path before launching decompile.");
            return;
        };
        let bspsource = blank_to_none(&self.bsp_decompile_bspsource_path).map(PathBuf::from);
        let bspsource_jar = blank_to_none(&self.bsp_decompile_jar_path).map(PathBuf::from);
        let wrapper = blank_to_none(&self.bsp_decompile_wrapper_path).map(PathBuf::from);
        let configured = usize::from(bspsource.is_some())
            + usize::from(bspsource_jar.is_some())
            + usize::from(wrapper.is_some());
        if configured != 1 {
            self.add_status("Select exactly one BSP decompiler mode: BSPSource launcher, BSPSource jar, or generic wrapper.");
            return;
        }
        let timeout_seconds = match blank_to_none(&self.bsp_decompile_timeout_seconds) {
            Some(value) => match value.parse::<u64>() {
                Ok(seconds) if seconds > 0 => Some(seconds),
                _ => {
                    self.add_status(
                        "BSP decompile timeout must be a positive integer number of seconds.",
                    );
                    return;
                }
            },
            None => None,
        };
        let report_path = blank_to_none(&self.bsp_decompile_report_path)
            .map(PathBuf::from)
            .unwrap_or_else(|| default_bsp_decompile_report_path(&input_bsp));
        let request = DesktopBspDecompileRequest {
            cli_path: sourceweaver_cli_executable(),
            input_bsp,
            output_vmf,
            bspsource,
            bspsource_jar,
            java: blank_to_none(&self.bsp_decompile_java_path).map(PathBuf::from),
            wrapper,
            tool_args: self
                .bsp_decompile_tool_args
                .split_whitespace()
                .map(ToOwned::to_owned)
                .collect(),
            log_path: blank_to_none(&self.bsp_decompile_log_path).map(PathBuf::from),
            report_path,
            timeout_seconds,
        };
        let command_preview = desktop_bsp_decompile_command_preview(&request);
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let message = run_desktop_bsp_decompile_request(request);
            let _ = sender.send(message);
        });
        self.bsp_decompile_receiver = Some(receiver);
        self.bsp_decompile_status = DesktopBspDecompileStatus {
            running: true,
            summary: "BSP decompile running in background. Output VMF will be imported and tagged when validation succeeds.".to_string(),
            command: command_preview.clone(),
            output_vmf: None,
            report_json: None,
            stdout_tail: Vec::new(),
            stderr_tail: Vec::new(),
        };
        self.add_status(format!(
            "Started BSP decompile: {}",
            command_preview.join(" ")
        ));
    }

    fn poll_bsp_decompile_status(&mut self) {
        let Some(receiver) = self.bsp_decompile_receiver.take() else {
            return;
        };
        match receiver.try_recv() {
            Ok(message) => {
                self.bsp_decompile_status.running = false;
                self.bsp_decompile_status.summary = message.summary.clone();
                self.bsp_decompile_status.command = message.command;
                self.bsp_decompile_status.output_vmf = message.output_vmf.clone();
                self.bsp_decompile_status.report_json = message.report_json;
                self.bsp_decompile_status.stdout_tail = message.stdout_tail;
                self.bsp_decompile_status.stderr_tail = message.stderr_tail;
                self.add_status(message.summary);
                if message.ok {
                    if let Some(output_vmf) = message.output_vmf {
                        self.bsp_derived_vmfs.insert(display_path(&output_vmf));
                        self.add_vmf_paths(vec![output_vmf]);
                        self.add_status("Imported BSP-derived VMF. Review decompile warnings before merge; decompiled VMFs are approximate and review-required.");
                    }
                } else {
                    self.last_error_dialog = Some(
                        "BSP decompile failed. Review the BSP decompile panel JSON/log details."
                            .to_string(),
                    );
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                self.bsp_decompile_receiver = Some(receiver);
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.bsp_decompile_status.running = false;
                self.bsp_decompile_status.summary =
                    "BSP decompile worker disconnected before reporting a result.".to_string();
                self.add_status("BSP decompile worker disconnected before reporting a result.");
            }
        }
    }

    fn choose_compile_profile_path(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select Source Weaver compile profile")
            .add_filter("TOML", &["toml"])
            .pick_file()
        {
            self.compile_profile_path = display_path(&path);
            self.remember_recent_project(path);
        }
    }

    fn choose_compile_log_dir(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select compile log directory")
            .pick_folder()
        {
            self.compile_log_dir = display_path(&path);
        }
    }

    fn choose_compile_report_path(&mut self) {
        let default_name = default_compile_report_path(&self.output_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("sourceweaver-compile-report.json")
            .to_string();
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Save compile report")
            .add_filter("JSON", &["json"])
            .set_file_name(&default_name)
            .save_file()
        {
            self.compile_report_path = display_path(&path);
        }
    }

    fn profile_wizard_request(
        &mut self,
        action: DesktopProfileWizardAction,
    ) -> Option<DesktopProfileWizardRequest> {
        if self.profile_wizard_status.running {
            self.add_status("A compile profile wizard action is already running.");
            return None;
        }
        let profile_path = blank_to_none(&self.profile_wizard_output_path)
            .or_else(|| blank_to_none(&self.compile_profile_path))
            .map(PathBuf::from);
        let Some(profile_path) = profile_path else {
            self.add_status("Set a compile profile output/validation path first.");
            return None;
        };
        let timeout_seconds = match blank_to_none(&self.profile_wizard_timeout_seconds) {
            Some(value) => match value.parse::<u64>() {
                Ok(seconds) if seconds > 0 => Some(seconds),
                _ => {
                    self.add_status(
                        "Profile timeout must be a positive integer number of seconds.",
                    );
                    return None;
                }
            },
            None => None,
        };
        Some(DesktopProfileWizardRequest {
            cli_path: sourceweaver_cli_executable(),
            action,
            profile_path,
            vbsp: blank_to_none(&self.profile_wizard_vbsp_path).map(PathBuf::from),
            vvis: blank_to_none(&self.profile_wizard_vvis_path).map(PathBuf::from),
            vrad: blank_to_none(&self.profile_wizard_vrad_path).map(PathBuf::from),
            game: blank_to_none(&self.profile_wizard_game_path).map(PathBuf::from),
            log_dir: blank_to_none(&self.profile_wizard_log_dir).map(PathBuf::from),
            steps: blank_to_none(&self.profile_wizard_steps),
            timeout_seconds,
            search_dir: blank_to_none(&self.profile_wizard_discover_dir).map(PathBuf::from),
        })
    }

    fn launch_profile_wizard(&mut self, action: DesktopProfileWizardAction) {
        let Some(request) = self.profile_wizard_request(action) else {
            return;
        };
        let command_preview = desktop_profile_wizard_command_preview(&request);
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let message = run_desktop_profile_wizard_request(request);
            let _ = sender.send(message);
        });
        self.profile_wizard_receiver = Some(receiver);
        self.profile_wizard_status = DesktopProfileWizardStatus {
            running: true,
            summary: "Compile profile wizard running in background.".to_string(),
            command: command_preview.clone(),
            report_json: None,
            stdout_tail: Vec::new(),
            stderr_tail: Vec::new(),
        };
        self.add_status(format!(
            "Started compile profile wizard: {}",
            command_preview.join(" ")
        ));
    }

    fn poll_profile_wizard_status(&mut self) {
        let Some(receiver) = self.profile_wizard_receiver.take() else {
            return;
        };
        match receiver.try_recv() {
            Ok(message) => {
                self.profile_wizard_status.running = false;
                self.profile_wizard_status.summary = message.summary.clone();
                self.profile_wizard_status.command = message.command;
                self.profile_wizard_status.report_json = message.report_json;
                self.profile_wizard_status.stdout_tail = message.stdout_tail;
                self.profile_wizard_status.stderr_tail = message.stderr_tail;
                self.add_status(message.summary);
                if message.ok {
                    if let Some(path) = blank_to_none(&self.profile_wizard_output_path) {
                        self.compile_profile_path = path;
                    }
                } else {
                    self.last_error_dialog = Some("Compile profile wizard reported missing tools/game paths or invalid settings. Review the JSON/output details.".to_string());
                }
            }
            Err(mpsc::TryRecvError::Empty) => self.profile_wizard_receiver = Some(receiver),
            Err(mpsc::TryRecvError::Disconnected) => {
                self.profile_wizard_status.running = false;
                self.profile_wizard_status.summary =
                    "Compile profile wizard worker disconnected before reporting a result."
                        .to_string();
                self.add_status(
                    "Compile profile wizard worker disconnected before reporting a result.",
                );
            }
        }
    }

    fn launch_compile_for_current_output(&mut self) {
        if self.output_path.trim().is_empty() {
            self.add_status("Choose or export an output VMF before launching compile.");
            return;
        }
        self.launch_compile_for_path(PathBuf::from(self.output_path.trim()));
    }

    fn launch_compile_for_path(&mut self, map_path: PathBuf) {
        if self.compile_status.running {
            self.add_status("A compile run is already in progress.");
            return;
        }
        if self.compile_profile_path.trim().is_empty() {
            self.add_status("Select a compile profile before launching external Source tools.");
            return;
        }
        if !map_path.exists() {
            self.add_status(format!(
                "Compile input VMF does not exist yet: {}",
                display_path(&map_path)
            ));
            return;
        }
        let profile_path = PathBuf::from(self.compile_profile_path.trim());
        if !profile_path.exists() {
            self.add_status(format!(
                "Compile profile does not exist: {}",
                display_path(&profile_path)
            ));
            return;
        }
        let timeout_seconds = match blank_to_none(&self.compile_timeout_seconds) {
            Some(value) => match value.parse::<u64>() {
                Ok(seconds) if seconds > 0 => Some(seconds),
                _ => {
                    self.add_status(
                        "Compile timeout must be a positive integer number of seconds.",
                    );
                    return;
                }
            },
            None => None,
        };
        let report_path = blank_to_none(&self.compile_report_path)
            .map(PathBuf::from)
            .unwrap_or_else(|| default_compile_report_path_for_map(&map_path));
        let request = DesktopCompileRequest {
            cli_path: sourceweaver_cli_executable(),
            map_path,
            profile_path,
            steps: blank_to_none(&self.compile_steps),
            log_dir: blank_to_none(&self.compile_log_dir).map(PathBuf::from),
            report_path,
            timeout_seconds,
        };
        let command_preview = desktop_compile_command_preview(&request);
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let message = run_desktop_compile_request(request);
            let _ = sender.send(message);
        });
        self.compile_receiver = Some(receiver);
        self.compile_status = DesktopCompileStatus {
            running: true,
            summary: "Compile running in background. Merge/export success remains separate from external tool results.".to_string(),
            command: command_preview.clone(),
            report_json: None,
            stdout_tail: Vec::new(),
            stderr_tail: Vec::new(),
        };
        self.add_status(format!(
            "Started external compile: {}",
            command_preview.join(" ")
        ));
    }

    fn launch_bsp_pack_for_current_output(&mut self) {
        let input = blank_to_none(&self.bsp_pack_input_bsp)
            .map(PathBuf::from)
            .or_else(|| {
                blank_to_none(&self.output_path)
                    .map(|path| PathBuf::from(path).with_extension("bsp"))
            });
        let Some(input_bsp) = input else {
            self.add_status("Select an input BSP or set an output VMF path so Source Weaver can infer the .bsp path.");
            return;
        };
        self.launch_bsp_pack_for_bsp(input_bsp);
    }

    fn launch_bsp_pack_for_bsp(&mut self, input_bsp: PathBuf) {
        if self.bsp_pack_status.running {
            self.add_status("A BSP packing run is already in progress.");
            return;
        }
        let Some(tool_path) = blank_to_none(&self.bsp_pack_tool_path).map(PathBuf::from) else {
            self.add_status(
                "Select a user-provided BSPZIP-compatible packing tool before packing.",
            );
            return;
        };
        let output_bsp = blank_to_none(&self.bsp_pack_output_bsp)
            .map(PathBuf::from)
            .unwrap_or_else(|| default_packed_bsp_path(&input_bsp));
        let report_path = blank_to_none(&self.bsp_pack_report_path)
            .map(PathBuf::from)
            .unwrap_or_else(|| default_pack_report_path_for_bsp(&output_bsp));
        let timeout_seconds = match blank_to_none(&self.bsp_pack_timeout_seconds) {
            Some(value) => match value.parse::<u64>() {
                Ok(seconds) if seconds > 0 => Some(seconds),
                _ => {
                    self.add_status(
                        "BSP pack timeout must be a positive integer number of seconds.",
                    );
                    return;
                }
            },
            None => None,
        };
        let request = DesktopBspPackRequest {
            cli_path: sourceweaver_cli_executable(),
            tool_path,
            input_bsp,
            output_bsp,
            asset_roots: split_csv(&self.bsp_pack_asset_roots)
                .map(PathBuf::from)
                .collect(),
            includes: split_csv(&self.bsp_pack_includes)
                .map(ToOwned::to_owned)
                .collect(),
            filelist_path: blank_to_none(&self.bsp_pack_filelist_path).map(PathBuf::from),
            log_path: blank_to_none(&self.bsp_pack_log_path).map(PathBuf::from),
            report_path,
            timeout_seconds,
        };
        let command_preview = desktop_bsp_pack_command_preview(&request);
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let message = run_desktop_bsp_pack_request(request);
            let _ = sender.send(message);
        });
        self.bsp_pack_receiver = Some(receiver);
        self.bsp_pack_status = DesktopBspPackStatus {
            running: true,
            summary: "BSP packing running in background. Packing remains optional and separate from VMF export/compile success.".to_string(),
            command: command_preview.clone(),
            report_json: None,
            stdout_tail: Vec::new(),
            stderr_tail: Vec::new(),
            missing_files: 0,
            packed_file_count: None,
        };
        self.add_status(format!(
            "Started BSP packing: {}",
            command_preview.join(" ")
        ));
    }

    fn poll_bsp_pack_status(&mut self) {
        let Some(receiver) = self.bsp_pack_receiver.take() else {
            return;
        };
        match receiver.try_recv() {
            Ok(message) => {
                self.bsp_pack_status.running = false;
                self.bsp_pack_status.summary = message.summary.clone();
                self.bsp_pack_status.command = message.command;
                self.bsp_pack_status.report_json = message.report_json;
                self.bsp_pack_status.stdout_tail = message.stdout_tail;
                self.bsp_pack_status.stderr_tail = message.stderr_tail;
                self.bsp_pack_status.missing_files = message.missing_files;
                self.bsp_pack_status.packed_file_count = message.packed_file_count;
                let compile_ok = message.ok;
                self.add_status(message.summary);
                if compile_ok && self.bsp_pack_after_compile {
                    self.add_status("Compile succeeded; launching optional BSP packing step.");
                    self.launch_bsp_pack_for_current_output();
                }
                if !compile_ok {
                    self.last_error_dialog = Some("BSP packing failed or reported missing files. Review the pack panel JSON/log details; VMF export and compile results remain separate.".to_string());
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                self.bsp_pack_receiver = Some(receiver);
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.bsp_pack_status.running = false;
                self.bsp_pack_status.summary =
                    "BSP pack worker disconnected before reporting a result.".to_string();
                self.add_status("BSP pack worker disconnected before reporting a result.");
            }
        }
    }

    fn launch_model_inspect(&mut self) {
        if self.model_inspect_status.running {
            self.add_status("A model inspect run is already in progress.");
            return;
        }
        let Some(mdl_path) = blank_to_none(&self.model_inspect_mdl_path).map(PathBuf::from) else {
            self.add_status("Select an MDL file before model inspection.");
            return;
        };
        let request = DesktopModelInspectRequest {
            cli_path: sourceweaver_cli_executable(),
            mdl_path,
        };
        let command_preview = desktop_model_inspect_command_preview(&request);
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let message = run_desktop_model_inspect_request(request);
            let _ = sender.send(message);
        });
        self.model_inspect_receiver = Some(receiver);
        self.model_inspect_status = DesktopModelInspectStatus {
            running: true,
            summary: "Model inspect running in background.".to_string(),
            command: command_preview.clone(),
            report_json: None,
            stdout_tail: Vec::new(),
            stderr_tail: Vec::new(),
        };
        self.add_status(format!(
            "Started model inspect: {}",
            command_preview.join(" ")
        ));
    }

    fn poll_model_inspect_status(&mut self) {
        let Some(receiver) = self.model_inspect_receiver.take() else {
            return;
        };
        match receiver.try_recv() {
            Ok(message) => {
                self.model_inspect_status.running = false;
                self.model_inspect_status.summary = message.summary.clone();
                self.model_inspect_status.command = message.command;
                self.model_inspect_status.report_json = message.report_json;
                self.model_inspect_status.stdout_tail = message.stdout_tail;
                self.model_inspect_status.stderr_tail = message.stderr_tail;
                self.add_status(message.summary);
                if !message.ok {
                    self.last_error_dialog = Some(
                        "Model inspect failed. Review the model tooling panel JSON/output details."
                            .to_string(),
                    );
                }
            }
            Err(mpsc::TryRecvError::Empty) => self.model_inspect_receiver = Some(receiver),
            Err(mpsc::TryRecvError::Disconnected) => {
                self.model_inspect_status.running = false;
                self.model_inspect_status.summary =
                    "Model inspect worker disconnected before reporting a result.".to_string();
                self.add_status("Model inspect worker disconnected before reporting a result.");
            }
        }
    }

    fn launch_model_compile(&mut self) {
        if self.model_compile_status.running {
            self.add_status("A model compile run is already in progress.");
            return;
        }
        let Some(qc_path) = blank_to_none(&self.model_compile_qc_path).map(PathBuf::from) else {
            self.add_status("Select a QC file before model compile.");
            return;
        };
        let Some(studiomdl_path) =
            blank_to_none(&self.model_compile_studiomdl_path).map(PathBuf::from)
        else {
            self.add_status(
                "Select a user-provided StudioMDL-compatible tool before model compile.",
            );
            return;
        };
        let timeout_seconds = match blank_to_none(&self.model_compile_timeout_seconds) {
            Some(value) => match value.parse::<u64>() {
                Ok(seconds) if seconds > 0 => Some(seconds),
                _ => {
                    self.add_status(
                        "Model compile timeout must be a positive integer number of seconds.",
                    );
                    return;
                }
            },
            None => None,
        };
        let report_path = blank_to_none(&self.model_compile_report_path)
            .map(PathBuf::from)
            .unwrap_or_else(|| default_model_compile_report_path_for_qc(&qc_path));
        let request = DesktopModelCompileRequest {
            cli_path: sourceweaver_cli_executable(),
            qc_path,
            studiomdl_path,
            game_path: blank_to_none(&self.model_compile_game_path).map(PathBuf::from),
            tool_args: split_whitespace_args(&self.model_compile_tool_args),
            log_path: blank_to_none(&self.model_compile_log_path).map(PathBuf::from),
            report_path,
            timeout_seconds,
        };
        let command_preview = desktop_model_compile_command_preview(&request);
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let message = run_desktop_model_compile_request(request);
            let _ = sender.send(message);
        });
        self.model_compile_receiver = Some(receiver);
        self.model_compile_status = DesktopModelCompileStatus {
            running: true,
            summary: "Model compile running in background. StudioMDL-compatible tools and assets are user-provided.".to_string(),
            command: command_preview.clone(),
            report_json: None,
            stdout_tail: Vec::new(),
            stderr_tail: Vec::new(),
        };
        self.add_status(format!(
            "Started model compile: {}",
            command_preview.join(" ")
        ));
    }

    fn poll_model_compile_status(&mut self) {
        let Some(receiver) = self.model_compile_receiver.take() else {
            return;
        };
        match receiver.try_recv() {
            Ok(message) => {
                self.model_compile_status.running = false;
                self.model_compile_status.summary = message.summary.clone();
                self.model_compile_status.command = message.command;
                self.model_compile_status.report_json = message.report_json;
                self.model_compile_status.stdout_tail = message.stdout_tail;
                self.model_compile_status.stderr_tail = message.stderr_tail;
                self.add_status(message.summary);
                if !message.ok {
                    self.last_error_dialog = Some(
                        "Model compile failed. Review the model tooling panel JSON/log details."
                            .to_string(),
                    );
                }
            }
            Err(mpsc::TryRecvError::Empty) => self.model_compile_receiver = Some(receiver),
            Err(mpsc::TryRecvError::Disconnected) => {
                self.model_compile_status.running = false;
                self.model_compile_status.summary =
                    "Model compile worker disconnected before reporting a result.".to_string();
                self.add_status("Model compile worker disconnected before reporting a result.");
            }
        }
    }

    fn poll_compile_status(&mut self) {
        let Some(receiver) = self.compile_receiver.take() else {
            return;
        };
        match receiver.try_recv() {
            Ok(message) => {
                self.compile_status.running = false;
                self.compile_status.summary = message.summary.clone();
                self.compile_status.command = message.command;
                self.compile_status.report_json = message.report_json;
                self.compile_status.stdout_tail = message.stdout_tail;
                self.compile_status.stderr_tail = message.stderr_tail;
                self.add_status(message.summary);
                if !message.ok {
                    self.last_error_dialog = Some("External compile failed. Review the compile panel JSON/log details; VMF export may still have succeeded.".to_string());
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                self.compile_receiver = Some(receiver);
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.compile_status.running = false;
                self.compile_status.summary =
                    "Compile worker disconnected before reporting a result.".to_string();
                self.add_status("Compile worker disconnected before reporting a result.");
            }
        }
    }
}

impl eframe::App for SourceWeaverApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.use_dark_theme {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }
        self.handle_dropped_files(ctx);
        self.poll_profile_wizard_status();
        self.poll_compile_status();
        self.poll_bsp_decompile_status();
        self.poll_bsp_pack_status();
        self.poll_model_inspect_status();
        self.poll_model_compile_status();

        if let Some(error) = self.last_error_dialog.clone() {
            egui::Window::new("Source Weaver needs attention")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                    ui.separator();
                    if ui.button("Dismiss").clicked() {
                        self.last_error_dialog = None;
                    }
                });
        }

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Source Weaver");
                ui.separator();
                ui.label("Drop .vmf, .toml, or .fgd files anywhere in this window.");
                ui.separator();
                if ui.button("Add VMFs...").clicked() {
                    self.add_vmf_files();
                }
                if ui.button("Add BSP-derived VMF...").clicked() {
                    self.add_bsp_derived_vmf_dialog();
                }
                if ui.button("Decompile BSP...").clicked() {
                    self.choose_bsp_decompile_input();
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
                ui.separator();
                if ui
                    .checkbox(&mut self.use_dark_theme, "Dark theme")
                    .on_hover_text("Toggle egui dark/light visuals.")
                    .changed()
                {
                    self.add_status(if self.use_dark_theme {
                        "Switched to dark theme."
                    } else {
                        "Switched to light theme."
                    });
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
                self.draw_scan_progress(ui);
                self.draw_recent_paths(ui);
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
                                    if self.bsp_derived_vmfs.contains(&display_path(&entry.path)) {
                                        ui.colored_label(
                                            egui::Color32::YELLOW,
                                            "BSP-derived VMF: review decompile warnings, broken solids/areaportals/materials/overlays, and missing editor metadata before merge.",
                                        );
                                    }
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
                self.bsp_decompile_panel(ui);
                ui.separator();
                self.compile_panel(ui);
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

        ui.horizontal_wrapped(|ui| {
            ui.label("Changelevel policy:");
            egui::ComboBox::from_id_salt("changelevel_policy_combo")
                .selected_text(self.changelevel_policy.to_string())
                .show_ui(ui, |ui| {
                    for policy in [
                        ChangelevelPolicy::Preserve,
                        ChangelevelPolicy::Disable,
                        ChangelevelPolicy::Delete,
                        ChangelevelPolicy::RewriteInternal,
                    ] {
                        ui.selectable_value(
                            &mut self.changelevel_policy,
                            policy,
                            policy.to_string(),
                        );
                    }
                });
            ui.weak("Portable VMF edit only; no compile or runtime validation is implied.");
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

    fn bsp_decompile_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("BSP decompile import");
        ui.label("Select a BSP and a user-provided BSPSource launcher or jar. Source Weaver does not bundle BSPSource, VMEX, game BSPs, or decompiled content.");
        ui.horizontal(|ui| {
            ui.label("Input BSP:");
            ui.add(
                egui::TextEdit::singleline(&mut self.bsp_decompile_bsp_path)
                    .desired_width(f32::INFINITY),
            );
            if ui.button("Browse...").clicked() {
                self.choose_bsp_decompile_input();
            }
        });
        ui.horizontal(|ui| {
            ui.label("Output VMF:");
            ui.add(
                egui::TextEdit::singleline(&mut self.bsp_decompile_output_vmf)
                    .desired_width(f32::INFINITY),
            );
            if ui.button("Choose output...").clicked() {
                self.choose_bsp_decompile_output();
            }
        });
        ui.collapsing("Decompiler tool", |ui| {
            ui.weak("Select exactly one mode. BSPSource launcher/jar avoids hand-written wrapper scripts; generic wrapper remains an escape hatch.");
            ui.horizontal(|ui| {
                ui.label("BSPSource launcher:");
                ui.add(egui::TextEdit::singleline(&mut self.bsp_decompile_bspsource_path).desired_width(f32::INFINITY));
                if ui.button("Choose launcher...").clicked() {
                    self.choose_bsp_decompile_bspsource();
                }
            });
            ui.horizontal(|ui| {
                ui.label("BSPSource jar:");
                ui.add(egui::TextEdit::singleline(&mut self.bsp_decompile_jar_path).desired_width(f32::INFINITY));
                if ui.button("Choose jar...").clicked() {
                    self.choose_bsp_decompile_jar();
                }
            });
            ui.horizontal(|ui| {
                ui.label("Java executable:");
                ui.add(egui::TextEdit::singleline(&mut self.bsp_decompile_java_path).desired_width(f32::INFINITY));
                if ui.button("Choose java...").clicked() {
                    self.choose_bsp_decompile_java();
                }
            });
            ui.horizontal(|ui| {
                ui.label("Generic wrapper:");
                ui.add(egui::TextEdit::singleline(&mut self.bsp_decompile_wrapper_path).desired_width(f32::INFINITY));
                if ui.button("Choose wrapper...").clicked() {
                    self.choose_bsp_decompile_wrapper();
                }
            });
            ui.horizontal(|ui| {
                ui.label("Tool args:");
                ui.add(egui::TextEdit::singleline(&mut self.bsp_decompile_tool_args).desired_width(f32::INFINITY))
                    .on_hover_text("Whitespace-separated args forwarded before -o for BSPSource. Use the CLI for complex quoting.");
            });
        });
        ui.horizontal(|ui| {
            ui.label("Log:");
            ui.add(
                egui::TextEdit::singleline(&mut self.bsp_decompile_log_path)
                    .desired_width(f32::INFINITY),
            );
            if ui.button("Choose log...").clicked() {
                self.choose_bsp_decompile_log();
            }
        });
        ui.horizontal(|ui| {
            ui.label("Report JSON:");
            ui.add(
                egui::TextEdit::singleline(&mut self.bsp_decompile_report_path)
                    .desired_width(f32::INFINITY),
            );
            if ui.button("Choose report...").clicked() {
                self.choose_bsp_decompile_report();
            }
            ui.label("Timeout:");
            ui.text_edit_singleline(&mut self.bsp_decompile_timeout_seconds);
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!self.bsp_decompile_status.running, egui::Button::new("Decompile and import VMF"))
                .clicked()
            {
                self.launch_bsp_decompile();
            }
            ui.weak("Successful output is parsed, integrity-checked by the CLI, imported, and tagged as BSP-derived.");
        });
        if self.bsp_decompile_status.running {
            ui.add(egui::Spinner::new());
        }
        ui.label(&self.bsp_decompile_status.summary);
        if !self.bsp_decompile_status.command.is_empty() {
            ui.collapsing("BSP decompile command", |ui| {
                ui.monospace(self.bsp_decompile_status.command.join(" "));
            });
        }
        if let Some(report_json) = &self.bsp_decompile_status.report_json {
            ui.collapsing("BSP import report JSON", |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut report_json.clone())
                        .desired_rows(12)
                        .code_editor(),
                );
            });
        }
        if !self.bsp_decompile_status.stdout_tail.is_empty()
            || !self.bsp_decompile_status.stderr_tail.is_empty()
        {
            ui.collapsing("BSP decompile output tail", |ui| {
                for line in &self.bsp_decompile_status.stdout_tail {
                    ui.small(format!("stdout: {line}"));
                }
                for line in &self.bsp_decompile_status.stderr_tail {
                    ui.colored_label(egui::Color32::YELLOW, format!("stderr: {line}"));
                }
            });
        }
        ui.colored_label(
            egui::Color32::YELLOW,
            "BSP-derived VMFs are approximate and review-required before merge.",
        );
    }

    fn compile_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Optional external compile");
        ui.label("This runs user-provided VBSP/VVIS/VRAD tools through a Source Weaver compile profile. Hammer and Valve tools are not bundled or required for normal VMF merge/edit use.");
        self.compile_profile_wizard_panel(ui);
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Compile profile:");
            ui.add(
                egui::TextEdit::singleline(&mut self.compile_profile_path)
                    .desired_width(f32::INFINITY),
            );
            if ui.button("Browse...").clicked() {
                self.choose_compile_profile_path();
            }
        });
        ui.horizontal(|ui| {
            ui.label("Steps:");
            ui.text_edit_singleline(&mut self.compile_steps)
                .on_hover_text("Comma-separated steps passed to sourceweaver compile, for example vbsp,vvis,vrad or vbsp only.");
            ui.label("Timeout seconds:");
            ui.text_edit_singleline(&mut self.compile_timeout_seconds);
        });
        ui.horizontal(|ui| {
            ui.label("Log directory:");
            ui.add(
                egui::TextEdit::singleline(&mut self.compile_log_dir).desired_width(f32::INFINITY),
            );
            if ui.button("Choose logs...").clicked() {
                self.choose_compile_log_dir();
            }
        });
        ui.horizontal(|ui| {
            ui.label("Report JSON:");
            ui.add(
                egui::TextEdit::singleline(&mut self.compile_report_path)
                    .desired_width(f32::INFINITY),
            );
            if ui.button("Choose report...").clicked() {
                self.choose_compile_report_path();
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(
                &mut self.compile_run_after_merge,
                "Run compile after successful Merge selected VMFs",
            );
            if ui
                .add_enabled(
                    !self.compile_status.running,
                    egui::Button::new("Run compile for output VMF"),
                )
                .clicked()
            {
                self.launch_compile_for_current_output();
            }
        });
        ui.weak("Compile runs in a background worker. A compile failure is reported separately from VMF export success.");
        ui.separator();
        if self.compile_status.running {
            ui.add(egui::Spinner::new());
        }
        ui.label(&self.compile_status.summary);
        if !self.compile_status.command.is_empty() {
            ui.collapsing("Compile command", |ui| {
                ui.monospace(self.compile_status.command.join(" "));
            });
        }
        if let Some(report_json) = &self.compile_status.report_json {
            ui.collapsing("Compile report JSON", |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut report_json.clone())
                        .desired_rows(12)
                        .code_editor(),
                );
            });
        }
        if !self.compile_status.stdout_tail.is_empty()
            || !self.compile_status.stderr_tail.is_empty()
        {
            ui.collapsing("Compile output tail", |ui| {
                for line in &self.compile_status.stdout_tail {
                    ui.small(format!("stdout: {line}"));
                }
                for line in &self.compile_status.stderr_tail {
                    ui.colored_label(egui::Color32::YELLOW, format!("stderr: {line}"));
                }
            });
        }
        ui.separator();
        self.bsp_pack_panel(ui);
        ui.separator();
        self.model_tooling_panel(ui);
    }

    fn compile_profile_wizard_panel(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Compile profile wizard", |ui| {
            ui.weak("Creates, discovers, and validates profile TOML for user-provided tools. It checks paths and settings; it does not run VBSP/VVIS/VRAD.");
            ui.horizontal(|ui| {
                ui.label("Profile TOML:");
                ui.add(egui::TextEdit::singleline(&mut self.profile_wizard_output_path).desired_width(f32::INFINITY));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("VBSP:");
                ui.add(egui::TextEdit::singleline(&mut self.profile_wizard_vbsp_path).desired_width(180.0));
                ui.label("VVIS:");
                ui.add(egui::TextEdit::singleline(&mut self.profile_wizard_vvis_path).desired_width(180.0));
                ui.label("VRAD:");
                ui.add(egui::TextEdit::singleline(&mut self.profile_wizard_vrad_path).desired_width(180.0));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Game path:");
                ui.add(egui::TextEdit::singleline(&mut self.profile_wizard_game_path).desired_width(240.0));
                ui.label("Log dir:");
                ui.add(egui::TextEdit::singleline(&mut self.profile_wizard_log_dir).desired_width(180.0));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Steps:");
                ui.add(egui::TextEdit::singleline(&mut self.profile_wizard_steps).desired_width(160.0));
                ui.label("Timeout seconds:");
                ui.add(egui::TextEdit::singleline(&mut self.profile_wizard_timeout_seconds).desired_width(80.0));
                ui.label("Discover dir:");
                ui.add(egui::TextEdit::singleline(&mut self.profile_wizard_discover_dir).desired_width(180.0));
            });
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(!self.profile_wizard_status.running, egui::Button::new("Create + validate profile"))
                    .clicked()
                {
                    self.launch_profile_wizard(DesktopProfileWizardAction::CreateValidate);
                }
                if ui
                    .add_enabled(!self.profile_wizard_status.running, egui::Button::new("Validate profile"))
                    .clicked()
                {
                    self.launch_profile_wizard(DesktopProfileWizardAction::Validate);
                }
                if ui
                    .add_enabled(!self.profile_wizard_status.running, egui::Button::new("Discover from directory"))
                    .clicked()
                {
                    self.launch_profile_wizard(DesktopProfileWizardAction::Discover);
                }
            });
            if self.profile_wizard_status.running {
                ui.add(egui::Spinner::new());
            }
            ui.label(&self.profile_wizard_status.summary);
            if !self.profile_wizard_status.command.is_empty() {
                ui.collapsing("Profile wizard command", |ui| {
                    ui.monospace(self.profile_wizard_status.command.join(" "));
                });
            }
            if let Some(report_json) = &self.profile_wizard_status.report_json {
                ui.collapsing("Profile validation/discovery JSON", |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut report_json.clone())
                            .desired_rows(12)
                            .code_editor(),
                    );
                });
            }
            if !self.profile_wizard_status.stdout_tail.is_empty()
                || !self.profile_wizard_status.stderr_tail.is_empty()
            {
                ui.collapsing("Profile wizard output tail", |ui| {
                    for line in &self.profile_wizard_status.stdout_tail {
                        ui.small(format!("stdout: {line}"));
                    }
                    for line in &self.profile_wizard_status.stderr_tail {
                        ui.colored_label(egui::Color32::YELLOW, format!("stderr: {line}"));
                    }
                });
            }
        });
    }

    fn bsp_pack_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Optional BSP packing");
        ui.label("Runs Source Weaver CLI `pack` with a user-provided BSPZIP-compatible tool. BSPZIP, game content, and SDK tools are not bundled.");
        ui.horizontal(|ui| {
            ui.label("Packer tool:");
            ui.add(
                egui::TextEdit::singleline(&mut self.bsp_pack_tool_path)
                    .desired_width(f32::INFINITY),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Input BSP:");
            ui.add(
                egui::TextEdit::singleline(&mut self.bsp_pack_input_bsp)
                    .desired_width(f32::INFINITY),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Output BSP:");
            ui.add(
                egui::TextEdit::singleline(&mut self.bsp_pack_output_bsp)
                    .desired_width(f32::INFINITY),
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Asset roots:");
            ui.add(
                egui::TextEdit::singleline(&mut self.bsp_pack_asset_roots)
                    .desired_width(260.0)
                    .hint_text("comma-separated folders"),
            );
            ui.label("Includes:");
            ui.add(
                egui::TextEdit::singleline(&mut self.bsp_pack_includes)
                    .desired_width(260.0)
                    .hint_text("materials/x.vmt, models/y.mdl"),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Filelist:");
            ui.add(
                egui::TextEdit::singleline(&mut self.bsp_pack_filelist_path)
                    .desired_width(f32::INFINITY),
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Log:");
            ui.add(egui::TextEdit::singleline(&mut self.bsp_pack_log_path).desired_width(240.0));
            ui.label("Report JSON:");
            ui.add(egui::TextEdit::singleline(&mut self.bsp_pack_report_path).desired_width(240.0));
            ui.label("Timeout seconds:");
            ui.add(
                egui::TextEdit::singleline(&mut self.bsp_pack_timeout_seconds).desired_width(80.0),
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(
                &mut self.bsp_pack_after_compile,
                "Pack after compile succeeds",
            );
            if ui
                .add_enabled(
                    !self.bsp_pack_status.running,
                    egui::Button::new("Run BSP pack now"),
                )
                .clicked()
            {
                self.launch_bsp_pack_for_current_output();
            }
        });
        ui.weak("Packing is optional and reported separately from VMF export and compile success.");
        if self.bsp_pack_status.running {
            ui.add(egui::Spinner::new());
        }
        ui.label(&self.bsp_pack_status.summary);
        ui.label(format!(
            "Missing files: {} | Packed-file count: {}",
            self.bsp_pack_status.missing_files,
            self.bsp_pack_status
                .packed_file_count
                .map(|count| count.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        ));
        if !self.bsp_pack_status.command.is_empty() {
            ui.collapsing("BSP pack command", |ui| {
                ui.monospace(self.bsp_pack_status.command.join(" "));
            });
        }
        if let Some(report_json) = &self.bsp_pack_status.report_json {
            ui.collapsing("BSP pack report JSON", |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut report_json.clone())
                        .desired_rows(12)
                        .code_editor(),
                );
            });
        }
        if !self.bsp_pack_status.stdout_tail.is_empty()
            || !self.bsp_pack_status.stderr_tail.is_empty()
        {
            ui.collapsing("BSP pack output tail", |ui| {
                for line in &self.bsp_pack_status.stdout_tail {
                    ui.small(format!("stdout: {line}"));
                }
                for line in &self.bsp_pack_status.stderr_tail {
                    ui.colored_label(egui::Color32::YELLOW, format!("stderr: {line}"));
                }
            });
        }
    }

    fn model_tooling_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Optional model tooling");
        ui.label("MDL inspection uses Source Weaver metadata parsing. Model compile runs a user-provided StudioMDL-compatible tool. StudioMDL, Crowbar, model assets, game content, and SDKs are not bundled.");
        ui.group(|ui| {
            ui.strong("Model inspect");
            ui.horizontal(|ui| {
                ui.label("MDL file:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.model_inspect_mdl_path)
                        .desired_width(f32::INFINITY),
                );
            });
            if ui
                .add_enabled(
                    !self.model_inspect_status.running,
                    egui::Button::new("Inspect MDL metadata"),
                )
                .clicked()
            {
                self.launch_model_inspect();
            }
            if self.model_inspect_status.running {
                ui.add(egui::Spinner::new());
            }
            ui.label(&self.model_inspect_status.summary);
            if !self.model_inspect_status.command.is_empty() {
                ui.collapsing("Model inspect command", |ui| {
                    ui.monospace(self.model_inspect_status.command.join(" "));
                });
            }
            if let Some(report_json) = &self.model_inspect_status.report_json {
                ui.collapsing("Model inspect report JSON", |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut report_json.clone())
                            .desired_rows(12)
                            .code_editor(),
                    );
                });
            }
            if !self.model_inspect_status.stdout_tail.is_empty()
                || !self.model_inspect_status.stderr_tail.is_empty()
            {
                ui.collapsing("Model inspect output tail", |ui| {
                    for line in &self.model_inspect_status.stdout_tail {
                        ui.small(format!("stdout: {line}"));
                    }
                    for line in &self.model_inspect_status.stderr_tail {
                        ui.colored_label(egui::Color32::YELLOW, format!("stderr: {line}"));
                    }
                });
            }
        });

        ui.group(|ui| {
            ui.strong("Model compile");
            ui.horizontal(|ui| {
                ui.label("QC file:");
                ui.add(egui::TextEdit::singleline(&mut self.model_compile_qc_path).desired_width(f32::INFINITY));
            });
            ui.horizontal(|ui| {
                ui.label("StudioMDL/wrapper:");
                ui.add(egui::TextEdit::singleline(&mut self.model_compile_studiomdl_path).desired_width(f32::INFINITY));
            });
            ui.horizontal(|ui| {
                ui.label("Game path:");
                ui.add(egui::TextEdit::singleline(&mut self.model_compile_game_path).desired_width(f32::INFINITY));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Tool args:");
                ui.add(egui::TextEdit::singleline(&mut self.model_compile_tool_args).desired_width(260.0));
                ui.label("Timeout seconds:");
                ui.add(egui::TextEdit::singleline(&mut self.model_compile_timeout_seconds).desired_width(80.0));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Log:");
                ui.add(egui::TextEdit::singleline(&mut self.model_compile_log_path).desired_width(240.0));
                ui.label("Report JSON:");
                ui.add(egui::TextEdit::singleline(&mut self.model_compile_report_path).desired_width(240.0));
            });
            if ui
                .add_enabled(
                    !self.model_compile_status.running,
                    egui::Button::new("Run model compile"),
                )
                .clicked()
            {
                self.launch_model_compile();
            }
            ui.weak("Model compile runs in a background worker and is reported separately from VMF/BSP workflows.");
            if self.model_compile_status.running {
                ui.add(egui::Spinner::new());
            }
            ui.label(&self.model_compile_status.summary);
            if !self.model_compile_status.command.is_empty() {
                ui.collapsing("Model compile command", |ui| {
                    ui.monospace(self.model_compile_status.command.join(" "));
                });
            }
            if let Some(report_json) = &self.model_compile_status.report_json {
                ui.collapsing("Model compile report JSON", |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut report_json.clone())
                            .desired_rows(12)
                            .code_editor(),
                    );
                });
            }
            if !self.model_compile_status.stdout_tail.is_empty()
                || !self.model_compile_status.stderr_tail.is_empty()
            {
                ui.collapsing("Model compile output tail", |ui| {
                    for line in &self.model_compile_status.stdout_tail {
                        ui.small(format!("stdout: {line}"));
                    }
                    for line in &self.model_compile_status.stderr_tail {
                        ui.colored_label(egui::Color32::YELLOW, format!("stderr: {line}"));
                    }
                });
            }
        });
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

        ui.collapsing("Custom deletion presets", |ui| {
            ui.weak("Custom presets use the same [delete] TOML fields as CLI jobs and desktop projects. Save/export current filters or load/import a preset file, then preview deletion before export.");
            ui.horizontal_wrapped(|ui| {
                ui.label("Name:");
                ui.add(egui::TextEdit::singleline(&mut self.custom_delete_preset_name).desired_width(180.0));
                ui.label("Path:");
                ui.add(egui::TextEdit::singleline(&mut self.custom_delete_preset_path).desired_width(320.0));
            });
            ui.horizontal(|ui| {
                ui.label("Description:");
                ui.add(egui::TextEdit::singleline(&mut self.custom_delete_preset_description).desired_width(f32::INFINITY));
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button("Save/export current preset").clicked() {
                    self.save_custom_deletion_preset();
                }
                if ui.button("Load/import preset").clicked() {
                    self.load_custom_deletion_preset();
                }
            });
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
            ui.radio_value(&mut self.preview_view, PreviewView::ThreeD, "3D iso");
            ui.separator();
            ui.checkbox(&mut self.preview_show_grid, "Grid");
            ui.checkbox(&mut self.preview_show_solids, "Solids");
            ui.checkbox(&mut self.preview_show_entities, "Entities");
            ui.label("Detail:");
            egui::ComboBox::from_id_salt("preview_detail_mode")
                .selected_text(preview_detail_mode_label(self.preview_detail_mode))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.preview_detail_mode,
                        PreviewDetailMode::Fast,
                        preview_detail_mode_label(PreviewDetailMode::Fast),
                    );
                    ui.selectable_value(
                        &mut self.preview_detail_mode,
                        PreviewDetailMode::Auto,
                        preview_detail_mode_label(PreviewDetailMode::Auto),
                    );
                    ui.selectable_value(
                        &mut self.preview_detail_mode,
                        PreviewDetailMode::Full,
                        preview_detail_mode_label(PreviewDetailMode::Full),
                    );
                });
            if ui.button("Reset view").clicked() {
                self.preview_zoom = 1.0;
                self.preview_pan = egui::Vec2::ZERO;
            }
        });

        if self.preview_view == PreviewView::ThreeD {
            ui.horizontal_wrapped(|ui| {
                ui.label("3D camera:");
                ui.add(egui::Slider::new(&mut self.preview_3d_yaw, -180.0..=180.0).text("Yaw"));
                ui.add(egui::Slider::new(&mut self.preview_3d_pitch, -85.0..=85.0).text("Pitch"));
                if ui.button("Reset 3D camera").clicked() {
                    self.preview_3d_yaw = 45.0;
                    self.preview_3d_pitch = 35.264;
                }
                ui.weak("Pan, zoom, and click selection work in 3D too.");
            });
        }

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
            ui.label(format!(
                "detail: {}",
                preview_detail_mode_description(self.preview_detail_mode, preview.solids.len())
            ));
            ui.separator();
            ui.add(egui::Slider::new(&mut self.preview_zoom, 0.1..=12.0).text("Zoom"));
            ui.add(
                egui::Slider::new(&mut self.preview_panel_height, 320.0..=900.0)
                    .text("Preview height"),
            );
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

        let desired_height = self.preview_panel_height.clamp(320.0, 900.0);
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
            self.preview_3d_yaw,
            self.preview_3d_pitch,
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

        let clicked_selection = response
            .clicked()
            .then(|| response.interact_pointer_pos())
            .flatten()
            .and_then(|click_position| {
                selection_context.and_then(|(path, records)| {
                    preview_hit_owner_index(
                        preview,
                        &transform,
                        click_position,
                        &deletion_criteria,
                        deletion_overlay_mode,
                    )
                    .and_then(|owner_index| {
                        entity_selection_key_for_owner(path, records, owner_index)
                            .map(|key| (key, owner_index))
                    })
                })
            });

        if let Some((key, owner_index)) = clicked_selection {
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
                    PreviewSolidDrawOptions {
                        deletion_mode: deletion_overlay_mode,
                        removed,
                        selected,
                        detail_mode: self.preview_detail_mode,
                        solid_count: preview.solids.len(),
                    },
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

fn default_bsp_decompile_output_path(input_bsp: &Path) -> PathBuf {
    let stem = input_bsp
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("decompiled_map");
    input_bsp.with_file_name(format!("{stem}_decompiled.vmf"))
}

fn default_bsp_decompile_report_path(input_bsp: &Path) -> PathBuf {
    let stem = input_bsp
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("bsp-import");
    input_bsp.with_file_name(format!("{stem}-bsp-import-report.json"))
}

fn default_bsp_decompile_log_path(input_bsp: &Path) -> PathBuf {
    let stem = input_bsp
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("bsp-import");
    input_bsp.with_file_name(format!("{stem}-bsp-import.log"))
}

fn desktop_bsp_decompile_command_preview(request: &DesktopBspDecompileRequest) -> Vec<String> {
    let mut parts = vec![
        request.cli_path.display().to_string(),
        "bsp-import".to_string(),
        request.input_bsp.display().to_string(),
    ];
    if let Some(bspsource) = &request.bspsource {
        parts.push("--bspsource".to_string());
        parts.push(bspsource.display().to_string());
    }
    if let Some(jar) = &request.bspsource_jar {
        parts.push("--bspsource-jar".to_string());
        parts.push(jar.display().to_string());
    }
    if let Some(java) = &request.java {
        parts.push("--java".to_string());
        parts.push(java.display().to_string());
    }
    if let Some(wrapper) = &request.wrapper {
        parts.push("--tool".to_string());
        parts.push(wrapper.display().to_string());
    }
    for arg in &request.tool_args {
        parts.push("--tool-arg".to_string());
        parts.push(arg.clone());
    }
    parts.push("--output".to_string());
    parts.push(request.output_vmf.display().to_string());
    if let Some(log_path) = &request.log_path {
        parts.push("--log".to_string());
        parts.push(log_path.display().to_string());
    }
    parts.push("--report".to_string());
    parts.push(request.report_path.display().to_string());
    if let Some(timeout) = request.timeout_seconds {
        parts.push("--timeout-seconds".to_string());
        parts.push(timeout.to_string());
    }
    parts.push("--json".to_string());
    parts
}

fn run_desktop_bsp_decompile_request(
    request: DesktopBspDecompileRequest,
) -> DesktopBspDecompileMessage {
    let command_preview = desktop_bsp_decompile_command_preview(&request);
    let mut command = Command::new(&request.cli_path);
    command.arg("bsp-import").arg(&request.input_bsp);
    if let Some(bspsource) = &request.bspsource {
        command.arg("--bspsource").arg(bspsource);
    }
    if let Some(jar) = &request.bspsource_jar {
        command.arg("--bspsource-jar").arg(jar);
    }
    if let Some(java) = &request.java {
        command.arg("--java").arg(java);
    }
    if let Some(wrapper) = &request.wrapper {
        command.arg("--tool").arg(wrapper);
    }
    for arg in &request.tool_args {
        command.arg("--tool-arg").arg(arg);
    }
    command.arg("--output").arg(&request.output_vmf);
    if let Some(log_path) = &request.log_path {
        command.arg("--log").arg(log_path);
    }
    command
        .arg("--report")
        .arg(&request.report_path)
        .arg("--json");
    if let Some(timeout) = request.timeout_seconds {
        command.arg("--timeout-seconds").arg(timeout.to_string());
    }

    match command.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let report_json = if stdout.trim_start().starts_with('{') {
                Some(stdout.clone())
            } else {
                fs::read_to_string(&request.report_path).ok()
            };
            let parsed_ok = report_json
                .as_ref()
                .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
                .and_then(|value| value.get("ok").and_then(serde_json::Value::as_bool))
                .unwrap_or(false);
            let ok = output.status.success() && parsed_ok && request.output_vmf.exists();
            let summary = if ok {
                format!(
                    "BSP decompile completed and VMF passed Source Weaver import validation: {}",
                    display_path(&request.output_vmf)
                )
            } else {
                format!(
                    "BSP decompile failed or import validation failed. Exit code: {:?}. Report: {}",
                    output.status.code(),
                    display_path(&request.report_path)
                )
            };
            DesktopBspDecompileMessage {
                ok,
                summary,
                command: command_preview,
                output_vmf: ok.then_some(request.output_vmf),
                report_json,
                stdout_tail: tail_lines(&stdout, 40),
                stderr_tail: tail_lines(&stderr, 40),
            }
        }
        Err(error) => DesktopBspDecompileMessage {
            ok: false,
            summary: format!(
                "Failed to start Source Weaver CLI BSP import command `{}`: {error}. Set SOURCEWEAVER_CLI to the CLI executable if needed.",
                request.cli_path.display()
            ),
            command: command_preview,
            output_vmf: None,
            report_json: None,
            stdout_tail: Vec::new(),
            stderr_tail: Vec::new(),
        },
    }
}

fn default_compile_report_path(output_path: &str) -> PathBuf {
    if output_path.trim().is_empty() {
        PathBuf::from("sourceweaver-compile-report.json")
    } else {
        default_compile_report_path_for_map(&PathBuf::from(output_path.trim()))
    }
}

fn desktop_profile_wizard_command_preview(request: &DesktopProfileWizardRequest) -> Vec<String> {
    let mut parts = vec![
        request.cli_path.display().to_string(),
        "compile-profile".to_string(),
    ];
    match request.action {
        DesktopProfileWizardAction::CreateValidate => {
            parts.push("create".to_string());
            parts.push("--output".to_string());
            parts.push(request.profile_path.display().to_string());
            push_optional_path_arg(&mut parts, "--vbsp", &request.vbsp);
            push_optional_path_arg(&mut parts, "--vvis", &request.vvis);
            push_optional_path_arg(&mut parts, "--vrad", &request.vrad);
            push_optional_path_arg(&mut parts, "--game", &request.game);
            push_optional_path_arg(&mut parts, "--log-dir", &request.log_dir);
            if let Some(steps) = &request.steps {
                parts.push("--steps".to_string());
                parts.push(steps.clone());
            }
            if let Some(timeout) = request.timeout_seconds {
                parts.push("--timeout-seconds".to_string());
                parts.push(timeout.to_string());
            }
            parts.push("--validate".to_string());
            parts.push("--json".to_string());
        }
        DesktopProfileWizardAction::Validate => {
            parts.push("validate".to_string());
            parts.push("--profile".to_string());
            parts.push(request.profile_path.display().to_string());
            parts.push("--json".to_string());
        }
        DesktopProfileWizardAction::Discover => {
            parts.push("discover".to_string());
            push_optional_path_arg(&mut parts, "--search-dir", &request.search_dir);
            parts.push("--output".to_string());
            parts.push(request.profile_path.display().to_string());
            push_optional_path_arg(&mut parts, "--game", &request.game);
            push_optional_path_arg(&mut parts, "--log-dir", &request.log_dir);
            if let Some(steps) = &request.steps {
                parts.push("--steps".to_string());
                parts.push(steps.clone());
            }
            if let Some(timeout) = request.timeout_seconds {
                parts.push("--timeout-seconds".to_string());
                parts.push(timeout.to_string());
            }
            parts.push("--json".to_string());
        }
    }
    parts
}

fn push_optional_path_arg(parts: &mut Vec<String>, flag: &str, value: &Option<PathBuf>) {
    if let Some(path) = value {
        parts.push(flag.to_string());
        parts.push(path.display().to_string());
    }
}

fn run_desktop_profile_wizard_request(
    request: DesktopProfileWizardRequest,
) -> DesktopProfileWizardMessage {
    let command_preview = desktop_profile_wizard_command_preview(&request);
    let mut command = Command::new(&request.cli_path);
    for arg in command_preview.iter().skip(1) {
        command.arg(arg);
    }
    match command.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let report_json = stdout
                .trim_start()
                .starts_with('{')
                .then_some(stdout.clone());
            let parsed_ok = report_json
                .as_ref()
                .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
                .and_then(|value| value.get("ok").and_then(serde_json::Value::as_bool))
                .unwrap_or(false);
            let ok = output.status.success() && parsed_ok;
            let summary = if ok {
                format!(
                    "Compile profile wizard completed successfully. Profile: {}",
                    display_path(&request.profile_path)
                )
            } else {
                format!(
                    "Compile profile wizard reported missing tools/game paths or invalid settings. Exit code: {:?}. Profile: {}",
                    output.status.code(),
                    display_path(&request.profile_path)
                )
            };
            DesktopProfileWizardMessage {
                ok,
                summary,
                command: command_preview,
                report_json,
                stdout_tail: tail_lines(&stdout, 40),
                stderr_tail: tail_lines(&stderr, 40),
            }
        }
        Err(error) => DesktopProfileWizardMessage {
            ok: false,
            summary: format!(
                "Failed to start Source Weaver CLI compile-profile command `{}`: {error}.",
                request.cli_path.display()
            ),
            command: command_preview,
            report_json: None,
            stdout_tail: Vec::new(),
            stderr_tail: Vec::new(),
        },
    }
}

fn split_whitespace_args(value: &str) -> Vec<String> {
    value.split_whitespace().map(ToOwned::to_owned).collect()
}

fn default_model_compile_report_path_for_qc(qc_path: &Path) -> PathBuf {
    let stem = qc_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("model");
    qc_path.with_file_name(format!("{stem}-model-compile-report.json"))
}

fn desktop_model_inspect_command_preview(request: &DesktopModelInspectRequest) -> Vec<String> {
    vec![
        request.cli_path.display().to_string(),
        "model-inspect".to_string(),
        request.mdl_path.display().to_string(),
        "--json".to_string(),
    ]
}

fn run_desktop_model_inspect_request(
    request: DesktopModelInspectRequest,
) -> DesktopModelInspectMessage {
    let command_preview = desktop_model_inspect_command_preview(&request);
    let output = Command::new(&request.cli_path)
        .arg("model-inspect")
        .arg(&request.mdl_path)
        .arg("--json")
        .output();
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let report_json = stdout
                .trim_start()
                .starts_with('{')
                .then_some(stdout.clone());
            let parsed_ok = report_json
                .as_ref()
                .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
                .and_then(|value| value.get("ok").and_then(serde_json::Value::as_bool))
                .unwrap_or(false);
            let ok = output.status.success() && parsed_ok;
            let summary = if ok {
                format!(
                    "Model inspect completed for {}.",
                    display_path(&request.mdl_path)
                )
            } else {
                format!(
                    "Model inspect failed. Exit code: {:?}. Input: {}",
                    output.status.code(),
                    display_path(&request.mdl_path)
                )
            };
            DesktopModelInspectMessage {
                ok,
                summary,
                command: command_preview,
                report_json,
                stdout_tail: tail_lines(&stdout, 40),
                stderr_tail: tail_lines(&stderr, 40),
            }
        }
        Err(error) => DesktopModelInspectMessage {
            ok: false,
            summary: format!(
                "Failed to start Source Weaver CLI model-inspect command `{}`: {error}.",
                request.cli_path.display()
            ),
            command: command_preview,
            report_json: None,
            stdout_tail: Vec::new(),
            stderr_tail: Vec::new(),
        },
    }
}

fn desktop_model_compile_command_preview(request: &DesktopModelCompileRequest) -> Vec<String> {
    let mut parts = vec![
        request.cli_path.display().to_string(),
        "model-compile".to_string(),
        request.qc_path.display().to_string(),
        "--studiomdl".to_string(),
        request.studiomdl_path.display().to_string(),
        "--report".to_string(),
        request.report_path.display().to_string(),
        "--json".to_string(),
    ];
    if let Some(game_path) = &request.game_path {
        parts.push("--game".to_string());
        parts.push(game_path.display().to_string());
    }
    for arg in &request.tool_args {
        parts.push("--tool-arg".to_string());
        parts.push(arg.clone());
    }
    if let Some(log_path) = &request.log_path {
        parts.push("--log".to_string());
        parts.push(log_path.display().to_string());
    }
    if let Some(timeout) = request.timeout_seconds {
        parts.push("--timeout-seconds".to_string());
        parts.push(timeout.to_string());
    }
    parts
}

fn run_desktop_model_compile_request(
    request: DesktopModelCompileRequest,
) -> DesktopModelCompileMessage {
    let command_preview = desktop_model_compile_command_preview(&request);
    let mut command = Command::new(&request.cli_path);
    command
        .arg("model-compile")
        .arg(&request.qc_path)
        .arg("--studiomdl")
        .arg(&request.studiomdl_path)
        .arg("--report")
        .arg(&request.report_path)
        .arg("--json");
    if let Some(game_path) = &request.game_path {
        command.arg("--game").arg(game_path);
    }
    for arg in &request.tool_args {
        command.arg("--tool-arg").arg(arg);
    }
    if let Some(log_path) = &request.log_path {
        command.arg("--log").arg(log_path);
    }
    if let Some(timeout) = request.timeout_seconds {
        command.arg("--timeout-seconds").arg(timeout.to_string());
    }

    match command.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let report_json = if stdout.trim_start().starts_with('{') {
                Some(stdout.clone())
            } else {
                fs::read_to_string(&request.report_path).ok()
            };
            let parsed_ok = report_json
                .as_ref()
                .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
                .and_then(|value| value.get("ok").and_then(serde_json::Value::as_bool))
                .unwrap_or(false);
            let ok = output.status.success() && parsed_ok;
            let summary = if ok {
                format!(
                    "Model compile completed successfully. Report: {}",
                    display_path(&request.report_path)
                )
            } else {
                format!(
                    "Model compile failed or reported errors. Exit code: {:?}. Report: {}",
                    output.status.code(),
                    display_path(&request.report_path)
                )
            };
            DesktopModelCompileMessage {
                ok,
                summary,
                command: command_preview,
                report_json,
                stdout_tail: tail_lines(&stdout, 40),
                stderr_tail: tail_lines(&stderr, 40),
            }
        }
        Err(error) => DesktopModelCompileMessage {
            ok: false,
            summary: format!(
                "Failed to start Source Weaver CLI model-compile command `{}`: {error}.",
                request.cli_path.display()
            ),
            command: command_preview,
            report_json: None,
            stdout_tail: Vec::new(),
            stderr_tail: Vec::new(),
        },
    }
}

fn default_packed_bsp_path(input_bsp: &Path) -> PathBuf {
    let stem = input_bsp
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("packed");
    input_bsp.with_file_name(format!("{stem}-packed.bsp"))
}

fn default_pack_report_path_for_bsp(output_bsp: &Path) -> PathBuf {
    let stem = output_bsp
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("sourceweaver-pack");
    output_bsp.with_file_name(format!("{stem}-pack-report.json"))
}

fn desktop_bsp_pack_command_preview(request: &DesktopBspPackRequest) -> Vec<String> {
    let mut parts = vec![
        request.cli_path.display().to_string(),
        "pack".to_string(),
        request.input_bsp.display().to_string(),
        "--tool".to_string(),
        request.tool_path.display().to_string(),
        "--output".to_string(),
        request.output_bsp.display().to_string(),
        "--report".to_string(),
        request.report_path.display().to_string(),
        "--json".to_string(),
    ];
    if let Some(filelist) = &request.filelist_path {
        parts.push("--filelist".to_string());
        parts.push(filelist.display().to_string());
    } else {
        for root in &request.asset_roots {
            parts.push("--asset-root".to_string());
            parts.push(root.display().to_string());
        }
        for include in &request.includes {
            parts.push("--include".to_string());
            parts.push(include.clone());
        }
    }
    if let Some(log_path) = &request.log_path {
        parts.push("--log".to_string());
        parts.push(log_path.display().to_string());
    }
    if let Some(timeout) = request.timeout_seconds {
        parts.push("--timeout-seconds".to_string());
        parts.push(timeout.to_string());
    }
    parts
}

fn run_desktop_bsp_pack_request(request: DesktopBspPackRequest) -> DesktopBspPackMessage {
    let command_preview = desktop_bsp_pack_command_preview(&request);
    let mut command = Command::new(&request.cli_path);
    command
        .arg("pack")
        .arg(&request.input_bsp)
        .arg("--tool")
        .arg(&request.tool_path)
        .arg("--output")
        .arg(&request.output_bsp)
        .arg("--report")
        .arg(&request.report_path)
        .arg("--json");
    if let Some(filelist) = &request.filelist_path {
        command.arg("--filelist").arg(filelist);
    } else {
        for root in &request.asset_roots {
            command.arg("--asset-root").arg(root);
        }
        for include in &request.includes {
            command.arg("--include").arg(include);
        }
    }
    if let Some(log_path) = &request.log_path {
        command.arg("--log").arg(log_path);
    }
    if let Some(timeout) = request.timeout_seconds {
        command.arg("--timeout-seconds").arg(timeout.to_string());
    }

    match command.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let report_json = if stdout.trim_start().starts_with('{') {
                Some(stdout.clone())
            } else {
                fs::read_to_string(&request.report_path).ok()
            };
            let parsed = report_json
                .as_ref()
                .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok());
            let parsed_ok = parsed
                .as_ref()
                .and_then(|value| value.get("ok").and_then(serde_json::Value::as_bool))
                .unwrap_or(false);
            let missing_files = parsed
                .as_ref()
                .and_then(|value| value.get("missing_files"))
                .and_then(serde_json::Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            let packed_file_count = parsed
                .as_ref()
                .and_then(|value| value.get("packed_file_count"))
                .and_then(serde_json::Value::as_u64);
            let ok = output.status.success() && parsed_ok;
            let summary = if ok {
                format!(
                    "BSP packing completed successfully. Packed files: {}. Report: {}",
                    packed_file_count
                        .map(|count| count.to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    display_path(&request.report_path)
                )
            } else {
                format!(
                    "BSP packing failed or reported missing files. Exit code: {:?}. Missing files: {missing_files}. Report: {}",
                    output.status.code(),
                    display_path(&request.report_path)
                )
            };
            DesktopBspPackMessage {
                ok,
                summary,
                command: command_preview,
                report_json,
                stdout_tail: tail_lines(&stdout, 40),
                stderr_tail: tail_lines(&stderr, 40),
                missing_files,
                packed_file_count,
            }
        }
        Err(error) => DesktopBspPackMessage {
            ok: false,
            summary: format!(
                "Failed to start Source Weaver CLI pack command `{}`: {error}. Set SOURCEWEAVER_CLI to the CLI executable if needed.",
                request.cli_path.display()
            ),
            command: command_preview,
            report_json: None,
            stdout_tail: Vec::new(),
            stderr_tail: Vec::new(),
            missing_files: 0,
            packed_file_count: None,
        },
    }
}

fn default_compile_report_path_for_map(map_path: &Path) -> PathBuf {
    let stem = map_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("sourceweaver");
    map_path.with_file_name(format!("{stem}-compile-report.json"))
}

fn sourceweaver_cli_executable() -> PathBuf {
    if let Ok(path) = std::env::var("SOURCEWEAVER_CLI") {
        return PathBuf::from(path);
    }
    let Ok(current) = std::env::current_exe() else {
        return PathBuf::from("sourceweaver");
    };
    let Some(dir) = current.parent() else {
        return PathBuf::from("sourceweaver");
    };
    for name in sourceweaver_cli_candidate_names() {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from("sourceweaver")
}

fn sourceweaver_cli_candidate_names() -> Vec<&'static str> {
    if cfg!(windows) {
        vec!["sourceweaver.exe", "sourceweaver-cli.exe"]
    } else {
        vec!["sourceweaver", "sourceweaver-cli"]
    }
}

fn desktop_compile_command_preview(request: &DesktopCompileRequest) -> Vec<String> {
    let mut parts = vec![
        request.cli_path.display().to_string(),
        "compile".to_string(),
        request.map_path.display().to_string(),
        "--profile".to_string(),
        request.profile_path.display().to_string(),
        "--report".to_string(),
        request.report_path.display().to_string(),
        "--json".to_string(),
    ];
    if let Some(steps) = &request.steps {
        parts.push("--steps".to_string());
        parts.push(steps.clone());
    }
    if let Some(log_dir) = &request.log_dir {
        parts.push("--log-dir".to_string());
        parts.push(log_dir.display().to_string());
    }
    if let Some(timeout) = request.timeout_seconds {
        parts.push("--timeout-seconds".to_string());
        parts.push(timeout.to_string());
    }
    parts
}

fn run_desktop_compile_request(request: DesktopCompileRequest) -> DesktopCompileMessage {
    let command_preview = desktop_compile_command_preview(&request);
    let mut command = Command::new(&request.cli_path);
    command
        .arg("compile")
        .arg(&request.map_path)
        .arg("--profile")
        .arg(&request.profile_path)
        .arg("--report")
        .arg(&request.report_path)
        .arg("--json");
    if let Some(steps) = &request.steps {
        command.arg("--steps").arg(steps);
    }
    if let Some(log_dir) = &request.log_dir {
        command.arg("--log-dir").arg(log_dir);
    }
    if let Some(timeout) = request.timeout_seconds {
        command.arg("--timeout-seconds").arg(timeout.to_string());
    }

    match command.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let report_json = if stdout.trim_start().starts_with('{') {
                Some(stdout.clone())
            } else {
                fs::read_to_string(&request.report_path).ok()
            };
            let parsed_ok = report_json
                .as_ref()
                .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
                .and_then(|value| value.get("ok").and_then(serde_json::Value::as_bool))
                .unwrap_or(false);
            let ok = output.status.success() && parsed_ok;
            let summary = if ok {
                format!(
                    "External compile completed successfully. Report: {}",
                    display_path(&request.report_path)
                )
            } else {
                format!(
                    "External compile failed or reported errors. Exit code: {:?}. Report: {}",
                    output.status.code(),
                    display_path(&request.report_path)
                )
            };
            DesktopCompileMessage {
                ok,
                summary,
                command: command_preview,
                report_json,
                stdout_tail: tail_lines(&stdout, 40),
                stderr_tail: tail_lines(&stderr, 40),
            }
        }
        Err(error) => DesktopCompileMessage {
            ok: false,
            summary: format!(
                "Failed to start Source Weaver CLI compile command `{}`: {error}. Set SOURCEWEAVER_CLI to the CLI executable if needed.",
                request.cli_path.display()
            ),
            command: command_preview,
            report_json: None,
            stdout_tail: Vec::new(),
            stderr_tail: Vec::new(),
        },
    }
}

fn tail_lines(text: &str, limit: usize) -> Vec<String> {
    let mut lines = text
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if lines.len() > limit {
        lines.drain(0..lines.len() - limit);
    }
    lines
}

impl MapEntry {
    fn load(path: PathBuf, rule_set_id: Option<&str>) -> Self {
        let analysis = match load_document(&path) {
            Ok(document) => {
                let label = display_path(&path);
                let rule_set = rule_set_id
                    .and_then(validation_rule_set_by_id)
                    .map(|rule_set| validate_document_with_rule_set(&document, &label, rule_set));
                Ok(MapAnalysis {
                    entity_records: inspect_entities(&document),
                    type_counts: summarize_entity_types(&document),
                    preview: preview_document(&document),
                    landmarks: discover_landmarks(&document),
                    transitions: discover_transitions(&document),
                    integrity: validate_document_integrity(&document, &label),
                    entity_semantics: sourceweaver_core::validate_entity_semantics(
                        &document, &label,
                    ),
                    complexity: sourceweaver_core::analyze_map_complexity(&document),
                    rule_set,
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
    yaw: f32,
    pitch: f32,
}

impl PreviewTransform {
    fn new(
        rect: egui::Rect,
        bounds: PreviewBounds,
        view: PreviewView,
        zoom: f32,
        pan: egui::Vec2,
        yaw: f32,
        pitch: f32,
    ) -> Self {
        let (min_u, min_v, max_u, max_v) = projected_bounds(bounds, view, yaw, pitch);
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
            yaw,
            pitch,
        }
    }

    fn world_to_screen(&self, point: sourceweaver_core::Vec3) -> egui::Pos2 {
        let (u, v) = self.project_vec(point);
        self.uv_to_screen(u, v)
    }

    fn project_vec(&self, point: sourceweaver_core::Vec3) -> (f64, f64) {
        project_vec(point, self.view, self.yaw, self.pitch)
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

#[allow(clippy::too_many_arguments)]
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
    if role_filter.is_some_and(|role| !record.roles.iter().any(|record_role| record_role == role)) {
        return false;
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
    let (min_u, min_v, max_u, max_v) = projected_bounds(
        transform.bounds,
        transform.view,
        transform.yaw,
        transform.pitch,
    );
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
        let direction = offset_vector_to_screen_delta(*offset, transform);
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

fn offset_vector_to_screen_delta(
    offset: sourceweaver_core::Vec3,
    transform: &PreviewTransform,
) -> egui::Vec2 {
    let (u, v) = transform.project_vec(offset);
    egui::vec2(u as f32, -(v as f32))
}

fn draw_preview_solid(
    painter: &egui::Painter,
    transform: &PreviewTransform,
    solid: &PreviewSolid,
    options: PreviewSolidDrawOptions,
) {
    let rect = preview_solid_screen_rect(solid, transform);
    if !rect.intersects(transform.rect) {
        return;
    }

    let mut role_color = solid_color(solid);
    let mut fill_color = solid.source_index.map(source_color).unwrap_or(role_color);
    if options.removed {
        match options.deletion_mode {
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

    let stroke = egui::Stroke::new(
        if options.selected {
            2.5_f32
        } else if options.removed {
            2.0_f32
        } else {
            1.25_f32
        },
        if options.selected {
            egui::Color32::YELLOW
        } else {
            role_color
        },
    );
    let fill = fill_color.gamma_multiply(if options.removed { 0.32 } else { 0.22 });
    let draw_faces =
        should_draw_reconstructed_faces(options.detail_mode, options.solid_count, options.selected);
    let draw_plane_edges =
        should_draw_plane_edges(options.detail_mode, options.solid_count, options.selected);

    let drew_reconstructed_faces = if draw_faces {
        draw_reconstructed_faces(painter, transform, solid, fill, stroke)
    } else {
        false
    };

    if !drew_reconstructed_faces {
        painter.rect_filled(rect, 0.0, fill);
        draw_rect_outline(painter, rect, stroke);
    }

    if !draw_plane_edges {
        return;
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

#[derive(Debug, Clone, Copy)]
struct PreviewSolidDrawOptions {
    deletion_mode: DeletionPreviewMode,
    removed: bool,
    selected: bool,
    detail_mode: PreviewDetailMode,
    solid_count: usize,
}

fn should_draw_reconstructed_faces(
    detail_mode: PreviewDetailMode,
    solid_count: usize,
    selected: bool,
) -> bool {
    selected
        || match detail_mode {
            PreviewDetailMode::Fast => false,
            PreviewDetailMode::Auto => solid_count <= 1_200,
            PreviewDetailMode::Full => true,
        }
}

fn should_draw_plane_edges(
    detail_mode: PreviewDetailMode,
    solid_count: usize,
    selected: bool,
) -> bool {
    selected
        || match detail_mode {
            PreviewDetailMode::Fast => false,
            PreviewDetailMode::Auto => solid_count <= 350,
            PreviewDetailMode::Full => true,
        }
}

fn draw_reconstructed_faces(
    painter: &egui::Painter,
    transform: &PreviewTransform,
    solid: &PreviewSolid,
    fill: egui::Color32,
    stroke: egui::Stroke,
) -> bool {
    let mut drew_any = false;
    for polygon in &solid.face_polygons {
        if polygon.len() < 3 {
            continue;
        }
        let screen_points = polygon
            .iter()
            .map(|point| transform.world_to_screen(*point))
            .collect::<Vec<_>>();
        if projected_polygon_area(&screen_points) < 0.75 {
            continue;
        }
        let rect = screen_points.iter().fold(
            egui::Rect::from_min_size(screen_points[0], egui::Vec2::ZERO),
            |rect, point| rect.union(egui::Rect::from_min_size(*point, egui::Vec2::ZERO)),
        );
        if !rect.intersects(transform.rect) {
            continue;
        }
        painter.add(egui::Shape::convex_polygon(screen_points, fill, stroke));
        drew_any = true;
    }
    drew_any
}

fn projected_polygon_area(points: &[egui::Pos2]) -> f32 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0_f32;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        area += points[index].x * points[next].y - points[next].x * points[index].y;
    }
    area.abs() * 0.5
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

    let Some(summary) = merged_summary else {
        return;
    };
    if summary.source_labels.is_empty() {
        return;
    }
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
        PreviewView::ThreeD => "3D isometric preview",
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

fn project_vec(
    point: sourceweaver_core::Vec3,
    view: PreviewView,
    yaw: f32,
    pitch: f32,
) -> (f64, f64) {
    match view {
        PreviewView::Top => (point.x, point.y),
        PreviewView::Front => (point.x, point.z),
        PreviewView::Side => (point.y, point.z),
        PreviewView::ThreeD => project_vec_isometric(point, yaw, pitch),
    }
}

fn projected_bounds(
    bounds: PreviewBounds,
    view: PreviewView,
    yaw: f32,
    pitch: f32,
) -> (f64, f64, f64, f64) {
    let corners = [
        sourceweaver_core::Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
        sourceweaver_core::Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
        sourceweaver_core::Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
        sourceweaver_core::Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
        sourceweaver_core::Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
        sourceweaver_core::Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        sourceweaver_core::Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
        sourceweaver_core::Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
    ];
    let mut min_u = f64::INFINITY;
    let mut min_v = f64::INFINITY;
    let mut max_u = f64::NEG_INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for corner in corners {
        let (u, v) = project_vec(corner, view, yaw, pitch);
        min_u = min_u.min(u);
        min_v = min_v.min(v);
        max_u = max_u.max(u);
        max_v = max_v.max(v);
    }
    (min_u, min_v, max_u, max_v)
}

fn project_vec_isometric(point: sourceweaver_core::Vec3, yaw: f32, pitch: f32) -> (f64, f64) {
    let yaw = (yaw as f64).to_radians();
    let pitch = (pitch as f64).to_radians();
    let cos_yaw = yaw.cos();
    let sin_yaw = yaw.sin();
    let cos_pitch = pitch.cos();
    let sin_pitch = pitch.sin();
    let x = point.x * cos_yaw - point.y * sin_yaw;
    let y = point.x * sin_yaw + point.y * cos_yaw;
    let z = point.z;
    (x, y * cos_pitch - z * sin_pitch)
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
    solid_bounds_corners(solid)
        .iter()
        .map(|corner| transform.world_to_screen(*corner))
        .fold(egui::Rect::NOTHING, |rect, point| {
            rect.union(egui::Rect::from_min_size(point, egui::Vec2::ZERO))
        })
}

fn solid_bounds_corners(solid: &PreviewSolid) -> [sourceweaver_core::Vec3; 8] {
    let min = solid.bounds.min;
    let max = solid.bounds.max;
    [
        sourceweaver_core::Vec3::new(min.x, min.y, min.z),
        sourceweaver_core::Vec3::new(min.x, min.y, max.z),
        sourceweaver_core::Vec3::new(min.x, max.y, min.z),
        sourceweaver_core::Vec3::new(min.x, max.y, max.z),
        sourceweaver_core::Vec3::new(max.x, min.y, min.z),
        sourceweaver_core::Vec3::new(max.x, min.y, max.z),
        sourceweaver_core::Vec3::new(max.x, max.y, min.z),
        sourceweaver_core::Vec3::new(max.x, max.y, max.z),
    ]
}

fn deletion_preview_mode_label(mode: DeletionPreviewMode) -> &'static str {
    match mode {
        DeletionPreviewMode::Off => "Off",
        DeletionPreviewMode::HighlightRemoved => "Highlight removed red",
        DeletionPreviewMode::DimRemoved => "Dim removed",
        DeletionPreviewMode::HideRemoved => "Hide removed",
    }
}

fn preview_detail_mode_label(mode: PreviewDetailMode) -> &'static str {
    match mode {
        PreviewDetailMode::Fast => "Fast boxes",
        PreviewDetailMode::Auto => "Auto",
        PreviewDetailMode::Full => "Full faces",
    }
}

fn preview_detail_mode_description(mode: PreviewDetailMode, solid_count: usize) -> &'static str {
    match mode {
        PreviewDetailMode::Fast => "bounding boxes only",
        PreviewDetailMode::Auto if solid_count > 1_200 => "auto fast boxes for large VMF",
        PreviewDetailMode::Auto if solid_count > 350 => "auto faces, side-edge overlay skipped",
        PreviewDetailMode::Auto => "auto full detail",
        PreviewDetailMode::Full => "all faces and side edges",
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

fn validation_rule_set_combo_label(value: &str) -> String {
    if value.trim().eq_ignore_ascii_case(NO_VALIDATION_RULE_SET_ID) {
        return "none: generic VMF integrity only".to_string();
    }
    validation_rule_set_by_id(value)
        .map(|rule_set| format!("{}: {}", rule_set.id, rule_set.name))
        .unwrap_or_else(|| format!("{}: unknown rule set", value.trim()))
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
    let mut criteria = DeletionCriteria {
        protect_critical_entities: true,
        ..DeletionCriteria::default()
    };

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

fn remember_recent_path(recent: &mut Vec<PathBuf>, path: PathBuf) {
    recent.retain(|existing| existing != &path);
    recent.insert(0, path);
    recent.truncate(8);
}

fn project_relative_path(path: &Path, base_dir: &Path) -> String {
    if path.is_absolute() {
        match path.strip_prefix(base_dir) {
            Ok(relative) if !relative.as_os_str().is_empty() => return display_path(relative),
            _ => {}
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
