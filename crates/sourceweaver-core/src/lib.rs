//! Core VMF parsing, inspection, pruning, and merge operations for Source Weaver.
//!
//! This crate intentionally has no UI dependencies. The desktop app, CLI, and future
//! automation layers should all call into this library so Linux and Windows builds
//! share the same VMF behavior.

pub mod classify;
pub mod compiler;
pub mod integrity;
pub mod landmark;
pub mod merge;
pub mod preview;
pub mod prune;
pub mod transform;
pub mod transition;
pub mod vmf;

pub use classify::{BrushRole, EntityRecord, inspect_entities, summarize_entity_types};
pub use compiler::{
    CompileLogSummary, VmfToolValidationReport, parse_compile_log, validate_for_source_tools,
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
pub use prune::{BrushEntityDeletionMode, DeletionCriteria, DeletionReport, prune_document};
pub use transform::{Vec3, find_landmark_origin, translate_block};
pub use transition::{CampaignTransition, discover_transitions};
pub use vmf::{Document, Node, ParseError, parse_document};
