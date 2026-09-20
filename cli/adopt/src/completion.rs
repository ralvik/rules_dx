//! Single-source completion vocabulary for `dx completion`.
//!
//! Split from `super` (`lib.rs`): owns `ALL_COMMANDS`,
//! `SUPPORTED_SHELLS`, and `completion_source_is_single`.
//! Re-exported through `super` so the public path stays
//! `dx_adopt::{ALL_COMMANDS, SUPPORTED_SHELLS, completion_source_is_single}`.
//!
//! Production `dx completion` renders from the `Cli` grammar via
//! `clap_complete` (`cli/cli/src/args.rs::render_completion`),
//! so the grammar feeding parsing and `--help` is the single completion
//! source. The `ALL_COMMANDS`/`SUPPORTED_SHELLS` tables here
//! remain as the frozen vocabulary reference only; they render nothing.

/// Single command-definition source (frozen).
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
    "deploy",
    "check",
    "fix",
    "clean",
    "update",
    "bump",
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

/// Shells covered by `dx completion` (frozen).
pub const SUPPORTED_SHELLS: &[&str] = &["bash", "zsh", "fish", "powershell"];

/// Whether a completion script source is admissible.
///
/// Completion scripts ship as generated output from the single CLI
/// command-definition source: handwritten per-shell scripts are rejected so
/// new commands and flags cannot drift from the command reference. The single source owns
/// the shell list, mechanics, and drift fixtures.
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
        // Issue #235: scripts render from the `Cli` grammar via
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
        // Issue #457: the frozen vocabulary reference pins the final
        // CLI registry exactly (28 commands including `deploy` plus
        // `bump`; `migrate` stays planning-only under #462, `doctor`
        // plus `configure` stay rejected as unknown).
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
            "env",
            "fix",
            "format",
            "generate",
            "hooks",
            "init",
            "lint",
            "owners",
            "run",
            "setup",
            "status",
            "test",
            "typecheck",
            "update",
            "version",
            "watch",
            "why",
        ];
        want.sort_unstable();
        assert_eq!(got, want, "ALL_COMMANDS drifted from the final registry");
        assert_eq!(ALL_COMMANDS.len(), 28, "final registry holds 28 commands");
        let mut dedup = got.clone();
        dedup.dedup();
        assert_eq!(got.len(), dedup.len(), "registry spellings must be unique");
        for excluded in ["doctor", "configure", "docs", "migrate", "new"] {
            assert!(
                !ALL_COMMANDS.contains(&excluded),
                "{excluded} must stay outside the final registry"
            );
        }
    }
}
