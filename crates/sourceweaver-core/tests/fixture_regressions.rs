use sourceweaver_core::transform::translate_document;
use sourceweaver_core::{
    BrushRole, DeletionCriteria, Vec3, discover_landmarks, discover_transitions, inspect_entities,
    merge_maps, parse_document, preview_document, prune_document,
};
use sourceweaver_core::{MergeInput, MergeOptions};

fn fixture(name: &str) -> String {
    let path = format!("{}/../../tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path).unwrap()
}

fn golden(name: &str) -> String {
    let path = format!("{}/../../tests/golden/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn complex_fixture_covers_roles_landmarks_transitions_and_preview() {
    let document = parse_document(&fixture("complex_roles.vmf")).unwrap();

    let records = inspect_entities(&document);
    assert_eq!(records.len(), 6);
    assert!(records[0].roles.contains(&BrushRole::WorldBrush));
    assert!(records[0].roles.contains(&BrushRole::Clip));
    assert!(records[0].roles.contains(&BrushRole::Skybox));
    assert!(records[0].roles.contains(&BrushRole::Areaportal));
    assert!(records[0].roles.contains(&BrushRole::Water));
    assert!(
        records
            .iter()
            .any(|record| record.roles.contains(&BrushRole::Trigger))
    );

    let landmarks = discover_landmarks(&document);
    assert_eq!(
        landmarks.targetnames,
        vec!["duplicate_lm", "map_transition"]
    );
    assert_eq!(landmarks.duplicates.len(), 1);
    assert_eq!(landmarks.duplicates[0].targetname, "duplicate_lm");

    let transitions = discover_transitions(&document);
    assert_eq!(transitions.len(), 1);
    assert_eq!(transitions[0].target_map.as_deref(), Some("incoming"));
    assert_eq!(transitions[0].landmark.as_deref(), Some("map_transition"));

    let preview = preview_document(&document);
    assert_eq!(preview.landmarks.len(), 3);
    assert!(preview.solids.len() >= 6);
    assert!(preview.bounds.is_some());
}

#[test]
fn prune_complex_fixture_matches_expected_counts() {
    let mut document = parse_document(&fixture("complex_roles.vmf")).unwrap();
    let mut criteria = DeletionCriteria::default();
    criteria.brush_roles.insert(BrushRole::Clip);
    criteria.brush_roles.insert(BrushRole::Trigger);
    criteria.brush_entity_mode = sourceweaver_core::BrushEntityDeletionMode::MatchingSolids;

    let report = prune_document(&mut document, &criteria);

    assert_eq!(report.removed_entities, 0);
    assert_eq!(report.removed_world_solids, 1);
    assert_eq!(report.removed_brush_entity_solids, 1);
    let rendered = document.to_vmf_string();
    assert!(!rendered.contains("TOOLS/TOOLSCLIP"));
    assert!(!rendered.contains("TOOLS/TOOLSTRIGGER"));
}

#[test]
fn translates_displacement_startposition_in_fixture() {
    let mut document = parse_document(&fixture("complex_roles.vmf")).unwrap();

    translate_document(&mut document, Vec3::new(32.0, -16.0, 8.0));
    let rendered = document.to_vmf_string();

    assert!(rendered.contains("\"startposition\" \"[32 -16 136]\""));
    assert!(rendered.contains("(32 -16 136) (96 -16 136) (96 48 136)"));
}

#[test]
fn fixture_merge_matches_golden_output() {
    let base = parse_document(&fixture("base.vmf")).unwrap();
    let incoming = parse_document(&fixture("incoming.vmf")).unwrap();

    let (merged, report) = merge_maps(
        vec![
            MergeInput {
                label: "tests/fixtures/base.vmf".to_string(),
                document: base,
            },
            MergeInput {
                label: "tests/fixtures/incoming.vmf".to_string(),
                document: incoming,
            },
        ],
        &MergeOptions {
            landmark: Some("map_transition".to_string()),
        },
    )
    .unwrap();

    assert_eq!(report.merged_maps, 2);
    assert_eq!(merged.to_vmf_string(), golden("fixture-merge.vmf"));
}

#[test]
fn malformed_fixture_reports_stable_parse_position() {
    let error = parse_document(&fixture("malformed_unclosed.vmf")).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("missing closing brace"), "{message}");
    assert!(message.contains("byte"), "{message}");
}
