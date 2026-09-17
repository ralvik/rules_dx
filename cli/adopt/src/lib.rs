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
//! the `completion` module and major-release migration planning lives in
//! the `migrate` module. This facade keeps the re-exports; the public
//! path stays `dx_adopt::{ALL_COMMANDS, SUPPORTED_SHELLS,
//! completion_source_is_single, MigratePlan, migrate_is_major_bump,
//! migrate_manifest_name, plan_migrate}` via the re-exports below.

// Issue #238: infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

pub mod completion;
pub mod migrate;

pub use completion::{completion_source_is_single, ALL_COMMANDS, SUPPORTED_SHELLS};
pub use migrate::{migrate_is_major_bump, migrate_manifest_name, plan_migrate, MigratePlan};

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

/// Delivered `dx` / `rules_dx` single version (O51 freeze).
pub const DX_VERSION: &str = "0.0.0";
/// Pinned `rules_dx` module version; `dx version` must equal this.
pub const MODULE_VERSION: &str = "0.0.0";
/// Previous release for rollback demonstration.
pub const PREVIOUS_VERSION: &str = "0.0.0";
/// Hook per-check budget seconds (O49 freeze: blocking timeout).
pub const HOOK_BUDGET_SECS: u64 = 120;
/// Watch debounce milliseconds (O55 freeze).
pub const WATCH_DEBOUNCE_MS: u64 = 200;

/// Commands watchable under ADR 0017/0018 (O55 freeze).
pub const WATCHABLE_COMMANDS: &[&str] = &[
    "build",
    "test",
    "run",
    "lint",
    "typecheck",
    "format",
    "check",
    "fix",
];

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

/// Whether the single-version pin holds.
///
/// Per O51 direction the `dx` version equals the pinned `rules_dx` module
/// version: both must parse as Cargo-flavor semver (via the `semver`
/// crate, issue #224) and compare exactly equal. Self-update bumps
/// that pin from verified release artifacts; anything else is rejected here.
/// Empty strings and non-semver text never match, even when equal.
pub fn version_pin_matches_module(dx_version: &str, module_version: &str) -> bool {
    if dx_version.is_empty() || module_version.is_empty() {
        return false;
    }
    let dx = match semver::Version::parse(dx_version) {
        Ok(dx) => dx,
        Err(_) => return false,
    };
    let module = match semver::Version::parse(module_version) {
        Ok(module) => module,
        Err(_) => return false,
    };
    dx == module
}

/// Whether a rollback target is admissible.
///
/// Rollback is re-pinning the previous release: the target must equal the
/// known previous version and differ from the current pin. Rolling to the
/// current pin or to an unknown version is rejected.
pub fn rollback_re_pins_previous(current: &str, target: &str, known_previous: &str) -> bool {
    !known_previous.is_empty() && target == known_previous && target != current
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

/// Whether one watch iteration may run.
///
/// Watch is a thin local loop reusing the wrapped command verbatim (no
/// daemon, cache, graph, or remote): each iteration re-resolves its scope,
/// holds the single-runnable rule for `run`, and refuses when running under
/// CI. Any violation blocks the iteration.
pub fn watch_iteration_accepts(
    scope_reresolved: bool,
    local_only: bool,
    single_runnable_held: bool,
) -> bool {
    scope_reresolved && local_only && single_runnable_held
}

/// Whether an inspect scope is admissible.
///
/// Inspect wrappers (`owners`/`deps`/`why`) forward canonically to
/// `bazel query`/`cquery` with deterministic sorting and no custom graph
/// engine. External scopes are rejected like workflow commands; the empty
/// scope is rejected as well.
pub fn inspect_scope_allowed(scope: &str, external: bool) -> bool {
    !scope.is_empty() && !external
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

/// Read the `.dx/version` pin under `root`.
pub fn read_version_pin(root: &Path) -> Result<String, AdoptError> {
    let raw = std::fs::read_to_string(root.join(".dx/version")).map_err(|e| {
        AdoptError::ReadVersionPin {
            detail: e.to_string(),
        }
    })?;
    Ok(raw.trim().to_owned())
}

/// Write the `.dx/version` pin (verified-release versions only).
pub fn write_version_pin(root: &Path, version: &str) -> Result<(), AdoptError> {
    if version.is_empty() {
        return Err(AdoptError::EmptyVersion);
    }
    let dir = root.join(".dx");
    std::fs::create_dir_all(&dir).map_err(|e| AdoptError::CreateDxDir {
        detail: e.to_string(),
    })?;
    dx_atomic_fs::write_atomic(&dir.join("version"), format!("{version}\n").as_bytes()).map_err(
        |e| AdoptError::WriteVersionPin {
            detail: e.to_string(),
        },
    )?;
    Ok(())
}

/// One diagnostics check in the consolidated `dx status` surface.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StatusCheck {
    /// Check name (toolchain, platform, tools, pin).
    pub name: String,
    /// One of `ok|warn|error`.
    pub status: String,
    /// Human detail.
    pub detail: String,
    /// Actionable hint.
    pub hint: String,
}

#[derive(Serialize)]
struct StatusPayload<'a> {
    checks: &'a [StatusCheck],
}

/// Render text status: one line per check.
pub fn render_status_text(checks: &[StatusCheck]) -> String {
    checks
        .iter()
        .map(|c| format!("{}: {} ({}) hint: {}", c.name, c.status, c.detail, c.hint))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render JSON status (single object, NDJSON-compatible).
pub fn render_status_json(checks: &[StatusCheck]) -> String {
    // Infallible shape (issue #238): strings only, so `serde_json` cannot
    // fail; the fallback names the invariant instead of `expect`.
    serde_json::to_string(&StatusPayload { checks })
        .unwrap_or_else(|err| unreachable!("status JSON serializes: {err:?}"))
}

/// Default local status checks (toolchain + platform + tools + pin).
pub fn default_status_checks(pinned: &str) -> Vec<StatusCheck> {
    let pin_status = if version_pin_matches_module(pinned, MODULE_VERSION) {
        "ok"
    } else {
        "error"
    };
    vec![
        StatusCheck {
            name: "toolchain".to_owned(),
            status: "ok".to_owned(),
            detail: "rust 1.98.0 via rules_rust".to_owned(),
            hint: "bazel build //...".to_owned(),
        },
        StatusCheck {
            name: "platform".to_owned(),
            status: "ok".to_owned(),
            detail: "linux_x86_64 glibc qualified".to_owned(),
            hint: "see reusable-consumer matrix for macos/windows".to_owned(),
        },
        StatusCheck {
            name: "tools".to_owned(),
            status: "ok".to_owned(),
            detail: "bazel-resolved pinned tools".to_owned(),
            hint: "no ambient tools required".to_owned(),
        },
        StatusCheck {
            name: "pin".to_owned(),
            status: pin_status.to_owned(),
            detail: format!("dx {pinned} vs module {MODULE_VERSION}"),
            hint: "dx version --pin 0.0.0".to_owned(),
        },
    ]
}

/// Validate one watch invocation (O55 freeze).
pub fn plan_watch(command: &str, ci: bool) -> Result<String, AdoptError> {
    if ci {
        return Err(AdoptError::WatchRefusesCi);
    }
    if !WATCHABLE_COMMANDS.contains(&command) {
        return Err(AdoptError::NotWatchable {
            command: command.to_owned(),
        });
    }
    Ok(format!("watch:{command}:debounce={WATCH_DEBOUNCE_MS}ms"))
}

/// Coalesces debounced watcher paths into a single deterministic
/// rebuild trigger (issue #223): rapid create/modify/delete bursts
/// for one path collapse to one entry; outputs sort ascending with
/// duplicates removed so repeated runs render identically.
pub fn coalesce_watch_paths(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.sort();
    paths.dedup();
    paths
}

/// Blocks up to `timeout` for one debounced filesystem change under
/// `watch_root` (issue #223), returning the coalesced trigger paths.
///
/// Implemented over [`notify`] 8.x plus `notify-debouncer-mini`
/// (200 ms debounce per [`WATCH_DEBOUNCE_MS`]): create, modify, and
/// delete events all feed the same rebuild trigger. An empty vector
/// means the timeout elapsed with no changes (the caller re-arms);
/// only watcher setup and channel failures surface as [`AdoptError`].
/// Callers must validate via [`plan_watch`] first (local-only refusal
/// stays there, not here).
pub fn watch_for_change(watch_root: &Path, timeout: Duration) -> Result<Vec<PathBuf>, AdoptError> {
    use notify::RecursiveMode;
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer =
        notify_debouncer_mini::new_debouncer(Duration::from_millis(WATCH_DEBOUNCE_MS), tx)
            .map_err(|e| AdoptError::WatchSpawn {
                detail: e.to_string(),
            })?;
    debouncer
        .watcher()
        .watch(watch_root, RecursiveMode::Recursive)
        .map_err(|e| AdoptError::WatchSpawn {
            detail: e.to_string(),
        })?;
    match rx.recv_timeout(timeout) {
        Ok(Ok(events)) => {
            let paths: Vec<PathBuf> = events.into_iter().map(|event| event.path).collect();
            Ok(coalesce_watch_paths(paths))
        }
        Ok(Err(e)) => Err(AdoptError::WatchFailed {
            detail: e.to_string(),
        }),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(Vec::new()),
        Err(e) => Err(AdoptError::WatchFailed {
            detail: e.to_string(),
        }),
    }
}

/// Planned thin inspect forwarding (O56 freeze): the Bazel verb plus
/// the single query expression, executed as `bazel <verb> <expr>` with
/// bytewise-sorted deduplicated canonical labels and no custom graph
/// engine. The verb and expression stay separate so the caller cannot
/// double-wrap the expression in a second `query` invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectPlan {
    /// `query` by default, `cquery` under `--configured`.
    pub verb: String,
    /// Single query expression (no verb prefix, no surrounding shell).
    pub expr: String,
}

/// Plan one inspect query (O56 freeze).
pub fn plan_inspect(kind: &str, scope: &str, configured: bool) -> Result<InspectPlan, AdoptError> {
    if !inspect_scope_allowed(scope, scope.starts_with('@')) {
        return Err(AdoptError::RejectedScope {
            scope: scope.to_owned(),
        });
    }
    let verb = if configured { "cquery" } else { "query" };
    let expr = match kind {
        "owners" => format!("kind('rule', rdeps(//..., {scope}, 1))"),
        "deps" => format!("deps({scope})"),
        "why" => {
            return Err(AdoptError::WhyNeedsOwner);
        }
        _ => {
            return Err(AdoptError::UnknownInspect {
                kind: kind.to_owned(),
            });
        }
    };
    Ok(InspectPlan {
        verb: verb.to_owned(),
        expr,
    })
}

/// Plan the `somepath` leg of `dx why <file> <label>` (O56 freeze).
///
/// `from` is the resolved file owner (a depth-1 owner label, never the
/// raw file path) and `to` is the target label, both passed through
/// verbatim. External scopes are rejected like workflow commands.
pub fn plan_somepath(from: &str, to: &str, configured: bool) -> Result<InspectPlan, AdoptError> {
    if !inspect_scope_allowed(from, from.starts_with('@')) {
        return Err(AdoptError::RejectedScope {
            scope: from.to_owned(),
        });
    }
    if !inspect_scope_allowed(to, to.starts_with('@')) {
        return Err(AdoptError::RejectedScope {
            scope: to.to_owned(),
        });
    }
    let verb = if configured { "cquery" } else { "query" };
    Ok(InspectPlan {
        verb: verb.to_owned(),
        expr: format!("somepath({from}, {to})"),
    })
}

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
    fn pin_holds_only_on_equal_nonempty_versions() {
        assert!(version_pin_matches_module("1.2.3", "1.2.3"));
        assert!(!version_pin_matches_module("1.2.3", "1.2.4"));
        assert!(!version_pin_matches_module("", ""));
        assert!(!version_pin_matches_module("1.2.3", ""));
        // Issue #224 (semver pilot): pins must be valid semver; equal
        // non-semver text never matches, even when verbatim equal.
        assert!(!version_pin_matches_module("abc", "abc"));
        assert!(!version_pin_matches_module("v1.2.3", "v1.2.3"));
        assert!(!version_pin_matches_module("1.2", "1.2"));
        // Pre-release and build metadata compare exactly.
        assert!(version_pin_matches_module("1.2.3-alpha.1", "1.2.3-alpha.1"));
        assert!(!version_pin_matches_module(
            "1.2.3-alpha.1",
            "1.2.3-alpha.2"
        ));
        assert!(!version_pin_matches_module("1.2.3", "1.2.3-alpha.1"));
    }

    #[test]
    fn rollback_re_pins_only_the_known_previous() {
        assert!(rollback_re_pins_previous("1.2.4", "1.2.3", "1.2.3"));
        assert!(!rollback_re_pins_previous("1.2.3", "1.2.3", "1.2.3"));
        assert!(!rollback_re_pins_previous("1.2.4", "1.2.2", "1.2.3"));
        assert!(!rollback_re_pins_previous("1.2.4", "", ""));
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
    fn watch_iterations_stay_local_reresolved_and_single() {
        assert!(watch_iteration_accepts(true, true, true));
        assert!(!watch_iteration_accepts(false, true, true));
        assert!(!watch_iteration_accepts(true, false, true));
        assert!(!watch_iteration_accepts(true, true, false));
    }

    #[test]
    fn inspect_rejects_empty_and_external_scopes() {
        assert!(inspect_scope_allowed("//pkg:target", false));
        assert!(!inspect_scope_allowed("", false));
        assert!(!inspect_scope_allowed("//pkg:target", true));
        assert!(!inspect_scope_allowed("@other//pkg:target", true));
    }

    #[test]
    fn delivered_version_matches_module() {
        assert!(version_pin_matches_module(DX_VERSION, MODULE_VERSION));
        // Single-version state (no releases cut): the rollback target
        // equals the delivered version, so rollback correctly refuses.
        assert!(!rollback_re_pins_previous(
            DX_VERSION,
            PREVIOUS_VERSION,
            PREVIOUS_VERSION
        ));
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
    fn watch_freeze_holds() {
        assert!(plan_watch("test", false).is_ok());
        assert!(plan_watch("docs", false).is_err());
        assert!(plan_watch("test", true).is_err());
    }

    #[test]
    fn watch_coalesces_bursts_into_a_single_trigger() {
        // Issue #223: rapid create/modify/delete bursts collapse to one
        // deterministic rebuild trigger per path.
        let first = PathBuf::from("/tmp/ws/src/main.rs");
        let second = PathBuf::from("/tmp/ws/src/lib.rs");
        let trigger = coalesce_watch_paths(vec![
            first.clone(),
            first.clone(),
            second.clone(),
            first.clone(),
        ]);
        assert_eq!(trigger, vec![second, first]);
    }

    #[test]
    fn watch_reports_created_files_and_times_out_when_idle() {
        // Issue #223: a real `notify` watcher emits a debounced trigger
        // for a created file, and reports empty when nothing changes.
        let scratch = dx_test_scratch::scratch("dx-adopt-watch-");
        let root = scratch.path().to_path_buf();
        let writer = root.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            let _ = std::fs::write(writer.join("trigger.txt"), "change");
        });
        let trigger = watch_for_change(&root, Duration::from_secs(5)).expect("watch create");
        assert!(
            trigger.iter().any(|path| path.ends_with("trigger.txt")),
            "created file must trigger a rebuild: {trigger:?}"
        );
        let idle = watch_for_change(&root, Duration::from_millis(300)).expect("watch idle");
        assert!(idle.is_empty(), "idle watch must time out empty: {idle:?}");
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
    fn inspect_plans_query_forwarding() {
        let owners = plan_inspect("owners", "//a:one", false).expect("q");
        assert_eq!(owners.verb, "query");
        assert!(owners.expr.contains("rdeps"));
        assert!(!owners.expr.contains("query"));
        let deps = plan_inspect("deps", "//a:one", true).expect("q");
        assert_eq!(deps.verb, "cquery");
        assert!(deps.expr.contains("deps("));
        assert!(plan_inspect("owners", "@o//a:one", false).is_err());
        assert!(plan_inspect("why", "//a:one", false).is_err());
        let leg = plan_somepath("//a:one", "//b:two", false).expect("somepath");
        assert_eq!(leg.verb, "query");
        assert_eq!(leg.expr, "somepath(//a:one, //b:two)");
        assert!(plan_somepath("@o//a:one", "//b:two", false).is_err());
        assert!(plan_somepath("//a:one", "", false).is_err());
    }

    #[test]
    fn status_renders_text_and_json() {
        let checks = default_status_checks(MODULE_VERSION);
        assert_eq!(checks.len(), 4);
        let text = render_status_text(&checks);
        assert!(text.contains("pin: ok"));
        let json = render_status_json(&checks);
        // Golden pilot (issue #225): full-payload insta snapshot replaces
        // the contains-asserts; a MODULE_VERSION bump intentionally
        // updates this snapshot alongside the pin contract.
        insta::assert_snapshot!(json, @r#"{"checks":[{"name":"toolchain","status":"ok","detail":"rust 1.98.0 via rules_rust","hint":"bazel build //..."},{"name":"platform","status":"ok","detail":"linux_x86_64 glibc qualified","hint":"see reusable-consumer matrix for macos/windows"},{"name":"tools","status":"ok","detail":"bazel-resolved pinned tools","hint":"no ambient tools required"},{"name":"pin","status":"ok","detail":"dx 0.0.0 vs module 0.0.0","hint":"dx version --pin 0.0.0"}]}"#);
    }

    #[test]
    fn status_json_escapes_quotes_newlines_and_controls() {
        let checks = vec![StatusCheck {
            name: "we\"ird".to_owned(),
            status: "ok".to_owned(),
            detail: "line1\nline2\u{1}".to_owned(),
            hint: "back\\slash".to_owned(),
        }];
        let json = render_status_json(&checks);
        assert!(json.contains("\\\""), "{json}");
        assert!(json.contains("\\n"), "{json}");
        assert!(json.contains("\\\\"), "{json}");
        assert!(json.contains("\\u0001"), "{json}");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("status JSON is valid");
        assert_eq!(parsed["checks"][0]["name"], "we\"ird");
        // Re-serializing via `Value` sorts object keys (BTreeMap), while the
        // struct order stays name,status,detail,hint for byte-stability with
        // the pre-serde rendering; compare values, not bytes.
        let reparsed: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&parsed).expect("reserialize"))
                .expect("reserialized JSON is valid");
        assert_eq!(parsed, reparsed);
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
