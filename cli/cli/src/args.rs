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
//! the `suggest` module, build-profile vocabulary in the `profile`
//! module, shared invocation types in the `invocation` module, error
//! vocabulary in the `error` module, and invocation parsing in the
//! `parser` module (the `parse` domain; named `parser` so the module
//! and the `parse` function coexist). This facade keeps the re-exports;
//! the public paths stay `crate::args::Command`,
//! `crate::args::{COMPLETION_SHELLS, render_completion}`,
//! `crate::args::{Invocation, ReportRequest}`,
//! `crate::args::ArgsError`, and
//! `crate::args::{parse, cli_command}` via the re-exports below.

pub mod command;
pub mod completion;
pub mod error;
pub mod help;
pub mod invocation;
pub mod parser;
pub mod profile;
pub mod suggest;

pub use command::Command;
pub use completion::{render_completion, COMPLETION_SHELLS};
pub use error::ArgsError;
pub use invocation::{Invocation, ReportRequest};
pub use parser::{cli_command, parse};
pub use profile::{resolve_profile, Profile, DX_PROFILE_ENV};

#[cfg(test)]
mod tests {
    use super::*;
    use dx_output::{OutputMode, Threshold};

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
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
}
