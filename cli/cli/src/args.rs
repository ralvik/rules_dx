//! Invocation parsing for the `dx` quality, workflow, run, clean, and
//! managed environment/codegen/setup commands (M07 WP1+WP3, M08 WP1+WP4,
//! M25 WP5).
//!
//! Contract: `docs/cli/cli-contract.md#invocation-shape`. Scope positionals
//! accept explicit Bazel labels and patterns (`//...`, `//pkg:target`,
//! `@repo//pkg/...`) as well as workspace-relative file and directory
//! paths. Package-relative labels (`:target`) and empty scopes fail with
//! [`ArgsError::RelativeLabel`] and [`ArgsError::EmptyScope`];
//! external-repository scopes parse but
//! fail during resolution, and file ownership resolves through Bazel
//! query per `docs/cli/target-resolution.md`. With no scope the
//! repository operation (`//...`) runs.
//!
//! `dx clean` takes no scopes: it prunes validated unselected managed
//! state per `docs/cli/commands/check-fix-clean.md#dx-clean`, with
//! `--dry-run` listing without deleting and `--bazel` additionally
//! forwarding `bazel clean`.
//!
//! Domain split (issue #236): the command vocabulary lives in the
//! `command` module, shell-completion rendering in the `completion`
//! module, help rendering in the `help` module, typo suggestions in
//! the `suggest` module, and invocation parsing in the `parser`
//! module (the `parse` domain; named `parser` so the module and the
//! `parse` function coexist). This facade keeps the shared types; the
//! public paths stay `crate::args::Command`,
//! `crate::args::{COMPLETION_SHELLS, render_completion}`, and
//! `crate::args::{parse, cli_command}` via the re-exports below.

use dx_output::{OutputMode, Threshold};

pub mod command;
pub mod completion;
pub mod help;
pub mod parser;
pub mod suggest;

pub use command::Command;
pub use completion::{render_completion, COMPLETION_SHELLS};
pub use parser::{cli_command, parse};

/// Build profile vocabulary (issue #179, ADR 0021): `--debug` selects
/// `dx_debug` (`dbg`), the bare invocation selects `dx_dev`
/// (`fastbuild`), and `--release` selects `dx_release` (`opt`). There
/// is no `--dev` flag: the bare invocation already means the middle
/// mode and keeps `dx build` short.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Debug,
    Dev,
    Release,
}

impl Profile {
    /// Stable profile name forwarded as `DX_PROFILE` to deploy programs.
    pub fn name(self) -> &'static str {
        match self {
            Profile::Debug => "debug",
            Profile::Dev => "dev",
            Profile::Release => "release",
        }
    }

    /// Shared Bazel config backing the profile (ADR 0021).
    pub fn config(self) -> &'static str {
        match self {
            Profile::Debug => "dx_debug",
            Profile::Dev => "dx_dev",
            Profile::Release => "dx_release",
        }
    }

    /// Required `--config=` flag pinning the profile on a workflow argv.
    pub fn config_flag(self) -> String {
        format!("--config={}", self.config())
    }

    /// Command default: `deploy` defaults to release, every other
    /// command defaults to dev.
    pub fn default_for(command: Command) -> Self {
        match command {
            Command::Deploy => Profile::Release,
            _ => Profile::Dev,
        }
    }

    /// Parses a deploy-target `profile` attribute value (`debug`, `dev`,
    /// `release`): `None` for anything else so analysis diagnostics own
    /// the spelling error.
    pub fn parse_attr(value: &str) -> Option<Self> {
        match value {
            "debug" => Some(Profile::Debug),
            "dev" => Some(Profile::Dev),
            "release" => Some(Profile::Release),
            _ => None,
        }
    }
}

/// Environment variable forwarding the resolved profile to the deploy
/// program (issue #179 item 3).
pub const DX_PROFILE_ENV: &str = "DX_PROFILE";

/// Precedence for the effective profile (issue #179 item 2): the
/// explicit `--debug`/`--release` flag wins over the deploy target
/// `profile` attribute, which wins over the command default. Build,
/// run, and test have no target attribute, so they resolve flag over
/// default; deploy resolves flag over target attribute over the
/// release default.
pub fn resolve_profile(flag: Option<Profile>, attr: Option<Profile>, default: Profile) -> Profile {
    flag.or(attr).unwrap_or(default)
}

/// One `--report <format>=<destination>` request. Format support is
/// validated against the command registry during planning; parsing only
/// checks the `format=destination` shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportRequest {
    pub format: String,
    pub destination: String,
}

/// Parsed `dx` invocation: command mode, global options, explicit scope,
/// and Bazel command options after `--`. An empty `targets` selects the
/// repository scope (`//...`). `bazel_clean` is set only by
/// `dx clean --bazel` (additionally forward `bazel clean` after pruning).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub command: Command,
    pub check: bool,
    /// `--debug` (build/run/test only).
    pub debug: bool,
    /// `--release` (build/run/test only).
    pub release: bool,
    pub workspace: Option<String>,
    pub dry_run: bool,
    pub quiet: bool,
    /// `--verbose` (issue #222): structured `tracing` diagnostics on
    /// stderr; orthogonal to `--quiet` (which suppresses human summaries).
    /// Default stays byte-identical (warn+error only).
    pub verbose: bool,
    pub output: OutputMode,
    pub reports: Vec<ReportRequest>,
    pub fail_on: Threshold,
    /// `dx coverage --min-coverage <percent>`: required line-coverage
    /// percent over the collected LCOV (Coverage only; `None` collects
    /// without enforcing a threshold).
    pub min_coverage: Option<u32>,
    pub targets: Vec<String>,
    pub bazel_options: Vec<String>,
    pub bazel_clean: bool,
    /// `dx version --pin <version>`: re-pin target (Version only).
    pub pin: Option<String>,
    /// `dx version --rollback`: re-pin the recorded previous release
    /// (Version only; rejected together with `--pin`).
    pub rollback: bool,
    /// Inspect wrappers use `cquery` instead of `query` (Owners, Deps,
    /// Why only).
    pub configured: bool,
}

impl Invocation {
    /// `command_started` mode: `check` for `--check`, else `default`.
    pub fn mode(self) -> &'static str {
        if self.check {
            "check"
        } else {
            "default"
        }
    }

    /// Explicit `--debug`/`--release` flag as a [`Profile`]: `None` for
    /// the bare invocation (which resolves to the command default).
    /// Parsing rejects both flags together, so the arms are exclusive.
    pub fn profile_flag(&self) -> Option<Profile> {
        if self.debug {
            Some(Profile::Debug)
        } else if self.release {
            Some(Profile::Release)
        } else {
            None
        }
    }

    /// Effective profile under issue #179 precedence: explicit flag over
    /// the command default. Deploy resolves flag over the target
    /// `profile` attribute over the release default (issue #180); the
    /// target attribute is read during execution via cquery, so this
    /// returns flag over command default and execution refines it.
    /// Build/run/test have no target attribute.
    pub fn profile(&self) -> Profile {
        resolve_profile(
            self.profile_flag(),
            None,
            Profile::default_for(self.command),
        )
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn command_names_are_stable() {
        assert_eq!(Command::Audit.name(), "audit");
        assert_eq!(Command::Update.name(), "update");
        assert_eq!(Command::Lint.name(), "lint");
        assert_eq!(Command::Typecheck.name(), "typecheck");
        assert_eq!(Command::Format.name(), "format");
        assert_eq!(Command::Generate.name(), "generate");
        assert_eq!(Command::Build.name(), "build");
        assert_eq!(Command::Test.name(), "test");
        assert_eq!(Command::Coverage.name(), "coverage");
        assert_eq!(Command::Run.name(), "run");
        assert_eq!(Command::Codegen.name(), "codegen");
        assert_eq!(Command::Env.name(), "env");
        assert_eq!(Command::Setup.name(), "setup");
        assert!(Command::Codegen.is_managed());
        assert!(Command::Env.is_managed());
        assert!(Command::Setup.is_managed());
        assert!(!Command::Codegen.is_workflow());
        assert!(!Command::Codegen.is_adoption());
        assert!(!Command::Clean.is_managed());
        assert_eq!(Command::Bazel.name(), "bazel");
        assert!(!Command::Bazel.is_workflow());
        assert!(!Command::Bazel.is_adoption());
        assert!(!Command::Lint.is_workflow());
        assert!(!Command::Typecheck.is_workflow());
        assert!(!Command::Format.is_workflow());
        assert!(!Command::Generate.is_workflow());
        assert!(Command::Build.is_workflow());
        assert!(Command::Test.is_workflow());
        assert!(Command::Coverage.is_workflow());
        assert!(Command::Run.is_workflow());
    }

    #[test]
    fn command_names_are_clap_value_enum() {
        use clap::ValueEnum;
        // Every stable name round-trips through the derive, case-sensitively.
        let commands = [
            Command::Audit,
            Command::Lint,
            Command::Typecheck,
            Command::Format,
            Command::Generate,
            Command::Build,
            Command::Test,
            Command::Coverage,
            Command::Run,
            Command::Deploy,
            Command::Check,
            Command::Fix,
            Command::Clean,
            Command::Update,
            Command::Codegen,
            Command::Env,
            Command::Setup,
            Command::Init,
            Command::Hooks,
            Command::Status,
            Command::Version,
            Command::Watch,
            Command::Owners,
            Command::Deps,
            Command::Why,
            Command::Completion,
            Command::Bazel,
        ];
        assert_eq!(commands.len(), Command::value_variants().len());
        for command in commands {
            assert_eq!(Command::parse(command.name()), Some(command));
            assert_eq!(
                command.to_possible_value().expect("named").get_name(),
                command.name()
            );
        }
        assert_eq!(Command::parse("Lint"), None);
        assert_eq!(Command::parse("type-check"), None);
        assert_eq!(Command::parse("dx"), None);
    }

    #[test]
    fn bare_command_parses_with_defaults() {
        let got = parse(&args(&["lint"])).expect("parse");
        assert_eq!(got.command, Command::Lint);
        assert!(!got.check);
        assert!(!got.debug);
        assert!(!got.release);
        assert_eq!(got.workspace, None);
        assert!(!got.dry_run);
        assert!(!got.quiet);
        assert!(!got.verbose);
        assert_eq!(got.output, OutputMode::Text { quiet: false });
        assert!(got.reports.is_empty());
        assert_eq!(got.fail_on, Threshold::Warning);
        assert!(got.targets.is_empty());
        assert!(got.bazel_options.is_empty());
        assert_eq!(got.mode(), "default");
    }

    #[test]
    fn min_coverage_parses_for_coverage_only() {
        let got = parse(&args(&["coverage", "--min-coverage", "80"])).expect("parse");
        assert_eq!(got.command, Command::Coverage);
        assert_eq!(got.min_coverage, Some(80));
        let inline = parse(&args(&["coverage", "--min-coverage=100"])).expect("parse");
        assert_eq!(inline.min_coverage, Some(100));
        let zero = parse(&args(&["coverage", "--min-coverage=0"])).expect("parse");
        assert_eq!(zero.min_coverage, Some(0));
        let bare = parse(&args(&["coverage"])).expect("parse");
        assert_eq!(bare.min_coverage, None);
    }

    #[test]
    fn min_coverage_rejects_bad_values_and_other_commands() {
        assert_eq!(
            parse(&args(&["coverage", "--min-coverage=eighty"])),
            Err(ArgsError::BadMinCoverage {
                value: "eighty".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["coverage", "--min-coverage=101"])),
            Err(ArgsError::BadMinCoverage {
                value: "101".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["coverage", "--min-coverage"])),
            Err(ArgsError::MissingValue {
                option: "--min-coverage".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["test", "--min-coverage=80"])),
            Err(ArgsError::UnsupportedOption {
                command: "test",
                option: "--min-coverage".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--min-coverage=80"])),
            Err(ArgsError::UnsupportedOption {
                command: "lint",
                option: "--min-coverage".to_owned(),
            })
        );
    }

    #[test]
    fn check_selects_check_mode() {
        let got = parse(&args(&["format", "--check"])).expect("parse");
        assert_eq!(got.command, Command::Format);
        assert!(got.check);
        assert_eq!(got.mode(), "check");
    }

    #[test]
    fn generate_parses_repo_wide_with_bazel_options() {
        let got = parse(&args(&["generate"])).expect("parse");
        assert_eq!(got.command, Command::Generate);
        assert!(!got.check);
        assert_eq!(got.output, OutputMode::Text { quiet: false });
        assert!(got.targets.is_empty());
        assert!(got.bazel_options.is_empty());
        assert_eq!(got.mode(), "default");
        let scoped =
            parse(&args(&["generate", "--check", "//a:one", "--", "--jobs=4"])).expect("parse");
        assert_eq!(scoped.command, Command::Generate);
        assert!(scoped.check);
        assert_eq!(scoped.targets, args(&["//a:one"]));
        assert_eq!(scoped.bazel_options, args(&["--jobs=4"]));
    }

    #[test]
    fn globals_parse_before_and_after_command() {
        let got = parse(&args(&[
            "--workspace",
            "/repo",
            "--dry-run",
            "--quiet",
            "typecheck",
            "--output=json",
            "--fail-on=error",
        ]))
        .expect("parse");
        assert_eq!(got.command, Command::Typecheck);
        assert_eq!(got.workspace, Some("/repo".to_owned()));
        assert!(got.dry_run);
        assert!(got.quiet);
        assert_eq!(got.output, OutputMode::Json);
        assert_eq!(got.fail_on, Threshold::Error);
    }

    #[test]
    fn quiet_applies_to_text_output() {
        let got = parse(&args(&["lint", "--quiet"])).expect("parse");
        assert_eq!(got.output, OutputMode::Text { quiet: true });
    }

    #[test]
    fn verbose_parses_before_and_after_command_and_stays_orthogonal_to_quiet() {
        // Issue #222: `--verbose` enables tracing diagnostics without
        // changing the machine-output contract; `--quiet` still controls
        // summaries independently.
        let bare = parse(&args(&["lint", "--verbose"])).expect("parse");
        assert!(bare.verbose);
        assert!(!bare.quiet);
        let before = parse(&args(&["--verbose", "lint"])).expect("parse");
        assert!(before.verbose);
        let both = parse(&args(&["lint", "--quiet", "--verbose"])).expect("parse");
        assert!(both.quiet);
        assert!(both.verbose);
        assert_eq!(both.output, OutputMode::Text { quiet: true });
        let help = match parse(&args(&["--verbose", "--help"])) {
            Err(ArgsError::Help { text }) => text,
            other => panic!("want Help, got {other:?}"),
        };
        assert!(help.contains("--verbose"), "top-level help:\n{help}");
    }

    #[test]
    fn reports_are_repeatable_with_format_shape() {
        let got = parse(&args(&[
            "lint",
            "--report",
            "sarif=reports/lint.sarif",
            "--report=sarif=/tmp/extra.sarif",
        ]))
        .expect("parse");
        assert_eq!(
            got.reports,
            vec![
                ReportRequest {
                    format: "sarif".to_owned(),
                    destination: "reports/lint.sarif".to_owned(),
                },
                ReportRequest {
                    format: "sarif".to_owned(),
                    destination: "/tmp/extra.sarif".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn bazel_options_forward_verbatim_after_separator() {
        let got = parse(&args(&["lint", "--", "--jobs=4", "--", "nokeep_going"])).expect("parse");
        assert_eq!(got.bazel_options, args(&["--jobs=4", "--", "nokeep_going"]));
    }

    #[test]
    fn missing_command_fails() {
        assert_eq!(parse(&args(&[])), Err(ArgsError::MissingCommand));
        assert_eq!(parse(&args(&["--quiet"])), Err(ArgsError::MissingCommand));
    }

    #[test]
    fn unknown_command_fails() {
        assert_eq!(
            parse(&args(&["bogus"])),
            Err(ArgsError::UnknownCommand {
                command: "bogus".to_owned(),
                suggestion: None,
            })
        );
    }

    #[test]
    fn explicit_label_scope_parses_in_order() {
        let got = parse(&args(&["lint", "//a:one", "@repo//b/...", "//c/..."])).expect("parse");
        assert_eq!(got.targets, args(&["//a:one", "@repo//b/...", "//c/..."]));
    }

    #[test]
    fn path_scopes_parse_for_resolution() {
        let got = parse(&args(&[
            "lint",
            "src/main.rs",
            "quality/testdata/",
            "./x.py",
        ]))
        .expect("parse");
        assert_eq!(
            got.targets,
            args(&["src/main.rs", "quality/testdata/", "./x.py"])
        );
    }

    #[test]
    fn relative_and_empty_scope_fail() {
        assert_eq!(
            parse(&args(&["lint", ":corpus"])),
            Err(ArgsError::RelativeLabel {
                scope: ":corpus".to_owned(),
            })
        );
        assert_eq!(parse(&args(&["lint", ""])), Err(ArgsError::EmptyScope));
    }

    #[test]
    fn workflow_commands_parse_scopes_and_options() {
        for command in ["build", "test", "coverage"] {
            let got =
                parse(&args(&[command, "//a:one", "pkg/a.py", "--", "--jobs=4"])).expect("parse");
            assert_eq!(got.command.name(), command);
            assert_eq!(
                got.targets,
                args(&["//a:one", "pkg/a.py"]),
                "scopes parse for resolution"
            );
            assert_eq!(got.bazel_options, args(&["--jobs=4"]));
        }
    }

    #[test]
    fn workflow_commands_reject_quality_only_options() {
        assert_eq!(
            parse(&args(&["build", "--check"])),
            Err(ArgsError::UnsupportedOption {
                command: "build",
                option: "--check".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["test", "--fail-on=error"])),
            Err(ArgsError::UnsupportedOption {
                command: "test",
                option: "--fail-on".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["coverage", "--check", "--fail-on=info"])),
            Err(ArgsError::UnsupportedOption {
                command: "coverage",
                option: "--check".to_owned(),
            }),
            "check is reported before fail-on"
        );
        // Quality commands keep both options.
        assert!(parse(&args(&["lint", "--check", "--fail-on=error"])).is_ok());
    }

    #[test]
    fn unknown_options_fail() {
        assert_eq!(
            parse(&args(&["lint", "--jobs=4"])),
            Err(ArgsError::UnknownOption {
                option: "--jobs=4".to_owned(),
                suggestion: None,
            })
        );
        assert_eq!(
            parse(&args(&["lint", "-q"])),
            Err(ArgsError::UnknownOption {
                option: "-q".to_owned(),
                suggestion: None,
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--dry-run=yes"])),
            Err(ArgsError::UnknownOption {
                option: "--dry-run=yes".to_owned(),
                suggestion: None,
            })
        );
        assert_eq!(
            parse(&args(&["-"])),
            Err(ArgsError::UnknownOption {
                option: "-".to_owned(),
                suggestion: None,
            })
        );
    }

    #[test]
    fn typo_recovery_suggests_commands_and_options() {
        assert_eq!(
            parse(&args(&["lintt"])),
            Err(ArgsError::UnknownCommand {
                command: "lintt".to_owned(),
                suggestion: Some("lint".to_owned()),
            })
        );
        assert_eq!(
            parse(&args(&["--ouptut=json"])),
            Err(ArgsError::UnknownOption {
                option: "--ouptut=json".to_owned(),
                suggestion: Some("--output".to_owned()),
            })
        );
        // Hints are additive: the usage line stays, the hint appends.
        let command = parse(&args(&["lintt"])).unwrap_err().to_string();
        assert!(command.contains("unknown command \"lintt\""));
        assert!(command.contains("did you mean \"lint\"?"));
        let option = parse(&args(&["--ouptut=json"])).unwrap_err().to_string();
        assert!(option.contains("unknown option \"--ouptut=json\""));
        assert!(option.contains("did you mean \"--output\"?"));
    }

    #[test]
    fn missing_values_fail() {
        assert_eq!(
            parse(&args(&["lint", "--output"])),
            Err(ArgsError::MissingValue {
                option: "--output".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--workspace", "--quiet", "format"])),
            Err(ArgsError::MissingValue {
                option: "--workspace".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--workspace="])),
            Err(ArgsError::MissingValue {
                option: "--workspace".to_owned(),
            })
        );
    }

    #[test]
    fn bad_values_fail() {
        assert_eq!(
            parse(&args(&["lint", "--output=yaml"])),
            Err(ArgsError::BadOutput {
                value: "yaml".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--fail-on=never"])),
            Err(ArgsError::BadFailOn {
                value: "never".to_owned(),
            })
        );
        for bad in ["sarif", "=out.sarif", "sarif=", ""] {
            assert_eq!(
                parse(&args(&["lint", &format!("--report={bad}")])),
                Err(ArgsError::BadReport {
                    value: bad.to_owned(),
                })
            );
        }
    }

    #[test]
    fn error_display_reports_variant() {
        assert!(format!("{}", ArgsError::MissingCommand).contains("missing command"));
        assert!(format!(
            "{}",
            ArgsError::InvalidScope {
                scope: "//...".to_owned(),
            }
        )
        .contains("//..."));
        assert!(format!("{}", ArgsError::EmptyScope).contains("empty scope"));
        assert!(format!(
            "{}",
            ArgsError::RelativeLabel {
                scope: ":corpus".to_owned(),
            }
        )
        .contains(":corpus"));
        assert!(format!(
            "{}",
            ArgsError::UnknownCommand {
                command: "bogus".to_owned(),
                suggestion: None,
            }
        )
        .contains("bogus"));
        assert!(format!(
            "{}",
            ArgsError::UnknownOption {
                option: "--bogus".to_owned(),
                suggestion: None,
            }
        )
        .contains("--bogus"));
        assert!(format!(
            "{}",
            ArgsError::UnsupportedOption {
                command: "build",
                option: "--check".to_owned(),
            }
        )
        .contains("--check"));
        assert!(format!(
            "{}",
            ArgsError::MissingValue {
                option: "--output".to_owned(),
            }
        )
        .contains("--output"));
        assert!(format!(
            "{}",
            ArgsError::BadOutput {
                value: "yaml".to_owned(),
            }
        )
        .contains("yaml"));
        assert!(format!(
            "{}",
            ArgsError::BadFailOn {
                value: "never".to_owned(),
            }
        )
        .contains("never"));
        assert!(format!(
            "{}",
            ArgsError::BadReport {
                value: "sarif".to_owned(),
            }
        )
        .contains("sarif"));
    }

    #[test]
    fn inline_flag_values_and_empty_workspace_fail() {
        assert_eq!(
            parse(&args(&["lint", "--workspace", ""])),
            Err(ArgsError::MissingValue {
                option: "--workspace".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--quiet=x"])),
            Err(ArgsError::UnknownOption {
                option: "--quiet=x".to_owned(),
                suggestion: None,
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--check=x"])),
            Err(ArgsError::UnknownOption {
                option: "--check=x".to_owned(),
                suggestion: None,
            })
        );
    }

    #[test]
    fn clean_parses_dry_run_and_bazel() {
        let got = parse(&args(&["clean"])).expect("parse");
        assert_eq!(got.command, Command::Clean);
        assert!(!got.bazel_clean);
        assert!(!got.dry_run);
        let got = parse(&args(&["clean", "--dry-run", "--bazel"])).expect("parse");
        assert_eq!(got.command, Command::Clean);
        assert!(got.dry_run);
        assert!(got.bazel_clean);
        assert_eq!(Command::Clean.name(), "clean");
        assert!(!Command::Clean.is_workflow());
        assert!(!Command::Clean.is_umbrella());
    }

    #[test]
    fn clean_rejects_scopes_and_quality_options() {
        assert_eq!(
            parse(&args(&["clean", "--check"])),
            Err(ArgsError::UnsupportedOption {
                command: "clean",
                option: "--check".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["clean", "--fail-on=error"])),
            Err(ArgsError::UnsupportedOption {
                command: "clean",
                option: "--fail-on".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["clean", "--output=json"])),
            Err(ArgsError::UnsupportedOption {
                command: "clean",
                option: "--output=json".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["clean", "--report=sarif=out.sarif"])),
            Err(ArgsError::UnsupportedOption {
                command: "clean",
                option: "--report=sarif=out.sarif".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["clean", "//a:one"])),
            Err(ArgsError::UnsupportedOption {
                command: "clean",
                option: "//a:one".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["clean", "--", "--jobs=4"])),
            Err(ArgsError::UnsupportedOption {
                command: "clean",
                option: "--".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["clean", "--bazel=yes"])),
            Err(ArgsError::UnknownOption {
                option: "--bazel=yes".to_owned(),
                suggestion: None,
            })
        );
        // `--bazel` belongs to clean only.
        assert_eq!(
            parse(&args(&["lint", "--bazel"])),
            Err(ArgsError::UnsupportedOption {
                command: "lint",
                option: "--bazel".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["build", "--bazel"])),
            Err(ArgsError::UnsupportedOption {
                command: "build",
                option: "--bazel".to_owned(),
            })
        );
    }

    #[test]
    fn audit_update_parse_and_reject_unsupported_options() {
        let audit = parse(&args(&["audit"])).expect("parse audit");
        assert_eq!(audit.command, Command::Audit);
        assert_eq!(audit.command.name(), "audit");
        assert!(audit.command.is_audit_update());
        assert!(!audit.command.is_workflow());
        assert!(!audit.command.is_adoption());
        assert!(!audit.command.is_managed());
        assert!(audit.targets.is_empty());
        let families = parse(&args(&["audit", "security"])).expect("parse audit family");
        assert_eq!(families.targets, vec!["security".to_owned()]);
        let update = parse(&args(&["update"])).expect("parse update");
        assert_eq!(update.command, Command::Update);
        assert_eq!(update.command.name(), "update");
        assert!(update.command.is_audit_update());
        let selected = parse(&args(&["update", "crates"])).expect("parse update selector");
        assert_eq!(selected.targets, vec!["crates".to_owned()]);
        assert_eq!(
            parse(&args(&["audit", "--check"])),
            Err(ArgsError::UnsupportedOption {
                command: "audit",
                option: "--check".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["update", "--check"])),
            Err(ArgsError::UnsupportedOption {
                command: "update",
                option: "--check".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["update", "--fail-on=error"])),
            Err(ArgsError::UnsupportedOption {
                command: "update",
                option: "--fail-on".to_owned(),
            })
        );
        // `update` supports `--output=json` (issue #200): dry-run planning
        // and the deferred-live error stream NDJSON like `audit`.
        let got = parse(&args(&["update", "--output=json"])).expect("update json");
        assert_eq!(got.command, Command::Update);
        assert_eq!(got.output, OutputMode::Json);
        assert_eq!(
            parse(&args(&["update", "--output=diff"])),
            Err(ArgsError::UnsupportedOption {
                command: "update",
                option: "--output=diff".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["update", "--report=sarif=out.sarif"])),
            Err(ArgsError::UnsupportedOption {
                command: "update",
                option: "--report=sarif=out.sarif".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["update", "--", "--jobs=4"])),
            Err(ArgsError::UnsupportedOption {
                command: "update",
                option: "--".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["audit", "--", "--jobs=4"])),
            Err(ArgsError::UnsupportedOption {
                command: "audit",
                option: "--".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["audit", ":target"])),
            Err(ArgsError::RelativeLabel {
                scope: ":target".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["update", ":target"])),
            Err(ArgsError::RelativeLabel {
                scope: ":target".to_owned(),
            })
        );
    }

    #[test]
    fn run_rejects_machine_output_and_reports() {
        assert_eq!(
            parse(&args(&["run", "//app:bin", "--output=json"])),
            Err(ArgsError::UnsupportedOption {
                command: "run",
                option: "--output=json".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["run", "//app:bin", "--output=diff"])),
            Err(ArgsError::UnsupportedOption {
                command: "run",
                option: "--output=diff".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["run", "--report=junit=out.xml"])),
            Err(ArgsError::UnsupportedOption {
                command: "run",
                option: "--report=junit=out.xml".to_owned(),
            })
        );
        let got = parse(&args(&["run", "//app:bin", "--", "--port=8080"])).expect("parse run");
        assert_eq!(got.command, Command::Run);
        assert_eq!(got.targets, vec!["//app:bin".to_owned()]);
        assert_eq!(got.bazel_options, vec!["--port=8080".to_owned()]);
    }

    #[test]
    fn output_contract_has_no_silent_ignore() {
        // Issue #200: every command either supports a machine-output mode or
        // rejects it pre-exec with `UnsupportedOption`. Silent ignore (accept
        // the flag, print text anyway) is never allowed.
        //
        // JSON-capable: quality, generate, workflow build/test/coverage,
        // umbrellas, audit, update, status.
        for command in [
            "lint",
            "typecheck",
            "format",
            "generate",
            "build",
            "test",
            "coverage",
            "check",
            "fix",
            "audit",
            "update",
        ] {
            let got = parse(&args(&[command, "--output=json"])).expect("json capable");
            assert_eq!(got.output, OutputMode::Json, "command: {command}");
            assert!(got.command.supports_json(), "command: {command}");
        }
        let got = parse(&args(&["status", "--output=json"])).expect("status json");
        assert_eq!(got.output, OutputMode::Json);
        assert!(Command::Status.supports_json());
        // Diff-capable: patch producers only.
        for command in ["lint", "typecheck", "format", "generate", "check", "fix"] {
            let got = parse(&args(&[command, "--output=diff"])).expect("diff capable");
            assert_eq!(got.output, OutputMode::Diff, "command: {command}");
            assert!(got.command.supports_diff(), "command: {command}");
        }
        // Text-only exemptions fail fast on both machine modes.
        // (`bazel` takes dx flags before the command word; tokens after it
        // forward verbatim to the launcher.)
        for words in [
            vec!["clean", "--output=json"],
            vec!["clean", "--output=diff"],
            vec!["codegen", "--output=json"],
            vec!["env", "--output=diff"],
            vec!["setup", "--output=json"],
            vec!["--output=json", "bazel", "version"],
            vec!["init", "--output=json"],
            vec!["init", "proj", "--output=diff"],
            vec!["hooks", "status", "--output=json"],
            vec!["version", "--output=json"],
            vec!["watch", "test", "--output=json"],
            vec!["owners", "//a:one", "--output=json"],
            vec!["deps", "//a:one", "--output=diff"],
            vec!["why", "src/main.rs", "//a:one", "--output=json"],
            vec!["completion", "bash", "--output=json"],
        ] {
            assert!(
                matches!(
                    parse(&args(&words)),
                    Err(ArgsError::UnsupportedOption { .. })
                ),
                "words: {words:?}"
            );
        }
        // No patch to emit: JSON-capable but diff-rejecting commands fail
        // fast on `--output=diff` instead of printing empty stdout.
        for words in [
            vec!["build", "//a:one", "--output=diff"],
            vec!["test", "//a:one", "--output=diff"],
            vec!["coverage", "--output=diff"],
            vec!["audit", "--output=diff"],
            vec!["update", "--output=diff"],
            vec!["status", "--output=diff"],
        ] {
            assert_eq!(
                parse(&args(&words)),
                Err(ArgsError::UnsupportedOption {
                    command: words[0],
                    option: "--output=diff".to_owned(),
                }),
                "words: {words:?}"
            );
        }
    }

    #[test]
    fn bazel_forwards_verbatim_and_rejects_dx_options() {
        let got = parse(&args(&["bazel", "build", "//...", "--", "--jobs=4"])).expect("parse");
        assert_eq!(got.command, Command::Bazel);
        assert_eq!(got.command.name(), "bazel");
        assert!(!got.command.is_workflow());
        assert!(!got.command.is_adoption());
        assert!(got.targets.is_empty());
        assert_eq!(
            got.bazel_options,
            vec![
                "build".to_owned(),
                "//...".to_owned(),
                "--jobs=4".to_owned()
            ]
        );
        // Tokens after the command word forward verbatim even when
        // they look like dx options: dx globals must precede `bazel`.
        for words in [
            vec!["bazel", "--jobs=4"],
            vec!["bazel", "--check"],
            vec!["bazel", "--pin=0.1.0"],
            vec!["bazel", "version", "--configured"],
            vec!["bazel", "--", "--fail-on=error"],
        ] {
            let got = parse(&args(&words)).expect("verbatim");
            assert_eq!(got.command, Command::Bazel, "words: {words:?}");
            assert!(got.targets.is_empty(), "words: {words:?}");
        }
        let got = parse(&args(&["bazel", "build", "--jobs", "4"])).expect("verbatim");
        assert_eq!(
            got.bazel_options,
            vec!["build".to_owned(), "--jobs".to_owned(), "4".to_owned()]
        );
        // dx-owned options before the command word still fail fast so
        // launcher flags can never be misread as dx flags.
        for words in [
            vec!["--output=json", "bazel", "version"],
            vec!["--check", "bazel", "version"],
            vec!["--pin=0.1.0", "bazel", "version"],
            vec!["--configured", "bazel", "version"],
            vec!["--fail-on=error", "bazel", "build", "//..."],
            vec!["--report=sarif=x.sarif", "bazel", "build"],
        ] {
            assert!(
                matches!(
                    parse(&args(&words)),
                    Err(ArgsError::UnsupportedOption { .. })
                ),
                "words: {words:?}"
            );
        }
    }

    #[test]
    fn version_rollback_check_and_configured_parse() {
        let got = parse(&args(&["version", "--rollback"])).expect("parse");
        assert_eq!(got.command, Command::Version);
        assert!(got.rollback);
        let got = parse(&args(&["version", "--check"])).expect("parse");
        assert!(got.check);
        let got = parse(&args(&["deps", "--configured", "//a:one"])).expect("parse");
        assert_eq!(got.command, Command::Deps);
        assert!(got.configured);
        for words in [
            vec!["status", "--rollback"],
            vec!["status", "--check"],
            vec!["owners", "//a:one", "--pin=0.1.0"],
            vec!["build", "//a:one", "--configured"],
            vec!["lint", "--rollback"],
            vec!["check", "//...", "--configured"],
            vec!["status", "--pin=0.1.0"],
        ] {
            assert!(
                matches!(
                    parse(&args(&words)),
                    Err(ArgsError::UnsupportedOption { .. })
                ),
                "words: {words:?}"
            );
        }
    }

    #[test]
    fn managed_commands_parse_repo_and_exact_scopes() {
        for command in ["codegen", "env", "setup"] {
            let got = parse(&args(&[command])).expect("parse");
            assert!(got.command.is_managed());
            assert!(got.targets.is_empty());
            assert!(got.bazel_options.is_empty());
            let got = parse(&args(&[command, "//a:one", "--", "--jobs=4"])).expect("scoped parse");
            assert_eq!(got.targets, args(&["//a:one"]));
            assert_eq!(got.bazel_options, args(&["--jobs=4"]));
            let got = parse(&args(&[command, "@repo//pkg:lib"])).expect("external label");
            assert_eq!(got.targets, args(&["@repo//pkg:lib"]));
        }
        // Scope rules are the shared setup scope rules: multiple
        // positionals, patterns, and non-labels fail before execution.
        assert_eq!(
            parse(&args(&["codegen", "//a:one", "//b:two"])),
            Err(ArgsError::UnsupportedOption {
                command: "codegen",
                option: "//b:two".to_owned(),
            })
        );
        for words in [
            vec!["env", "//a/..."],
            vec!["setup", "src/main.rs"],
            vec!["codegen", ":target"],
            vec!["env", ""],
            vec!["setup", "--config=release"],
        ] {
            assert!(
                matches!(
                    parse(&args(&words)),
                    Err(ArgsError::EmptyScope)
                        | Err(ArgsError::RelativeLabel { .. })
                        | Err(ArgsError::InvalidScope { .. })
                        | Err(ArgsError::UnknownOption { .. })
                ),
                "words: {words:?}"
            );
        }
        // Quality-only, version-only, and clean-only options
        // fail fast on managed commands.
        for words in [
            vec!["codegen", "--check"],
            vec!["env", "--fail-on=error"],
            vec!["setup", "--output=json"],
            vec!["codegen", "--report=sarif=out.sarif"],
            vec!["env", "--pin=0.1.0"],
            vec!["setup", "--rollback"],
            vec!["codegen", "--configured"],
            vec!["env", "--configured"],
            vec!["setup", "--pin=0.2.0"],
            vec!["codegen", "--bazel"],
        ] {
            assert!(
                matches!(
                    parse(&args(&words)),
                    Err(ArgsError::UnsupportedOption { .. })
                ),
                "words: {words:?}"
            );
        }
    }

    #[test]
    fn why_requires_file_and_label() {
        let got = parse(&args(&["why", "src/a.rs", "//a:one"])).expect("parse");
        assert_eq!(got.command, Command::Why);
        assert_eq!(
            got.targets,
            vec!["src/a.rs".to_owned(), "//a:one".to_owned()]
        );
        for words in [
            vec!["why", "src/a.rs"],
            vec!["why", "src/a.rs", "//a:one", "//b:two"],
        ] {
            assert_eq!(
                parse(&args(&words)),
                Err(ArgsError::MissingValue {
                    option: "<file> <label>".to_owned(),
                }),
                "words: {words:?}"
            );
        }
    }

    #[test]
    fn profile_vocabulary_maps_to_shared_configs() {
        assert_eq!(Profile::Debug.name(), "debug");
        assert_eq!(Profile::Dev.name(), "dev");
        assert_eq!(Profile::Release.name(), "release");
        assert_eq!(Profile::Debug.config(), "dx_debug");
        assert_eq!(Profile::Dev.config(), "dx_dev");
        assert_eq!(Profile::Release.config(), "dx_release");
        assert_eq!(Profile::Debug.config_flag(), "--config=dx_debug");
        assert_eq!(Profile::Dev.config_flag(), "--config=dx_dev");
        assert_eq!(Profile::Release.config_flag(), "--config=dx_release");
        assert_eq!(DX_PROFILE_ENV, "DX_PROFILE");
    }

    #[test]
    fn profile_attr_parses_deploy_vocabulary() {
        assert_eq!(Profile::parse_attr("debug"), Some(Profile::Debug));
        assert_eq!(Profile::parse_attr("dev"), Some(Profile::Dev));
        assert_eq!(Profile::parse_attr("release"), Some(Profile::Release));
        assert_eq!(Profile::parse_attr("staging"), None);
        assert_eq!(Profile::parse_attr(""), None);
    }

    #[test]
    fn profile_precedence_is_flag_over_attr_over_default() {
        // Explicit flag wins over the deploy target attribute.
        assert_eq!(
            resolve_profile(Some(Profile::Debug), Some(Profile::Release), Profile::Dev),
            Profile::Debug
        );
        // Target attribute wins over the command default.
        assert_eq!(
            resolve_profile(None, Some(Profile::Release), Profile::Dev),
            Profile::Release
        );
        // Bare invocation resolves to the command default.
        assert_eq!(resolve_profile(None, None, Profile::Dev), Profile::Dev);
        assert_eq!(
            resolve_profile(None, None, Profile::Release),
            Profile::Release
        );
    }

    #[test]
    fn profile_flags_parse_on_build_run_test() {
        for command in ["build", "run", "test"] {
            let bare = parse(&args(&[command])).expect("bare parse");
            assert!(!bare.debug);
            assert!(!bare.release);
            assert_eq!(bare.profile_flag(), None);
            assert_eq!(bare.profile(), Profile::Dev);
            let debug = parse(&args(&[command, "--debug"])).expect("debug parse");
            assert!(debug.debug);
            assert!(!debug.release);
            assert_eq!(debug.profile_flag(), Some(Profile::Debug));
            assert_eq!(debug.profile(), Profile::Debug);
            let release = parse(&args(&[command, "--release"])).expect("release parse");
            assert!(!release.debug);
            assert!(release.release);
            assert_eq!(release.profile_flag(), Some(Profile::Release));
            assert_eq!(release.profile(), Profile::Release);
        }
        // Flags parse before the command word too.
        let got = parse(&args(&["--debug", "build"])).expect("parse");
        assert_eq!(got.profile_flag(), Some(Profile::Debug));
        let got = parse(&args(&["test", "--release", "//a:t"])).expect("parse");
        assert_eq!(got.profile_flag(), Some(Profile::Release));
    }

    #[test]
    fn profile_flags_reject_conflicts_and_foreign_commands() {
        for command in ["build", "run", "test"] {
            assert_eq!(
                parse(&args(&[command, "--debug", "--release"])),
                Err(ArgsError::ConflictingProfiles)
            );
        }
        // Coverage, quality, umbrellas, managed, and adoption commands
        // own no profile flags.
        for words in [
            vec!["coverage", "--debug"],
            vec!["coverage", "--release"],
            vec!["lint", "--debug"],
            vec!["check", "--release"],
            vec!["generate", "--debug"],
            vec!["codegen", "--release"],
            vec!["status", "--debug"],
        ] {
            assert!(
                matches!(
                    parse(&args(&words)),
                    Err(ArgsError::UnsupportedOption { .. })
                ),
                "words: {words:?}"
            );
        }
    }

    #[test]
    fn top_level_help_lists_commands_and_flags() {
        for flag in ["--help", "-h"] {
            let text = match parse(&args(&[flag])) {
                Err(ArgsError::Help { text }) => text,
                other => panic!("{flag}: want Help, got {other:?}"),
            };
            for needle in [
                "dx",
                "lint",
                "build",
                "--workspace",
                "--output",
                "--fail-on",
                "Exit codes",
            ] {
                assert!(text.contains(needle), "{flag}: missing {needle:?}");
            }
        }
    }

    #[test]
    fn help_process_exits_zero_with_brand() {
        // Process pilot (issue #225): the built binary serves `--help`
        // hermetically — parsing answers before workspace discovery, so
        // no workspace is needed — with exit 0 and the brand on stdout.
        // The binary arrives via test `data`; the path joins the Bazel
        // runfiles layout (`$TEST_SRCDIR/$TEST_WORKSPACE/cli/cli/dx`).
        let root = std::env::var("TEST_SRCDIR").expect("TEST_SRCDIR is set under Bazel");
        let workspace = std::env::var("TEST_WORKSPACE").expect("TEST_WORKSPACE is set under Bazel");
        let binary = std::path::Path::new(&root)
            .join(workspace)
            .join("cli/cli/dx");
        assert_cmd::Command::new(binary)
            .arg("--help")
            .assert()
            .success()
            .stdout(predicates::str::contains("Transparent UI over Bazel"));
    }

    #[test]
    fn per_command_help_covers_usage_scopes_exits_and_output() {
        for argv in [
            vec!["lint", "--help"],
            vec!["--help", "lint"],
            vec!["clean", "-h"],
        ] {
            let text = match parse(&args(&argv)) {
                Err(ArgsError::Help { text }) => text,
                other => panic!("{argv:?}: want Help, got {other:?}"),
            };
            let command = argv
                .iter()
                .find(|word| Command::parse(word).is_some())
                .expect("command");
            for needle in [
                command,
                "Usage:",
                "Scopes:",
                "Exit codes:",
                "Output:",
                "--workspace",
            ] {
                assert!(text.contains(needle), "{argv:?}: missing {needle:?}");
            }
        }
    }

    #[test]
    fn per_command_help_names_owned_flags_and_reconciles_bazel_naming() {
        // Issue #211: per-command `--bazel`/`--configured` naming is
        // reconciled by keeping both names (different semantics) and
        // documenting the distinction; usage/help must show the owned
        // flags instead of hiding them.
        let clean = match parse(&args(&["clean", "--help"])) {
            Err(ArgsError::Help { text }) => text,
            other => panic!("clean --help: want Help, got {other:?}"),
        };
        assert!(
            clean.contains("clean [--dry-run] [--bazel]"),
            "clean usage:\n{clean}"
        );
        assert!(clean.contains("--bazel"), "clean flags:\n{clean}");
        assert!(
            clean.contains("distinct from `dx bazel`"),
            "clean disambiguation:\n{clean}"
        );
        for command in ["owners", "deps", "why"] {
            let text = match parse(&args(&[command, "--help"])) {
                Err(ArgsError::Help { text }) => text,
                other => panic!("{command} --help: want Help, got {other:?}"),
            };
            assert!(text.contains("[--configured]"), "{command} usage:\n{text}");
            assert!(
                text.contains("distinct from `dx clean --bazel`"),
                "{command} disambiguation:\n{text}"
            );
        }
        let coverage = match parse(&args(&["coverage", "--help"])) {
            Err(ArgsError::Help { text }) => text,
            other => panic!("coverage --help: want Help, got {other:?}"),
        };
        assert!(
            coverage.contains("--min-coverage"),
            "coverage flags:\n{coverage}"
        );
        let build = match parse(&args(&["build", "--help"])) {
            Err(ArgsError::Help { text }) => text,
            other => panic!("build --help: want Help, got {other:?}"),
        };
        assert!(build.contains("--debug"), "build flags:\n{build}");
        let version = match parse(&args(&["version", "--help"])) {
            Err(ArgsError::Help { text }) => text,
            other => panic!("version --help: want Help, got {other:?}"),
        };
        assert!(version.contains("--pin"), "version flags:\n{version}");
        // Every per-command help names its owned flags line. `dx bazel
        // --help` forwards verbatim to Bazel by design (raw passthrough
        // owns the tail), so Bazel help is checked via the renderer
        // directly rather than the parse path.
        for argv in [["lint", "--help"], ["status", "--help"]] {
            let text = match parse(&args(&argv)) {
                Err(ArgsError::Help { text }) => text,
                other => panic!("{argv:?}: want Help, got {other:?}"),
            };
            assert!(text.contains("Per-command flags:"), "{argv:?}:\n{text}");
        }
        let bazel_help = help::render_command_help(Command::Bazel);
        assert!(
            bazel_help.contains("Per-command flags:"),
            "bazel help:\n{bazel_help}"
        );
    }

    #[test]
    fn help_value_option_payload_is_not_a_command() {
        // `--output bazel` binds bazel as the value, so `--help` still
        // renders top-level help rather than bazel help.
        let text = match parse(&args(&["--output", "bazel", "--help"])) {
            Err(ArgsError::Help { text }) => text,
            other => panic!("want Help, got {other:?}"),
        };
        assert!(text.contains("--output"));
    }
}
