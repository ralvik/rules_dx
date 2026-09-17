//! Invocation error vocabulary (issue #236).
//!
//! Split from `super` (`args.rs`): owns [`ArgsError`] and the additive
//! typo-hint renderer. The parser ([`super::parser`]) constructs these;
//! `super` re-exports them so `crate::args::ArgsError` paths are
//! unchanged.

/// Renders the additive typo hint for unknown commands/options:
/// empty without a suggestion, `. did you mean "lint"?` with one.
fn suggestion_hint(suggestion: &Option<String>) -> String {
    match suggestion {
        Some(name) => format!(". did you mean {name:?}?"),
        None => String::new(),
    }
}

/// Invocation parsing failure or help request. Usage errors are
/// CLI-detected pre-execution failures (exit code 2); [`ArgsError::Help`]
/// is the `--help`/`-h` early exit (exit code 0, human text on stdout,
/// deliberately outside machine-output guarantees per issue #203).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArgsError {
    /// Rendered help text (`dx --help` or `dx <cmd> --help`).
    #[error("{text}")]
    Help { text: String },
    #[error(
        "missing command: want audit|lint|typecheck|format|generate|build|test|coverage|run|deploy|check|fix|clean|update|codegen|env|setup|init|hooks|status|version|watch|owners|deps|why|completion|bazel"
    )]
    MissingCommand,
    #[error(
        "unknown command {command:?}: want audit|lint|typecheck|format|generate|build|test|coverage|run|deploy|check|fix|clean|update|codegen|env|setup|init|hooks|status|version|watch|owners|deps|why|completion|bazel{suggestion_hint}",
        suggestion_hint = suggestion_hint(suggestion)
    )]
    UnknownCommand {
        command: String,
        suggestion: Option<String>,
    },
    #[error("unknown option {option:?}{suggestion_hint}", suggestion_hint = suggestion_hint(suggestion))]
    UnknownOption {
        option: String,
        suggestion: Option<String>,
    },
    #[error("option {option:?} is not supported by dx {command}")]
    UnsupportedOption {
        command: &'static str,
        option: String,
    },
    #[error("missing value for {option:?}")]
    MissingValue { option: String },
    #[error("unknown --output {value:?}: want text|diff|json")]
    BadOutput { value: String },
    #[error("unknown --fail-on {value:?}: want info|warning|error")]
    BadFailOn { value: String },
    #[error("invalid --min-coverage {value:?}: want an integer 0-100")]
    BadMinCoverage { value: String },
    #[error("malformed --report {value:?}: want <format>=<destination>")]
    BadReport { value: String },
    /// Unknown `dx completion` shell (contract: `bash|zsh|fish|powershell`).
    #[error("unknown-shell: {shell}")]
    UnknownShell { shell: String },
    #[error("options --debug and --release are mutually exclusive")]
    ConflictingProfiles,
    #[error(
        "empty scope: pass no scope for repository-wide //... or a //, @, file, or directory scope"
    )]
    EmptyScope,
    #[error(
        "unsupported scope {scope:?}: package-relative labels resolve against the current directory; spell the workspace label starting with //"
    )]
    RelativeLabel { scope: String },
    #[error(
        "unsupported scope {scope:?}: want // or @ labels, or workspace-relative file and directory paths"
    )]
    InvalidScope { scope: String },
}
