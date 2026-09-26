pub fn devcontainer_is_admissible(
    pinned_bootstrap: bool,
    delegates_to_bazel: bool,
    uses_ambient_tools: bool,
) -> bool {
    pinned_bootstrap && delegates_to_bazel && !uses_ambient_tools
}

pub fn diagnostics_command_allowed(name: &str) -> bool {
    !name.is_empty() && name != "doctor"
}

#[cfg(test)]
mod tests {
    use super::super::{
        devcontainer_is_admissible as facade_admissible,
        diagnostics_command_allowed as facade_allowed,
    };
    use super::{devcontainer_is_admissible, diagnostics_command_allowed};

    #[test]
    fn policy_reexports_match_local_definitions() {
        assert_eq!(
            facade_admissible(true, true, false),
            devcontainer_is_admissible(true, true, false)
        );
        assert_eq!(
            facade_allowed("status"),
            diagnostics_command_allowed("status")
        );
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
}
