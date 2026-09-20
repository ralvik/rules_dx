//! Hermetic hook runner and merged status view.
//!
//! Split from `super` (`lib.rs`): owns `HOOK_BUDGET_SECS`,
//! `hook_git_is_hermetic`, `hook_shim_overwrite_allowed`,
//! `hook_status_shows_merged`, `HOOK_MANAGED_MARKER`,
//! `render_hook_shim`, `install_hooks`, `uninstall_hooks`, and
//! `render_hooks_status`. Re-exported through `super` so the public
//! path stays `dx_adopt::{HOOK_BUDGET_SECS, hook_git_is_hermetic,
//! hook_shim_overwrite_allowed, hook_status_shows_merged,
//! HOOK_MANAGED_MARKER, render_hook_shim, install_hooks,
//! uninstall_hooks, render_hooks_status}`.

use std::path::Path;

use super::AdoptError;

/// Hook per-check budget seconds (frozen: blocking timeout).
pub const HOOK_BUDGET_SECS: u64 = 120;

/// Whether hook Git sourcing is hermetic.
///
/// Hook Git operations use hermetically acquired Git only, never ambient
/// Git. Any ambient fallback fails the gate; this compares injected flags
/// and acquires nothing.
pub fn hook_git_is_hermetic(uses_hermetic_git: bool, uses_ambient_git: bool) -> bool {
    uses_hermetic_git && !uses_ambient_git
}

/// Whether installing a managed hook shim may overwrite the existing file.
///
/// Unmanaged hooks cannot be overwritten even with force; only a managed
/// shim may be refreshed. The force flag never authorizes overwriting an
/// unmanaged hook.
pub fn hook_shim_overwrite_allowed(existing_managed: bool, _force: bool) -> bool {
    existing_managed
}

/// Whether `dx hooks status` shows the effective merged configuration.
///
/// Status must show the committed baseline, the personal overlay, and
/// per-check timings together. Any missing layer fails the gate.
pub fn hook_status_shows_merged(
    shows_baseline: bool,
    shows_overlay: bool,
    shows_timings: bool,
) -> bool {
    shows_baseline && shows_overlay && shows_timings
}

/// Managed hook-shim marker.
pub const HOOK_MANAGED_MARKER: &str = "# managed by dx hooks";

/// Render one hook shim for `trigger`.
pub fn render_hook_shim(trigger: &str) -> String {
    format!(
        "#!/bin/sh\n{HOOK_MANAGED_MARKER} {trigger}\nexec bazel run //cli/cli:dx -- hooks run {trigger} -- \"$@\"\n"
    )
}

/// Install `pre-commit` + `pre-push` shims under `root/.git/hooks`.
///
/// Refuses unmanaged existing hooks even with force. Bootstraps the
/// gitignored `dx.local.toml` overlay absent-only. Returns installed paths.
pub fn install_hooks(root: &Path) -> Result<Vec<String>, AdoptError> {
    let hooks_dir = root.join(".git/hooks");
    std::fs::create_dir_all(&hooks_dir).map_err(|e| AdoptError::CreateHooksDir {
        detail: e.to_string(),
    })?;
    let mut installed = Vec::new();
    for trigger in ["pre-commit", "pre-push"] {
        let dest = hooks_dir.join(trigger);
        if dest.exists() {
            let existing = std::fs::read_to_string(&dest).map_err(|e| AdoptError::ReadHook {
                trigger: trigger.to_owned(),
                detail: e.to_string(),
            })?;
            if !existing.contains(HOOK_MANAGED_MARKER) {
                return Err(AdoptError::UnmanagedInstall {
                    trigger: trigger.to_owned(),
                });
            }
        }
        dx_atomic_fs::write_atomic(&dest, render_hook_shim(trigger).as_bytes()).map_err(|e| {
            AdoptError::WriteHook {
                trigger: trigger.to_owned(),
                detail: e.to_string(),
            }
        })?;
        // Issue #320 portable route: hook shims need the executable bit
        // only on unix; Windows runs them through the shell association,
        // so non-unix skips chmod instead of failing.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&dest)
                .map_err(|e| AdoptError::StatHook {
                    trigger: trigger.to_owned(),
                    detail: e.to_string(),
                })?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&dest, perms).map_err(|e| AdoptError::ChmodHook {
                trigger: trigger.to_owned(),
                detail: e.to_string(),
            })?;
        }
        installed.push(format!(".git/hooks/{trigger}"));
    }
    let overlay = root.join("dx.local.toml");
    if !overlay.exists() {
        dx_atomic_fs::write_atomic(&overlay, b"# Local-only overrides (gitignored).\n[hooks]\n")
            .map_err(|e| AdoptError::WriteOverlay {
                detail: e.to_string(),
            })?;
        installed.push("dx.local.toml".to_owned());
    }
    Ok(installed)
}

/// Remove only managed shims; unmanaged files are never touched.
pub fn uninstall_hooks(root: &Path) -> Result<Vec<String>, AdoptError> {
    let mut removed = Vec::new();
    for trigger in ["pre-commit", "pre-push"] {
        let dest = root.join(".git/hooks").join(trigger);
        if !dest.exists() {
            continue;
        }
        let existing = std::fs::read_to_string(&dest).map_err(|e| AdoptError::ReadHook {
            trigger: trigger.to_owned(),
            detail: e.to_string(),
        })?;
        if !existing.contains(HOOK_MANAGED_MARKER) {
            return Err(AdoptError::UnmanagedUninstall {
                trigger: trigger.to_owned(),
            });
        }
        std::fs::remove_file(&dest).map_err(|e| AdoptError::RemoveHook {
            trigger: trigger.to_owned(),
            detail: e.to_string(),
        })?;
        removed.push(format!(".git/hooks/{trigger}"));
    }
    Ok(removed)
}

/// Render the merged `dx hooks status` view.
pub fn render_hooks_status(baseline: &str, overlay: &str, timings: &str) -> String {
    format!("baseline:\n{baseline}\noverlay:\n{overlay}\ntimings:\n{timings}\n")
}

#[cfg(test)]
mod tests {
    use super::super::{
        hook_git_is_hermetic, hook_shim_overwrite_allowed, hook_status_shows_merged, install_hooks,
        uninstall_hooks, HOOK_BUDGET_SECS, HOOK_MANAGED_MARKER,
    };
    use super::{render_hook_shim, render_hooks_status};

    #[test]
    fn hooks_reexports_match_local_definitions() {
        assert_eq!(super::HOOK_BUDGET_SECS, HOOK_BUDGET_SECS);
        assert_eq!(super::HOOK_MANAGED_MARKER, HOOK_MANAGED_MARKER);
        assert!(render_hook_shim("pre-commit").contains(HOOK_MANAGED_MARKER));
        assert!(render_hooks_status("b", "o", "t").contains("baseline:\nb"));
    }

    #[test]
    fn hook_git_never_falls_back_to_ambient() {
        assert!(hook_git_is_hermetic(true, false));
        assert!(!hook_git_is_hermetic(true, true));
        assert!(!hook_git_is_hermetic(false, false));
        assert!(!hook_git_is_hermetic(false, true));
    }

    #[test]
    fn unmanaged_hooks_survive_force() {
        assert!(hook_shim_overwrite_allowed(true, false));
        assert!(hook_shim_overwrite_allowed(true, true));
        assert!(!hook_shim_overwrite_allowed(false, false));
        assert!(!hook_shim_overwrite_allowed(false, true));
    }

    #[test]
    fn hook_status_shows_baseline_overlay_and_timings() {
        assert!(hook_status_shows_merged(true, true, true));
        assert!(!hook_status_shows_merged(false, true, true));
        assert!(!hook_status_shows_merged(true, false, true));
        assert!(!hook_status_shows_merged(true, true, false));
    }

    #[test]
    fn hook_budget_freeze_holds() {
        assert_eq!(HOOK_BUDGET_SECS, 120);
    }

    #[test]
    fn hooks_install_refuses_unmanaged_and_manages_shims() {
        let scratch = dx_test_scratch::scratch("dx-adopt-hook-");
        let root = scratch.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git/hooks")).expect("tmp");
        let installed = install_hooks(&root).expect("install");
        assert!(installed.iter().any(|p| p == ".git/hooks/pre-commit"));
        std::fs::write(root.join(".git/hooks/pre-commit"), "# custom hook\n").expect("unmanaged");
        assert!(install_hooks(&root).is_err());
        scratch.close().expect("cleanup");
    }

    #[test]
    fn hooks_uninstall_removes_only_managed_shims() {
        let scratch = dx_test_scratch::scratch("dx-adopt-hook-uninstall-");
        let root = scratch.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git/hooks")).expect("tmp");
        install_hooks(&root).expect("install");
        let removed = uninstall_hooks(&root).expect("uninstall");
        assert!(removed.iter().any(|p| p == ".git/hooks/pre-commit"));
        assert!(!root.join(".git/hooks/pre-commit").exists());
        // Second uninstall is a no-op over absent shims.
        let again = uninstall_hooks(&root).expect("uninstall again");
        assert!(again.is_empty());
        // Unmanaged files are never touched.
        std::fs::write(root.join(".git/hooks/pre-commit"), "# custom hook\n").expect("unmanaged");
        assert!(uninstall_hooks(&root).is_err());
        assert!(root.join(".git/hooks/pre-commit").exists());
        scratch.close().expect("cleanup");
    }
}
