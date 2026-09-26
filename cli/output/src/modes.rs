use super::OutputError;

/// Live output mode selecting stdout ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Concise human summaries; preserves subprocess stdout and stderr.
    Text { quiet: bool },
    /// Stdout carries only the complete unified patch.
    Diff,
    /// Stdout carries only NDJSON; subprocess output moves to stderr.
    Json,
}

impl OutputMode {
    pub fn parse(text: &str, quiet: bool) -> Result<Self, OutputError> {
        Ok(OutputModeName::parse(text)?.resolve(quiet))
    }

    pub fn name(&self) -> &'static str {
        match self {
            OutputMode::Text { .. } => "text",
            OutputMode::Diff => "diff",
            OutputMode::Json => "json",
        }
    }
}

/// Fieldless `--output` spelling: the `clap::ValueEnum` source of truth
/// for the accepted mode names (`text|diff|json`). `quiet` cannot live on
/// the enum (value enums are fieldless), so it resolves separately via
/// [`OutputModeName::resolve`]: only text mode observes `--quiet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputModeName {
    Text,
    Diff,
    Json,
}

impl OutputModeName {
    pub fn parse(text: &str) -> Result<Self, OutputError> {
        use clap::ValueEnum;
        Self::from_str(text, false).map_err(|_| OutputError::UnknownOutputMode {
            value: text.to_owned(),
        })
    }

    pub fn resolve(self, quiet: bool) -> OutputMode {
        match self {
            OutputModeName::Text => OutputMode::Text { quiet },
            OutputModeName::Diff => OutputMode::Diff,
            OutputModeName::Json => OutputMode::Json,
        }
    }
}

/// Who owns stdout under a mode and report selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdoutOwner {
    /// `dx` text summaries share stdout with subprocess passthrough.
    DxText,
    /// Complete unified patch only; summaries and normalized diagnostics
    /// are suppressed rather than moved to stderr.
    Patch,
    /// One JSON object per line; no prose or subprocess bytes.
    Ndjson,
    /// Exactly one standard-report document; summaries and subprocess
    /// output move to stderr.
    Report,
}

pub fn stdout_owner(mode: &OutputMode, stdout_report: bool) -> StdoutOwner {
    if stdout_report {
        return StdoutOwner::Report;
    }
    match mode {
        OutputMode::Text { .. } => StdoutOwner::DxText,
        OutputMode::Diff => StdoutOwner::Patch,
        OutputMode::Json => StdoutOwner::Ndjson,
    }
}

pub fn dx_text_visible(mode: &OutputMode) -> bool {
    matches!(mode, OutputMode::Text { quiet: false })
}

pub fn check_output_conflict(mode: &OutputMode, stdout_reports: usize) -> Result<(), OutputError> {
    if stdout_reports > 1 {
        return Err(OutputError::SecondStdoutReport);
    }
    if stdout_reports == 1 && !matches!(mode, OutputMode::Text { .. }) {
        return Err(OutputError::ConflictingStdoutReport { mode: mode.name() });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutputError;

    #[test]
    fn modes_parse_and_name() {
        assert_eq!(
            OutputMode::parse("text", false).expect("text"),
            OutputMode::Text { quiet: false }
        );
        assert_eq!(
            OutputMode::parse("text", true).expect("quiet"),
            OutputMode::Text { quiet: true }
        );
        assert_eq!(
            OutputMode::parse("diff", false).expect("diff"),
            OutputMode::Diff
        );
        assert_eq!(
            OutputMode::parse("json", true).expect("json"),
            OutputMode::Json
        );
        assert!(OutputMode::parse("xml", false).is_err());
    }

    #[test]
    fn output_mode_names_are_clap_value_enum() {
        use clap::ValueEnum;
        // Spellings stay exact: the retired match rejected `TEXT`.
        assert_eq!(
            OutputModeName::parse("text").expect("text"),
            OutputModeName::Text
        );
        assert_eq!(
            OutputModeName::parse("diff").expect("diff"),
            OutputModeName::Diff
        );
        assert_eq!(
            OutputModeName::parse("json").expect("json"),
            OutputModeName::Json
        );
        assert!(OutputModeName::parse("TEXT").is_err());
        assert!(OutputModeName::parse("xml").is_err());
        let mut values: Vec<String> = OutputModeName::value_variants()
            .iter()
            .map(|variant| {
                variant
                    .to_possible_value()
                    .expect("named")
                    .get_name()
                    .to_owned()
            })
            .collect();
        values.sort_unstable();
        assert_eq!(values, ["diff", "json", "text"]);
        // `quiet` resolves into text mode only.
        assert_eq!(
            OutputModeName::Text.resolve(true),
            OutputMode::Text { quiet: true }
        );
        assert_eq!(OutputModeName::Diff.resolve(true), OutputMode::Diff);
        assert_eq!(OutputModeName::Json.resolve(false), OutputMode::Json);
    }

    #[test]
    fn stdout_ownership_per_mode() {
        let text = OutputMode::Text { quiet: false };
        assert_eq!(stdout_owner(&text, false), StdoutOwner::DxText);
        assert_eq!(stdout_owner(&text, true), StdoutOwner::Report);
        assert_eq!(stdout_owner(&OutputMode::Diff, false), StdoutOwner::Patch);
        assert_eq!(stdout_owner(&OutputMode::Json, false), StdoutOwner::Ndjson);
        assert_eq!(stdout_owner(&OutputMode::Json, true), StdoutOwner::Report);
    }

    #[test]
    fn quiet_suppresses_text_only() {
        assert!(dx_text_visible(&OutputMode::Text { quiet: false }));
        assert!(!dx_text_visible(&OutputMode::Text { quiet: true }));
        assert!(!dx_text_visible(&OutputMode::Diff));
        assert!(!dx_text_visible(&OutputMode::Json));
    }

    #[test]
    fn stdout_report_conflicts() {
        let text = OutputMode::Text { quiet: false };
        check_output_conflict(&text, 0).expect("none ok");
        check_output_conflict(&text, 1).expect("one text report ok");
        assert_eq!(
            check_output_conflict(&text, 2).expect_err("second stdout report"),
            OutputError::SecondStdoutReport
        );
        assert_eq!(
            check_output_conflict(&OutputMode::Diff, 1).expect_err("diff conflict"),
            OutputError::ConflictingStdoutReport { mode: "diff" }
        );
        assert_eq!(
            check_output_conflict(&OutputMode::Json, 1).expect_err("json conflict"),
            OutputError::ConflictingStdoutReport { mode: "json" }
        );
    }

    #[test]
    fn mode_names_cover_all_modes() {
        assert_eq!(OutputMode::Text { quiet: false }.name(), "text");
        assert_eq!(OutputMode::Diff.name(), "diff");
        assert_eq!(OutputMode::Json.name(), "json");
    }
}
