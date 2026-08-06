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
echo "'$3' - Decompiled successfully."
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
    assert!(output_vmf.exists());
    assert!(report_path.exists());

    let command_args = report["command_args"].as_array().unwrap();
    assert_eq!(command_args[0], "-o");
    assert_eq!(command_args[1], output_vmf.display().to_string());
    assert_eq!(command_args[2], input_bsp.display().to_string());

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
