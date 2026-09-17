//! Adoption admissibility policy (issue #236).
//!
//! Split from `super` (`lib.rs`): owns `devcontainer_is_admissible`
//! and `diagnostics_command_allowed`. Re-exported through `super` so
//! the public paths stay
//! `dx_adopt::{devcontainer_is_admissible, diagnostics_command_allowed}`.

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
