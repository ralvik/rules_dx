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
//! module, thin inspect forwarding lives in the `inspect` module, and
//! the local watch loop lives in the `watch` module. This facade keeps
//! the re-exports; the public path stays stable via the re-exports below.

// Issue #238: infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::path::Path;

pub mod completion;
pub mod inspect;
pub mod migrate;
pub mod status;
pub mod version;
pub mod watch;

pub use completion::{completion_source_is_single, ALL_COMMANDS, SUPPORTED_SHELLS};
pub use inspect::{inspect_scope_allowed, plan_inspect, plan_somepath, InspectPlan};
pub use migrate::{migrate_is_major_bump, migrate_manifest_name, plan_migrate, MigratePlan};
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

/// Hook per-check budget seconds (O49 freeze: blocking timeout).
pub const HOOK_BUDGET_SECS: u64 = 120;

// Commands watchable under ADR 0017/0018 live in the `watch` module
// (issue #236); the re-exports above keep `WATCHABLE_COMMANDS` and
// `WATCH_DEBOUNCE_MS` on the `dx_adopt` facade.

/// Whether `dx init` may write one scaffolded file.
///
/// Init writes absent-only: a missing path may be written, an existing path
/// is left untouched (tracked files are never mutated). The caller checks
/// each path independently.
pub fn absent_only_write_allowed(target_exists: bool) -> bool {
    !target_exists
}

/// Whether `dx init` must refuse one scaffolded path.
///
/// An existing file is refused even with force: an unmanaged `.envrc` or any
/// other present file is never overwritten by scaffolding. Absent paths go
/// through [`absent_only_write_allowed`].
pub fn init_must_refuse(target_exists: bool, _force: bool) -> bool {
    target_exists
}

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

/// One scaffolded file planned by [`plan_init_files`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScaffoldFile {
    /// Workspace-relative path.
    pub path: String,
    /// Exact bytes to write when absent.
    pub content: String,
}

/// `dx init` devcontainer definition (issue #183).
///
/// Single source for the scaffolded `.devcontainer/devcontainer.json`:
/// the repository's own `.devcontainer/devcontainer.json` is held
/// byte-identical to this string by
/// `//.devcontainer:devcontainer_parity_test`, so the definition we ship
/// is the one we boot. `postCreateCommand` runs the user-path bootstrap
/// (`bazel run //dx:env`, then `dx setup`) instead of a full build, so
/// container create pays only the managed-environment setup.
pub const DEVCONTAINER_JSON: &str = concat!(
    "{\n",
    "  \"name\": \"rules_dx\",\n",
    "  \"image\": \"mcr.microsoft.com/devcontainers/base:ubuntu\",\n",
    "  \"features\": {\n",
    "    \"ghcr.io/devcontainers/features/bazel:1\": {}\n",
    "  },\n",
    "  \"customizations\": {\n",
    "    \"vscode\": {\n",
    "      \"extensions\": [\"rust-lang.rust-analyzer\"]\n",
    "    }\n",
    "  },\n",
    "  \"postCreateCommand\": \"bazel run //dx:env && bazel run //cli/cli:dx -- setup\"\n",
    "}\n",
);

/// `dx init` Renovate definition (issue #3).
///
/// Single source for the scaffolded `renovate.json`: the repository's
/// own `renovate.json` is held byte-identical to this string by
/// `//:renovate_parity_test`, so the config we ship is the one we run.
/// Full manager set from the start (owner-confirmed): `bazel` over the
/// `.bazelversion` pin surface plus Cargo, npm/pnpm, GitHub Actions,
/// and Go — grouped, scheduled weekly, reviewable PRs. Auto-merge is
/// off by default (opt-in); when enabled it is update-only under the
/// automation guardrails (`docs/contributing/automation.md`): green
/// required checks, patch/minor preferred, no bot push to `main`
/// outside the merge path, publication gate intact (issue #5).
/// Renovate proposes pins; `dx update` (issue #19) applies/verifies
/// with resolver-owned continuation/reporting — they complement,
/// not replace.
pub const RENOVATE_JSON: &str = concat!(
    "{\n",
    "  \"$schema\": \"https://docs.renovatebot.com/renovate-schema.json\",\n",
    "  \"extends\": [\"config:recommended\"],\n",
    "  \"schedule\": [\"before 5am on Monday\"],\n",
    "  \"labels\": [\"dependencies\"],\n",
    "  \"enabledManagers\": [\"bazel\", \"cargo\", \"github-actions\", \"gomod\", \"npm\"],\n",
    "  \"packageRules\": [\n",
    "    {\n",
    "      \"description\": \"Bazel pin surface (.bazelversion + MODULE.bazel direct pins); regen + flag-diff review per preset loop\",\n",
    "      \"matchManagers\": [\"bazel\"],\n",
    "      \"groupName\": \"bazel surface\"\n",
    "    },\n",
    "    {\n",
    "      \"description\": \"Rust dependencies\",\n",
    "      \"matchManagers\": [\"cargo\"],\n",
    "      \"groupName\": \"rust dependencies\"\n",
    "    },\n",
    "    {\n",
    "      \"description\": \"JavaScript/TypeScript dependencies (npm/pnpm)\",\n",
    "      \"matchManagers\": [\"npm\"],\n",
    "      \"groupName\": \"js dependencies\"\n",
    "    },\n",
    "    {\n",
    "      \"description\": \"GitHub Actions\",\n",
    "      \"matchManagers\": [\"github-actions\"],\n",
    "      \"groupName\": \"github actions\"\n",
    "    },\n",
    "    {\n",
    "      \"description\": \"Go dependencies\",\n",
    "      \"matchManagers\": [\"gomod\"],\n",
    "      \"groupName\": \"go dependencies\"\n",
    "    }\n",
    "  ],\n",
    "  \"automerge\": false,\n",
    "  \"platformAutomerge\": false,\n",
    "  \"prCreation\": \"not-pending\",\n",
    "  \"dependencyDashboard\": true\n",
    "}\n",
);

/// Plan the `dx init` scaffold for a module name.
///
/// All writes are absent-only; the caller refuses existing paths even with
/// force (see [`init_must_refuse`]). Contents are pinned (no network) and
/// point editors at `.dx` projections, checked-in native configs, and
/// managed `.dx/bin` tools.
pub fn plan_init_files(module_name: &str) -> Vec<ScaffoldFile> {
    let module = if module_name.is_empty() {
        "my_project"
    } else {
        module_name
    };
    vec![
        ScaffoldFile {
            path: ".dx/version".to_owned(),
            content: format!("{DX_VERSION}\n"),
        },
        ScaffoldFile {
            path: "dx.local.toml".to_owned(),
            content: "# Local-only overrides (gitignored). See dx hooks status.\n[hooks]\n".to_owned(),
        },
        ScaffoldFile {
            path: "dx.hooks.toml".to_owned(),
            content: "[hooks]\npre_commit = [\"format --check\", \"lint --check\"]\npre_push = [\"typecheck --check\", \"generate --check\"]\nbudget_secs = 120\n"
                .to_owned(),
        },
        ScaffoldFile {
            path: ".devcontainer/devcontainer.json".to_owned(),
            content: DEVCONTAINER_JSON.to_owned(),
        },
        ScaffoldFile {
            path: "renovate.json".to_owned(),
            content: RENOVATE_JSON.to_owned(),
        },
        ScaffoldFile {
            path: ".vscode/settings.json".to_owned(),
            content: "{\"rust-analyzer.check.command\":\"bazel\",\"python.defaultInterpreterPath\":\".dx/setups/current/.venv/bin/python\",\"typescript.tsdk\":\".dx/setups/current/node_modules/typescript/lib\",\"go.toolsManagement.checkForUpdates\":\"off\"}\n"
                .to_owned(),
        },
        ScaffoldFile {
            path: ".vscode/extensions.json".to_owned(),
            content: "{\"recommendations\":[\"rust-lang.rust-analyzer\",\"ms-python.python\",\"bradlc.vscode-tailwindcss\"]}\n"
                .to_owned(),
        },
        ScaffoldFile {
            path: ".github/workflows/ci.yml".to_owned(),
            content: format!(
                "# Caller template: pins the qualified reusable workflow at a reviewed commit.\nname: ci\non:\n  push: {{}}\n  pull_request: {{}}\njobs:\n  dx:\n    uses: {module}/.github/workflows/reusable-consumer.yml@<reviewed-commit>\n"
            ),
        },
        ScaffoldFile {
            path: "MODULE.bazel.snippet".to_owned(),
            content: format!(
                "# Add to MODULE.bazel:\nbazel_dep(name = \"rules_dx\", version = \"{DX_VERSION}\")\n# module: {module}\n"
            ),
        },
    ]
}

/// Apply the init scaffold under `root`, writing absent-only.
///
/// Returns the written workspace-relative paths. Existing files are left
/// untouched and reported as refusals in the returned `Vec` prefix
/// `refused:` entries follow written entries after a `---` separator.
pub fn apply_init(root: &Path, module_name: &str) -> Result<Vec<String>, AdoptError> {
    let mut written = Vec::new();
    let mut refused = Vec::new();
    for file in plan_init_files(module_name) {
        let dest = root.join(&file.path);
        if dest.exists() {
            refused.push(format!("refused:{}", file.path));
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AdoptError::CreateParent {
                parent: parent.display().to_string(),
                detail: e.to_string(),
            })?;
        }
        dx_atomic_fs::write_atomic(&dest, file.content.as_ref()).map_err(|e| {
            AdoptError::WriteFile {
                path: dest.display().to_string(),
                detail: e.to_string(),
            }
        })?;
        written.push(file.path);
    }
    written.push("---".to_owned());
    written.extend(refused);
    Ok(written)
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
    fn init_writes_only_absent_paths() {
        assert!(absent_only_write_allowed(false));
        assert!(!absent_only_write_allowed(true));
    }

    #[test]
    fn init_refuses_existing_paths_even_with_force() {
        assert!(init_must_refuse(true, false));
        assert!(init_must_refuse(true, true));
        assert!(!init_must_refuse(false, false));
        assert!(!init_must_refuse(false, true));
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
    fn init_plans_nine_absent_only_files() {
        let files = plan_init_files("demo");
        assert_eq!(files.len(), 9);
        assert!(files.iter().any(|f| f.path == ".dx/version"));
        assert!(files
            .iter()
            .any(|f| f.path == ".devcontainer/devcontainer.json"));
        assert!(files.iter().any(|f| f.path == "renovate.json"));
        assert!(files.iter().any(|f| f.path == ".vscode/settings.json"));
    }

    #[test]
    fn devcontainer_scaffold_runs_bootstrap_not_full_build() {
        // Issue #183: the scaffolded definition must stay admissible and
        // bootstrap-shaped. The byte parity with the checked-in definition
        // lives in //.devcontainer:devcontainer_parity_test; this pins the
        // contract fields here so scaffold drift fails at the source.
        let files = plan_init_files("demo");
        let scaffold = files
            .iter()
            .find(|f| f.path == ".devcontainer/devcontainer.json")
            .expect("devcontainer scaffold");
        assert_eq!(scaffold.content, DEVCONTAINER_JSON);
        let parsed: serde_json::Value =
            serde_json::from_str(&scaffold.content).expect("valid JSON");
        assert_eq!(parsed["name"], serde_json::Value::from("rules_dx"));
        assert_eq!(
            parsed["image"],
            serde_json::Value::from("mcr.microsoft.com/devcontainers/base:ubuntu")
        );
        let post_create = parsed["postCreateCommand"]
            .as_str()
            .expect("postCreateCommand string");
        assert!(
            post_create.contains("bazel run //dx:env"),
            "bootstrap first: {post_create}"
        );
        assert!(
            !post_create.contains("bazel build //..."),
            "no full build on create: {post_create}"
        );
        assert!(devcontainer_is_admissible(true, true, false));
    }

    #[test]
    fn renovate_scaffold_ships_full_manager_set_automerge_off() {
        // Issue #3: `dx init` ships `renovate.json` absent-only with the
        // full manager set from the start, grouped, scheduled, reviewable.
        // Byte parity with the checked-in definition lives in
        // `//:renovate_parity_test`; this pins the contract fields here
        // so scaffold drift fails at the source.
        let files = plan_init_files("demo");
        let scaffold = files
            .iter()
            .find(|f| f.path == "renovate.json")
            .expect("renovate scaffold");
        assert_eq!(scaffold.content, RENOVATE_JSON);
        let parsed: serde_json::Value =
            serde_json::from_str(&scaffold.content).expect("valid JSON");
        // Full manager set: bazel over the .bazelversion pin surface plus
        // Cargo, npm/pnpm, GitHub Actions, Go.
        let managers = parsed["enabledManagers"]
            .as_array()
            .expect("enabledManagers array")
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>();
        for want in ["bazel", "cargo", "npm", "github-actions", "gomod"] {
            assert!(
                managers.contains(&want),
                "missing manager {want}: {managers:?}"
            );
        }
        // Grouped per manager.
        let rules = parsed["packageRules"]
            .as_array()
            .expect("packageRules array");
        assert!(rules
            .iter()
            .any(|r| r["matchManagers"] == serde_json::json!(["bazel"])));
        // Scheduled weekly, reviewable PRs.
        assert!(parsed["schedule"].as_array().is_some_and(|s| !s.is_empty()));
        assert_eq!(parsed["dependencyDashboard"], serde_json::Value::from(true));
        // Auto-merge off by default (opt-in); when enabled it is
        // update-only per docs/contributing/automation.md.
        assert_eq!(parsed["automerge"], serde_json::Value::from(false));
        assert_eq!(parsed["platformAutomerge"], serde_json::Value::from(false));
    }

    #[test]
    fn apply_init_writes_absent_only_and_refuses_existing() {
        let scratch = dx_test_scratch::scratch("dx-adopt-init-");
        let root = scratch.path().to_path_buf();
        std::fs::create_dir_all(&root).expect("tmp");
        let first = apply_init(&root, "demo").expect("init");
        assert!(first.iter().any(|p| p == ".dx/version"));
        assert!(root.join(".dx/version").exists());
        std::fs::write(root.join(".dx/version"), "custom\n").expect("custom");
        let second = apply_init(&root, "demo").expect("init again");
        assert!(second.iter().any(|p| p == "refused:.dx/version"));
        assert_eq!(
            std::fs::read_to_string(root.join(".dx/version")).expect("read"),
            "custom\n"
        );
        scratch.close().expect("cleanup");
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
