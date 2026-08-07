//! Core VMF parsing, inspection, pruning, and merge operations for Source Weaver.
//!
//! This crate intentionally has no UI dependencies. The desktop app, CLI, and future
//! automation layers should all call into this library so Linux and Windows builds
//! share the same VMF behavior.

pub mod campaign;
pub mod changelevel;
pub mod classify;
pub mod compiler;
pub mod complexity;
pub mod entity_metadata;
pub mod entity_semantics;
pub mod id_references;
pub mod integrity;
pub mod landmark;
pub mod merge;
pub mod preview;
pub mod prune;
pub mod transform;
pub mod transition;
pub mod validation_rules;
pub mod vmf;

pub use campaign::{
    CampaignAdjacencyEdge, CampaignAdjacencyGraph, CampaignLandmarkPairSuggestion,
    CampaignMapInput, CampaignOrderSuggestion, build_campaign_adjacency_graph,
    suggest_campaign_order,
};
pub use changelevel::{
    ChangelevelChange, ChangelevelPolicy, ChangelevelPolicyOptions, ChangelevelPolicyReport,
    ChangelevelPreserveRule, ChangelevelPreservedTransition, ChangelevelScope,
    apply_changelevel_policy, normalize_map_name,
};
pub use classify::{BrushRole, EntityRecord, inspect_entities, summarize_entity_types};
pub use compiler::{
    CompileLogSummary, VmfToolValidationReport, parse_compile_log, validate_for_source_tools,
    validate_for_source_tools_with_rule_set,
};
pub use complexity::{
    MapComplexityReport, MapComplexityRisk, SOURCE_COMPLEXITY_WARN_RATIO, SOURCE_MAX_MAP_BRUSHES,
    SOURCE_MAX_MAP_BRUSHSIDES, SOURCE_MAX_MAP_DISPINFO, SOURCE_MAX_MAP_ENTITIES,
    SOURCE_MAX_MAP_FACES, SOURCE_MAX_MAP_OVERLAYS, analyze_map_complexity,
};
pub use entity_metadata::{
    EntityCategory, EntityMetadata, EntityMetadataSource, EntityPropertyChoice,
    EntityPropertyMetadata, metadata_for_classname, metadata_for_classname_with_overrides,
    parse_fgd_metadata,
};
pub use entity_semantics::{
    EntitySemanticsIssue, EntitySemanticsReport, format_entity_semantics_issue,
    validate_entity_semantics,
};
pub use id_references::{
    KNOWN_ID_LIKE_NON_REFERENCE_KEYS, SUPPORTED_LIST_ID_REFERENCE_KEYS,
    SUPPORTED_SINGLE_ID_REFERENCE_KEYS, is_list_id_reference_key, is_numeric_id_or_id_list,
    is_single_id_reference_key, is_supported_id_reference_key, is_suspected_id_reference_key,
    supported_id_reference_summary,
};
pub use integrity::{
    IntegrityIssue, IntegrityReport, IntegritySeverity, format_integrity_issue,
    validate_document_integrity, validate_merge_inputs,
};
pub use landmark::{
    DiscoveredLandmark, LandmarkDiscovery, LandmarkDuplicate, LandmarkTargetStatus,
    discover_landmarks, landmark_status,
};
pub use merge::{MergeInput, MergeOptions, MergeReport, merge_maps};
pub use preview::{
    PreviewBounds, PreviewDocument, PreviewEntityMarker, PreviewLandmarkMarker, PreviewSolid,
    combine_preview_documents, preview_document, preview_document_with_source,
    translate_preview_document,
};
pub use prune::{
    BrushEntityDeletionMode, DeletionCriteria, DeletionReport, is_critical_entity_classname,
    prune_document,
};
pub use transform::{Vec3, find_landmark_origin, translate_block};
pub use transition::{CampaignTransition, discover_transitions};
pub use validation_rules::{
    BUILTIN_VALIDATION_RULE_SETS, NO_VALIDATION_RULE_SET_ID, RuleSetIssue, RuleSetValidationReport,
    ValidationRuleSet, format_rule_set_issue, validate_document_with_rule_set,
    validation_rule_set_by_id, validation_rule_set_choices,
};
pub use vmf::{Document, Node, ParseError, parse_document};
