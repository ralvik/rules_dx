//! Invocation parsing for the `dx` quality, workflow, run, clean, and
//! managed environment/codegen/setup commands (M07 WP1+WP3, M08 WP1+WP4,
//! M25 WP5).
//!
//! Contract: `docs/cli/cli-contract.md#invocation-shape`. Scope positionals
//! accept explicit Bazel labels and patterns (`//...`, `//pkg:target`,
//! `@repo//pkg/...`) as well as workspace-relative file and directory
//! paths. Package-relative labels (`:target`) and empty scopes fail with
//! [`ArgsError::ScopeNotSupported`]; external-repository scopes parse but
//! fail during resolution, and file ownership resolves through Bazel
//! query per `docs/cli/target-resolution.md`. With no scope the
//! repository operation (`//...`) runs.
//!
//! `dx clean` takes no scopes: it prunes validated unselected managed
//! state per `docs/cli/commands/check-fix-clean.md#dx-clean`, with
//! `--dry-run` listing without deleting and `--bazel` additionally
//! forwarding `bazel clean`.

use dx_output::{OutputMode, Threshold};

/// Quality, generation, workflow, run, clean, managed
/// environment/codegen/setup, adoption, and inspect command selected by
/// the first positional argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Audit,
    Lint,
    Typecheck,
    Format,
    Generate,
    Build,
    Test,
    Coverage,
    Run,
    Check,
    Fix,
    Clean,
    Update,
    Codegen,
    Env,
    Setup,
    Init,
    Hooks,
    Status,
    Version,
    Watch,
    Owners,
    Deps,
    Why,
    Completion,
    Bazel,
}

impl Command {
    /// Stable command name used in summaries and `command_started`.
    pub fn name(self) -> &'static str {
        match self {
            Command::Audit => "audit",
            Command::Lint => "lint",
            Command::Typecheck => "typecheck",
            Command::Format => "format",
            Command::Generate => "generate",
            Command::Build => "build",
            Command::Test => "test",
            Command::Coverage => "coverage",
            Command::Run => "run",
            Command::Check => "check",
            Command::Fix => "fix",
            Command::Clean => "clean",
            Command::Update => "update",
            Command::Codegen => "codegen",
            Command::Env => "env",
            Command::Setup => "setup",
            Command::Init => "init",
            Command::Hooks => "hooks",
            Command::Status => "status",
            Command::Version => "version",
            Command::Watch => "watch",
            Command::Owners => "owners",
            Command::Deps => "deps",
            Command::Why => "why",
            Command::Completion => "completion",
            Command::Bazel => "bazel",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "audit" => Some(Command::Audit),
            "lint" => Some(Command::Lint),
            "typecheck" => Some(Command::Typecheck),
            "format" => Some(Command::Format),
            "generate" => Some(Command::Generate),
            "build" => Some(Command::Build),
            "test" => Some(Command::Test),
            "coverage" => Some(Command::Coverage),
            "run" => Some(Command::Run),
            "check" => Some(Command::Check),
            "fix" => Some(Command::Fix),
            "clean" => Some(Command::Clean),
            "update" => Some(Command::Update),
            "codegen" => Some(Command::Codegen),
            "env" => Some(Command::Env),
            "setup" => Some(Command::Setup),
            "init" => Some(Command::Init),
            "hooks" => Some(Command::Hooks),
            "status" => Some(Command::Status),
            "version" => Some(Command::Version),
            "watch" => Some(Command::Watch),
            "owners" => Some(Command::Owners),
            "deps" => Some(Command::Deps),
            "why" => Some(Command::Why),
            "completion" => Some(Command::Completion),
            "bazel" => Some(Command::Bazel),
            _ => None,
        }
    }

    /// True for the Bazel-passthrough workflow commands (`build`, `test`,
    /// `coverage`, `run`): they run Bazel verbs directly instead of the quality
    /// aspect pipeline, so quality-only options do not apply to them.
    pub fn is_workflow(self) -> bool {
        matches!(
            self,
            Command::Build | Command::Test | Command::Coverage | Command::Run
        )
    }

    /// True for the sequential `check`/`fix` umbrellas over
    /// format, lint, typecheck, and generate (M10 WP4, O59): phases
    /// run in order with stop-on-first-failure under one NDJSON frame.
    pub fn is_umbrella(self) -> bool {
        matches!(self, Command::Check | Command::Fix)
    }

    /// True for the delivered audit/update surfaces (`audit`, `update`):
    /// they plan through the `dx_audit`/`dx_update` libraries over
    /// family selectors and dependency-set selectors, never the quality
    /// aspect pipeline. Audit is non-mutating; update is mutating
    /// without confirmation. Tool and resolver backends land in later
    /// M26 slices; this only records the planned request.
    pub fn is_audit_update(self) -> bool {
        matches!(self, Command::Audit | Command::Update)
    }

    /// True for the managed environment/codegen/setup surfaces
    /// (`codegen`, `env`, `setup`): they run the Bazel collection request
    /// behind one canonical selection plus generation commit, never the
    /// quality aspect pipeline. Live selection commits through the
    /// managed-state commit layer; quality-only options do not apply.
    pub fn is_managed(self) -> bool {
        matches!(self, Command::Codegen | Command::Env | Command::Setup)
    }

    /// True for the delivered adoption/inspect surfaces (`init`, `hooks`,
    /// `status`, `version`, `watch`, `owners`, `deps`, `why`,
    /// `completion`): they run local adoption helpers or thin Bazel-query
    /// forwarding instead of the quality aspect pipeline. `bazel` is not
    /// adoption: it forwards raw arguments to the Bazel launcher.
    /// Managed commands (`codegen`, `env`, `setup`) are not adoption
    /// either: they plan a Bazel collection request of their own.
    pub fn is_adoption(self) -> bool {
        matches!(
            self,
            Command::Init
                | Command::Hooks
                | Command::Status
                | Command::Version
                | Command::Watch
                | Command::Owners
                | Command::Deps
                | Command::Why
                | Command::Completion
        )
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
/// repository scope (`//...`). `bazel_clean` is set only by
/// `dx clean --bazel` (additionally forward `bazel clean` after pruning).
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
}

/// Invocation parsing failure. Every variant is a CLI-detected
/// pre-execution usage error (exit code 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgsError {
    MissingCommand,
    UnknownCommand {
        command: String,
    },
    UnknownOption {
        option: String,
    },
    UnsupportedOption {
        command: &'static str,
        option: String,
    },
    MissingValue {
        option: String,
    },
    BadOutput {
        value: String,
    },
    BadFailOn {
        value: String,
    },
    BadReport {
        value: String,
    },
    ScopeNotSupported {
        scope: String,
    },
}

impl std::fmt::Display for ArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgsError::MissingCommand => write!(
                f,
                "missing command: want audit|lint|typecheck|format|generate|build|test|coverage|run|check|fix|clean|update|codegen|env|setup|init|hooks|status|version|watch|owners|deps|why|completion|bazel"
            ),
            ArgsError::UnknownCommand { command } => {
                write!(
                    f,
                    "unknown command {command:?}: want audit|lint|typecheck|format|generate|build|test|coverage|run|check|fix|clean|update|codegen|env|setup|init|hooks|status|version|watch|owners|deps|why|completion|bazel"
                )
            }
            ArgsError::UnknownOption { option } => write!(f, "unknown option {option:?}"),
            ArgsError::UnsupportedOption { command, option } => write!(
                f,
                "option {option:?} is not supported by dx {command}"
            ),
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
                    "unsupported scope {scope:?}: want // or @ labels, or workspace-relative file and directory paths"
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
/// explicit Bazel labels, patterns, or workspace-relative file and
/// directory paths resolved through Bazel during execution; only
/// package-relative labels and empty scopes fail here. Arguments after
/// the first bare `--` forward to Bazel as command options verbatim.
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
    let mut bazel_clean = false;
    let mut pin: Option<String> = None;
    let mut rollback = false;
    let mut configured = false;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--" {
            bazel_options.extend(args[index + 1..].iter().cloned());
            break;
        }
        // `dx bazel` owns no options of its own: once the command word
        // is seen, every following token is launcher-owned and forwards
        // verbatim (flags, labels, and `=` forms alike), per
        // `docs/cli/commands/audit-update-bazel.md`. dx globals must
        // precede the command word (`dx --dry-run bazel ...`).
        if command == Some(Command::Bazel) {
            bazel_options.push(arg.clone());
            index += 1;
            continue;
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
                "--bazel" => {
                    if inline.is_some() {
                        return Err(ArgsError::UnknownOption {
                            option: arg.clone(),
                        });
                    }
                    bazel_clean = true;
                }
                "--pin" => {
                    let value = take_value(args, &mut index, "--pin", inline)?;
                    if value.is_empty() {
                        return Err(ArgsError::MissingValue {
                            option: "--pin".to_owned(),
                        });
                    }
                    pin = Some(value.to_owned());
                }
                "--rollback" => {
                    if inline.is_some() {
                        return Err(ArgsError::UnknownOption {
                            option: arg.clone(),
                        });
                    }
                    rollback = true;
                }
                "--configured" => {
                    if inline.is_some() {
                        return Err(ArgsError::UnknownOption {
                            option: arg.clone(),
                        });
                    }
                    configured = true;
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
            // `dx bazel` tokens never reach this arm: the verbatim
            // forwarding above owns every token after the command word.
            Some(_) => {
                if arg.is_empty() || arg.starts_with(':') {
                    return Err(ArgsError::ScopeNotSupported { scope: arg.clone() });
                }
                targets.push(arg.clone());
            }
        }
        index += 1;
    }
    let command = command.ok_or(ArgsError::MissingCommand)?;
    if command == Command::Clean {
        // `dx clean [--dry-run] [--bazel]` prunes validated unselected
        // managed state with no scopes and no quality/workflow options:
        // `--dry-run` deletes nothing (a `--bazel` forward is listed,
        // never run), and only `--workspace`, `--dry-run`, `--quiet`,
        // and `--bazel` apply.
        if check {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--check".to_owned(),
            });
        }
        if fail_on_name != "warning" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--fail-on".to_owned(),
            });
        }
        if output_name != "text" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--output={output_name}"),
            });
        }
        if let Some(request) = reports.first() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--report={}={}", request.format, request.destination),
            });
        }
        if let Some(scope) = targets.first() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: scope.clone(),
            });
        }
        if !bazel_options.is_empty() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--".to_owned(),
            });
        }
    } else if bazel_clean {
        return Err(ArgsError::UnsupportedOption {
            command: command.name(),
            option: "--bazel".to_owned(),
        });
    }
    if command.is_managed() {
        // Managed environment/codegen/setup commands (M25 WP5) run one
        // Bazel collection request behind a canonical selection with
        // text prose only: no check mode, no finding thresholds, no
        // standard reports, and no version/clean-only flags.
        // `--bazel` is rejected by the clean-ownership arm above;
        // `--rollback`/`--configured` by the catch-alls below. Scope is
        // repository-wide by default or one exact target label, validated
        // through the shared setup scope rules (identical across the
        // codegen, env, and setup libs by contract); user Bazel options
        // after `--` forward to the collection build.
        if check {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--check".to_owned(),
            });
        }
        if fail_on_name != "warning" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--fail-on".to_owned(),
            });
        }
        if output_name != "text" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--output={output_name}"),
            });
        }
        if let Some(request) = reports.first() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--report={}={}", request.format, request.destination),
            });
        }
        if pin.is_some() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--pin".to_owned(),
            });
        }
        if let Err(error) = dx_setup::resolve_scope(&targets) {
            return Err(match error {
                dx_setup::ScopeError::MultipleTargets { .. } => ArgsError::UnsupportedOption {
                    command: command.name(),
                    // `MultipleTargets` guarantees at least two
                    // positionals, so the second one exists.
                    option: targets[1].clone(),
                },
                dx_setup::ScopeError::TargetPattern { value }
                | dx_setup::ScopeError::NotTargetLabel { value } => {
                    ArgsError::ScopeNotSupported { scope: value }
                }
            });
        }
    }
    if command == Command::Audit {
        // Audit plans through `dx_audit` (M26 WP1): family selection
        // plus scope spellings, non-mutating, with SARIF reports and
        // `--fail-on` thresholds. `--check` is meaningless (audit never
        // mutates), Bazel forwards do not apply (no collection build
        // yet), and version/clean-only flags do not apply.
        // Family parsing itself stays in `dx_audit::plan_audit`; args
        // only preserve positionals verbatim (family or scopes).
        if check {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--check".to_owned(),
            });
        }
        if pin.is_some() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--pin".to_owned(),
            });
        }
        if !bazel_options.is_empty() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--".to_owned(),
            });
        }
        // Scopes use the shared label/path shape; the first positional
        // may also be a family (`security`/`license`) preserved verbatim
        // for `dx_audit::plan_audit`. Only package-relative labels and
        // empty scopes fail here.
        for scope in &targets {
            if scope.is_empty() || scope.starts_with(':') {
                return Err(ArgsError::ScopeNotSupported {
                    scope: scope.clone(),
                });
            }
        }
    }
    if command == Command::Update {
        // Update plans through `dx_update` (M26 WP2): dependency-set /
        // package selectors preserved verbatim, mutating without
        // confirmation. Thresholds, standard reports, and check mode do
        // not apply on this path; exact aggregate exit/report mappings
        // stay under O12 qualification.
        if check {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--check".to_owned(),
            });
        }
        if fail_on_name != "warning" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--fail-on".to_owned(),
            });
        }
        if output_name != "text" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--output={output_name}"),
            });
        }
        if let Some(request) = reports.first() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--report={}={}", request.format, request.destination),
            });
        }
        if pin.is_some() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--pin".to_owned(),
            });
        }
        if !bazel_options.is_empty() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--".to_owned(),
            });
        }
        for scope in &targets {
            if scope.is_empty() || scope.starts_with(':') {
                return Err(ArgsError::ScopeNotSupported {
                    scope: scope.clone(),
                });
            }
        }
    }
    if command.is_workflow() {
        // Workflow commands run Bazel verbs directly with Bazel-owned
        // status: finding thresholds and check-mode mutation previews do
        // not apply, so explicit uses fail fast instead of silently
        // doing nothing.
        if check {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--check".to_owned(),
            });
        }
        if fail_on_name != "warning" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--fail-on".to_owned(),
            });
        }
    }
    let output = OutputMode::parse(&output_name, quiet).map_err(|_| ArgsError::BadOutput {
        value: output_name.clone(),
    })?;
    let fail_on = Threshold::parse(&fail_on_name).map_err(|_| ArgsError::BadFailOn {
        value: fail_on_name.clone(),
    })?;
    if command.is_adoption() {
        // Adoption/inspect surfaces run local helpers or thin query
        // forwarding: quality-only thresholds/reports and Bazel forwards
        // do not apply. `--check` belongs to `version` (pin drift)
        // only; `--pin` belongs to `version`
        // only. `--rollback`
        // and `--configured` ownership is enforced by the catch-all
        // below.
        if fail_on_name != "warning" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--fail-on".to_owned(),
            });
        }
        if let Some(request) = reports.first() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--report={}={}", request.format, request.destination),
            });
        }
        if bazel_clean {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--bazel".to_owned(),
            });
        }
        if !bazel_options.is_empty() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--".to_owned(),
            });
        }
        if check && command != Command::Version {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--check".to_owned(),
            });
        }
        if pin.is_some() && command != Command::Version {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--pin".to_owned(),
            });
        }
        match command {
            Command::Status | Command::Version => {
                if !targets.is_empty() && command == Command::Status {
                    return Err(ArgsError::UnsupportedOption {
                        command: command.name(),
                        option: targets[0].clone(),
                    });
                }
                if !targets.is_empty() && command == Command::Version && pin.is_none() {
                    return Err(ArgsError::UnsupportedOption {
                        command: command.name(),
                        option: targets[0].clone(),
                    });
                }
            }
            Command::Completion => {
                if targets.len() != 1 {
                    return Err(ArgsError::MissingValue {
                        option: "<shell>".to_owned(),
                    });
                }
            }
            Command::Hooks | Command::Watch | Command::Owners | Command::Deps
                if targets.is_empty() =>
            {
                return Err(ArgsError::MissingValue {
                    option: "<scope>".to_owned(),
                });
            }
            // `dx why` resolves one ownership edge per call: exactly one
            // file scope and one target label, e.g.
            // `dx why src/main.rs //app:server`.
            Command::Why if targets.len() != 2 => {
                return Err(ArgsError::MissingValue {
                    option: "<file> <label>".to_owned(),
                });
            }
            _ => {}
        }
    }
    if command == Command::Bazel {
        // `dx bazel` forwards arguments unchanged to the workspace
        // Bazel launcher: no scope resolution, no quality thresholds or
        // reports, and text terminal output only (the child inherits
        // stdio). dx-owned options are rejected only when given before
        // the command word (everything after it already forwarded
        // verbatim above); `--rollback` and `--configured` ownership is
        // enforced by the catch-all below.
        if check {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--check".to_owned(),
            });
        }
        if fail_on_name != "warning" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--fail-on".to_owned(),
            });
        }
        if output_name != "text" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--output={output_name}"),
            });
        }
        if let Some(request) = reports.first() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--report={}={}", request.format, request.destination),
            });
        }
        if pin.is_some() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--pin".to_owned(),
            });
        }
        if bazel_clean {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--bazel".to_owned(),
            });
        }
    }
    if command == Command::Run {
        // O52: `dx run` is a local-only single-target launcher with prose
        // lifecycle on stderr. Machine-owned stdout modes are rejected
        // pre-exec so the application keeps the terminal.
        if !matches!(output, OutputMode::Text { .. }) {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--output={output_name}"),
            });
        }
        if let Some(request) = reports.first() {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: format!("--report={}={}", request.format, request.destination),
            });
        }
    }
    // Flag ownership (M30b): `--rollback` belongs to `version` only
    // and `--configured` to the inspect wrappers only. Command blocks
    // above already reject them on their own surfaces; this catch-all
    // keeps every other command (quality, umbrellas, `run`,
    // `generate`, `clean`, managed) failing fast instead of silently
    // ignoring them.
    if rollback && command != Command::Version {
        return Err(ArgsError::UnsupportedOption {
            command: command.name(),
            option: "--rollback".to_owned(),
        });
    }
    if configured
        && command != Command::Owners
        && command != Command::Deps
        && command != Command::Why
    {
        return Err(ArgsError::UnsupportedOption {
            command: command.name(),
            option: "--configured".to_owned(),
        });
    }
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
        bazel_clean,
        pin,
        rollback,
        configured,
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
            Err(ArgsError::ScopeNotSupported {
                scope: ":corpus".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["lint", ""])),
            Err(ArgsError::ScopeNotSupported {
                scope: String::new(),
            })
        );
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
            })
        );
        assert_eq!(
            parse(&args(&["lint", "--check=x"])),
            Err(ArgsError::UnknownOption {
                option: "--check=x".to_owned(),
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
        assert_eq!(
            parse(&args(&["update", "--output=json"])),
            Err(ArgsError::UnsupportedOption {
                command: "update",
                option: "--output=json".to_owned(),
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
            Err(ArgsError::ScopeNotSupported {
                scope: ":target".to_owned(),
            })
        );
        assert_eq!(
            parse(&args(&["update", ":target"])),
            Err(ArgsError::ScopeNotSupported {
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
                    Err(ArgsError::ScopeNotSupported { .. }) | Err(ArgsError::UnknownOption { .. })
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
}
