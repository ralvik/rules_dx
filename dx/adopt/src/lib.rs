//! Pure post-release adoption planning (M30b slices 1-3: `dx init` scaffolding
//! discipline and single-version `dx` pinning with rollback; hook-runner
//! hermeticity/overwrite/config shape and devcontainer bootstrap discipline;
//! diagnostics naming, watch locality/scope, inspect forwarding, and
//! completion single-source generation).
//!
//! This crate owns the adoption shape before any scaffolding, hook,
//! devcontainer, diagnostics, versioning, watch, inspect, or completion
//! behavior lands: absent-only writes, unmanaged refusal, the single tested
//! version (`dx` version equals the pinned `rules_dx` module version),
//! rollback as re-pinning, hermetic-only hook Git, unmanaged-hook overwrite
//! refusal, the two-layer hook configuration, pinned Bazel-delegated
//! devcontainers, the rejected-`doctor` diagnostics name, local-only watch
//! with per-iteration re-resolution, thin inspect forwarding with
//! external-scope rejection, and single-source completion generation. It
//! plans over injected booleans/strings only, so the rules stay deterministic
//! and unit-testable without repositories, editors, containers, networks,
//! or shells.
//!
//! This crate owns the adoption shape before any scaffolding, hook,
//! devcontainer, diagnostics, versioning, watch, inspect, or completion
//! behavior lands: absent-only writes, unmanaged refusal, the single tested
//! version (`dx` version equals the pinned `rules_dx` module version),
//! rollback as re-pinning, hermetic-only hook Git, unmanaged-hook overwrite
//! refusal, the two-layer hook configuration, and pinned Bazel-delegated
//! devcontainers. It plans over injected booleans/strings only, so the rules
//! stay deterministic and unit-testable without repositories, editors,
//! containers, networks, or shells.
//!
//! Out of scope here (M30b delivery, all gated): guide prose and CI wiring
//! beyond the M30a handoff, hook-runner execution and hermetic-Git
//! acquisition, devcontainer builds, the consolidated diagnostics surface
//! (O50), launcher/self-update mechanics (O51), watch mechanics (O55),
//! inspect forwarding details (O56), and completion script generation (O61).
//! This crate writes no files, runs no containers, and registers no CLI.

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
}
