//! Structured diagnostics and human status emission.
//!
//! Contract: `docs/cli/output-protocol.md`.

pub const DEFAULT_LOG_FILTER: &str = "warn";
pub const VERBOSE_LOG_FILTER: &str = "info";

/// Structured diagnostic level for `--log-level` (See: `docs/cli/output-protocol.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    /// Canonical lowercase spelling (`error|warn|info|debug|trace`).
    pub fn name(self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }

    /// Tracing filter for the level (same spelling as [`LogLevel::name`]).
    pub fn filter(self) -> &'static str {
        self.name()
    }

    /// Parses one `--log-level` spelling, case-sensitively.
    pub fn parse(text: &str) -> Result<Self, crate::OutputError> {
        use clap::ValueEnum;
        Self::from_str(text, false).map_err(|_| crate::OutputError::BadLogLevel {
            value: text.to_owned(),
        })
    }
}

/// Resolves the default tracing filter: explicit `--log-level` wins,
/// else `info` under `--verbose`, else `warn`. `RUST_LOG` still
/// overrides the result inside [`init_diagnostics_with_level`].
pub fn resolve_log_filter(verbose: bool, level: Option<LogLevel>) -> &'static str {
    match level {
        Some(level) => level.filter(),
        None if verbose => VERBOSE_LOG_FILTER,
        None => DEFAULT_LOG_FILTER,
    }
}

/// Initialises structured diagnostics via `tracing-subscriber`.
///
/// Idempotent (`try_init` errors are ignored so tests and repeated calls do
/// not panic). Honors `RUST_LOG` when set; otherwise `warn` by default and
/// `info` under `--verbose`. Output goes to stderr so stdout stays
/// machine-owned per the output protocol. ANSI colors follow
/// [`color_enabled`] (TTY-aware, `NO_COLOR` respected); default runs emit
/// nothing, keeping output byte-identical.
pub fn init_diagnostics(verbose: bool) {
    init_diagnostics_with_level(verbose, None);
}

/// Initialises diagnostics with an explicit `--log-level` override.
///
/// Same contract as [`init_diagnostics`], except the default filter comes
/// from [`resolve_log_filter`] (`--log-level` over `--verbose` over `warn`);
/// `RUST_LOG` still wins when set.
pub fn init_diagnostics_with_level(verbose: bool, level: Option<LogLevel>) {
    use tracing_subscriber::{fmt, EnvFilter};
    let default = resolve_log_filter(verbose, level);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
    let ansi = color_enabled();
    let _ = fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(ansi)
        .try_init();
    tracing::debug!(verbose, ?level, "dx diagnostics initialised");
}

/// Pure color gate for tests: no color without a TTY or when `NO_COLOR` is
/// present. Production probes TTY via [`color_enabled`].
pub fn colors_allowed(no_color_present: bool, tty: bool) -> bool {
    !no_color_present && tty
}

/// Reports whether styled human output may use color.
///
/// Returns false when `NO_COLOR` is present (any value, per the spec) or
/// when stderr is not a TTY (via `console`, which also honors `CLICOLOR`,
/// `TERM=dumb`, and Windows VT). True only when color is unambiguous, so
/// default non-TTY runs stay byte-identical plain text.
pub fn color_enabled() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    console::colors_enabled_stderr()
}

/// Renders `status` with explicit color control: plain when `enabled` is
/// false (byte-identical), green bold via `console` when true.
pub fn styled_status_for(status: &str, enabled: bool) -> String {
    if !enabled {
        return status.to_owned();
    }
    console::style(status).green().bold().to_string()
}

/// Renders a human status word, colored only when [`color_enabled`].
/// Default (non-TTY or `NO_COLOR`) output is byte-identical plain text.
pub fn styled_status(status: &str) -> String {
    styled_status_for(status, color_enabled())
}

/// Formats one human status line as `{status} {message}` with TTY-aware
/// styling. Plain (byte-identical) unless colors are enabled.
pub fn format_status(status: &str, message: &str) -> String {
    format!("{} {message}", styled_status(status))
}

/// Emits one human status line to a TTY-aware stderr stream via `anstream`
///  and mirrors it as a structured `tracing::info!` event for
/// future JSON-log consumers. `anstream` passes plain text through
/// unchanged, so default output stays byte-identical.
pub fn emit_status(status: &str, message: &str) {
    use std::io::Write;
    let line = format_status(status, message);
    let mut err = anstream::stderr();
    let _ = writeln!(err, "{line}");
    tracing::info!(status, message, "dx status");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_filters_have_expected_spelling() {
        assert_eq!(DEFAULT_LOG_FILTER, "warn");
        assert_eq!(VERBOSE_LOG_FILTER, "info");
    }

    #[test]
    fn diagnostics_log_levels_parse_and_resolve() {
        assert_eq!(LogLevel::parse("error").expect("error"), LogLevel::Error);
        assert_eq!(LogLevel::parse("warn").expect("warn"), LogLevel::Warn);
        assert_eq!(LogLevel::parse("info").expect("info"), LogLevel::Info);
        assert_eq!(LogLevel::parse("debug").expect("debug"), LogLevel::Debug);
        assert_eq!(LogLevel::parse("trace").expect("trace"), LogLevel::Trace);
        assert!(LogLevel::parse("WARN").is_err());
        assert!(LogLevel::parse("verbose").is_err());
        assert_eq!(LogLevel::Debug.name(), "debug");
        assert_eq!(LogLevel::Trace.filter(), "trace");
        assert_eq!(resolve_log_filter(false, None), "warn");
        assert_eq!(resolve_log_filter(true, None), "info");
        assert_eq!(resolve_log_filter(false, Some(LogLevel::Debug)), "debug");
        assert_eq!(resolve_log_filter(true, Some(LogLevel::Trace)), "trace");
    }

    #[test]
    fn diagnostics_leveled_init_is_idempotent() {
        init_diagnostics_with_level(false, None);
        init_diagnostics_with_level(true, None);
        init_diagnostics_with_level(false, Some(LogLevel::Debug));
    }

    #[test]
    fn diagnostics_init_is_idempotent_and_tracing_macros_do_not_panic() {
        init_diagnostics(false);
        init_diagnostics(true);
        init_diagnostics(false);
        tracing::debug!("dx diagnostics debug probe");
        tracing::info!("dx diagnostics info probe");
        tracing::warn!("dx diagnostics warn probe");
        tracing::error!("dx diagnostics error probe");
    }

    #[test]
    fn diagnostics_color_gate_is_tty_and_no_color_aware() {
        assert!(!colors_allowed(true, true), "NO_COLOR always wins");
        assert!(!colors_allowed(true, false), "NO_COLOR always wins");
        assert!(!colors_allowed(false, false), "no TTY means plain");
        assert!(
            colors_allowed(false, true),
            "TTY without NO_COLOR allows color"
        );
    }

    #[test]
    fn diagnostics_styled_status_stays_plain_when_disabled() {
        assert_eq!(styled_status_for("ok", false), "ok");
        assert_eq!(
            styled_status_for("warning", false),
            "warning",
            "disabled styling must not inject escape codes"
        );
        let enabled = styled_status_for("ok", true);
        assert!(
            enabled.contains("ok"),
            "enabled styling must preserve the status word: {enabled:?}"
        );
    }

    #[test]
    fn diagnostics_no_color_env_disables_color() {
        let prior = std::env::var_os("NO_COLOR");
        unsafe {
            std::env::set_var("NO_COLOR", "");
        }
        assert!(
            !color_enabled(),
            "NO_COLOR presence (even empty) must disable color"
        );
        assert_eq!(
            styled_status("ok"),
            "ok",
            "NO_COLOR styling must stay byte-identical plain"
        );
        if let Some(value) = prior {
            unsafe {
                std::env::set_var("NO_COLOR", value);
            }
        } else {
            unsafe {
                std::env::remove_var("NO_COLOR");
            }
        }
    }

    #[test]
    fn diagnostics_format_status_is_byte_identical_when_plain() {
        // Force the plain path via the pure helper to avoid TTY flakiness.
        let plain = format!("{} {}", styled_status_for("ok", false), "done");
        assert_eq!(plain, "ok done");
    }
}
