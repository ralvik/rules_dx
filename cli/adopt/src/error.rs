//! Typed adoption failure (pilot, split).
//!
//! Split from `super` (`lib.rs`): owns [`AdoptError`]. Re-exported
//! through `super` so the public path stays
//! `dx_adopt::AdoptError`.
//!
//! Every variant renders byte-identical to the historical `String` error
//! it replaces, so CLI operational diagnostics stay stable while callers
//! gain matchable structure instead of `format!` string plumbing.
//! Binary edges (`dx` mains) keep rendering via `Display` (`to_string()`).

/// Typed adoption failure (pilot).
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

#[cfg(test)]
mod tests {
    use super::super::{plan_watch, AdoptError as FacadeError};
    use super::AdoptError;

    #[test]
    fn error_reexports_match_local_definitions() {
        assert_eq!(FacadeError::EmptyVersion, AdoptError::EmptyVersion);
        assert_eq!(
            FacadeError::WatchRefusesCi.to_string(),
            AdoptError::WatchRefusesCi.to_string()
        );
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
