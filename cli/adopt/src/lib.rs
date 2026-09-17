//! Delivered adoption behavior (M30b: `dx init` scaffolding, hermetic hook
//! runner, devcontainer admission, `dx status` diagnostics, single-version
//! `dx version` with rollback, local watch loop, thin inspect forwarding,
//! and single-source completion generation).
//!
//! Planning predicates below own the adoption shape; the I/O helpers after
//! them deliver it: absent-only scaffolding, unmanaged refusal, pin files
//! that equal the `rules_dx` module version, hermetic-only hook Git,
//! two-layer hook configuration, pinned Bazel-delegated devcontainers, the
//! consolidated status surface (never `doctor`), local-only re-resolved
//! watch iterations, thin `query`/`cquery` forwarding, and completion
//! scripts generated from the single command table. Helpers operate on
//! injected paths only and touch no network.
//!
//! Domain split (issue #236): single-source completion vocabulary lives in
//! the `completion` module, major-release migration planning lives in
//! the `migrate` module, the single-version pin lives in the `version`
//! module, thin inspect forwarding lives in the `inspect` module, the
//! local watch loop lives in the `watch` module, absent-only `dx init`
//! scaffolding lives in the `scaffold` module, and the hermetic hook
//! runner lives in the `hooks` module. This facade keeps
//! the re-exports; the public path stays stable via the re-exports below.

// Issue #238: infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

pub mod completion;
pub mod hooks;
pub mod inspect;
pub mod migrate;
pub mod scaffold;
pub mod status;
pub mod version;
pub mod watch;

pub use completion::{completion_source_is_single, ALL_COMMANDS, SUPPORTED_SHELLS};
pub use hooks::{
    hook_git_is_hermetic, hook_shim_overwrite_allowed, hook_status_shows_merged, install_hooks,
    render_hook_shim, render_hooks_status, uninstall_hooks, HOOK_BUDGET_SECS, HOOK_MANAGED_MARKER,
};
pub use inspect::{inspect_scope_allowed, plan_inspect, plan_somepath, InspectPlan};
pub use migrate::{migrate_is_major_bump, migrate_manifest_name, plan_migrate, MigratePlan};
pub use scaffold::{
    absent_only_write_allowed, apply_init, init_must_refuse, plan_init_files, ScaffoldFile,
    DEVCONTAINER_JSON, RENOVATE_JSON,
};
pub use status::{default_status_checks, render_status_json, render_status_text, StatusCheck};
pub use version::{
    read_version_pin, rollback_re_pins_previous, version_pin_matches_module, write_version_pin,
    DX_VERSION, MODULE_VERSION, PREVIOUS_VERSION,
};
pub use watch::{
    coalesce_watch_paths, plan_watch, watch_for_change, watch_iteration_accepts,
    WATCHABLE_COMMANDS, WATCH_DEBOUNCE_MS,
};

/// Typed adoption failure (issue #221 pilot).
///
/// Every variant renders byte-identical to the historical `String` error
/// it replaces, so CLI operational diagnostics stay stable while callers
/// gain matchable structure instead of `format!` string plumbing.
/// Binary edges (`dx` mains) keep rendering via `Display` (`to_string()`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AdoptError {
    /// Failed to create a parent directory for scaffolding output.
    #[error("create parent {parent}: {detail}")]
    CreateParent { parent: String, detail: String },
    /// Failed to write a scaffolded or pin file.
    #[error("write {path}: {detail}")]
    WriteFile { path: String, detail: String },
    /// Failed to create `.git/hooks`.
    #[error("create hooks dir: {detail}")]
    CreateHooksDir { detail: String },
    /// Failed to read an existing hook shim.
    #[error("read hook {trigger}: {detail}")]
    ReadHook { trigger: String, detail: String },
    /// Existing unmanaged hook refuses install.
    #[error("unmanaged hook refuses install: {trigger}")]
    UnmanagedInstall { trigger: String },
    /// Failed to write a hook shim.
    #[error("write hook {trigger}: {detail}")]
    WriteHook { trigger: String, detail: String },
    /// Failed to stat a hook shim for chmod.
    #[error("stat hook {trigger}: {detail}")]
    StatHook { trigger: String, detail: String },
    /// Failed to chmod a hook shim.
    #[error("chmod hook {trigger}: {detail}")]
    ChmodHook { trigger: String, detail: String },
    /// Failed to write the `dx.local.toml` overlay.
    #[error("write overlay: {detail}")]
    WriteOverlay { detail: String },
    /// Existing unmanaged hook refuses uninstall.
    #[error("unmanaged hook refuses uninstall: {trigger}")]
    UnmanagedUninstall { trigger: String },
    /// Failed to remove a hook shim.
    #[error("remove hook {trigger}: {detail}")]
    RemoveHook { trigger: String, detail: String },
    /// Failed to read the `.dx/version` pin.
    #[error("read version pin: {detail}")]
    ReadVersionPin { detail: String },
    /// Refused an empty version pin write.
    #[error("refuses empty version")]
    EmptyVersion,
    /// Failed to create `.dx`.
    #[error("create .dx: {detail}")]
    CreateDxDir { detail: String },
    /// Failed to write the version pin.
    #[error("write version pin: {detail}")]
    WriteVersionPin { detail: String },
    /// `dx watch` refuses CI (local-only).
    #[error("dx watch refuses CI (local-only)")]
    WatchRefusesCi,
    /// Command is not watchable.
    #[error("not watchable: {command}")]
    NotWatchable { command: String },
    /// Inspect scope rejected.
    #[error("rejected scope: {scope}")]
    RejectedScope { scope: String },
    /// `dx why` called without the file-owner resolution leg.
    #[error("dx why needs <file> <label>: resolve the file owner first, then plan_somepath")]
    WhyNeedsOwner,
    /// Unknown inspect kind.
    #[error("unknown inspect: {kind}")]
    UnknownInspect { kind: String },
    /// Unknown completion shell.
    #[error("unknown-shell: {shell}")]
    UnknownShell { shell: String },
    /// Failed to spawn the filesystem watcher.
    #[error("watch spawn failed: {detail}")]
    WatchSpawn { detail: String },
    /// Filesystem watcher reported errors.
    #[error("watch failed: {detail}")]
    WatchFailed { detail: String },
    /// `dx migrate` requires distinct valid semver versions.
    #[error("migrate needs distinct versions: {detail}")]
    MigrateVersions { detail: String },
    /// `dx migrate` is major-release-only.
    #[error("migrate is major-release-only: {from} -> {to}")]
    MigrateNotMajor { from: String, to: String },
}

// Hook per-check budget seconds (O49 freeze: blocking timeout) lives in
// the `hooks` module (issue #236); the re-exports above keep
// `HOOK_BUDGET_SECS` on the `dx_adopt` facade.

// Commands watchable under ADR 0017/0018 live in the `watch` module
// (issue #236); the re-exports above keep `WATCHABLE_COMMANDS` and
// `WATCH_DEBOUNCE_MS` on the `dx_adopt` facade.

// Absent-only `dx init` scaffolding lives in the `scaffold` module
// (issue #236); the re-exports above keep `absent_only_write_allowed`,
// `init_must_refuse`, `ScaffoldFile`, `DEVCONTAINER_JSON`,
// `RENOVATE_JSON`, `plan_init_files`, and `apply_init` on the
// `dx_adopt` facade.

// Hermetic hook runner (`hook_git_is_hermetic`,
// `hook_shim_overwrite_allowed`, `hook_status_shows_merged`) lives in the
// `hooks` module (issue #236); the re-exports above keep those paths on
// the `dx_adopt` facade.

/// Whether a devcontainer definition is admissible.
///
/// Setup uses only pinned artifacts and every tool execution delegates to
/// Bazel actions: pinned bootstrap plus Bazel delegation with no ambient
/// tools. Any ambient tool use fails the gate.
pub fn devcontainer_is_admissible(
    pinned_bootstrap: bool,
    delegates_to_bazel: bool,
    uses_ambient_tools: bool,
) -> bool {
    pinned_bootstrap && delegates_to_bazel && !uses_ambient_tools
}

/// Whether a diagnostics command name is admissible.
///
/// Per ADR 0006 there is no `dx doctor`: that name is rejected outright and
/// the consolidated status surface (O50) must ship under another name. The
/// empty name is rejected as well; vocabulary and shape stay O50-gated.
pub fn diagnostics_command_allowed(name: &str) -> bool {
    !name.is_empty() && name != "doctor"
}

// ---------------------------------------------------------------------------
// Delivered I/O (M30b WPs 2-4, 6-7 + O61).
// ---------------------------------------------------------------------------

// Absent-only `dx init` scaffolding (`ScaffoldFile`, `DEVCONTAINER_JSON`,
// `RENOVATE_JSON`, `plan_init_files`, `apply_init`) lives in the
// `scaffold` module (issue #236); the re-exports above keep those paths
// on the `dx_adopt` facade.

// Hermetic hook I/O (`HOOK_MANAGED_MARKER`, `render_hook_shim`,
// `install_hooks`, `uninstall_hooks`, `render_hooks_status`) lives in the
// `hooks` module (issue #236); the re-exports above keep those paths on
// the `dx_adopt` facade.

// The consolidated `dx status` surface lives in the `status` module
// (issue #236); the re-exports above keep `StatusCheck`,
// `render_status_text`, `render_status_json`, and
// `default_status_checks` on the `dx_adopt` facade.

/// Completion vocabulary lives in the `completion` module (issue #236):
/// production rendering uses the `Cli` grammar, the tables there remain
/// the O61 frozen vocabulary reference only.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devcontainer_needs_pins_and_bazel_delegation() {
        assert!(devcontainer_is_admissible(true, true, false));
        assert!(!devcontainer_is_admissible(false, true, false));
        assert!(!devcontainer_is_admissible(true, false, false));
        assert!(!devcontainer_is_admissible(true, true, true));
    }

    #[test]
    fn doctor_stays_rejected_for_diagnostics() {
        assert!(diagnostics_command_allowed("status"));
        assert!(diagnostics_command_allowed("env"));
        assert!(!diagnostics_command_allowed("doctor"));
        assert!(!diagnostics_command_allowed(""));
    }

    #[test]
    fn watch_errors_render_stably() {
        assert_eq!(
            AdoptError::WatchSpawn {
                detail: "denied".to_owned()
            }
            .to_string(),
            "watch spawn failed: denied"
        );
        assert_eq!(
            AdoptError::WatchFailed {
                detail: "boom".to_owned()
            }
            .to_string(),
            "watch failed: boom"
        );
    }

    #[test]
    fn adopt_errors_render_byte_identical_to_legacy_strings() {
        // Pilot gate for #221: typed errors must preserve the historical
        // user-facing strings so CLI operational diagnostics stay stable.
        assert_eq!(
            AdoptError::WatchRefusesCi.to_string(),
            "dx watch refuses CI (local-only)"
        );
        assert_eq!(
            AdoptError::NotWatchable {
                command: "docs".to_owned()
            }
            .to_string(),
            "not watchable: docs"
        );
        assert_eq!(
            AdoptError::RejectedScope {
                scope: "//a:one".to_owned()
            }
            .to_string(),
            "rejected scope: //a:one"
        );
        assert_eq!(
            AdoptError::UnknownShell {
                shell: "tcsh".to_owned()
            }
            .to_string(),
            "unknown-shell: tcsh"
        );
        assert_eq!(
            AdoptError::UnknownInspect {
                kind: "bogus".to_owned()
            }
            .to_string(),
            "unknown inspect: bogus"
        );
        assert_eq!(
            AdoptError::WhyNeedsOwner.to_string(),
            "dx why needs <file> <label>: resolve the file owner first, then plan_somepath"
        );
        assert_eq!(
            AdoptError::EmptyVersion.to_string(),
            "refuses empty version"
        );
        assert_eq!(
            AdoptError::UnmanagedInstall {
                trigger: "pre-commit".to_owned()
            }
            .to_string(),
            "unmanaged hook refuses install: pre-commit"
        );
        assert_eq!(
            AdoptError::ReadVersionPin {
                detail: "denied".to_owned()
            }
            .to_string(),
            "read version pin: denied"
        );
        // Call sites surface the typed errors through `Display`.
        assert_eq!(
            plan_watch("docs", false).unwrap_err().to_string(),
            "not watchable: docs"
        );
        assert_eq!(
            AdoptError::UnknownShell {
                shell: "tcsh".to_owned()
            }
            .to_string(),
            "unknown-shell: tcsh"
        );
    }
}
