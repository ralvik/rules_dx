//! Startup version-skew gate for the `dx` CLI.
//!
//! Skew detection used to be opt-in only (`dx version --check`): a drifted
//! tree (checked-in `.dx/version` pin disagreeing with the module version)
//! proceeded into execution and failed cryptically downstream. This gate
//! surfaces the skew at startup on every workspace command, before any
//! Bazel work starts: one small pin-file read, no subprocesses, zero cost
//! on conforming trees.
//!
//! Fail-vs-warn follows command class, recorded in
//! `docs/cli/commands/status-version.md#startup-skew-gate`: the
//! diagnose/repair path (`version`, `status`) and the version-free shell
//! helper (`completion`) are exempt; read-only commands (`check`, `audit`,
//! `owners`, `deps`, `why`) warn and proceed; every mutating or generating
//! command refuses. `--dry-run` downgrades refusal to a warning because a
//! preview never mutates. A missing or empty pin is a never-pinned tree,
//! not skew, so fresh checkouts proceed (this matches `dx status`, which
//! reports an absent pin as `ok`).

use crate::args::Command;

/// Startup disposition for one invocation under the observed pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkewDisposition {
    /// No skew, or an exempt command: proceed silently.
    Proceed,
    /// Skew on a read-only (or dry-run preview) invocation: warn on
    /// stderr and proceed.
    Warn,
    /// Skew on a mutating or generating invocation: refuse before any
    /// Bazel work starts.
    Refuse,
}

/// Classifies one invocation against the observed pin (`None` when the
/// pin file is missing or empty: a never-pinned tree, never skew).
/// `skewed` must already be resolved against the module version; this
/// function only maps command class (plus `--dry-run`) to disposition.
pub fn disposition(command: Command, dry_run: bool, skewed: bool) -> SkewDisposition {
    if !skewed {
        return SkewDisposition::Proceed;
    }
    match command {
        // Diagnose/repair path and the version-free shell helper stay
        // usable on a drifted tree; gating them would block the fix.
        Command::Version | Command::Status | Command::Completion => SkewDisposition::Proceed,
        // Read-only commands proceed with a warning: the check umbrella
        // runs check-only phases, audit is non-mutating, and the inspect
        // wrappers only forward Bazel queries.
        Command::Check | Command::Audit | Command::Owners | Command::Deps | Command::Why => {
            SkewDisposition::Warn
        }
        // Mutating or generating commands refuse; a dry-run preview
        // never mutates, so it warns instead.
        _ => {
            if dry_run {
                SkewDisposition::Warn
            } else {
                SkewDisposition::Refuse
            }
        }
    }
}

/// Human diagnostic naming the three versions (binary, pin, module) and
/// the repair. `pin` is the observed non-empty drifted pin.
pub fn diagnostic(pin: &str) -> String {
    format!(
        "version skew: binary {} pin {pin} module {}; fix with `dx version --pin {}` or `dx version --rollback`",
        dx_adopt::DX_VERSION,
        dx_adopt::MODULE_VERSION,
        dx_adopt::MODULE_VERSION,
    )
}

/// Reads the workspace `.dx/version` pin (`""` when missing,
/// unreadable, or empty). One small file read, no subprocesses.
pub fn read_pin(workspace: &std::path::Path) -> String {
    dx_adopt::read_version_pin(workspace).unwrap_or_default()
}

/// Whether an observed pin is skewed: present, non-empty, and disagreeing
/// with the module version. A missing or empty pin is a never-pinned
/// tree, not skew.
pub fn is_skewed(pin: &str) -> bool {
    let pin = pin.trim();
    !pin.is_empty() && !dx_adopt::version_pin_matches_module(pin, dx_adopt::MODULE_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SKEWED: bool = true;
    const CLEAN: bool = false;

    #[test]
    fn clean_tree_proceeds_on_every_command() {
        for command in [
            Command::Build,
            Command::Generate,
            Command::Update,
            Command::Bump,
            Command::Check,
            Command::Audit,
            Command::Owners,
            Command::Version,
            Command::Status,
            Command::Completion,
            Command::Bazel,
        ] {
            assert_eq!(
                disposition(command, false, CLEAN),
                SkewDisposition::Proceed,
                "{command:?}"
            );
        }
    }

    #[test]
    fn repair_path_stays_usable_on_skew() {
        for command in [Command::Version, Command::Status, Command::Completion] {
            assert_eq!(
                disposition(command, false, SKEWED),
                SkewDisposition::Proceed,
                "{command:?}"
            );
        }
    }

    #[test]
    fn read_only_commands_warn_on_skew() {
        for command in [
            Command::Check,
            Command::Audit,
            Command::Owners,
            Command::Deps,
            Command::Why,
        ] {
            assert_eq!(
                disposition(command, false, SKEWED),
                SkewDisposition::Warn,
                "{command:?}"
            );
        }
    }

    #[test]
    fn mutating_commands_refuse_on_skew() {
        for command in [
            Command::Build,
            Command::Test,
            Command::Coverage,
            Command::Run,
            Command::Generate,
            Command::Fix,
            Command::Format,
            Command::Lint,
            Command::Typecheck,
            Command::Clean,
            Command::Update,
            Command::Bump,
            Command::Migrate,
            Command::Codegen,
            Command::Env,
            Command::Setup,
            Command::Init,
            Command::Hooks,
            Command::Watch,
            Command::Bazel,
        ] {
            assert_eq!(
                disposition(command, false, SKEWED),
                SkewDisposition::Refuse,
                "{command:?}"
            );
        }
    }

    #[test]
    fn dry_run_downgrades_refusal_to_warning() {
        assert_eq!(
            disposition(Command::Build, true, SKEWED),
            SkewDisposition::Warn
        );
        assert_eq!(
            disposition(Command::Generate, true, SKEWED),
            SkewDisposition::Warn
        );
    }

    #[test]
    fn diagnostic_names_three_versions_and_fix() {
        let message = diagnostic("9.9.9");
        assert!(message.contains(dx_adopt::DX_VERSION), "{message}");
        assert!(message.contains("9.9.9"), "{message}");
        assert!(message.contains(dx_adopt::MODULE_VERSION), "{message}");
        assert!(message.contains("dx version --pin"), "{message}");
        assert!(message.contains("dx version --rollback"), "{message}");
    }

    #[test]
    fn missing_and_empty_pins_are_not_skew() {
        assert!(!is_skewed(""));
        assert!(!is_skewed("   "));
    }

    #[test]
    fn matching_pin_is_not_skew() {
        assert!(!is_skewed(dx_adopt::MODULE_VERSION));
    }

    #[test]
    fn mismatched_pin_is_skew() {
        assert!(is_skewed("9.9.9"));
    }

    #[test]
    fn read_pin_round_trips_through_workspace() {
        let scratch = dx_test_scratch::scratch("dx-skew-pin-");
        let dir = scratch.path().to_path_buf();
        let _ = std::fs::create_dir_all(dir.join(".dx"));
        std::fs::write(dir.join(".dx/version"), "9.9.9\n").expect("write pin");
        let pin = read_pin(&dir);
        assert_eq!(pin, "9.9.9");
        assert!(is_skewed(&pin));
    }

    #[test]
    fn read_pin_defaults_to_empty_off_workspace() {
        let scratch = dx_test_scratch::scratch("dx-skew-nopin-");
        let dir = scratch.path().to_path_buf();
        let _ = std::fs::create_dir_all(&dir);
        assert_eq!(read_pin(&dir), "");
    }
}
