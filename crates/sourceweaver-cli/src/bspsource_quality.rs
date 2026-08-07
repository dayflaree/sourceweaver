use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct BspSourceQualitySnapshot {
    pub ok: bool,
    pub issue_count: usize,
    pub errors: usize,
    pub warnings: usize,
    pub quality_risks: usize,
    pub skipped_data: usize,
    pub unsupported_lumps: usize,
    pub configuration_noise: usize,
    pub issues: Vec<BspSourceQualityIssueSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BspSourceQualityIssueSnapshot {
    pub severity: String,
    pub category: String,
    pub fatal: bool,
    pub line: usize,
    pub message: String,
    pub rationale: String,
}

pub fn parse_bspsource_quality_log(log: &str) -> BspSourceQualitySnapshot {
    let mut issues = Vec::new();
    for (line_index, raw_line) in log.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(issue) = classify_bspsource_line(line_index + 1, line) {
            issues.push(issue);
        }
    }

    let errors = issues
        .iter()
        .filter(|issue| issue.severity == "error")
        .count();
    let warnings = issues
        .iter()
        .filter(|issue| issue.severity == "warning")
        .count();
    let quality_risks = issues
        .iter()
        .filter(|issue| issue.category == "quality-risk")
        .count();
    let skipped_data = issues
        .iter()
        .filter(|issue| issue.category == "skipped-data")
        .count();
    let unsupported_lumps = issues
        .iter()
        .filter(|issue| issue.category == "unsupported-lump")
        .count();
    let configuration_noise = issues
        .iter()
        .filter(|issue| issue.category == "tool-configuration-noise")
        .count();
    let fatal = issues.iter().any(|issue| issue.fatal);

    BspSourceQualitySnapshot {
        ok: !fatal,
        issue_count: issues.len(),
        errors,
        warnings,
        quality_risks,
        skipped_data,
        unsupported_lumps,
        configuration_noise,
        issues,
    }
}

fn classify_bspsource_line(
    line_number: usize,
    line: &str,
) -> Option<BspSourceQualityIssueSnapshot> {
    let lower = line.to_ascii_lowercase();
    if is_configuration_noise(&lower) {
        return Some(issue(
            "info",
            "tool-configuration-noise",
            false,
            line_number,
            line,
            "non-fatal JVM/logger/tool configuration noise; reported separately from decompile quality",
        ));
    }
    if contains_any(
        &lower,
        &[
            "unsupported lump",
            "unknown lump",
            "lump not supported",
            "unsupported gamelump",
            "unknown gamelump",
        ],
    ) {
        return Some(issue(
            "warning",
            "unsupported-lump",
            false,
            line_number,
            line,
            "BSPSource reported data/lump support limitations that can reduce decompile completeness",
        ));
    }
    if contains_any(
        &lower,
        &[
            "skipped",
            "skipping",
            "not decompiled",
            "ignored",
            "omitted",
            "discarded",
        ],
    ) {
        return Some(issue(
            "warning",
            "skipped-data",
            false,
            line_number,
            line,
            "BSPSource skipped or omitted source data while producing a VMF",
        ));
    }
    if contains_any(
        &lower,
        &[
            "anti-decompile",
            "protected map",
            "protection",
            "could not restore",
            "couldn't restore",
            "invalid solid",
            "bad brush",
            "degenerate",
            "displacement",
            "overlay",
            "cubemap",
            "pakfile",
            "embedded file",
            "missing texture",
            "texture not found",
            "model not found",
        ],
    ) {
        return Some(issue(
            "warning",
            "quality-risk",
            false,
            line_number,
            line,
            "line indicates a likely decompile-quality risk requiring manual review",
        ));
    }
    if contains_any(&lower, &["exception", "fatal", "severe", "traceback"])
        || starts_with_log_level(&lower, "error")
    {
        return Some(issue(
            "error",
            "tool-error",
            true,
            line_number,
            line,
            "BSPSource reported an error-like line that may indicate failed or incomplete decompile output",
        ));
    }
    if lower.contains("warning") || starts_with_log_level(&lower, "warn") {
        return Some(issue(
            "warning",
            "decompile-warning",
            false,
            line_number,
            line,
            "generic BSPSource warning; inspect the generated VMF before merge",
        ));
    }
    None
}

fn issue(
    severity: &str,
    category: &str,
    fatal: bool,
    line: usize,
    message: &str,
    rationale: &str,
) -> BspSourceQualityIssueSnapshot {
    BspSourceQualityIssueSnapshot {
        severity: severity.to_string(),
        category: category.to_string(),
        fatal,
        line,
        message: message.to_string(),
        rationale: rationale.to_string(),
    }
}

fn is_configuration_noise(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "picked up java_tool_options",
            "invalid element or attribute \"isdecompiletaskfilter\"",
            "log4j",
            "slf4j",
            "illegal reflective access",
            "gtk warning",
            "fontconfig warning",
            "awt-appkit",
        ],
    )
}

fn starts_with_log_level(lower: &str, level: &str) -> bool {
    lower.starts_with(level)
        || lower.starts_with(&format!("[{level}]"))
        || lower.contains(&format!(" {level} "))
        || lower.contains(&format!(" {level}:"))
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorizes_representative_bspsource_quality_log() {
        let report = parse_bspsource_quality_log(include_str!(
            "../../../tests/fixtures/bspsource_quality.log"
        ));
        assert_eq!(report.configuration_noise, 2);
        assert_eq!(report.unsupported_lumps, 1);
        assert_eq!(report.skipped_data, 2);
        assert_eq!(report.quality_risks, 2);
        assert_eq!(report.errors, 0);
        assert!(report.ok);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.category == "tool-configuration-noise" && !issue.fatal)
        );
    }

    #[test]
    fn marks_real_error_like_lines_as_fatal() {
        let report = parse_bspsource_quality_log("ERROR Failed to read BSP header\n");
        assert!(!report.ok);
        assert_eq!(report.errors, 1);
        assert!(report.issues[0].fatal);
    }
}
