//! Pure consumer-CI check-selection planning (M27 WP1 slice 1).
//!
//! This crate owns the check-selection surface before any reusable
//! workflow, caller template, or reporter lands: the nine accepted CI
//! checks, starter defaults, explicit opt-outs, and the explicit
//! platform-selection gate for per-platform checks. It plans over
//! injected argument strings only, so selection stays deterministic and
//! unit-testable without GitHub Actions, a Bazel server, or any runner.
//!
//! Frozen check table (`docs/github-ci.md#check-selection`): six checks run
//! once on Linux (`lint`, `typecheck`, `format`, `generate`,
//! `security-audit`, `license-audit`); three run on every
//! consumer-selected platform (`test`, `build`, `coverage`). The starter
//! enables all nine; callers disable individual checks by ID without
//! affecting unrelated checks, and disabled checks are omitted, never
//! reported as passed.
//!
//! Platform identities stay opaque here: when any per-platform check is
//! enabled, the caller must supply an explicit nonempty platform list,
//! and missing/empty selections fail closed. Runner/OS/arch mapping and
//! the supported-identity set arrive in later M27 slices; this crate
//! preserves spellings verbatim and never substitutes an implicit
//! Linux/current-runner/all-platforms default.
//!
//! Out of scope here (M27 qualification): workflow APIs/pins, event/ref
//! bindings, scheduling, review-thread/reporting mechanics, fork
//! security, merge gating, `.bazelrc` preset onboarding, and any YAML or
//! reporter implementation. Those arrive in later M27 slices.

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
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionError {
    /// Caller disabled an ID outside the frozen nine.
    UnknownCheck { value: String },
    /// A per-platform check is enabled but no platforms were supplied.
    MissingPlatforms,
}

impl std::fmt::Display for SelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectionError::UnknownCheck { value } => {
                write!(
                    f,
                    "unknown CI check {value:?}; want one of the nine starter checks"
                )
            }
            SelectionError::MissingPlatforms => {
                write!(
                    f,
                    "explicit platform selection is required when test, build, or coverage is enabled"
                )
            }
        }
    }
}

impl std::error::Error for SelectionError {}

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
