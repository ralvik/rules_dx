//! Direct-Bazel per-result threshold evaluator (M03 WP3).
//!
//! Contract: `docs/cli/cli-contract.md` (direct Bazel CI) and result
//! semantics in `docs/quality/quality-result-protocol.md#execution-and-policy`.
//! One evaluator consumes one validated `QualityResult` and fails when any
//! initial or terminal diagnostic meets the `--fail_on` threshold, when the
//! result proposes any replacement independently of severity, or when
//! convergence is not `STABLE`. `fixable` never weakens evaluation: direct
//! Bazel applies no fix, so every original finding still counts, exactly as
//! CLI check mode evaluates every original finding including
//! guaranteed-fixable ones.

use quality_result::proto::{Convergence, Diagnostic, QualityResult, Severity};

/// Lowest diagnostic severity that fails evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Threshold {
    Info,
    Warning,
    Error,
}

impl Threshold {
    /// Numeric rank on the frozen `Severity` scale.
    pub fn rank(self) -> i32 {
        match self {
            Threshold::Info => Severity::Info as i32,
            Threshold::Warning => Severity::Warning as i32,
            Threshold::Error => Severity::Error as i32,
        }
    }

    /// Canonical flag spelling, matching `@rules_dx//config:fail_on`.
    pub fn name(self) -> &'static str {
        match self {
            Threshold::Info => "info",
            Threshold::Warning => "warning",
            Threshold::Error => "error",
        }
    }
}

/// Parses a `--fail_on` value. There is no `never`: any spelling outside
/// `info|warning|error` is an evaluator failure.
pub fn parse_threshold(text: &str) -> Result<Threshold, String> {
    match text {
        "info" => Ok(Threshold::Info),
        "warning" => Ok(Threshold::Warning),
        "error" => Ok(Threshold::Error),
        _ => Err(format!("unknown fail_on {text:?}, want info|warning|error")),
    }
}

/// Policy outcome for one result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    pub passed: bool,
    pub reasons: Vec<String>,
}

fn describe(set: &str, diagnostic: &Diagnostic) -> String {
    format!(
        "{set} diagnostic {:?}:{}:{}:{:?}:{:?}:{}",
        diagnostic.severity,
        diagnostic.tool_id,
        diagnostic.path,
        diagnostic.start_byte,
        diagnostic.end_byte,
        diagnostic.message
    )
}

/// Applies the threshold policy to one validated result. Malformed results
/// never reach this function: the binary decodes with
/// `quality_result::decode_validated` first, so decode failure fails the
/// evaluator action at every threshold before any comparison runs.
pub fn evaluate(result: &QualityResult, threshold: Threshold) -> Evaluation {
    let mut reasons = Vec::new();
    if result.convergence != Convergence::Stable as i32 {
        reasons.push(format!(
            "convergence is {}, want STABLE",
            result.convergence
        ));
    }
    if !result.replacements.is_empty() {
        reasons.push(format!(
            "proposes {} replacement file(s), independently of severity",
            result.replacements.len()
        ));
    }
    let rank = threshold.rank();
    for (set, diagnostics) in [
        ("initial", &result.initial_diagnostics),
        ("terminal", &result.terminal_diagnostics),
    ] {
        for diagnostic in diagnostics {
            if diagnostic.severity >= rank {
                reasons.push(describe(set, diagnostic));
            }
        }
    }
    let passed = reasons.is_empty();
    Evaluation { passed, reasons }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quality_result::proto::FileEdits;

    fn finding(severity: i32) -> Diagnostic {
        Diagnostic {
            severity,
            message: "synthetic finding".to_owned(),
            tool_id: "lint-a".to_owned(),
            path: "quality/testdata/dirty.py".to_owned(),
            start_byte: Some(0),
            end_byte: Some(3),
            ..Default::default()
        }
    }

    fn clean() -> QualityResult {
        QualityResult {
            convergence: Convergence::Stable as i32,
            ..Default::default()
        }
    }

    #[test]
    fn rank_orders_severity_scale() {
        assert!(Threshold::Info.rank() < Threshold::Warning.rank());
        assert!(Threshold::Warning.rank() < Threshold::Error.rank());
    }

    #[test]
    fn names_match_fail_on_values() {
        assert_eq!(Threshold::Info.name(), "info");
        assert_eq!(Threshold::Warning.name(), "warning");
        assert_eq!(Threshold::Error.name(), "error");
    }

    #[test]
    fn parses_all_thresholds() {
        assert_eq!(parse_threshold("info"), Ok(Threshold::Info));
        assert_eq!(parse_threshold("warning"), Ok(Threshold::Warning));
        assert_eq!(parse_threshold("error"), Ok(Threshold::Error));
    }

    #[test]
    fn rejects_unknown_and_never() {
        assert!(parse_threshold("never").is_err());
        assert!(parse_threshold("WARNING").is_err());
        assert!(parse_threshold("").is_err());
    }

    #[test]
    fn clean_result_passes_every_threshold() {
        let result = clean();
        for threshold in [Threshold::Info, Threshold::Warning, Threshold::Error] {
            let evaluation = evaluate(&result, threshold);
            assert!(evaluation.passed);
            assert!(evaluation.reasons.is_empty());
        }
    }

    #[test]
    fn info_passes_below_threshold_and_fails_at_info() {
        let mut result = clean();
        result
            .initial_diagnostics
            .push(finding(Severity::Info as i32));
        assert!(evaluate(&result, Threshold::Warning).passed);
        assert!(evaluate(&result, Threshold::Error).passed);
        let evaluation = evaluate(&result, Threshold::Info);
        assert!(!evaluation.passed);
        assert_eq!(evaluation.reasons.len(), 1);
    }

    #[test]
    fn warning_fails_warning_and_info_but_passes_error() {
        let mut result = clean();
        result
            .initial_diagnostics
            .push(finding(Severity::Warning as i32));
        assert!(!evaluate(&result, Threshold::Warning).passed);
        assert!(!evaluate(&result, Threshold::Info).passed);
        assert!(evaluate(&result, Threshold::Error).passed);
    }

    #[test]
    fn error_fails_every_threshold() {
        let mut result = clean();
        result
            .initial_diagnostics
            .push(finding(Severity::Error as i32));
        for threshold in [Threshold::Info, Threshold::Warning, Threshold::Error] {
            assert!(!evaluate(&result, threshold).passed);
        }
    }

    #[test]
    fn terminal_only_finding_still_fails() {
        let mut result = clean();
        result
            .terminal_diagnostics
            .push(finding(Severity::Error as i32));
        let evaluation = evaluate(&result, Threshold::Error);
        assert!(!evaluation.passed);
        assert_eq!(evaluation.reasons.len(), 1);
    }

    #[test]
    fn fixable_finding_still_fails_without_apply() {
        let mut fixable = finding(Severity::Warning as i32);
        fixable.fixable = true;
        let mut result = clean();
        result.initial_diagnostics.push(fixable);
        assert!(!evaluate(&result, Threshold::Warning).passed);
    }

    #[test]
    fn replacements_fail_at_every_threshold_without_diagnostics() {
        let mut result = clean();
        result.replacements.push(FileEdits::default());
        for threshold in [Threshold::Info, Threshold::Warning, Threshold::Error] {
            let evaluation = evaluate(&result, threshold);
            assert!(!evaluation.passed);
            assert_eq!(evaluation.reasons.len(), 1);
        }
    }

    #[test]
    fn non_stable_convergence_fails_with_no_replacements() {
        for convergence in [Convergence::Oscillation, Convergence::IterationLimit] {
            let result = QualityResult {
                convergence: convergence as i32,
                ..Default::default()
            };
            let evaluation = evaluate(&result, Threshold::Error);
            assert!(!evaluation.passed);
            assert_eq!(evaluation.reasons.len(), 1);
        }
    }
}
