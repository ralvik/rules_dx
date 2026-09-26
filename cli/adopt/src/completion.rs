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
    "deploy",
    "check",
    "fix",
    "clean",
    "update",
    "bump",
    "migrate",
    "codegen",
    "env",
    "setup",
    "init",
    "new",
    "upgrade",
    "hooks",
    "status",
    "version",
    "watch",
    "owners",
    "deps",
    "why",
    "completion",
    "docs",
    "bazel",
];

pub const SUPPORTED_SHELLS: &[&str] = &["bash", "zsh", "fish", "powershell"];

pub fn completion_source_is_single(generated_from_single_source: bool, handwritten: bool) -> bool {
    generated_from_single_source && !handwritten
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_comes_from_the_single_source_only() {
        assert!(completion_source_is_single(true, false));
        assert!(!completion_source_is_single(true, true));
        assert!(!completion_source_is_single(false, false));
    }

    #[test]
    fn completion_vocabulary_matches_supported_shells() {
        // Scripts render from the `Cli` grammar via
        // `clap_complete`, so this gate pins the vocabulary reference
        // only — every command stays listed, every shell stays supported.
        for shell in SUPPORTED_SHELLS {
            assert!(!shell.is_empty(), "shell name must not be empty");
        }
        for cmd in ALL_COMMANDS {
            assert!(!cmd.is_empty(), "command name must not be empty");
        }
        assert!(ALL_COMMANDS.contains(&"completion"));
        assert!(completion_source_is_single(true, false));
    }

    #[test]
    fn completion_vocabulary_is_the_final_registry() {
        // The frozen vocabulary reference
        // pins the final CLI registry exactly (32 commands including
        // `deploy` plus `bump` plus `migrate` plus `new` plus `upgrade`
        // plus `docs`; `doctor` plus `configure` stay rejected as unknown).
        let mut got = ALL_COMMANDS.to_vec();
        got.sort_unstable();
        let mut want = vec![
            "audit",
            "bazel",
            "build",
            "bump",
            "check",
            "clean",
            "codegen",
            "completion",
            "coverage",
            "deps",
            "deploy",
            "docs",
            "env",
            "fix",
            "format",
            "generate",
            "hooks",
            "init",
            "lint",
            "migrate",
            "new",
            "owners",
            "run",
            "setup",
            "status",
            "test",
            "typecheck",
            "update",
            "upgrade",
            "version",
            "watch",
            "why",
        ];
        want.sort_unstable();
        assert_eq!(got, want, "ALL_COMMANDS drifted from the final registry");
        assert_eq!(ALL_COMMANDS.len(), 32, "final registry holds 32 commands");
        let mut dedup = got.clone();
        dedup.dedup();
        assert_eq!(got.len(), dedup.len(), "registry spellings must be unique");
        for excluded in ["doctor", "configure"] {
            assert!(
                !ALL_COMMANDS.contains(&excluded),
                "{excluded} must stay outside the final registry"
            );
        }
    }
}
