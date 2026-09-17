//! Check-selection planning for consumer CI (issue #236, M27 WP1 slice 1).
//!
//! Split from `super` (`lib.rs`): owns the frozen check table
//! ([`LINT_CHECK`] through [`COVERAGE_CHECK`], [`CHECK_COUNT`],
//! [`ALL_CHECKS`]), [`ExecutionScope`], [`Check`], [`find_check`],
//! [`CiSelection`] (with [`CiSelection::enabled_ids`],
//! [`CiSelection::requires_platforms`], [`CiSelection::is_mutating`]),
//! [`SelectionError`], and [`plan_selection`] (starter enables all nine
//! in canonical order; callers disable individual checks by ID;
//! per-platform checks require an explicit verbatim platform list).
//! Re-exported through `super` so the public paths stay
//! `dx_ci::{LINT_CHECK, TYPECHECK_CHECK, FORMAT_CHECK, GENERATE_CHECK,
//! SECURITY_AUDIT_CHECK, LICENSE_AUDIT_CHECK, TEST_CHECK, BUILD_CHECK,
//! COVERAGE_CHECK, CHECK_COUNT, ExecutionScope, Check, ALL_CHECKS,
//! find_check, CiSelection, SelectionError, plan_selection}`. Distinct
//! from the revision, scheduling, run, thread, approval, aggregate,
//! coverage, platform, caller, pin, audit, artifact, metadata, and
//! preset modules.

/// Frozen CI check identifiers, in canonical starter order.
pub const LINT_CHECK: &str = "lint";
/// Frozen CI check identifiers, in canonical starter order.
pub const TYPECHECK_CHECK: &str = "typecheck";
/// Frozen CI check identifiers, in canonical starter order.
pub const FORMAT_CHECK: &str = "format";
/// Frozen CI check identifiers, in canonical starter order.
pub const GENERATE_CHECK: &str = "generate";
/// Frozen CI check identifiers, in canonical starter order.
pub const SECURITY_AUDIT_CHECK: &str = "security-audit";
/// Frozen CI check identifiers, in canonical starter order.
pub const LICENSE_AUDIT_CHECK: &str = "license-audit";
/// Frozen CI check identifiers, in canonical starter order.
pub const TEST_CHECK: &str = "test";
/// Frozen CI check identifiers, in canonical starter order.
pub const BUILD_CHECK: &str = "build";
/// Frozen CI check identifiers, in canonical starter order.
pub const COVERAGE_CHECK: &str = "coverage";

/// Number of accepted CI checks in the starter.
pub const CHECK_COUNT: usize = 9;

/// Where a check executes: once on Linux, or on every selected platform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionScope {
    LinuxOnce,
    PerPlatform,
}

/// One accepted CI check: frozen ID, owning command, and execution scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Check {
    /// Frozen caller-visible identifier.
    pub id: &'static str,
    /// Owning `dx` command invocation (check/consistency mode).
    pub command: &'static str,
    /// Linux-once or every-selected-platform execution.
    pub scope: ExecutionScope,
}

/// Canonical starter order: all nine checks enabled.
pub const ALL_CHECKS: [Check; CHECK_COUNT] = [
    Check {
        id: LINT_CHECK,
        command: "dx lint --check",
        scope: ExecutionScope::LinuxOnce,
    },
    Check {
        id: TYPECHECK_CHECK,
        command: "dx typecheck --check",
        scope: ExecutionScope::LinuxOnce,
    },
    Check {
        id: FORMAT_CHECK,
        command: "dx format --check",
        scope: ExecutionScope::LinuxOnce,
    },
    Check {
        id: GENERATE_CHECK,
        command: "dx generate --check",
        scope: ExecutionScope::LinuxOnce,
    },
    Check {
        id: SECURITY_AUDIT_CHECK,
        command: "dx audit security",
        scope: ExecutionScope::LinuxOnce,
    },
    Check {
        id: LICENSE_AUDIT_CHECK,
        command: "dx audit license",
        scope: ExecutionScope::LinuxOnce,
    },
    Check {
        id: TEST_CHECK,
        command: "dx test",
        scope: ExecutionScope::PerPlatform,
    },
    Check {
        id: BUILD_CHECK,
        command: "dx build",
        scope: ExecutionScope::PerPlatform,
    },
    Check {
        id: COVERAGE_CHECK,
        command: "dx coverage",
        scope: ExecutionScope::PerPlatform,
    },
];

/// Look up one check by frozen ID.
pub fn find_check(id: &str) -> Option<Check> {
    ALL_CHECKS.iter().copied().find(|check| check.id == id)
}

/// Planned CI selection: enabled checks in canonical order plus the
/// verbatim platform list the caller supplied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CiSelection {
    /// Enabled checks in canonical starter order.
    pub enabled: Vec<Check>,
    /// Caller-supplied platform spellings, verbatim. Empty unless a
    /// per-platform check is enabled and platforms were provided.
    pub platforms: Vec<String>,
}

impl CiSelection {
    /// Enabled check IDs in canonical order.
    pub fn enabled_ids(&self) -> Vec<&'static str> {
        self.enabled.iter().map(|check| check.id).collect()
    }

    /// Whether any enabled check runs on every selected platform.
    pub fn requires_platforms(&self) -> bool {
        self.enabled
            .iter()
            .any(|check| check.scope == ExecutionScope::PerPlatform)
    }

    /// CI selection never mutates: checks run in consistency modes and
    /// never authorize automatic fixes, dependency updates, or bot
    /// commits.
    pub fn is_mutating() -> bool {
        false
    }
}

/// Malformed CI selection: unknown opt-outs or missing platform lists
/// fail closed instead of silently narrowing validation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SelectionError {
    /// Caller disabled an ID outside the frozen nine.
    #[error("unknown CI check {value:?}; want one of the nine starter checks")]
    UnknownCheck { value: String },
    /// A per-platform check is enabled but no platforms were supplied.
    #[error("explicit platform selection is required when test, build, or coverage is enabled")]
    MissingPlatforms,
}

/// Plan a CI selection from caller opt-outs and platform spellings.
///
/// `disabled` names checks to omit; every other starter check stays
/// enabled in canonical order. Unknown names fail closed. When any
/// enabled check needs per-platform execution, `platforms` must be
/// nonempty; spellings pass through verbatim for later qualification.
/// Linux-once-only selections accept an empty platform list.
pub fn plan_selection(
    disabled: &[String],
    platforms: &[String],
) -> Result<CiSelection, SelectionError> {
    for name in disabled {
        if find_check(name).is_none() {
            return Err(SelectionError::UnknownCheck {
                value: name.clone(),
            });
        }
    }
    let enabled: Vec<Check> = ALL_CHECKS
        .iter()
        .copied()
        .filter(|check| !disabled.iter().any(|name| name == check.id))
        .collect();
    let needs_platforms = enabled
        .iter()
        .any(|check| check.scope == ExecutionScope::PerPlatform);
    if needs_platforms && platforms.is_empty() {
        return Err(SelectionError::MissingPlatforms);
    }
    Ok(CiSelection {
        enabled,
        platforms: platforms.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(words: &[&str]) -> Vec<String> {
        words.iter().copied().map(str::to_owned).collect()
    }

    #[test]
    fn starter_enables_all_nine_in_canonical_order() {
        let selection = plan_selection(&[], &strings(&["linux_x86_64"])).expect("starter plans");
        assert_eq!(selection.enabled.len(), CHECK_COUNT);
        assert_eq!(
            selection.enabled_ids(),
            vec![
                "lint",
                "typecheck",
                "format",
                "generate",
                "security-audit",
                "license-audit",
                "test",
                "build",
                "coverage",
            ]
        );
    }

    #[test]
    fn linux_once_checks_need_no_platforms() {
        let only_linux: Vec<Check> = ALL_CHECKS
            .iter()
            .copied()
            .filter(|check| check.scope == ExecutionScope::LinuxOnce)
            .collect();
        assert_eq!(only_linux.len(), 6);
        let disabled = strings(&["test", "build", "coverage"]);
        let selection = plan_selection(&disabled, &[]).expect("linux-only plans");
        assert!(!selection.requires_platforms());
        assert_eq!(selection.enabled.len(), 6);
        assert!(selection.platforms.is_empty());
    }

    #[test]
    fn disabling_one_check_omits_only_that_check() {
        let selection =
            plan_selection(&strings(&["lint"]), &strings(&["linux_x86_64"])).expect("plans");
        assert_eq!(selection.enabled.len(), CHECK_COUNT - 1);
        assert!(!selection.enabled_ids().contains(&"lint"));
        assert!(selection.enabled_ids().contains(&"typecheck"));
    }

    #[test]
    fn duplicate_disables_are_idempotent() {
        let once = plan_selection(&strings(&["coverage"]), &strings(&["p1"])).expect("plans");
        let twice =
            plan_selection(&strings(&["coverage", "coverage"]), &strings(&["p1"])).expect("plans");
        assert_eq!(once, twice);
    }

    #[test]
    fn unknown_disable_fails_closed() {
        let error = plan_selection(&strings(&["audit"]), &strings(&["p1"])).expect_err("rejects");
        assert_eq!(
            error,
            SelectionError::UnknownCheck {
                value: "audit".to_owned()
            }
        );
    }

    #[test]
    fn missing_platforms_fail_when_per_platform_enabled() {
        let error = plan_selection(&[], &[]).expect_err("requires platforms");
        assert_eq!(error, SelectionError::MissingPlatforms);
    }

    #[test]
    fn platforms_pass_through_verbatim() {
        let selection =
            plan_selection(&[], &strings(&["linux_x86_64", "macos_arm64"])).expect("plans");
        assert_eq!(
            selection.platforms,
            vec!["linux_x86_64".to_owned(), "macos_arm64".to_owned()]
        );
    }

    #[test]
    fn owning_commands_are_frozen() {
        let by_id = |id: &str| find_check(id).expect("known check").command;
        assert_eq!(by_id("lint"), "dx lint --check");
        assert_eq!(by_id("typecheck"), "dx typecheck --check");
        assert_eq!(by_id("format"), "dx format --check");
        assert_eq!(by_id("generate"), "dx generate --check");
        assert_eq!(by_id("security-audit"), "dx audit security");
        assert_eq!(by_id("license-audit"), "dx audit license");
        assert_eq!(by_id("test"), "dx test");
        assert_eq!(by_id("build"), "dx build");
        assert_eq!(by_id("coverage"), "dx coverage");
        assert_eq!(find_check("audit"), None);
    }

    #[test]
    fn selection_is_non_mutating() {
        assert!(!CiSelection::is_mutating());
    }
}
