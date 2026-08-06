use crate::integrity::{IntegrityReport, validate_document_integrity};
use crate::vmf::Document;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompileLogSummary {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub leak_detected: bool,
    pub successful: bool,
}

impl CompileLogSummary {
    pub fn is_ok(&self) -> bool {
        self.successful && self.errors.is_empty() && !self.leak_detected
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmfToolValidationReport {
    pub map_label: String,
    pub integrity: IntegrityReport,
    pub compile_log: Option<CompileLogSummary>,
}

impl VmfToolValidationReport {
    pub fn is_ok(&self) -> bool {
        self.integrity.is_ok()
            && self
                .compile_log
                .as_ref()
                .map(CompileLogSummary::is_ok)
                .unwrap_or(true)
    }
}

pub fn validate_for_source_tools(
    document: &Document,
    label: &str,
    compile_log: Option<&str>,
) -> VmfToolValidationReport {
    VmfToolValidationReport {
        map_label: label.to_string(),
        integrity: validate_document_integrity(document, label),
        compile_log: compile_log.map(parse_compile_log),
    }
}

pub fn parse_compile_log(log: &str) -> CompileLogSummary {
    let mut summary = CompileLogSummary::default();

    for raw_line in log.lines() {
        let line = raw_line.trim();
        let lowercase = line.to_ascii_lowercase();
        if line.is_empty() {
            continue;
        }

        if lowercase.contains("vbsp finished")
            || lowercase.contains("bspzip finished")
            || lowercase.contains("process complete")
            || lowercase.contains("0 errors")
        {
            summary.successful = true;
            continue;
        }

        if lowercase.contains("leaked!") || lowercase.contains("entity leaked") {
            summary.leak_detected = true;
            summary.errors.push(line.to_string());
            continue;
        }

        if lowercase.contains("error")
            || lowercase.starts_with("***")
            || lowercase.contains("can't load")
            || lowercase.contains("cannot load")
            || lowercase.contains("invalid")
        {
            summary.errors.push(line.to_string());
            continue;
        }

        if lowercase.contains("warning") || lowercase.contains("material not found") {
            summary.warnings.push(line.to_string());
            continue;
        }
    }

    if summary.errors.is_empty()
        && !summary.leak_detected
        && log
            .lines()
            .any(|line| line.to_ascii_lowercase().contains("vbsp"))
    {
        summary.successful = true;
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmf::parse_document;

    #[test]
    fn parses_successful_vbsp_log() {
        let log = r#"
Valve Software - vbsp.exe (Nov 10 2025)
materialPath: c:\source\hl2\materials
Loading test.vmf
0 errors, 0 warnings
VBSP finished successfully
"#;

        let summary = parse_compile_log(log);

        assert!(summary.is_ok());
        assert!(summary.errors.is_empty());
        assert!(summary.warnings.is_empty());
    }

    #[test]
    fn parses_errors_warnings_and_leaks() {
        let log = r#"
WARNING: node without a volume
**** leaked ****
Entity prop_static (-64 0 0) leaked!
Error opening c:\maps\broken.vmf
"#;

        let summary = parse_compile_log(log);

        assert!(!summary.is_ok());
        assert!(summary.leak_detected);
        assert_eq!(summary.warnings.len(), 1);
        assert_eq!(summary.errors.len(), 3);
    }

    #[test]
    fn combines_integrity_and_compile_log_validation() {
        let document = parse_document(
            r#"
versioninfo { "editorversion" "400" }
viewsettings { "bSnapToGrid" "1" }
world { "id" "1" }
"#,
        )
        .unwrap();

        let report = validate_for_source_tools(
            &document,
            "fixture.vmf",
            Some("Valve Software - vbsp\n0 errors, 0 warnings\n"),
        );

        assert!(report.is_ok());
        assert_eq!(report.map_label, "fixture.vmf");
    }
}
