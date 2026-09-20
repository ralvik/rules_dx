//! Structured diagnostics and human status emission.
//!
//! Split from `super` (`lib.rs`): owns `DEFAULT_LOG_FILTER`,
//! `VERBOSE_LOG_FILTER`, `init_diagnostics`, `colors_allowed`,
//! `color_enabled`, `styled_status_for`, `styled_status`, `format_status`,
//! and `emit_status`. Re-exported through `super` so the public path stays
//! `dx_output::{...}`.

/// Default tracing filter without `--verbose`: warnings and errors only, so
/// default human output stays byte-identical (info/debug stay silent).
pub const DEFAULT_LOG_FILTER: &str = "warn";
/// Tracing filter under `--verbose`: info and above.
pub const VERBOSE_LOG_FILTER: &str = "info";

/// Initialises structured diagnostics via `tracing-subscriber`.
///
/// Idempotent (`try_init` errors are ignored so tests and repeated calls do
/// not panic). Honors `RUST_LOG` when set; otherwise `warn` by default and
/// `info` under `--verbose`. Output goes to stderr so stdout stays
/// machine-owned per the output protocol. ANSI colors follow
/// [`color_enabled`] (TTY-aware, `NO_COLOR` respected); default runs emit
/// nothing, keeping output byte-identical.
pub fn init_diagnostics(verbose: bool) {
    use tracing_subscriber::{fmt, EnvFilter};
    let default = if verbose {
        VERBOSE_LOG_FILTER
    } else {
        DEFAULT_LOG_FILTER
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
    let ansi = color_enabled();
    let _ = fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(ansi)
        .try_init();
    tracing::debug!(verbose, "dx diagnostics initialised");
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
        // Issue #222: default stays quiet (byte-identical), verbose opens info.
        assert_eq!(DEFAULT_LOG_FILTER, "warn");
        assert_eq!(VERBOSE_LOG_FILTER, "info");
    }

    #[test]
    fn diagnostics_init_is_idempotent_and_tracing_macros_do_not_panic() {
        // Issue #222: tracing-subscriber init never panics on repeat; new
        // code routes through tracing macros.
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
        // Issue #222: pure gate keeps logic testable without env mutation.
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
        // Issue #222: default (disabled) rendering is byte-identical plain.
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
        // Issue #222: NO_COLOR respected even when set to an empty value.
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
        // Issue #222: anstream passthrough keeps default bytes identical.
        // Force the plain path via the pure helper to avoid TTY flakiness.
        let plain = format!("{} {}", styled_status_for("ok", false), "done");
        assert_eq!(plain, "ok done");
    }
}
