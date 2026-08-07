use std::path::PathBuf;
use std::process::Command;

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn fixture_job_report_matches_golden_snapshot() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args(["run", "--job", "tests/jobs/fixture-merge.toml", "--dry-run"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let mut actual: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let golden_text =
        std::fs::read_to_string(repo_path("tests/golden/fixture-job-report.json")).unwrap();
    let mut golden: serde_json::Value = serde_json::from_str(&golden_text).unwrap();

    normalize_json_path_separators(&mut actual);
    normalize_json_path_separators(&mut golden);

    assert_eq!(actual, golden);
}

fn normalize_json_path_separators(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text) => {
            *text = text.replace('\\', "/");
        }
        serde_json::Value::Array(values) => {
            for value in values {
                normalize_json_path_separators(value);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values_mut() {
                normalize_json_path_separators(value);
            }
        }
        _ => {}
    }
}

#[test]
fn malformed_input_has_actionable_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "inspect",
            repo_path("tests/fixtures/malformed_unclosed.vmf")
                .to_str()
                .unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("malformed_unclosed.vmf"), "{stderr}");
    assert!(stderr.contains("byte"), "{stderr}");
}

#[test]
fn validate_reports_hl2_rule_set_separately() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "validate",
            repo_path("tests/fixtures/hl2_ruleset_warnings.vmf")
                .to_str()
                .unwrap(),
            "--rule-set",
            "hl2",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["integrity"]["errors"], 0);
    assert_eq!(report["rule_set"]["id"], "hl2");
    assert_eq!(report["rule_set"]["errors"], 1);
    assert!(
        report["rule_set"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["rule_id"] == "hl2.changelevel_map")
    );
}

#[test]
fn validate_accepts_hl2_rule_set_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "validate",
            repo_path("tests/fixtures/hl2_ruleset_ok.vmf")
                .to_str()
                .unwrap(),
            "--rule-set",
            "hl2",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:
{}
stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["rule_set"]["id"], "hl2");
    assert_eq!(report["rule_set"]["errors"], 0);
    assert_eq!(report["rule_set"]["warnings"], 0);
}

#[test]
fn validate_reports_entity_semantics_separately() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "validate",
            repo_path("tests/fixtures/entity_semantics_issues.vmf")
                .to_str()
                .unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:
{}
stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["integrity"]["errors"], 0);
    assert_eq!(report["entity_semantics"]["errors"], 0);
    assert_eq!(report["entity_semantics"]["warnings"], 3);
    assert!(
        report["entity_semantics"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["category"] == "duplicate-targetname"
                && issue["targetname"] == "exit_a")
    );
    assert!(
        report["entity_semantics"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["category"] == "missing-target-reference"
                && issue["key"] == "OnTrigger"
                && issue["targetname"] == "door_missing")
    );
}

#[test]
fn validate_warns_for_intentional_duplicate_targetname_groups() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "validate",
            repo_path("tests/fixtures/entity_semantics_group_warning.vmf")
                .to_str()
                .unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:
{}
stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["entity_semantics"]["errors"], 0);
    assert_eq!(report["entity_semantics"]["warnings"], 1);
    assert_eq!(
        report["entity_semantics"]["issues"][0]["category"],
        "duplicate-targetname"
    );
}

#[test]
fn validate_reports_complexity_summary() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "validate",
            repo_path("tests/fixtures/complexity_counts.vmf")
                .to_str()
                .unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:
{}
stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["complexity"]["entities"], 3);
    assert_eq!(report["complexity"]["point_entities"], 2);
    assert_eq!(report["complexity"]["brush_entities"], 1);
    assert_eq!(report["complexity"]["brush_solids"], 2);
    assert_eq!(report["complexity"]["sides"], 2);
    assert_eq!(report["complexity"]["displacements"], 1);
    assert_eq!(report["complexity"]["overlays"], 1);
    assert_eq!(report["complexity"]["warnings"], 0);
}

#[test]
fn job_reports_changelevel_rewrite_policy() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args(["run", "--job", "tests/jobs/changelevel-rewrite.toml"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:
{}
stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["changelevel"]["policy"], "rewrite-internal");
    assert_eq!(
        report["changelevel"]["changed"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        report["changelevel"]["changed"][0]["action"],
        "rewrite-internal"
    );
    assert_eq!(
        report["changelevel"]["changed"][0]["old_map"],
        "changelevel_d1_b"
    );
    assert_eq!(
        report["changelevel"]["changed"][0]["new_map"],
        "stitched_campaign"
    );
    assert_eq!(
        report["changelevel"]["warnings"].as_array().unwrap().len(),
        0
    );
    assert_eq!(
        report["merge"]["changelevel"]["changed"][0]["new_map"],
        "stitched_campaign"
    );
}

#[test]
fn merge_command_applies_disable_changelevel_policy() {
    let output_path = repo_path("target/test-output/changelevel_disabled.vmf");
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args([
            "merge",
            "-o",
            output_path.to_str().unwrap(),
            "--changelevel-policy",
            "disable",
            "tests/fixtures/changelevel_d1_a.vmf",
            "tests/fixtures/changelevel_d1_b.vmf",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:
{}
stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("changelevel policy: disable"), "{stdout}");
    assert!(stdout.contains("changelevel changes: 2"), "{stdout}");
    let merged = std::fs::read_to_string(output_path).unwrap();
    assert!(merged.contains("\"StartDisabled\" \"1\""));
}

#[test]
fn job_reports_transition_cleanup_preserve_diff() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args(["run", "--job", "tests/jobs/transition-cleanup.toml"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:
{}
stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["dry_run"], true);
    assert_eq!(report["output_written"], false);
    assert_eq!(report["changelevel"]["policy"], "delete");
    assert_eq!(report["changelevel"]["scope"], "all");
    assert_eq!(
        report["changelevel"]["changed"].as_array().unwrap().len(),
        1
    );
    assert_eq!(report["changelevel"]["changed"][0]["action"], "delete");
    assert_eq!(
        report["changelevel"]["changed"][0]["old_map"],
        "changelevel_d1_b"
    );
    assert_eq!(
        report["changelevel"]["preserved"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        report["changelevel"]["preserved"][0]["map"],
        "external_entry"
    );
    assert!(
        report["changelevel"]["preserved"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("preserve rule")
    );
}

#[test]
fn merge_command_internal_only_scope_preserves_external_transition() {
    let output_path = repo_path("target/test-output/changelevel_internal_only.vmf");
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args([
            "merge",
            "-o",
            output_path.to_str().unwrap(),
            "--changelevel-policy",
            "delete",
            "--changelevel-scope",
            "internal-only",
            "tests/fixtures/changelevel_d1_a.vmf",
            "tests/fixtures/changelevel_d1_b.vmf",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:
{}
stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("changelevel scope: internal-only"),
        "{stdout}"
    );
    assert!(stdout.contains("changelevel changes: 1"), "{stdout}");
    assert!(stdout.contains("changelevel preserved: 1"), "{stdout}");
    let merged = std::fs::read_to_string(output_path).unwrap();
    assert!(!merged.contains("\"targetname\" \"to_internal\""));
    assert!(merged.contains("\"targetname\" \"to_external\""));
}

#[test]
fn job_reports_campaign_adjacency_graph() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args(["run", "--job", "tests/jobs/campaign-adjacency.toml"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:
{}
stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let edges = report["campaign_adjacency"]["edges"].as_array().unwrap();
    assert!(edges.iter().any(|edge| {
        edge["evidence_kind"] == "trigger_changelevel"
            && edge["confidence"] == "high"
            && edge["from_map"]
                .as_str()
                .unwrap()
                .ends_with("campaign_adjacency_01.vmf")
            && edge["to_map"]
                .as_str()
                .unwrap()
                .ends_with("campaign_adjacency_02.vmf")
    }));
    assert!(edges.iter().any(|edge| {
        edge["evidence_kind"] == "filename_sequence"
            && edge["confidence"] == "low"
            && edge["from_map"]
                .as_str()
                .unwrap()
                .ends_with("campaign_adjacency_02.vmf")
            && edge["to_map"]
                .as_str()
                .unwrap()
                .ends_with("campaign_adjacency_03.vmf")
    }));
    assert!(!edges.iter().any(|edge| {
        edge["evidence_kind"] != "trigger_changelevel"
            && edge["from_map"]
                .as_str()
                .unwrap()
                .ends_with("campaign_adjacency_01.vmf")
            && edge["to_map"]
                .as_str()
                .unwrap()
                .ends_with("campaign_adjacency_02.vmf")
    }));
}

#[test]
fn campaign_plan_reports_all_steps_and_artifacts() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args(["campaign-run", "--plan", "tests/jobs/campaign-plan.toml"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:
{}
stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["name"], "fixture campaign batch");
    assert_eq!(report["dry_run"], true);
    assert_eq!(report["step_count"], 2);
    assert_eq!(report["outputs_written"], 0);
    assert_eq!(report["steps"].as_array().unwrap().len(), 2);
    assert_eq!(report["step_reports"].as_array().unwrap().len(), 2);
    assert!(
        report["steps"][0]["report"]
            .as_str()
            .unwrap()
            .ends_with("campaign_step_adjacency.json")
    );
    assert!(
        report["steps"][1]["report"]
            .as_str()
            .unwrap()
            .ends_with("campaign_step_cleanup.json")
    );
    assert!(report["steps"][0]["adjacency_edges"].as_u64().unwrap() >= 2);
    assert_eq!(report["steps"][1]["changelevel_changed"], 1);
    assert_eq!(report["steps"][1]["changelevel_preserved"], 1);

    let summary_path = repo_path("target/test-output/campaign-plan-summary.json");
    let first_step_report = repo_path("target/test-output/campaign_step_adjacency.json");
    let second_step_report = repo_path("target/test-output/campaign_step_cleanup.json");
    assert!(summary_path.exists());
    assert!(first_step_report.exists());
    assert!(second_step_report.exists());
    assert!(!repo_path("target/test-output/campaign_step_adjacency.vmf").exists());
    assert!(!repo_path("target/test-output/campaign_step_cleanup.vmf").exists());
}

#[test]
fn campaign_run_help_mentions_plan_and_dry_run() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args(["campaign-run", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--plan"), "{stdout}");
    assert!(stdout.contains("Dry-run"), "{stdout}");
}

#[test]
fn job_applies_custom_deletion_preset() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args(["run", "--job", "tests/jobs/deletion-preset.toml"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:
{}
stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        report["deletion_preset"]
            .as_str()
            .unwrap()
            .contains("remove-prop-detail.toml")
    );
    assert_eq!(
        report["deletion"]["classnames"],
        serde_json::json!(["prop_detail"])
    );
    assert_eq!(report["deletion"]["removed_entities"], 1);
}

#[test]
fn inspect_reports_fgd_property_metadata_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args([
            "inspect",
            "tests/fixtures/fgd_property_metadata.vmf",
            "--fgd",
            "tests/fixtures/fgd_property_metadata.fgd",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:
{}
stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let entity = report["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entity| entity["classname"] == "trigger_custom")
        .expect("trigger_custom entity reported");
    let properties = entity["metadata"]["properties"].as_array().unwrap();
    let targetname = properties
        .iter()
        .find(|property| property["key"] == "targetname")
        .expect("targetname property metadata");
    assert_eq!(targetname["type"], "target_source");
    assert_eq!(targetname["label"], "Name");
    assert_eq!(targetname["description"], "Entity name used by Source I/O");
    let mode = properties
        .iter()
        .find(|property| property["key"] == "mode")
        .expect("mode property metadata");
    assert_eq!(mode["default"], "0");
    assert_eq!(mode["choices"].as_array().unwrap().len(), 2);
    assert_eq!(mode["choices"][1]["value"], "1");
    assert_eq!(mode["choices"][1]["label"], "Enabled");
}

#[test]
fn inspect_text_reports_fgd_property_labels() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args([
            "inspect",
            "tests/fixtures/fgd_property_metadata.vmf",
            "--fgd",
            "tests/fixtures/fgd_property_metadata.fgd",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("property	trigger_custom	targetname	Name"),
        "{stdout}"
    );
    assert!(
        stdout.contains("property	trigger_custom	mode	Mode"),
        "{stdout}"
    );
    assert!(
        stdout.contains("Entity name used by Source I/O"),
        "{stdout}"
    );
}

#[test]
fn bspsource_manifest_policy_and_cache_are_reported() {
    let manifest = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args(["bspsource", "manifest", "--json"])
        .output()
        .unwrap();
    assert!(manifest.status.success());
    let manifest_json: serde_json::Value = serde_json::from_slice(&manifest.stdout).unwrap();
    assert_eq!(manifest_json["version"], "v1.4.8");
    assert!(
        manifest_json["assets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|asset| {
                asset["id"] == "jar-only"
                    && asset["sha256"]
                        == "d5effc38b78c4f60f8eb4f9be1db717bb808227a9013f82d20f34860a128b0e7"
            })
    );

    let policy = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args(["bspsource", "policy", "--json"])
        .output()
        .unwrap();
    assert!(policy.status.success());
    let policy_json: serde_json::Value = serde_json::from_slice(&policy.stdout).unwrap();
    assert_eq!(
        policy_json["redistribution_decision"],
        "do-not-bundle; user-initiated download/cache only"
    );
    assert_eq!(policy_json["local_paths_supported"], true);

    let cache = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args([
            "bspsource",
            "cache-path",
            "--asset",
            "jar-only",
            "--cache-dir",
            "target/test-output/bspsource-cache",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(cache.status.success());
    let cache_json: serde_json::Value = serde_json::from_slice(&cache.stdout).unwrap();
    let cache_path = cache_json["path"].as_str().unwrap().replace('\\', "/");
    assert!(cache_path.ends_with("bspsource/v1.4.8/bspsrc-jar-only.zip"));
}

#[test]
fn bspsource_verify_supports_explicit_sha256() {
    let fixture = repo_path("target/test-output/bspsource-checksum-fixture.txt");
    if let Some(parent) = fixture.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&fixture, b"abc").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args([
            "bspsource",
            "verify",
            "--file",
            fixture.to_str().unwrap(),
            "--sha256",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:
{}
stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["size"], 3);
    assert_eq!(
        report["actual_sha256"],
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn bspsource_download_requires_explicit_policy_acceptance() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args([
            "bspsource",
            "download",
            "--asset",
            "jar-only",
            "--cache-dir",
            "target/test-output/bspsource-cache-no-download",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--accept-download-policy"), "{stderr}");
}

#[test]
fn bsp_import_presets_list_known_arguments() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args(["bsp-import-presets", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["raw_tool_arg_supported"], true);
    let presets = report["presets"].as_array().unwrap();
    assert!(presets.iter().any(|preset| {
        preset["id"] == "extract-embedded"
            && preset["args"] == serde_json::json!(["-unpack_embedded"])
    }));
    assert!(presets.iter().any(|preset| {
        preset["id"] == "audit-raw-output"
            && preset["args"] == serde_json::json!(["--no_ttfix", "--no_cubemaptexfix"])
    }));
}

#[cfg(unix)]
#[test]
fn bsp_import_applies_presets_before_raw_tool_args() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = std::env::temp_dir().join(format!(
        "sourceweaver-bspsource-preset-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let fake_bspsource = temp_dir.join("bspsrc.sh");
    let fixture_vmf = repo_path("tests/fixtures/base.vmf");
    let script = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${{1:-}}" == "--version" ]]; then
  echo "BSPSource 1.4.8"
  exit 0
fi
out=""
for ((i=1; i<=$#; i++)); do
  if [[ "${{!i}}" == "-o" ]]; then
    next=$((i+1))
    out="${{!next}}"
  fi
done
if [[ -z "$out" ]]; then
  echo "missing -o" >&2
  exit 64
fi
cp '{}' "$out"
echo "preset args accepted"
"#,
        fixture_vmf.display()
    );
    let mut file = std::fs::File::create(&fake_bspsource).unwrap();
    file.write_all(script.as_bytes()).unwrap();
    drop(file);
    let mut permissions = std::fs::metadata(&fake_bspsource).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_bspsource, permissions).unwrap();

    let input_bsp = temp_dir.join("map.bsp");
    std::fs::write(&input_bsp, b"fake bsp placeholder").unwrap();
    let output_vmf = temp_dir.join("out.vmf");
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args([
            "bsp-import",
            input_bsp.to_str().unwrap(),
            "--bspsource",
            fake_bspsource.to_str().unwrap(),
            "--preset",
            "extract-embedded-all",
            "--tool-arg",
            "--custom-raw",
            "--output",
            output_vmf.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:
{}
stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["tool_arg_presets"][0]["id"], "extract-embedded-all");
    assert_eq!(report["raw_tool_args"], serde_json::json!(["--custom-raw"]));
    let args = report["command_args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    let unpack = args
        .iter()
        .position(|arg| arg == "-unpack_embedded")
        .unwrap();
    let no_smart = args
        .iter()
        .position(|arg| arg == "-no_smart_unpack")
        .unwrap();
    let raw = args.iter().position(|arg| arg == "--custom-raw").unwrap();
    let output_flag = args.iter().position(|arg| arg == "-o").unwrap();
    assert!(
        unpack < no_smart && no_smart < raw && raw < output_flag,
        "{args:?}"
    );
    assert!(output_vmf.is_file());
}

#[test]
fn bsp_derived_fixture_manifest_records_redistributable_boundary() {
    let manifest_text =
        std::fs::read_to_string(repo_path("tests/fixtures/bsp-derived/manifest.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_text).unwrap();
    assert_eq!(manifest["real_external_tool_validation"], false);
    assert!(manifest["license"].as_str().unwrap().contains("CC0-1.0"));
    assert!(
        manifest["bsp_fixture_kind"]
            .as_str()
            .unwrap()
            .contains("Synthetic minimal Source BSP-style header")
    );
    assert!(
        manifest["tool_version"]
            .as_str()
            .unwrap()
            .contains("no real BSPSource")
    );
    assert_eq!(manifest["files"].as_array().unwrap().len(), 2);
}

#[cfg(unix)]
#[test]
fn bsp_import_uses_committed_synthetic_fixture_without_external_tools() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = std::env::temp_dir().join(format!(
        "sourceweaver-bsp-derived-fixture-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let fake_bspsource = temp_dir.join("fake-bspsource.sh");
    let generated_vmf = repo_path("tests/fixtures/bsp-derived/tiny_synthetic_generated.vmf");
    let script = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${{1:-}}" == "--version" ]]; then
  echo "Source Weaver fixture wrapper 1.0"
  exit 0
fi
out=""
for ((i=1; i<=$#; i++)); do
  if [[ "${{!i}}" == "-o" ]]; then
    next=$((i+1))
    out="${{!next}}"
  fi
done
if [[ -z "$out" ]]; then
  echo "missing -o" >&2
  exit 64
fi
cp '{}' "$out"
echo "Synthetic BSP-derived fixture copied by fake wrapper."
"#,
        generated_vmf.display()
    );
    let mut file = std::fs::File::create(&fake_bspsource).unwrap();
    file.write_all(script.as_bytes()).unwrap();
    drop(file);
    let mut permissions = std::fs::metadata(&fake_bspsource).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_bspsource, permissions).unwrap();

    let input_bsp = repo_path("tests/fixtures/bsp-derived/tiny_synthetic_header.bsp");
    let output_vmf = temp_dir.join("synthetic_out.vmf");
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args([
            "bsp-import",
            input_bsp.to_str().unwrap(),
            "--bspsource",
            fake_bspsource.to_str().unwrap(),
            "--output",
            output_vmf.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:
{}
stderr:
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["generated_vmf_exists"], true);
    assert_eq!(report["integrity"]["errors"], 0);
    assert_eq!(report["entity_count"], 2);
    assert!(output_vmf.is_file());
    let generated = std::fs::read_to_string(output_vmf).unwrap();
    assert!(generated.contains("synthetic_start"));
}

#[test]
fn external_decompiler_presets_document_vmex_legacy_boundary() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .current_dir(repo_root())
        .args(["external-decompiler-presets", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert!(
        report["bundle_policy"]
            .as_str()
            .unwrap()
            .contains("does not bundle")
    );
    let presets = report["presets"].as_array().unwrap();
    let vmex = presets
        .iter()
        .find(|preset| preset["id"] == "vmex-legacy-wrapper")
        .expect("VMEX legacy wrapper status reported");
    assert_eq!(vmex["status"], "legacy-documentation-only");
    assert_eq!(vmex["real_tool_validation"], false);
    assert!(vmex["maintenance"].as_str().unwrap().contains("obsolete"));
    assert!(
        vmex["wrapper_example"]
            .as_str()
            .unwrap()
            .ends_with("examples/wrappers/vmex-wrapper.sh")
    );
    assert!(
        vmex["bundle_policy"]
            .as_str()
            .unwrap()
            .contains("do-not-bundle")
    );
    let bspsource = presets
        .iter()
        .find(|preset| preset["id"] == "bspsource-supported")
        .expect("BSPSource supported status reported");
    assert!(
        bspsource["sourceweaver_workflow"]
            .as_str()
            .unwrap()
            .contains("--bspsource")
    );
}

#[cfg(unix)]
#[test]
fn bsp_import_supports_bspsource_cli_argument_shape() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "sourceweaver-bspsource-cli-test-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let fake_bspsource = temp_dir.join("bspsrc.sh");
    let fixture_vmf = repo_path("tests/fixtures/base.vmf");
    let quality_log = repo_path("tests/fixtures/bspsource_quality.log");
    let script = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${{1:-}}" == "--version" ]]; then
  echo "BSPSource 1.4.8"
  exit 0
fi
if [[ "$#" -ne 3 || "$1" != "-o" ]]; then
  echo "unexpected args: $*" >&2
  exit 64
fi
cp '{}' "$2"
cat '{}' >&2
echo "'$3' - Decompiled successfully."
"#,
        fixture_vmf.display(),
        quality_log.display()
    );
    let mut file = std::fs::File::create(&fake_bspsource).unwrap();
    file.write_all(script.as_bytes()).unwrap();
    drop(file);
    let mut permissions = std::fs::metadata(&fake_bspsource).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_bspsource, permissions).unwrap();

    let input_bsp = temp_dir.join("map.bsp");
    std::fs::write(&input_bsp, b"fake bsp placeholder").unwrap();
    let output_vmf = temp_dir.join("out.vmf");
    let report_path = temp_dir.join("report.json");

    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "bsp-import",
            input_bsp.to_str().unwrap(),
            "--bspsource",
            fake_bspsource.to_str().unwrap(),
            "--output",
            output_vmf.to_str().unwrap(),
            "--report",
            report_path.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["tool_kind"], "bspsource-cli");
    assert_eq!(report["tool_version"], "BSPSource 1.4.8");
    assert_eq!(
        report["command_shape"],
        "bspsrc [tool-args] -o <out.vmf> <input.bsp>"
    );
    assert_eq!(report["generated_vmf_exists"], true);
    assert_eq!(report["integrity"]["errors"], 0);
    assert!(report["log_summary"]["errors"].as_u64().unwrap() >= 1);
    assert_eq!(report["decompile_quality"]["ok"], true);
    assert_eq!(report["decompile_quality"]["configuration_noise"], 2);
    assert_eq!(report["decompile_quality"]["unsupported_lumps"], 1);
    assert_eq!(report["decompile_quality"]["skipped_data"], 2);
    assert_eq!(report["decompile_quality"]["quality_risks"], 2);
    assert!(report["decompile_quality"]["issues"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["category"] == "tool-configuration-noise" && issue["fatal"] == false));
    assert!(output_vmf.is_file());
    assert!(report_path.exists());

    let command_args = report["command_args"].as_array().unwrap();
    assert_eq!(command_args[0], "-o");
    assert_eq!(command_args[1], output_vmf.display().to_string());
    assert_eq!(command_args[2], input_bsp.display().to_string());

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[cfg(unix)]
#[test]
fn bsp_import_rejects_directory_at_output_vmf_path() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "sourceweaver-bspsource-directory-test-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let fake_bspsource = temp_dir.join("bspsrc.sh");
    let script = r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--version" ]]; then
  echo "BSPSource 1.4.8"
  exit 0
fi
mkdir -p "$2"
echo "wrote directory instead of VMF"
"#;
    let mut file = std::fs::File::create(&fake_bspsource).unwrap();
    file.write_all(script.as_bytes()).unwrap();
    drop(file);
    let mut permissions = std::fs::metadata(&fake_bspsource).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_bspsource, permissions).unwrap();

    let input_bsp = temp_dir.join("map.bsp");
    std::fs::write(&input_bsp, b"fake bsp placeholder").unwrap();
    let output_vmf = temp_dir.join("out.vmf");

    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "bsp-import",
            input_bsp.to_str().unwrap(),
            "--bspsource",
            fake_bspsource.to_str().unwrap(),
            "--output",
            output_vmf.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], false);
    assert_eq!(report["generated_vmf_exists"], false);
    assert!(output_vmf.is_dir());

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[cfg(unix)]
#[test]
fn pack_generates_bspzip_filelist_and_report() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "sourceweaver-bspzip-test-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let fake_bspzip = temp_dir.join("bspzip.sh");
    let script = r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--version" ]]; then
  echo "Fake BSPZIP 1.0"
  exit 0
fi
if [[ "$#" -ne 4 || "$1" != "-addlist" ]]; then
  echo "unexpected args: $*" >&2
  exit 64
fi
if [[ ! -f "$2" ]]; then
  echo "missing input bsp" >&2
  exit 65
fi
if [[ ! -f "$3" ]]; then
  echo "missing file list" >&2
  exit 66
fi
cp "$2" "$4"
while IFS= read -r internal && IFS= read -r external; do
  [[ -z "$internal" ]] && continue
  echo "Adding file: $external"
done < "$3"
echo "BSPZIP finished"
"#;
    let mut file = std::fs::File::create(&fake_bspzip).unwrap();
    file.write_all(script.as_bytes()).unwrap();
    drop(file);
    let mut permissions = std::fs::metadata(&fake_bspzip).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_bspzip, permissions).unwrap();

    let input_bsp = temp_dir.join("map.bsp");
    std::fs::write(&input_bsp, b"fake bsp").unwrap();
    let asset_root = temp_dir.join("game");
    let material_dir = asset_root.join("materials/custom");
    std::fs::create_dir_all(&material_dir).unwrap();
    std::fs::write(material_dir.join("wall01.vmt"), b"LightmappedGeneric {}").unwrap();
    std::fs::write(material_dir.join("wall01.vtf"), b"fake vtf").unwrap();
    let output_bsp = temp_dir.join("packed.bsp");
    let report_path = temp_dir.join("pack-report.json");
    let log_path = temp_dir.join("pack.log");

    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "pack",
            input_bsp.to_str().unwrap(),
            "--tool",
            fake_bspzip.to_str().unwrap(),
            "--output",
            output_bsp.to_str().unwrap(),
            "--asset-root",
            asset_root.to_str().unwrap(),
            "--include",
            "materials/custom/wall01.vmt",
            "--include",
            "materials\\custom\\wall01.vtf",
            "--log",
            log_path.to_str().unwrap(),
            "--report",
            report_path.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["tool_kind"], "bspzip-addlist");
    assert_eq!(report["tool_version"], "Fake BSPZIP 1.0");
    assert_eq!(
        report["command_shape"],
        "bspzip -addlist <input.bsp> <filelist.txt> <output.bsp>"
    );
    assert_eq!(report["output_bsp_exists"], true);
    assert_eq!(report["missing_files"].as_array().unwrap().len(), 0);
    assert_eq!(report["requested_files"].as_array().unwrap().len(), 2);
    assert_eq!(report["packed_file_count"], 2);
    assert!(output_bsp.exists());
    assert!(report_path.exists());
    assert!(log_path.exists());

    let filelist_path = report["filelist_path"].as_str().unwrap();
    let filelist = std::fs::read_to_string(filelist_path).unwrap();
    assert!(
        filelist.contains("materials/custom/wall01.vmt"),
        "{filelist}"
    );
    assert!(
        filelist.contains("materials/custom/wall01.vtf"),
        "{filelist}"
    );

    let command_args = report["command_args"].as_array().unwrap();
    assert_eq!(command_args[0], "-addlist");
    assert_eq!(command_args[1], input_bsp.display().to_string());
    assert_eq!(command_args[3], output_bsp.display().to_string());

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[cfg(unix)]
#[test]
fn pack_applies_bspzip_context_wrapper_fields() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "sourceweaver-bspzip-context-test-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let tool_cwd = temp_dir.join("game/bin");
    let game_dir = temp_dir.join("game/tf");
    let lib_dir = temp_dir.join("game/bin/linux64");
    std::fs::create_dir_all(&tool_cwd).unwrap();
    std::fs::create_dir_all(&game_dir).unwrap();
    std::fs::create_dir_all(&lib_dir).unwrap();
    let context_log = temp_dir.join("context.log");

    let fake_bspzip = temp_dir.join("bspzip-context.sh");
    let script = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${{1:-}}" == "--version" ]]; then
  echo "Fake BSPZIP context 1.0"
  exit 0
fi
echo "PWD=$(pwd)" > "{log}"
echo "LD_LIBRARY_PATH=${{LD_LIBRARY_PATH:-}}" >> "{log}"
printf 'ARGS=%s\n' "$*" >> "{log}"
if [[ "$#" -ne 6 || "$1" != "-game" || "$3" != "-addlist" ]]; then
  echo "unexpected args: $*" >&2
  exit 64
fi
cp "$4" "$6"
while IFS= read -r internal && IFS= read -r external; do
  [[ -z "$internal" ]] && continue
  echo "Adding file: $internal -> $external"
done < "$5"
"#,
        log = context_log.display()
    );
    let mut file = std::fs::File::create(&fake_bspzip).unwrap();
    file.write_all(script.as_bytes()).unwrap();
    drop(file);
    let mut permissions = std::fs::metadata(&fake_bspzip).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_bspzip, permissions).unwrap();

    let input_bsp = temp_dir.join("map.bsp");
    std::fs::write(&input_bsp, b"fake bsp").unwrap();
    let material_dir = game_dir.join("materials/custom");
    std::fs::create_dir_all(&material_dir).unwrap();
    std::fs::write(material_dir.join("wall01.vmt"), b"LightmappedGeneric {}").unwrap();
    let output_bsp = temp_dir.join("packed.bsp");
    let report_path = temp_dir.join("pack-context-report.json");

    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "pack",
            input_bsp.to_str().unwrap(),
            "--tool",
            fake_bspzip.to_str().unwrap(),
            "--output",
            output_bsp.to_str().unwrap(),
            "--asset-root",
            game_dir.to_str().unwrap(),
            "--include",
            "materials/custom/wall01.vmt",
            "--context-profile",
            "explicit-game-arg-wrapper",
            "--tool-cwd",
            tool_cwd.to_str().unwrap(),
            "--library-path",
            lib_dir.to_str().unwrap(),
            "--game-dir",
            game_dir.to_str().unwrap(),
            "--pass-game-dir",
            "--report",
            report_path.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["tool_version"], "Fake BSPZIP context 1.0");
    assert_eq!(
        report["command_shape"],
        "bspzip -game <game-dir> -addlist <input.bsp> <filelist.txt> <output.bsp>"
    );
    assert_eq!(report["command_args"][0], "-game");
    assert_eq!(report["command_args"][1], game_dir.display().to_string());
    assert_eq!(report["command_args"][2], "-addlist");
    assert_eq!(
        report["tool_context"]["profile_id"],
        "explicit-game-arg-wrapper"
    );
    assert_eq!(
        report["tool_context"]["tool_cwd"],
        tool_cwd.display().to_string()
    );
    assert_eq!(
        report["tool_context"]["game_dir"],
        game_dir.display().to_string()
    );
    assert_eq!(report["tool_context"]["pass_game_dir"], true);
    assert_eq!(report["tool_context"]["real_tool_validation"], false);
    assert!(
        report["tool_context"]["environment_keys"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "LD_LIBRARY_PATH")
    );
    assert!(
        report["tool_context"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str().unwrap().contains("--pass-game-dir"))
    );
    let context_log_text = std::fs::read_to_string(&context_log).unwrap();
    assert!(context_log_text.contains(&format!("PWD={}", tool_cwd.display())));
    assert!(context_log_text.contains(&format!("LD_LIBRARY_PATH={}", lib_dir.display())));
    assert!(context_log_text.contains("ARGS=-game"));
    assert!(output_bsp.exists());
    assert!(report_path.exists());

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn bspzip_context_profiles_document_boundaries() {
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args(["bspzip-context-profiles", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert!(
        report["bundle_policy"]
            .as_str()
            .unwrap()
            .contains("does not bundle")
    );
    assert!(
        report["external_tool_boundary"]
            .as_str()
            .unwrap()
            .contains("does not run BSPZIP")
    );
    let profiles = report["profiles"].as_array().unwrap();
    for id in [
        "stock-game-bin",
        "linux-ld-library-path",
        "bspzipplusplus-sdk2013-x64",
        "explicit-game-arg-wrapper",
    ] {
        let profile = profiles
            .iter()
            .find(|profile| profile["id"] == id)
            .unwrap_or_else(|| panic!("missing profile {id}"));
        assert_eq!(profile["real_tool_validation"], false);
    }
    let bspzippp = profiles
        .iter()
        .find(|profile| profile["id"] == "bspzipplusplus-sdk2013-x64")
        .unwrap();
    assert!(
        bspzippp["requirements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value
                .as_str()
                .unwrap()
                .contains("Garry's Mod is unsupported"))
    );
    for wrapper in [
        "examples/wrappers/bspzip-linux-ld-library-path-wrapper.sh",
        "examples/wrappers/bspzip-game-arg-wrapper.sh",
        "examples/wrappers/bspzip-windows-game-bin-wrapper.ps1",
    ] {
        assert!(repo_root().join(wrapper).is_file(), "missing {wrapper}");
    }
}

#[cfg(unix)]
#[test]
fn pack_discovers_vmf_asset_dependencies_and_related_files() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "sourceweaver-pack-discovery-test-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let fake_bspzip = temp_dir.join("bspzip.sh");
    let script = r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--version" ]]; then
  echo "Fake BSPZIP discovery 1.0"
  exit 0
fi
if [[ "$#" -ne 4 || "$1" != "-addlist" ]]; then
  echo "unexpected args: $*" >&2
  exit 64
fi
cp "$2" "$4"
while IFS= read -r internal && IFS= read -r external; do
  [[ -z "$internal" ]] && continue
  echo "Adding file: $internal -> $external"
done < "$3"
"#;
    let mut file = std::fs::File::create(&fake_bspzip).unwrap();
    file.write_all(script.as_bytes()).unwrap();
    drop(file);
    let mut permissions = std::fs::metadata(&fake_bspzip).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_bspzip, permissions).unwrap();

    let input_bsp = temp_dir.join("map.bsp");
    std::fs::write(&input_bsp, b"fake bsp").unwrap();
    let vmf_path = temp_dir.join("map.vmf");
    std::fs::write(
        &vmf_path,
        r#"
versioninfo { "editorversion" "400" }
world
{
    "id" "1"
    solid
    {
        side { "material" "custom/wall01" }
    }
}
entity
{
    "classname" "prop_static"
    "model" "models/props/sourceweaver_crate.mdl"
}
entity
{
    "classname" "ambient_generic"
    "message" "custom/hum.wav"
}
entity
{
    "classname" "logic_script"
    "vscripts" "scripts/vscripts/sourceweaver_logic.nut"
}
entity
{
    "classname" "info_particle_system"
    "effect_name" "sourceweaver_sparks"
}
"#,
    )
    .unwrap();

    let asset_root = temp_dir.join("game");
    for dir in [
        "materials/custom",
        "models/props",
        "sound/custom",
        "scripts/vscripts",
    ] {
        std::fs::create_dir_all(asset_root.join(dir)).unwrap();
    }
    std::fs::write(
        asset_root.join("materials/custom/wall01.vmt"),
        r#"LightmappedGeneric
{
    "$basetexture" "custom/wall01_color"
    "$bumpmap" "custom/wall01_normal"
}
"#,
    )
    .unwrap();
    for path in [
        "materials/custom/wall01_color.vtf",
        "materials/custom/wall01_normal.vtf",
        "models/props/sourceweaver_crate.mdl",
        "models/props/sourceweaver_crate.vvd",
        "models/props/sourceweaver_crate.dx90.vtx",
        "models/props/sourceweaver_crate.phy",
        "sound/custom/hum.wav",
        "scripts/vscripts/sourceweaver_logic.nut",
    ] {
        std::fs::write(asset_root.join(path), b"fixture").unwrap();
    }

    let output_bsp = temp_dir.join("packed.bsp");
    let report_path = temp_dir.join("pack-report.json");
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "pack",
            input_bsp.to_str().unwrap(),
            "--tool",
            fake_bspzip.to_str().unwrap(),
            "--output",
            output_bsp.to_str().unwrap(),
            "--asset-root",
            asset_root.to_str().unwrap(),
            "--discover-from-vmf",
            vmf_path.to_str().unwrap(),
            "--report",
            report_path.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["missing_files"].as_array().unwrap().len(), 0);
    assert_eq!(
        report["discovered_dependencies"]["missing_assets"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert!(
        report["discovered_dependencies"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning
                .as_str()
                .unwrap()
                .contains("cannot infer the owning PCF"))
    );

    let requested = report["requested_files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|asset| asset["internal_path"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    for expected in [
        "materials/custom/wall01.vmt",
        "materials/custom/wall01_color.vtf",
        "materials/custom/wall01_normal.vtf",
        "models/props/sourceweaver_crate.mdl",
        "models/props/sourceweaver_crate.vvd",
        "models/props/sourceweaver_crate.dx90.vtx",
        "models/props/sourceweaver_crate.phy",
        "sound/custom/hum.wav",
        "scripts/vscripts/sourceweaver_logic.nut",
    ] {
        assert!(
            requested.contains(expected),
            "missing {expected}: {requested:?}"
        );
    }
    assert_eq!(report["packed_file_count"], requested.len());
    assert!(output_bsp.exists());
    assert!(report_path.exists());

    let filelist = std::fs::read_to_string(report["filelist_path"].as_str().unwrap()).unwrap();
    assert!(filelist.contains("materials/custom/wall01.vmt"));
    assert!(filelist.contains("models/props/sourceweaver_crate.dx90.vtx"));

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn cubemap_workflow_writes_cfg_and_reports_runtime_boundary() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "sourceweaver-cubemap-workflow-test-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let bsp_path = temp_dir.join("sw_cubemap_test.bsp");
    std::fs::write(&bsp_path, b"synthetic bsp placeholder").unwrap();
    let cfg_path = temp_dir.join("cfg/sourceweaver_buildcubemaps.cfg");
    let report_path = temp_dir.join("cubemap-report.json");

    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "cubemap-workflow",
            bsp_path.to_str().unwrap(),
            "--profile",
            "hl2-hdr",
            "--steam-app-id",
            "220",
            "--write-cfg",
            cfg_path.to_str().unwrap(),
            "--report",
            report_path.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["map_name"], "sw_cubemap_test");
    assert_eq!(report["profile"]["id"], "hl2-hdr");
    assert_eq!(report["writes_bsp"], true);
    assert_eq!(report["real_game_runtime_validation"], false);
    assert_eq!(report["cfg_written"], true);
    assert_eq!(report["steam_app_id"], "220");
    assert!(
        report["suggested_steam_command"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "-condebug")
    );
    assert!(
        report["console_commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "buildcubemaps")
    );
    assert!(
        report["external_tool_boundary"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value
                .as_str()
                .unwrap()
                .contains("No Steam client, Source game executable, game runtime"))
    );
    assert!(cfg_path.exists());
    assert!(report_path.exists());
    let cfg = std::fs::read_to_string(&cfg_path).unwrap();
    assert!(cfg.contains("mat_hdr_level 0"));
    assert!(cfg.contains("buildcubemaps"));
    assert!(cfg.contains("sw_cubemap_test"));

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[cfg(unix)]
#[test]
fn compile_profile_create_validate_and_discover_reports_tools() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "sourceweaver-compile-profile-test-{}-{nonce}",
        std::process::id()
    ));
    let bin_dir = temp_dir.join("bin");
    let game_dir = temp_dir.join("game");
    let log_dir = temp_dir.join("logs");
    std::fs::create_dir_all(&bin_dir).unwrap();
    std::fs::create_dir_all(&game_dir).unwrap();

    for tool in ["vbsp", "vvis", "vrad"] {
        let path = bin_dir.join(tool);
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "#!/usr/bin/env bash\necho {tool} fixture\n").unwrap();
        drop(file);
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).unwrap();
    }

    let profile_path = temp_dir.join("compile-profile.toml");
    let create_output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "compile-profile",
            "create",
            "--output",
            profile_path.to_str().unwrap(),
            "--vbsp",
            bin_dir.join("vbsp").to_str().unwrap(),
            "--vvis",
            bin_dir.join("vvis").to_str().unwrap(),
            "--vrad",
            bin_dir.join("vrad").to_str().unwrap(),
            "--game",
            game_dir.to_str().unwrap(),
            "--steps",
            "vbsp,vvis,vrad",
            "--log-dir",
            log_dir.to_str().unwrap(),
            "--timeout-seconds",
            "42",
            "--validate",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        create_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&create_output.stdout),
        String::from_utf8_lossy(&create_output.stderr)
    );
    assert!(profile_path.exists());
    let create_report: serde_json::Value = serde_json::from_slice(&create_output.stdout).unwrap();
    assert_eq!(create_report["ok"], true);
    assert_eq!(create_report["validation"]["ok"], true);
    assert_eq!(
        create_report["validation"]["tools"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let validate_output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "compile-profile",
            "validate",
            "--profile",
            profile_path.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        validate_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&validate_output.stdout),
        String::from_utf8_lossy(&validate_output.stderr)
    );
    let validate_report: serde_json::Value =
        serde_json::from_slice(&validate_output.stdout).unwrap();
    assert_eq!(validate_report["ok"], true);
    assert_eq!(validate_report["timeout_seconds"], 42);

    let discovered_profile = temp_dir.join("discovered.toml");
    let discover_output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "compile-profile",
            "discover",
            "--search-dir",
            bin_dir.to_str().unwrap(),
            "--output",
            discovered_profile.to_str().unwrap(),
            "--game",
            game_dir.to_str().unwrap(),
            "--log-dir",
            log_dir.to_str().unwrap(),
            "--timeout-seconds",
            "99",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        discover_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&discover_output.stdout),
        String::from_utf8_lossy(&discover_output.stderr)
    );
    assert!(discovered_profile.exists());
    let discover_report: serde_json::Value =
        serde_json::from_slice(&discover_output.stdout).unwrap();
    assert_eq!(discover_report["ok"], true);
    assert_eq!(discover_report["tools"].as_array().unwrap().len(), 3);
    for tool in discover_report["tools"].as_array().unwrap() {
        assert!(
            tool["selected"]
                .as_str()
                .unwrap()
                .contains("sourceweaver-compile-profile-test")
        );
    }

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn model_inspect_reads_mdl_header_prefix() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "sourceweaver-model-inspect-test-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let mdl_path = temp_dir.join("sample.mdl");
    let mut bytes = vec![0_u8; 160];
    bytes[0..4].copy_from_slice(b"IDST");
    bytes[4..8].copy_from_slice(&48_i32.to_le_bytes());
    bytes[8..12].copy_from_slice(&1234_i32.to_le_bytes());
    let name = b"models/sourceweaver/sample.mdl";
    bytes[12..12 + name.len()].copy_from_slice(name);
    bytes[76..80].copy_from_slice(&160_i32.to_le_bytes());
    std::fs::write(&mdl_path, bytes).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args(["model-inspect", mdl_path.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["header"]["magic"], "IDST");
    assert_eq!(report["header"]["version"], 48);
    assert_eq!(report["header"]["checksum"], 1234);
    assert_eq!(report["header"]["name"], "models/sourceweaver/sample.mdl");
    assert_eq!(report["header"]["data_length"], 160);
    assert_eq!(report["header"]["supported_magic"], true);

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[cfg(unix)]
#[test]
fn model_compile_runs_external_studiomdl_and_reports() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "sourceweaver-model-compile-test-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let game_dir = temp_dir.join("game");
    std::fs::create_dir_all(&game_dir).unwrap();
    let qc_path = temp_dir.join("sample.qc");
    std::fs::write(&qc_path, "$modelname \"sourceweaver/sample.mdl\"\n").unwrap();

    let fake_studiomdl = temp_dir.join("studiomdl.sh");
    let script = r#"#!/usr/bin/env bash
set -euo pipefail
if [[ "$#" -ne 4 || "$1" != "-nop4" || "$2" != "-game" ]]; then
  echo "unexpected args: $*" >&2
  exit 64
fi
if [[ ! -d "$3" ]]; then
  echo "game dir missing" >&2
  exit 65
fi
if [[ ! -f "$4" ]]; then
  echo "qc missing" >&2
  exit 66
fi
echo "StudioMDL fake compile"
echo "0 errors, 0 warnings"
"#;
    let mut file = std::fs::File::create(&fake_studiomdl).unwrap();
    file.write_all(script.as_bytes()).unwrap();
    drop(file);
    let mut permissions = std::fs::metadata(&fake_studiomdl).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_studiomdl, permissions).unwrap();

    let log_path = temp_dir.join("model-compile.log");
    let report_path = temp_dir.join("model-compile-report.json");
    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "model-compile",
            qc_path.to_str().unwrap(),
            "--studiomdl",
            fake_studiomdl.to_str().unwrap(),
            "--tool-arg",
            "-nop4",
            "--game",
            game_dir.to_str().unwrap(),
            "--log",
            log_path.to_str().unwrap(),
            "--report",
            report_path.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(
        report["command_shape"],
        "studiomdl [tool-args] [-game <game-dir>] <model.qc>"
    );
    assert_eq!(report["exit_code"], 0);
    assert_eq!(report["log_summary"]["errors"], 0);
    assert_eq!(report["log_summary"]["warnings"], 0);
    assert!(log_path.exists());
    assert!(report_path.exists());
    let command_args = report["command_args"].as_array().unwrap();
    assert_eq!(command_args[0], "-nop4");
    assert_eq!(command_args[1], "-game");
    assert_eq!(command_args[2], game_dir.display().to_string());
    assert_eq!(command_args[3], qc_path.display().to_string());

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[cfg(unix)]
#[test]
fn model_decompile_generic_wrapper_captures_outputs_and_boundary() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "sourceweaver-model-decompile-test-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let wrapper = temp_dir.join("model-decompile-wrapper.sh");
    let script = r#"#!/usr/bin/env bash
set -euo pipefail
input=""
output=""
game=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --input) input="$2"; shift 2 ;;
    --output|--output-dir) output="$2"; shift 2 ;;
    --game) game="$2"; shift 2 ;;
    *) echo "unexpected arg: $1" >&2; exit 64 ;;
  esac
done
[[ -n "$input" && -n "$output" && -n "$game" ]]
mkdir -p "$output/anims"
cat > "$output/model.qc" <<'QC'
$modelname "props/sourceweaver_fixture.mdl"
QC
echo "version 1" > "$output/reference.smd"
echo "version 1" > "$output/anims/idle.smd"
echo "WARNING: fake wrapper emitted a recoverable decompile note"
"#;
    let mut file = std::fs::File::create(&wrapper).unwrap();
    file.write_all(script.as_bytes()).unwrap();
    drop(file);
    let mut permissions = std::fs::metadata(&wrapper).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&wrapper, permissions).unwrap();

    let mdl_path = temp_dir.join("sourceweaver_fixture.mdl");
    let mut mdl = vec![0_u8; 80];
    mdl[0..4].copy_from_slice(b"IDST");
    std::fs::write(&mdl_path, mdl).unwrap();
    let game_dir = temp_dir.join("game");
    std::fs::create_dir_all(&game_dir).unwrap();
    let output_dir = temp_dir.join("decompiled");
    let log_path = temp_dir.join("model-decompile.log");
    let report_path = temp_dir.join("model-decompile-report.json");

    let output = Command::new(env!("CARGO_BIN_EXE_sourceweaver"))
        .args([
            "model-decompile",
            mdl_path.to_str().unwrap(),
            "--tool",
            wrapper.to_str().unwrap(),
            "--output-dir",
            output_dir.to_str().unwrap(),
            "--game",
            game_dir.to_str().unwrap(),
            "--tool-arg",
            "--input",
            "--tool-arg",
            "{input}",
            "--tool-arg",
            "--output",
            "--tool-arg",
            "{output-dir}",
            "--tool-arg",
            "--game",
            "--tool-arg",
            "{game}",
            "--log",
            log_path.to_str().unwrap(),
            "--report",
            report_path.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["tool_kind"], "generic-headless-wrapper");
    assert_eq!(report["uses_argument_template"], true);
    assert_eq!(report["real_tool_validation"], false);
    assert_eq!(report["game"], game_dir.display().to_string());
    assert_eq!(report["log_summary"]["warnings"], 1);
    assert!(
        report["external_tool_boundary"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str().unwrap().contains("does not bundle Crowbar"))
    );
    let outputs = report["discovered_outputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    for expected in ["model.qc", "reference.smd", "anims/idle.smd"] {
        assert!(
            outputs.contains(expected),
            "missing {expected}: {outputs:?}"
        );
    }
    assert!(log_path.exists());
    assert!(report_path.exists());
    assert!(
        repo_root()
            .join("examples/wrappers/model-decompile-wrapper.sh")
            .is_file()
    );

    let _ = std::fs::remove_dir_all(temp_dir);
}
