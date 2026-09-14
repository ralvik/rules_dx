//! Pure post-release adoption planning (M30b slice 1: `dx init` scaffolding
//! discipline and single-version `dx` pinning with rollback).
//!
//! This crate owns the adoption shape before any scaffolding, hook,
//! devcontainer, diagnostics, versioning, watch, inspect, or completion
//! behavior lands: absent-only writes, unmanaged refusal, the single tested
//! version (`dx` version equals the pinned `rules_dx` module version), and
//! rollback as re-pinning. It plans over injected booleans/strings only, so
//! the rules stay deterministic and unit-testable without repositories,
//! editors, containers, networks, or shells.
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
}
