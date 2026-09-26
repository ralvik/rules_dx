pub const LINT_CHECK: &str = "lint";
pub const TYPECHECK_CHECK: &str = "typecheck";
pub const FORMAT_CHECK: &str = "format";
pub const GENERATE_CHECK: &str = "generate";
pub const SECURITY_AUDIT_CHECK: &str = "security-audit";
pub const LICENSE_AUDIT_CHECK: &str = "license-audit";
pub const TEST_CHECK: &str = "test";
pub const BUILD_CHECK: &str = "build";
pub const COVERAGE_CHECK: &str = "coverage";

pub const CHECK_COUNT: usize = 9;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionScope {
    LinuxOnce,
    PerPlatform,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Check {
    pub id: &'static str,
    pub command: &'static str,
    pub scope: ExecutionScope,
}

pub const ALL_CHECKS: [Check; CHECK_COUNT] = [
    Check {
        id: LINT_CHECK,
        command: "dx lint --check",
        scope: ExecutionScope::PerPlatform,
    },
    Check {
        id: TYPECHECK_CHECK,
        command: "dx typecheck --check",
        scope: ExecutionScope::PerPlatform,
    },
    Check {
        id: FORMAT_CHECK,
        command: "dx format --check",
        scope: ExecutionScope::PerPlatform,
    },
    Check {
        id: GENERATE_CHECK,
        command: "dx generate --check",
        scope: ExecutionScope::PerPlatform,
    },
    Check {
        id: SECURITY_AUDIT_CHECK,
        command: "dx security",
        scope: ExecutionScope::PerPlatform,
    },
    Check {
        id: LICENSE_AUDIT_CHECK,
        command: "dx license",
        scope: ExecutionScope::PerPlatform,
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

pub fn find_check(id: &str) -> Option<Check> {
    ALL_CHECKS.iter().copied().find(|check| check.id == id)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CiSelection {
    pub enabled: Vec<Check>,
    pub platforms: Vec<String>,
}

impl CiSelection {
    pub fn enabled_ids(&self) -> Vec<&'static str> {
        self.enabled.iter().map(|check| check.id).collect()
    }

    pub fn requires_platforms(&self) -> bool {
        self.enabled
            .iter()
            .any(|check| check.scope == ExecutionScope::PerPlatform)
    }

    /// CI selection never mutates: checks run in consistency modes and
    pub fn is_mutating() -> bool {
        false
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SelectionError {
    #[error("unknown CI check {value:?}; want one of the nine starter checks")]
    UnknownCheck { value: String },
    #[error("explicit platform selection is required when any check is enabled")]
    MissingPlatforms,
}

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
    fn every_starter_check_runs_per_platform() {
        for check in ALL_CHECKS {
            assert_eq!(
                check.scope,
                ExecutionScope::PerPlatform,
                "{} must run per-platform",
                check.id
            );
        }
        let disabled = strings(&["test", "build", "coverage"]);
        let error = plan_selection(&disabled, &[]).expect_err("quality-only still needs platforms");
        assert_eq!(error, SelectionError::MissingPlatforms);
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
        assert_eq!(by_id("security-audit"), "dx security");
        assert_eq!(by_id("license-audit"), "dx license");
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
