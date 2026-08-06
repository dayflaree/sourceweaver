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

    let actual: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let golden_text =
        std::fs::read_to_string(repo_path("tests/golden/fixture-job-report.json")).unwrap();
    let golden: serde_json::Value = serde_json::from_str(&golden_text).unwrap();

    assert_eq!(actual, golden);
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
