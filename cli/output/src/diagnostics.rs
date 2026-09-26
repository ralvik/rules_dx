pub const DEFAULT_LOG_FILTER: &str = "warn";
pub const VERBOSE_LOG_FILTER: &str = "info";

/// Structured color mode for `--color`.
/// See: `docs/cli/cli-contract.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// TTY-aware color (`NO_COLOR` respected, non-TTY stays plain).
    #[default]
    Auto,
    /// Force color even when non-TTY or `NO_COLOR` is set.
    Always,
    /// Never use color, even on a TTY.
    Never,
}

impl ColorMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ColorMode::Auto => "auto",
            ColorMode::Always => "always",
            ColorMode::Never => "never",
        }
    }

    pub fn parse(text: &str) -> Result<Self, crate::OutputError> {
        match text {
            "auto" => Ok(ColorMode::Auto),
            "always" => Ok(ColorMode::Always),
            "never" => Ok(ColorMode::Never),
            _ => Err(crate::OutputError::BadColor {
                value: text.to_owned(),
            }),
        }
    }
}

static COLOR_OVERRIDE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn set_color_override(mode: ColorMode) {
    let value = match mode {
        ColorMode::Auto => 0,
        ColorMode::Always => 1,
        ColorMode::Never => 2,
    };
    COLOR_OVERRIDE.store(value, std::sync::atomic::Ordering::SeqCst);
}

pub fn color_override() -> ColorMode {
    match COLOR_OVERRIDE.load(std::sync::atomic::Ordering::SeqCst) {
        1 => ColorMode::Always,
        2 => ColorMode::Never,
        _ => ColorMode::Auto,
    }
}

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
    pub fn name(self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }

    pub fn filter(self) -> &'static str {
        self.name()
    }

    pub fn parse(text: &str) -> Result<Self, crate::OutputError> {
        use clap::ValueEnum;
        Self::from_str(text, false).map_err(|_| crate::OutputError::BadLogLevel {
            value: text.to_owned(),
        })
    }
}

pub fn resolve_log_filter(verbose: bool, level: Option<LogLevel>) -> &'static str {
    match level {
        Some(level) => level.filter(),
        None if verbose => VERBOSE_LOG_FILTER,
        None => DEFAULT_LOG_FILTER,
    }
}

pub fn init_diagnostics(verbose: bool) {
    init_diagnostics_with_level(verbose, None);
}

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

pub fn colors_allowed(no_color_present: bool, tty: bool) -> bool {
    colors_allowed_for(ColorMode::Auto, no_color_present, tty)
}

pub fn colors_allowed_for(mode: ColorMode, no_color_present: bool, tty: bool) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => !no_color_present && tty,
    }
}

pub fn color_enabled_for(mode: ColorMode) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => {
            if std::env::var_os("NO_COLOR").is_some() {
                return false;
            }
            console::colors_enabled_stderr()
        }
    }
}

pub fn color_enabled() -> bool {
    color_enabled_for(color_override())
}

pub fn styled_status_for(status: &str, enabled: bool) -> String {
    if !enabled {
        return status.to_owned();
    }
    console::style(status).green().bold().to_string()
}

pub fn styled_status(status: &str) -> String {
    styled_status_for(status, color_enabled())
}

pub fn format_status(status: &str, message: &str) -> String {
    format!("{} {message}", styled_status(status))
}

pub fn emit_status(status: &str, message: &str) {
    use std::io::Write;
    let line = format_status(status, message);
    match color_override() {
        ColorMode::Auto => {
            let mut err = anstream::stderr();
            let _ = writeln!(err, "{line}");
        }
        ColorMode::Always | ColorMode::Never => {
            let mut err = std::io::stderr();
            let _ = writeln!(err, "{line}");
        }
    }
    tracing::info!(status, message, "dx status");
}

pub fn init_diagnostics_with_color(verbose: bool, level: Option<LogLevel>, color: ColorMode) {
    set_color_override(color);
    init_diagnostics_with_level(verbose, level);
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
        let prior_override = color_override();
        set_color_override(ColorMode::Auto);
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
        set_color_override(prior_override);
    }

    #[test]
    fn diagnostics_format_status_is_byte_identical_when_plain() {
        // Force the plain path via the pure helper to avoid TTY flakiness.
        let plain = format!("{} {}", styled_status_for("ok", false), "done");
        assert_eq!(plain, "ok done");
    }

    #[test]
    fn diagnostics_color_mode_parses_known_spellings() {
        assert_eq!(ColorMode::parse("auto").expect("auto"), ColorMode::Auto);
        assert_eq!(
            ColorMode::parse("always").expect("always"),
            ColorMode::Always
        );
        assert_eq!(ColorMode::parse("never").expect("never"), ColorMode::Never);
        assert!(ColorMode::parse("AUTO").is_err());
        assert!(ColorMode::parse("yes").is_err());
        assert_eq!(ColorMode::Auto.as_str(), "auto");
        assert_eq!(ColorMode::Always.as_str(), "always");
        assert_eq!(ColorMode::Never.as_str(), "never");
        assert_eq!(ColorMode::default(), ColorMode::Auto);
    }

    #[test]
    fn diagnostics_color_gates_follow_explicit_mode() {
        assert!(colors_allowed_for(ColorMode::Always, true, false));
        assert!(colors_allowed_for(ColorMode::Always, false, false));
        assert!(!colors_allowed_for(ColorMode::Never, false, true));
        assert!(!colors_allowed_for(ColorMode::Never, true, true));
        assert!(!colors_allowed_for(ColorMode::Auto, true, true));
        assert!(!colors_allowed_for(ColorMode::Auto, false, false));
        assert!(colors_allowed_for(ColorMode::Auto, false, true));
        assert!(color_enabled_for(ColorMode::Always));
        assert!(!color_enabled_for(ColorMode::Never));
    }
}
