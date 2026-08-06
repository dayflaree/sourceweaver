//! Core VMF parsing, inspection, pruning, and merge operations for Source Weaver.
//!
//! This crate intentionally has no UI dependencies. The desktop app, CLI, and future
//! automation layers should all call into this library so Linux and Windows builds
//! share the same VMF behavior.

pub mod classify;
pub mod integrity;
pub mod landmark;
pub mod merge;
pub mod preview;
pub mod prune;
pub mod transform;
pub mod vmf;

pub use classify::{BrushRole, EntityRecord, inspect_entities, summarize_entity_types};
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
    PreviewBounds, PreviewDocument, PreviewEntityMarker, PreviewSolid, preview_document,
};
pub use prune::{BrushEntityDeletionMode, DeletionCriteria, DeletionReport, prune_document};
pub use transform::{Vec3, find_landmark_origin, translate_block};
pub use vmf::{Document, Node, ParseError, parse_document};
