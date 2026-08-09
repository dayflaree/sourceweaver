//! Desktop CLI command preview and execution helpers.

use super::*;
use std::process::Command;

pub(crate) fn default_bsp_decompile_output_path(input_bsp: &Path) -> PathBuf {
    let stem = input_bsp
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("decompiled_map");
    input_bsp.with_file_name(format!("{stem}_decompiled.vmf"))
}

pub(crate) fn default_bsp_decompile_report_path(input_bsp: &Path) -> PathBuf {
    let stem = input_bsp
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("bsp-import");
    input_bsp.with_file_name(format!("{stem}-bsp-import-report.json"))
}

pub(crate) fn default_bsp_decompile_log_path(input_bsp: &Path) -> PathBuf {
    let stem = input_bsp
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("bsp-import");
    input_bsp.with_file_name(format!("{stem}-bsp-import.log"))
}

pub(crate) fn desktop_bsp_decompile_command_preview(
    request: &DesktopBspDecompileRequest,
) -> Vec<String> {
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
    if let Some(preset) = &request.preset {
        parts.push("--preset".to_string());
        parts.push(preset.clone());
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

pub(crate) fn run_desktop_bsp_decompile_request(
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
    if let Some(preset) = &request.preset {
        command.arg("--preset").arg(preset);
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
            let parsed_report = report_json
                .as_ref()
                .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok());
            let parsed_ok = parsed_report
                .as_ref()
                .and_then(|value| value.get("ok").and_then(serde_json::Value::as_bool))
                .unwrap_or(false);
            let quality_summary = parsed_report
                .as_ref()
                .map(desktop_bsp_quality_summary)
                .unwrap_or_default();
            let quality_issues = parsed_report
                .as_ref()
                .map(desktop_bsp_quality_issue_labels)
                .unwrap_or_default();
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
                quality_summary,
                quality_issues,
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
            quality_summary: String::new(),
            quality_issues: Vec::new(),
            stdout_tail: Vec::new(),
            stderr_tail: Vec::new(),
        },
    }
}

pub(crate) fn desktop_bsp_quality_summary(report: &serde_json::Value) -> String {
    let Some(quality) = report.get("decompile_quality") else {
        return String::new();
    };
    format!(
        "{} issue(s): {} unsupported lump(s), {} skipped-data item(s), {} quality risk(s), {} configuration-noise line(s)",
        quality
            .get("issue_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0),
        quality
            .get("unsupported_lumps")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0),
        quality
            .get("skipped_data")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0),
        quality
            .get("quality_risks")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0),
        quality
            .get("configuration_noise")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    )
}

pub(crate) fn desktop_bsp_quality_issue_labels(report: &serde_json::Value) -> Vec<String> {
    report
        .get("decompile_quality")
        .and_then(|quality| quality.get("issues"))
        .and_then(serde_json::Value::as_array)
        .map(|issues| {
            issues
                .iter()
                .take(20)
                .map(|issue| {
                    format!(
                        "{}:{} line {} — {}",
                        issue
                            .get("severity")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("unknown"),
                        issue
                            .get("category")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("uncategorized"),
                        issue
                            .get("line")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0),
                        issue
                            .get("message")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("")
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn default_compile_report_path(output_path: &str) -> PathBuf {
    if output_path.trim().is_empty() {
        PathBuf::from("sourceweaver-compile-report.json")
    } else {
        default_compile_report_path_for_map(&PathBuf::from(output_path.trim()))
    }
}

pub(crate) fn desktop_profile_wizard_command_preview(
    request: &DesktopProfileWizardRequest,
) -> Vec<String> {
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

pub(crate) fn push_optional_path_arg(parts: &mut Vec<String>, flag: &str, value: &Option<PathBuf>) {
    if let Some(path) = value {
        parts.push(flag.to_string());
        parts.push(path.display().to_string());
    }
}

pub(crate) fn run_desktop_profile_wizard_request(
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

pub(crate) fn split_whitespace_args(value: &str) -> Vec<String> {
    value.split_whitespace().map(ToOwned::to_owned).collect()
}

pub(crate) fn default_model_compile_report_path_for_qc(qc_path: &Path) -> PathBuf {
    let stem = qc_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("model");
    qc_path.with_file_name(format!("{stem}-model-compile-report.json"))
}

pub(crate) fn desktop_model_inspect_command_preview(
    request: &DesktopModelInspectRequest,
) -> Vec<String> {
    vec![
        request.cli_path.display().to_string(),
        "model-inspect".to_string(),
        request.mdl_path.display().to_string(),
        "--json".to_string(),
    ]
}

pub(crate) fn run_desktop_model_inspect_request(
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

pub(crate) fn desktop_model_compile_command_preview(
    request: &DesktopModelCompileRequest,
) -> Vec<String> {
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

pub(crate) fn run_desktop_model_compile_request(
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

pub(crate) fn default_packed_bsp_path(input_bsp: &Path) -> PathBuf {
    let stem = input_bsp
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("packed");
    input_bsp.with_file_name(format!("{stem}-packed.bsp"))
}

pub(crate) fn default_pack_report_path_for_bsp(output_bsp: &Path) -> PathBuf {
    let stem = output_bsp
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("sourceweaver-pack");
    output_bsp.with_file_name(format!("{stem}-pack-report.json"))
}

pub(crate) fn desktop_bsp_pack_command_preview(request: &DesktopBspPackRequest) -> Vec<String> {
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

pub(crate) fn run_desktop_bsp_pack_request(
    request: DesktopBspPackRequest,
) -> DesktopBspPackMessage {
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

pub(crate) fn default_compile_report_path_for_map(map_path: &Path) -> PathBuf {
    let stem = map_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("sourceweaver");
    map_path.with_file_name(format!("{stem}-compile-report.json"))
}

pub(crate) fn desktop_bspsource_preset_label(id: &str) -> String {
    BSPSOURCE_DESKTOP_PRESETS
        .iter()
        .find(|(preset_id, _label, _args, _tradeoff)| *preset_id == id)
        .map(|(_id, label, args, _tradeoff)| {
            if args.is_empty() {
                (*label).to_string()
            } else {
                format!("{label} ({args})")
            }
        })
        .unwrap_or_else(|| id.to_string())
}

pub(crate) fn desktop_bspsource_preset_tradeoff(id: &str) -> &'static str {
    BSPSOURCE_DESKTOP_PRESETS
        .iter()
        .find(|(preset_id, _label, _args, _tradeoff)| *preset_id == id)
        .map(|(_id, _label, _args, tradeoff)| *tradeoff)
        .unwrap_or("Unknown preset; raw args remain available.")
}

pub(crate) fn sourceweaver_cli_executable() -> PathBuf {
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

pub(crate) fn sourceweaver_cli_candidate_names() -> Vec<&'static str> {
    if cfg!(windows) {
        vec!["sourceweaver.exe", "sourceweaver-cli.exe"]
    } else {
        vec!["sourceweaver", "sourceweaver-cli"]
    }
}

pub(crate) fn desktop_compile_command_preview(request: &DesktopCompileRequest) -> Vec<String> {
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

pub(crate) fn run_desktop_compile_request(request: DesktopCompileRequest) -> DesktopCompileMessage {
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

pub(crate) fn tail_lines(text: &str, limit: usize) -> Vec<String> {
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
