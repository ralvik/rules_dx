//! Invocation parsing for the `dx` quality commands (M07 WP1+WP3).
//!
//! Contract: `docs/cli/cli-contract.md#invocation-shape`. Scope positionals
//! accept explicit Bazel labels and patterns (`//...`, `//pkg:target`,
//! `@repo//pkg/...`); anything else fails with
//! [`ArgsError::ScopeNotSupported`] because path resolution belongs to a
//! later milestone. With no scope the repository operation (`//...`) runs.

use dx_output::{OutputMode, Threshold};

/// Quality command selected by the first positional argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Lint,
    Typecheck,
    Format,
}

impl Command {
    /// Stable command name used in summaries and `command_started`.
    pub fn name(self) -> &'static str {
        match self {
            Command::Lint => "lint",
            Command::Typecheck => "typecheck",
            Command::Format => "format",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "lint" => Some(Command::Lint),
            "typecheck" => Some(Command::Typecheck),
            "format" => Some(Command::Format),
            _ => None,
        }
    }
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
/// repository scope (`//...`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub command: Command,
    pub check: bool,
    pub workspace: Option<String>,
    pub dry_run: bool,
    pub quiet: bool,
    pub output: OutputMode,
    pub reports: Vec<ReportRequest>,
    pub fail_on: Threshold,
    pub targets: Vec<String>,
    pub bazel_options: Vec<String>,
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
}

/// Invocation parsing failure. Every variant is a CLI-detected
/// pre-execution usage error (exit code 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgsError {
    MissingCommand,
    UnknownCommand { command: String },
    UnknownOption { option: String },
    MissingValue { option: String },
    BadOutput { value: String },
    BadFailOn { value: String },
    BadReport { value: String },
    ScopeNotSupported { scope: String },
}

impl std::fmt::Display for ArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgsError::MissingCommand => write!(f, "missing command: want lint|typecheck|format"),
            ArgsError::UnknownCommand { command } => {
                write!(f, "unknown command {command:?}: want lint|typecheck|format")
            }
            ArgsError::UnknownOption { option } => write!(f, "unknown option {option:?}"),
            ArgsError::MissingValue { option } => write!(f, "missing value for {option:?}"),
            ArgsError::BadOutput { value } => {
                write!(f, "unknown --output {value:?}: want text|diff|json")
            }
            ArgsError::BadFailOn { value } => {
                write!(f, "unknown --fail-on {value:?}: want info|warning|error")
            }
            ArgsError::BadReport { value } => {
                write!(
                    f,
                    "malformed --report {value:?}: want <format>=<destination>"
                )
            }
            ArgsError::ScopeNotSupported { scope } => {
                write!(
                    f,
                    "unsupported scope {scope:?}: want Bazel labels starting with // or @; file paths need later-milestone resolution"
                )
            }
        }
    }
}

impl std::error::Error for ArgsError {}

/// Splits a `--name=value` argument into its bare name and value.
fn split_inline(arg: &str) -> (&str, Option<&str>) {
    match arg.split_once('=') {
        Some((name, value)) => (name, Some(value)),
        None => (arg, None),
    }
}

/// Takes the value for a value option: the inline `=value` when present,
/// else the next argument, which must exist and must not be another
/// option or the `--` separator.
fn take_value<'a>(
    args: &'a [String],
    index: &mut usize,
    option: &str,
    inline: Option<&'a str>,
) -> Result<&'a str, ArgsError> {
    if let Some(value) = inline {
        return Ok(value);
    }
    match args.get(*index + 1) {
        Some(next) if !next.starts_with("--") && next != "--" => {
            *index += 1;
            Ok(next.as_str())
        }
        _ => Err(ArgsError::MissingValue {
            option: option.to_owned(),
        }),
    }
}

/// Parses one `--report` value into its `format=destination` shape.
fn parse_report(value: &str) -> Result<ReportRequest, ArgsError> {
    match value.split_once('=') {
        Some((format, destination)) if !format.is_empty() && !destination.is_empty() => {
            Ok(ReportRequest {
                format: format.to_owned(),
                destination: destination.to_owned(),
            })
        }
        _ => Err(ArgsError::BadReport {
            value: value.to_owned(),
        }),
    }
}

/// Parses a full `dx` command line without the executable name.
///
/// Global options may appear before or after the command; the first
/// positional argument selects the command. Later positionals are
/// explicit Bazel labels or patterns resolved through Bazel; file paths
/// stay unsupported. Arguments after the first bare `--` forward to
/// Bazel as command options verbatim.
pub fn parse(args: &[String]) -> Result<Invocation, ArgsError> {
    let mut command: Option<Command> = None;
    let mut check = false;
    let mut workspace: Option<String> = None;
    let mut dry_run = false;
    let mut quiet = false;
    let mut output_name = "text".to_owned();
    let mut reports = Vec::new();
    let mut fail_on_name = "warning".to_owned();
    let mut targets = Vec::new();
    let mut bazel_options = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--" {
            bazel_options.extend(args[index + 1..].iter().cloned());
            break;
        }
        if arg.starts_with('-') {
            let (name, inline) = split_inline(arg);
            match name {
                "--workspace" => {
                    if inline.is_some_and(str::is_empty) {
                        return Err(ArgsError::MissingValue {
                            option: "--workspace".to_owned(),
                        });
                    }
                    let value = take_value(args, &mut index, "--workspace", inline)?;
                    if value.is_empty() {
                        return Err(ArgsError::MissingValue {
                            option: "--workspace".to_owned(),
                        });
                    }
                    workspace = Some(value.to_owned());
                }
                "--dry-run" => {
                    if inline.is_some() {
                        return Err(ArgsError::UnknownOption {
                            option: arg.clone(),
                        });
                    }
                    dry_run = true;
                }
                "--quiet" => {
                    if inline.is_some() {
                        return Err(ArgsError::UnknownOption {
                            option: arg.clone(),
                        });
                    }
                    quiet = true;
                }
                "--output" => {
                    let value = take_value(args, &mut index, "--output", inline)?;
                    output_name = value.to_owned();
                }
                "--report" => {
                    let value = take_value(args, &mut index, "--report", inline)?;
                    reports.push(parse_report(value)?);
                }
                "--fail-on" => {
                    let value = take_value(args, &mut index, "--fail-on", inline)?;
                    fail_on_name = value.to_owned();
                }
                "--check" => {
                    if inline.is_some() {
                        return Err(ArgsError::UnknownOption {
                            option: arg.clone(),
                        });
                    }
                    check = true;
                }
                _ => {
                    return Err(ArgsError::UnknownOption {
                        option: arg.clone(),
                    });
                }
            }
            index += 1;
            continue;
        }
        match command {
            None => {
                command = Some(
                    Command::parse(arg).ok_or_else(|| ArgsError::UnknownCommand {
                        command: arg.clone(),
                    })?,
                );
            }
            Some(_) => {
                if arg.starts_with("//") || arg.starts_with('@') {
                    targets.push(arg.clone());
                } else {
                    return Err(ArgsError::ScopeNotSupported { scope: arg.clone() });
                }
            }
        }
        index += 1;
    }
    let command = command.ok_or(ArgsError::MissingCommand)?;
    let output = OutputMode::parse(&output_name, quiet).map_err(|_| ArgsError::BadOutput {
        value: output_name.clone(),
    })?;
    let fail_on = Threshold::parse(&fail_on_name).map_err(|_| ArgsError::BadFailOn {
        value: fail_on_name.clone(),
    })?;
    Ok(Invocation {
        command,
        check,
        workspace,
        dry_run,
        quiet,
        output,
        reports,
        fail_on,
        targets,
        bazel_options,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn command_names_are_stable() {
        assert_eq!(Command::Lint.name(), "lint");
        assert_eq!(Command::Typecheck.name(), "typecheck");
        assert_eq!(Command::Format.name(), "format");
    }

    #[test]
    fn bare_command_parses_with_defaults() {
        let got = parse(&args(&["lint"])).expect("parse");
        assert_eq!(got.command, Command::Lint);
        assert!(!got.check);
        assert_eq!(got.workspace, None);
        assert!(!got.dry_run);
        assert!(!got.quiet);
        assert_eq!(got.output, OutputMode::Text { quiet: false });
        assert!(got.reports.is_empty());
        assert_eq!(got.fail_on, Threshold::Warning);
        assert!(got.targets.is_empty());
        assert!(got.bazel_options.is_empty());
        assert_eq!(got.mode(), "default");
    }

    #[test]
    fn check_selects_check_mode() {
        let got = parse(&args(&["format", "--check"])).expect("parse");
        assert_eq!(got.command, Command::Format);
        assert!(got.check);
        assert_eq!(got.mode(), "check");
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
            parse(&args(&["build"])),
            Err(ArgsError::UnknownCommand {
                command: "build".to_owned(),
            })
        );
    }

    #[test]
    fn explicit_label_scope_parses_in_order() {
        let got = parse(&args(&["lint", "//a:one", "@repo//b/...", "//c/..."])).expect("parse");
        assert_eq!(got.targets, args(&["//a:one", "@repo//b/...", "//c/..."]));
    }

    #[test]
    fn positional_scope_fails() {
        assert_eq!(
            parse(&args(&["lint", "src/main.rs"])),
            Err(ArgsError::ScopeNotSupported {
                scope: "src/main.rs".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["lint", ":corpus"])),
            Err(ArgsError::ScopeNotSupported {
                scope: ":corpus".to_owned(),
            })
        );
    }

    #[test]
    fn unknown_options_fail() {
        assert_eq!(
            parse(&args(&["lint", "--jobs=4"])),
            Err(ArgsError::UnknownOption {
                option: "--jobs=4".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["lint", "-q"])),
            Err(ArgsError::UnknownOption {
                option: "-q".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--dry-run=yes"])),
            Err(ArgsError::UnknownOption {
                option: "--dry-run=yes".to_owned(),
            })
        );
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
            ArgsError::ScopeNotSupported {
                scope: "//...".to_owned(),
            }
        )
        .contains("//..."));
        assert!(format!(
            "{}",
            ArgsError::UnknownCommand {
                command: "bogus".to_owned(),
            }
        )
        .contains("bogus"));
        assert!(format!(
            "{}",
            ArgsError::UnknownOption {
                option: "--bogus".to_owned(),
            }
        )
        .contains("--bogus"));
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
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--check=x"])),
            Err(ArgsError::UnknownOption {
                option: "--check=x".to_owned(),
            })
        );
    }
}
