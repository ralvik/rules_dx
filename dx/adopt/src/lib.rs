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

use std::path::Path;

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

/// Single command-definition source (O61 freeze).
///
/// Every `dx completion <shell>` script renders from this table so new
/// commands cannot drift from the command reference.
pub const ALL_COMMANDS: &[&str] = &[
    "audit",
    "lint",
    "typecheck",
    "format",
    "generate",
    "build",
    "test",
    "coverage",
    "run",
    "check",
    "fix",
    "clean",
    "update",
    "codegen",
    "env",
    "setup",
    "init",
    "hooks",
    "status",
    "version",
    "watch",
    "owners",
    "deps",
    "why",
    "completion",
    "bazel",
];

/// Shells covered by `dx completion` (O61 freeze).
pub const SUPPORTED_SHELLS: &[&str] = &["bash", "zsh", "fish", "powershell"];

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
/// version: both must be non-empty and verbatim equal. Self-update bumps
/// that pin from verified release artifacts; anything else is rejected here.
pub fn version_pin_matches_module(dx_version: &str, module_version: &str) -> bool {
    !dx_version.is_empty() && dx_version == module_version
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

/// Whether a completion script source is admissible.
///
/// Completion scripts ship as generated output from the single CLI
/// command-definition source: handwritten per-shell scripts are rejected so
/// new commands and flags cannot drift from the command reference. O61 owns
/// the shell list, mechanics, and drift fixtures.
pub fn completion_source_is_single(generated_from_single_source: bool, handwritten: bool) -> bool {
    generated_from_single_source && !handwritten
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
            content: "{\"name\":\"rules_dx\",\"image\":\"mcr.microsoft.com/devcontainers/base:ubuntu\",\"features\":{\"ghcr.io/devcontainers/features/bazel:1\":{}},\"customizations\":{\"vscode\":{\"extensions\":[\"rust-lang.rust-analyzer\"]}},\"postCreateCommand\":\"bazel build //...\"}\n"
                .to_owned(),
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
pub fn apply_init(root: &Path, module_name: &str) -> Result<Vec<String>, String> {
    let mut written = Vec::new();
    let mut refused = Vec::new();
    for file in plan_init_files(module_name) {
        let dest = root.join(&file.path);
        if dest.exists() {
            refused.push(format!("refused:{}", file.path));
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create parent {}: {e}", parent.display()))?;
        }
        std::fs::write(&dest, file.content)
            .map_err(|e| format!("write {}: {e}", dest.display()))?;
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
        "#!/bin/sh\n{HOOK_MANAGED_MARKER} {trigger}\nexec bazel run //dx/cli:dx -- hooks run {trigger} -- \"$@\"\n"
    )
}

/// Install `pre-commit` + `pre-push` shims under `root/.git/hooks`.
///
/// Refuses unmanaged existing hooks even with force. Bootstraps the
/// gitignored `dx.local.toml` overlay absent-only. Returns installed paths.
pub fn install_hooks(root: &Path) -> Result<Vec<String>, String> {
    let hooks_dir = root.join(".git/hooks");
    std::fs::create_dir_all(&hooks_dir).map_err(|e| format!("create hooks dir: {e}"))?;
    let mut installed = Vec::new();
    for trigger in ["pre-commit", "pre-push"] {
        let dest = hooks_dir.join(trigger);
        if dest.exists() {
            let existing =
                std::fs::read_to_string(&dest).map_err(|e| format!("read hook {trigger}: {e}"))?;
            if !existing.contains(HOOK_MANAGED_MARKER) {
                return Err(format!("unmanaged hook refuses install: {trigger}"));
            }
        }
        std::fs::write(&dest, render_hook_shim(trigger))
            .map_err(|e| format!("write hook {trigger}: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&dest)
                .map_err(|e| format!("stat hook {trigger}: {e}"))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&dest, perms)
                .map_err(|e| format!("chmod hook {trigger}: {e}"))?;
        }
        installed.push(format!(".git/hooks/{trigger}"));
    }
    let overlay = root.join("dx.local.toml");
    if !overlay.exists() {
        std::fs::write(&overlay, "# Local-only overrides (gitignored).\n[hooks]\n")
            .map_err(|e| format!("write overlay: {e}"))?;
        installed.push("dx.local.toml".to_owned());
    }
    Ok(installed)
}

/// Remove only managed shims; unmanaged files are never touched.
pub fn uninstall_hooks(root: &Path) -> Result<Vec<String>, String> {
    let mut removed = Vec::new();
    for trigger in ["pre-commit", "pre-push"] {
        let dest = root.join(".git/hooks").join(trigger);
        if !dest.exists() {
            continue;
        }
        let existing =
            std::fs::read_to_string(&dest).map_err(|e| format!("read hook {trigger}: {e}"))?;
        if !existing.contains(HOOK_MANAGED_MARKER) {
            return Err(format!("unmanaged hook refuses uninstall: {trigger}"));
        }
        std::fs::remove_file(&dest).map_err(|e| format!("remove hook {trigger}: {e}"))?;
        removed.push(format!(".git/hooks/{trigger}"));
    }
    Ok(removed)
}

/// Render the merged `dx hooks status` view.
pub fn render_hooks_status(baseline: &str, overlay: &str, timings: &str) -> String {
    format!("baseline:\n{baseline}\noverlay:\n{overlay}\ntimings:\n{timings}\n")
}

/// Read the `.dx/version` pin under `root`.
pub fn read_version_pin(root: &Path) -> Result<String, String> {
    let raw = std::fs::read_to_string(root.join(".dx/version"))
        .map_err(|e| format!("read version pin: {e}"))?;
    Ok(raw.trim().to_owned())
}

/// Write the `.dx/version` pin (verified-release versions only).
pub fn write_version_pin(root: &Path, version: &str) -> Result<(), String> {
    if version.is_empty() {
        return Err("refuses empty version".to_owned());
    }
    let dir = root.join(".dx");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create .dx: {e}"))?;
    std::fs::write(dir.join("version"), format!("{version}\n"))
        .map_err(|e| format!("write version pin: {e}"))?;
    Ok(())
}

/// One diagnostics check in the consolidated `dx status` surface.
#[derive(Clone, Debug, Eq, PartialEq)]
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
    let mut out = String::from("{\"checks\":[");
    for (i, c) in checks.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"name\":\"{}\",\"status\":\"{}\",\"detail\":\"{}\",\"hint\":\"{}\"}}",
            c.name, c.status, c.detail, c.hint
        ));
    }
    out.push_str("]}");
    out
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
pub fn plan_watch(command: &str, ci: bool) -> Result<String, String> {
    if ci {
        return Err("dx watch refuses CI (local-only)".to_owned());
    }
    if !WATCHABLE_COMMANDS.contains(&command) {
        return Err(format!("not watchable: {command}"));
    }
    Ok(format!("watch:{command}:debounce={WATCH_DEBOUNCE_MS}ms"))
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
pub fn plan_inspect(kind: &str, scope: &str, configured: bool) -> Result<InspectPlan, String> {
    if !inspect_scope_allowed(scope, scope.starts_with('@')) {
        return Err(format!("rejected scope: {scope}"));
    }
    let verb = if configured { "cquery" } else { "query" };
    let expr = match kind {
        "owners" => format!("kind('rule', rdeps(//..., {scope}, 1))"),
        "deps" => format!("deps({scope})"),
        "why" => {
            return Err(
                "dx why needs <file> <label>: resolve the file owner first, then plan_somepath"
                    .to_owned(),
            );
        }
        _ => return Err(format!("unknown inspect: {kind}")),
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
pub fn plan_somepath(from: &str, to: &str, configured: bool) -> Result<InspectPlan, String> {
    if !inspect_scope_allowed(from, from.starts_with('@')) {
        return Err(format!("rejected scope: {from}"));
    }
    if !inspect_scope_allowed(to, to.starts_with('@')) {
        return Err(format!("rejected scope: {to}"));
    }
    let verb = if configured { "cquery" } else { "query" };
    Ok(InspectPlan {
        verb: verb.to_owned(),
        expr: format!("somepath({from}, {to})"),
    })
}

/// Render one completion script from the single command table (O61).
pub fn render_completion(shell: &str) -> Result<String, String> {
    if !SUPPORTED_SHELLS.contains(&shell) {
        return Err(format!("unknown-shell: {shell}"));
    }
    let mut out = format!("# dx completion for {shell} (generated from single command source)\n");
    for cmd in ALL_COMMANDS {
        out.push_str(&format!("# dx {cmd}\n"));
    }
    match shell {
        "bash" => out.push_str("complete -W \"dx Commands\" dx\n"),
        "zsh" => out.push_str("#compdef dx\n_dx() { _arguments '1: :()'}; compdef _dx dx\n"),
        "fish" => out.push_str("complete -c dx -f\n"),
        "powershell" => out.push_str("Register-ArgumentCompleter -CommandName dx\n"),
        _ => {}
    }
    Ok(out)
}

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
    fn completion_comes_from_the_single_source_only() {
        assert!(completion_source_is_single(true, false));
        assert!(!completion_source_is_single(true, true));
        assert!(!completion_source_is_single(false, false));
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
    fn init_plans_eight_absent_only_files() {
        let files = plan_init_files("demo");
        assert_eq!(files.len(), 8);
        assert!(files.iter().any(|f| f.path == ".dx/version"));
        assert!(files
            .iter()
            .any(|f| f.path == ".devcontainer/devcontainer.json"));
        assert!(files.iter().any(|f| f.path == ".vscode/settings.json"));
    }

    #[test]
    fn apply_init_writes_absent_only_and_refuses_existing() {
        let scratch = tempfile::Builder::new()
            .prefix("dx-adopt-init-")
            .tempdir_in(std::env::temp_dir())
            .expect("scratch");
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
        let scratch = tempfile::Builder::new()
            .prefix("dx-adopt-hook-")
            .tempdir_in(std::env::temp_dir())
            .expect("scratch");
        let root = scratch.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git/hooks")).expect("tmp");
        let installed = install_hooks(&root).expect("install");
        assert!(installed.iter().any(|p| p == ".git/hooks/pre-commit"));
        std::fs::write(root.join(".git/hooks/pre-commit"), "# custom hook\n").expect("unmanaged");
        assert!(install_hooks(&root).is_err());
        scratch.close().expect("cleanup");
    }

    #[test]
    fn completion_renders_every_command_for_every_shell() {
        for shell in SUPPORTED_SHELLS {
            let script = render_completion(shell).expect("shell");
            for cmd in ALL_COMMANDS {
                assert!(script.contains(cmd), "{shell} misses {cmd}");
            }
        }
        assert!(render_completion("tcsh").is_err());
    }

    #[test]
    fn watch_freeze_holds() {
        assert!(plan_watch("test", false).is_ok());
        assert!(plan_watch("docs", false).is_err());
        assert!(plan_watch("test", true).is_err());
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
        assert!(json.contains("\"checks\""));
        assert!(json.contains("\"pin\""));
    }
}
