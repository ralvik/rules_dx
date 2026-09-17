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

use clap::Parser;
use dx_output::{OutputMode, Threshold};

/// Quality, generation, workflow, run, clean, managed
/// environment/codegen/setup, adoption, and inspect command selected by
/// the first positional argument. The variant spellings double as the
/// `clap::ValueEnum` source of truth for the command word.
/// Single-word lowercase variants map to identical clap values, so
/// [`Command::parse`] delegates to the derive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
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
    Deploy,
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
            Command::Deploy => "deploy",
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
        use clap::ValueEnum;
        Self::from_str(text, false).ok()
    }

    /// True for the Bazel-passthrough workflow commands (`build`, `test`,
    /// `coverage`, `run`, `deploy`): they run Bazel verbs directly instead of the quality
    /// aspect pipeline, so quality-only options do not apply to them.
    pub fn is_workflow(self) -> bool {
        matches!(
            self,
            Command::Build | Command::Test | Command::Coverage | Command::Run | Command::Deploy
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

    /// True when `--output=json` (NDJSON) is supported (issue #200).
    /// JSON-capable commands stream one object per line via `write_event`
    /// (never buffer-then-dump). Text-only commands reject `--output=json`
    /// pre-exec with `UnsupportedOption` instead of silently ignoring it:
    /// clean/managed print prose lifecycle, `bazel`/`run`/`deploy` own the terminal
    /// for passthrough applications, and adoption helpers (except
    /// `status`) print local-helper prose or thin query lines.
    /// `update` supports JSON: dry-run planning and the deferred-live
    /// error both emit `command_started`/`command_finished` like `audit`.
    pub fn supports_json(self) -> bool {
        matches!(
            self,
            Command::Audit
                | Command::Lint
                | Command::Typecheck
                | Command::Format
                | Command::Generate
                | Command::Build
                | Command::Test
                | Command::Coverage
                | Command::Update
                | Command::Check
                | Command::Fix
                | Command::Status
        )
    }

    /// True when `--output=diff` emits a unified patch (issue #200).
    /// Only patch-producing commands accept it (lint, typecheck, format,
    /// generate, check, fix per the output protocol). Every other command
    /// rejects `--output=diff` pre-exec: workflow/audit/update/status have
    /// no patch to emit (empty stdout would mislead), and text-only
    /// commands have no machine patch surface at all.
    pub fn supports_diff(self) -> bool {
        matches!(
            self,
            Command::Lint
                | Command::Typecheck
                | Command::Format
                | Command::Generate
                | Command::Check
                | Command::Fix
        )
    }

    /// One-line summary for `--help` (single source with [`Command::name`];
    /// longer behavior lives in docs/cli/commands/, not duplicated here).
    pub fn describe(self) -> &'static str {
        match self {
            Command::Audit => "plan a security/license audit (tool backends land later)",
            Command::Lint => "run lint analysis over resolved scopes",
            Command::Typecheck => "run typecheck analysis over resolved scopes",
            Command::Format => "check or rewrite formatting over resolved scopes",
            Command::Generate => "emit/sync BUILD files (Gazelle pipeline)",
            Command::Build => "run Bazel build over resolved targets",
            Command::Test => "run Bazel test over resolved targets",
            Command::Coverage => "collect LCOV coverage with optional threshold",
            Command::Run => "build and run a single runnable target",
            Command::Deploy => "build and run a single deployable target",
            Command::Check => "run format+lint+typecheck+generate checks in order",
            Command::Fix => "apply format+lint+typecheck+generate fixes in order",
            Command::Clean => "prune unselected managed state (no scopes)",
            Command::Update => "plan dependency updates per set (resolvers land later)",
            Command::Codegen => "collect codegen outputs with atomic commit",
            Command::Env => "collect the managed development environment",
            Command::Setup => "collect setup outputs with atomic commit",
            Command::Init => "scaffold dx into a foreign tree (absent-only)",
            Command::Hooks => "manage Git hooks via hermetic Git",
            Command::Status => "report workspace and target status",
            Command::Version => "report version and pin drift",
            Command::Watch => "watch for changes and rebuild (local only)",
            Command::Owners => "query owners of files via Bazel query",
            Command::Deps => "query dependencies of targets",
            Command::Why => "explain why a target depends on another",
            Command::Completion => "emit shell completions from the CLI grammar",
            Command::Bazel => "forward raw arguments to the Bazel launcher",
        }
    }
}

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

/// Raw `dx` command-line tokens as classified by `clap`: flags may appear
/// before or after the command word, repeated scalars keep the last
/// occurrence, and slice shapes (`--report`, scopes, Bazel forwards) keep
/// `argv` order. `--help`/`-h` render from this same grammar definition
/// (issue #203): one source feeds parsing, help, and completions/man
/// pages, never hand-maintained usage strings.
#[derive(Parser)]
#[command(
    name = "dx",
    about = "Transparent UI over Bazel: quality, workflow, and environment commands",
    long_about = "dx [global-options] <command> [scope ...] [-- bazel-options ...]\n\nScopes: explicit Bazel labels/patterns (//..., //pkg:target, @repo//...), or workspace-relative files/dirs resolved via Bazel query. No scope selects //....\n\nExit codes: 0 success; 2 CLI-detected usage/scope/owner errors; 1 operational failures; Bazel-authoritative commands preserve Bazel's code.\n\nOutput: --output text|diff|json (NDJSON machine protocol on stdout, human text otherwise). See docs/cli/cli-contract.md.",
    version
)]
struct Cli {
    /// Override upward workspace discovery (find MODULE.bazel).
    #[arg(long, allow_negative_numbers = true, overrides_with = "workspace")]
    workspace: Option<String>,
    /// Resolve and summarize the plan without executing workflows/mutations.
    #[arg(long)]
    dry_run: bool,
    /// Suppress dx operation summaries (tool diagnostics still print).
    #[arg(long)]
    quiet: bool,
    /// Enable structured diagnostics on stderr via tracing (issue #222).
    /// Default stays byte-identical; `--verbose` adds info-level logs.
    /// Orthogonal to `--quiet` (summaries vs logs).
    #[arg(long)]
    verbose: bool,
    /// Select concise text, unified patches, or versioned NDJSON events.
    #[arg(long, allow_negative_numbers = true, overrides_with = "output")]
    output: Option<String>,
    /// Write a standard report (sarif/junit/lcov) to file or -; repeatable.
    #[arg(long, allow_negative_numbers = true)]
    report: Vec<String>,
    /// Lowest diagnostic severity that fails quality commands.
    #[arg(long, allow_negative_numbers = true, overrides_with = "fail_on")]
    fail_on: Option<String>,
    /// Required line-coverage percent (coverage only, 0-100).
    #[arg(long, allow_negative_numbers = true, overrides_with = "min_coverage")]
    min_coverage: Option<String>,
    /// Check mode (quality/version only; no mutations).
    #[arg(long)]
    check: bool,
    /// Use dx_debug config (build/run/test/deploy only; conflicts with --release).
    #[arg(long)]
    debug: bool,
    /// Use dx_release config (build/run/test/deploy only; conflicts with --debug).
    #[arg(long)]
    release: bool,
    /// Additionally forward `bazel clean` after pruning (clean only).
    #[arg(long = "bazel")]
    bazel_clean: bool,
    /// Re-pin to <version> (version only).
    #[arg(long, allow_negative_numbers = true, overrides_with = "pin")]
    pin: Option<String>,
    /// Re-pin the recorded previous release (version only).
    #[arg(long)]
    rollback: bool,
    /// Use cquery instead of query (owners/deps/why only).
    #[arg(long)]
    configured: bool,
    /// First positional: the command word (a [`Command`] value so the
    /// same grammar feeds parsing, `--help`, and shell completions).
    #[arg(value_enum)]
    command: Option<Command>,
    /// Later positionals: explicit scopes/targets.
    targets: Vec<String>,
    /// Everything after the first bare `--`, forwarded verbatim.
    #[arg(last = true)]
    bazel_options: Vec<String>,
}

/// Grammar accessor for build steps (issue #225): the `dx_man` binary
/// renders `man/dx.1` from this command so the manual page tracks the
/// same grammar as parsing, `--help`, and completions.
pub fn cli_command() -> clap::Command {
    use clap::CommandFactory;
    Cli::command()
}

/// Maps one scope positional onto its shape-specific parse failure:
/// empty scopes name the repository-wide default, package-relative
/// labels name the `//` qualification, and anything else keeps the
/// generic label/path guidance (typo paths and `@` scopes fail later
/// in resolution with `PathNotFound`/`ExternalScope` context).
fn scope_error(scope: &str) -> ArgsError {
    if scope.is_empty() {
        ArgsError::EmptyScope
    } else if scope.starts_with(':') {
        ArgsError::RelativeLabel {
            scope: scope.to_owned(),
        }
    } else {
        ArgsError::InvalidScope {
            scope: scope.to_owned(),
        }
    }
}

/// Value options whose next token the tokenizer consumes as their value:
/// any token not starting with `--`, including single-dash spellings and
/// the empty string.
const VALUE_OPTIONS: &[&str] = &[
    "--workspace",
    "--output",
    "--report",
    "--fail-on",
    "--min-coverage",
    "--pin",
];

/// Finds the `bazel` command word when it owns the tail: the first
/// positional token, skipping value-option payloads exactly like the
/// legacy hand-rolled tokenizer did, so `dx bazel ...` forwards verbatim
/// while `dx --output bazel build` still binds `bazel` as the output
/// value. Returns `None` once a bare `--` is seen (everything after it
/// is Bazel-owned regardless of command) or when a value option is
/// missing its payload (the full parse then reports the missing value).
///
/// Stays hand-rolled (issue #233 fallback): it routes `argv` *before* the
/// grammar runs, deciding which prefix clap parses and which tail forwards
/// verbatim. A `value_parser` runs inside parsing on one value and cannot
/// own the tail, and the Bazel tail is foreign syntax by contract.
fn split_bazel_verbatim(args: &[String]) -> Option<usize> {
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--" {
            return None;
        }
        if arg.starts_with('-') {
            let name = arg.split_once('=').map_or(arg.as_str(), |(name, _)| name);
            if !arg.contains('=') && VALUE_OPTIONS.contains(&name) {
                match args.get(index + 1) {
                    Some(next) if !next.starts_with("--") && next != "--" => index += 2,
                    _ => return None,
                }
                continue;
            }
            index += 1;
            continue;
        }
        return (arg == "bazel").then_some(index);
    }
    None
}

/// Reads the clap invalid-argument context (`--flag <VALUE>` render or
/// bare token) as a string.
fn invalid_token(error: &clap::Error) -> Option<String> {
    match error.get(clap::error::ContextKind::InvalidArg) {
        Some(clap::error::ContextValue::String(token)) => Some(token.clone()),
        Some(clap::error::ContextValue::Strings(tokens)) => tokens.first().cloned(),
        _ => None,
    }
}

/// Reads the clap invalid-value context (empty when an option value is
/// missing, the offending value otherwise).
fn invalid_value(error: &clap::Error) -> Option<String> {
    match error.get(clap::error::ContextKind::InvalidValue) {
        Some(clap::error::ContextValue::String(value)) => Some(value.clone()),
        Some(clap::error::ContextValue::Strings(values)) => values.first().cloned(),
        _ => None,
    }
}

/// Recovers the exact offending `argv` token for an unknown option:
/// clap reports the bare flag name for `--flag=value` spellings, while
/// the contract pins the whole token.
fn recover_token(args: &[String], token: Option<String>) -> String {
    let token = token.unwrap_or_default();
    if args.contains(&token) {
        return token;
    }
    let inline = format!("{token}=");
    if let Some(arg) = args.iter().rfind(|arg| arg.starts_with(&inline)) {
        return arg.clone();
    }
    token
}

/// Extracts the leading `--flag` from a clap missing-value render such
/// as `--output <OUTPUT>`.
fn leading_flag(token: &str) -> String {
    token.split_whitespace().next().unwrap_or(token).to_owned()
}

/// Best candidate above clap's confidence bar. Mirrors
/// `clap_builder::parser::features::suggestions::did_you_mean` (same
/// upstream `strsim::jaro` + `0.7` threshold): that helper is
/// crate-private, so the typo path re-applies its rule over our own
/// candidate list instead of hand-rolling edit distance.
fn best_match(input: &str, candidates: impl Iterator<Item = impl AsRef<str>>) -> Option<String> {
    let mut best: Option<(f64, String)> = None;
    for candidate in candidates {
        let confidence = strsim::jaro(input, candidate.as_ref());
        if confidence > 0.7 && best.as_ref().is_none_or(|(score, _)| confidence > *score) {
            best = Some((confidence, candidate.as_ref().to_owned()));
        }
    }
    best.map(|(_, name)| name)
}

/// Suggests the closest command word from the [`Command`] derive (the
/// single grammar source), so the hint can never drift from the
/// accepted spellings.
fn suggest_command(input: &str) -> Option<String> {
    use clap::ValueEnum;
    best_match(
        input,
        Command::value_variants().iter().copied().map(Command::name),
    )
}

/// Suggests the closest `--flag` for an offending token. Candidates
/// come from the clap grammar (`Cli::command()` longs), so the hint
/// tracks `Cli` renames without a second list. Exact matches yield no
/// hint (the flag is right; the `=value` is wrong), and non-flag
/// tokens yield none.
fn suggest_option(token: &str) -> Option<String> {
    use clap::CommandFactory;
    let name = token.split(['=', ' ', '\t']).next().unwrap_or(token);
    let bare = name.strip_prefix("--").or_else(|| name.strip_prefix('-'))?;
    if bare.is_empty() {
        return None;
    }
    // Single-character `-q`-style tokens never suggest: jaro("q","quiet")
    // clears 0.7 but a one-letter flag is a missing-short attempt, not a
    // `--long` typo. Longer typos (`--ouptut`) still flow to best_match.
    if bare.len() < 2 {
        return None;
    }
    let cmd = Cli::command();
    let longs: Vec<&str> = cmd
        .get_arguments()
        .filter_map(|arg| arg.get_long())
        .collect();
    if longs.contains(&bare) {
        return None;
    }
    best_match(bare, longs.into_iter()).map(|hit| format!("--{hit}"))
}

/// Reads clap's own suggestion off a parse failure (`--flag` render),
/// when the unknown flag is close enough for clap to name one.
fn clap_suggestion(error: &clap::Error) -> Option<String> {
    use clap::error::{ContextKind, ContextValue};
    for kind in [ContextKind::SuggestedArg, ContextKind::Suggested] {
        match error.get(kind) {
            Some(ContextValue::String(hit)) => return Some(hit.clone()),
            Some(ContextValue::Strings(hits)) => {
                if let Some(hit) = hits.first() {
                    return Some(hit.clone());
                }
            }
            _ => {}
        }
    }
    None
}

/// Shells covered by `dx completion` (contract freeze).
pub const COMPLETION_SHELLS: &[&str] = &["bash", "zsh", "fish", "powershell"];

/// Renders one completion script from the [`Cli`] grammar definition
/// (issue #202): commands, flags, and fixed value sets come from the
/// same source that feeds parsing and `--help`, so generated scripts
/// cannot drift from the command reference. Generation is an explicit
/// `dx completion` cost only, never per-invocation. Unknown shells fail
/// with the contract's `unknown-shell` text.
pub fn render_completion(shell: &str) -> Result<String, ArgsError> {
    use clap::CommandFactory;
    if !COMPLETION_SHELLS.contains(&shell) {
        return Err(ArgsError::UnknownShell {
            shell: shell.to_owned(),
        });
    }
    let generator =
        shell
            .parse::<clap_complete::aot::Shell>()
            .map_err(|_| ArgsError::UnknownShell {
                shell: shell.to_owned(),
            })?;
    let mut command = Cli::command();
    let mut script = Vec::new();
    clap_complete::generate(generator, &mut command, "dx", &mut script);
    let mut text = String::from_utf8(script).map_err(|_| ArgsError::UnknownShell {
        shell: shell.to_owned(),
    })?;
    // Fish/powershell generators omit positional `ValueEnum` values, so
    // commands would be missing there while bash/zsh list them. Append
    // command completions derived from [`Command`] (same source as
    // parsing), never hand-maintained, so every shell completes every
    // command (issue #202).
    match shell {
        "fish" => {
            use clap::ValueEnum;
            text.push_str("\n# dx commands from the single Command source (issue #202)\n");
            for cmd in Command::value_variants() {
                let desc = cmd.describe().replace('\'', "\\'");
                text.push_str(&format!(
                    "complete -c dx -f -n '__fish_use_subcommand' -a {} -d '{}'\n",
                    cmd.name(),
                    desc
                ));
            }
        }
        "powershell" => {
            use clap::ValueEnum;
            let mut additions = String::new();
            for cmd in Command::value_variants() {
                let desc = cmd.describe().replace('\'', "''");
                additions.push_str(&format!(
                    "            [CompletionResult]::new('{}', '{}', [CompletionResultType]::ParameterValue, '{}')\n",
                    cmd.name(),
                    cmd.name(),
                    desc
                ));
            }
            let anchor = "            break\n        }\n    })";
            if let Some(pos) = text.find(anchor) {
                text.insert_str(pos, &additions);
            } else {
                text.push_str("\n# dx commands from the single Command source (issue #202)\n");
                for cmd in Command::value_variants() {
                    text.push_str(&format!("# dx {}\n", cmd.name()));
                }
            }
        }
        _ => {}
    }
    Ok(text)
}

/// Finds the command word for `--help` routing: the first positional
/// token that parses as [`Command`], skipping flag payloads exactly
/// like [`split_bazel_verbatim`]. Stops at `--` (everything after is
/// Bazel-owned). Returns `None` for top-level help when no command
/// word is present or the first positional is not a command.
///
/// Stays hand-rolled with [`split_bazel_verbatim`] (issue #233 fallback):
/// help routing inspects `argv` before the grammar runs, so it cannot
/// itself be a `value_parser`.
fn help_command_in(args: &[String]) -> Option<Command> {
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--" {
            return None;
        }
        if arg.starts_with('-') {
            let name = arg.split_once('=').map_or(arg.as_str(), |(name, _)| name);
            if !arg.contains('=') && VALUE_OPTIONS.contains(&name) {
                match args.get(index + 1) {
                    Some(next) if !next.starts_with("--") && next != "--" => index += 2,
                    _ => index += 1,
                }
                continue;
            }
            index += 1;
            continue;
        }
        return Command::parse(arg);
    }
    None
}

/// Renders top-level `--help` from the [`Cli`] grammar definition (one
/// source for parsing/help/completions; no hand-maintained flag list).
/// The command list is derived from [`Command`] (the same source as
/// parsing), because commands are a validated positional rather than
/// clap subcommands.
fn render_top_help() -> String {
    use clap::{CommandFactory, ValueEnum};
    let mut out = String::new();
    // Brand line (issue #225): `render_long_help` below shows `long_about`
    // but not `about`, so `--help` would otherwise omit the brand that `-h`
    // shows. Prepend it so both spellings carry the same identity.
    if let Some(about) = Cli::command().get_about() {
        out.push_str(&format!("dx - {about}\n\n"));
    }
    out.push_str("Commands:\n");
    for command in Command::value_variants() {
        out.push_str(&format!(
            "  {:<12} {}\n",
            command.name(),
            command.describe()
        ));
    }
    out.push('\n');
    out.push_str(&Cli::command().render_long_help().to_string());
    out
}

/// Renders per-command `--help`: one-line summary from
/// [`Command::describe`], usage/scopes/exit/output lines consistent
/// with `docs/cli/cli-contract.md`, then the full grammar help so the
/// flag list can never drift.
///
/// Issue #211 reconciliation: `--bazel` (clean only, forwards
/// `bazel clean`) and `--configured` (owners/deps/why only, selects
/// `cquery`) keep their names because they mean different things, and
/// both stay distinct from the `dx bazel` passthrough command. The
/// per-command usage + flags lines below name that distinction so the
/// collision is documented, not hidden.
fn per_command_flags(command: Command) -> &'static str {
    match command {
        Command::Clean => {
            "Per-command flags: --bazel (also run `bazel clean` after pruning; distinct from `dx bazel`, which forwards raw args)."
        }
        Command::Owners | Command::Deps | Command::Why => {
            "Per-command flags: --configured (use `bazel cquery` instead of `bazel query`; distinct from `dx clean --bazel`, which forwards `bazel clean`)."
        }
        Command::Coverage => {
            "Per-command flags: --min-coverage <0-100> (coverage only; collects without enforcing when absent)."
        }
        Command::Build | Command::Test | Command::Run | Command::Deploy => {
            "Per-command flags: --debug | --release (build/run/test/deploy only; mutually exclusive; bare invocation means dev, except deploy means release)."
        }
        Command::Version => {
            "Per-command flags: --check (drift check), --pin <version>, --rollback (version only; --pin and --rollback conflict)."
        }
        Command::Bazel => {
            "Per-command flags: none (raw Bazel forwarding; dx-owned options must precede the command word and most are rejected)."
        }
        _ => {
            "Per-command flags: --check/--fail-on/--report/--min-coverage/--debug/--release/--bazel/--pin/--rollback/--configured are owned per command; unsupported uses fail with `option \"--flag\" is not supported by dx <command>`."
        }
    }
}

fn render_command_help(command: Command) -> String {
    let usage = match command {
        Command::Clean => "Usage: dx [global-options] clean [--dry-run] [--bazel]",
        Command::Bazel => "Usage: dx [global-options] bazel [-- bazel-args ...]",
        Command::Run => "Usage: dx [global-options] run [--debug|--release] <target> [-- app-args ...]",
        Command::Deploy => "Usage: dx [global-options] deploy [--debug|--release] <label> [-- app-args ...]",
        Command::Build | Command::Test => {
            "Usage: dx [global-options] build|test [--debug|--release] [scope ...] [-- bazel-options ...]"
        }
        Command::Coverage => {
            "Usage: dx [global-options] coverage [--min-coverage 0-100] [scope ...] [-- bazel-options ...]"
        }
        Command::Owners => "Usage: dx [global-options] owners [--configured] <scope> ...",
        Command::Deps => "Usage: dx [global-options] deps [--configured] <scope> ...",
        Command::Why => "Usage: dx [global-options] why [--configured] <file> <label>",
        Command::Version => {
            "Usage: dx [global-options] version [--check] [--pin <version>|--rollback]"
        }
        _ => "Usage: dx [global-options] <command> [scope ...] [-- bazel-options ...]",
    };
    let scopes = match command {
        Command::Clean => "Scopes: none (clean takes no scopes).",
        Command::Bazel => "Scopes: none (raw Bazel forwarding; no dx scope resolution).",
        Command::Deploy => "Scopes: exactly one main-workspace label (//pkg:target); patterns (//...), multiple labels, and file/path scopes are usage failures.",
        Command::Run => "Scopes: explicit Bazel labels/patterns (//..., //pkg:target, @repo//...), or workspace-relative files/dirs resolved via Bazel query. No scope selects //....",
        _ => "Scopes: explicit Bazel labels/patterns (//..., //pkg:target, @repo//...), or workspace-relative files/dirs resolved via Bazel query. No scope selects //....",
    };
    let mut out = String::new();
    out.push_str(&format!(
        "dx {} - {}\n\n",
        command.name(),
        command.describe()
    ));
    out.push_str(usage);
    out.push('\n');
    out.push_str(per_command_flags(command));
    out.push_str("\n\n");
    out.push_str(scopes);
    out.push_str("\nExit codes: 0 success; 2 usage/scope/owner errors; 1 operational failures; Bazel-authoritative failures preserve Bazel's code.");
    out.push_str(
        "\nOutput: --output text|diff|json; --report <format>=<destination> (repeatable).",
    );
    out.push_str("\n\n");
    out.push_str(&render_top_help());
    out
}

/// True when a clap invalid-argument render names the command
/// positional (`<COMMAND>`): the only `InvalidValue` source that is a
/// command word rather than an option value.
fn is_command_positional(token: &str) -> bool {
    token
        .trim_matches(|cut| cut == '<' || cut == '>' || cut == '[' || cut == ']')
        .eq_ignore_ascii_case("command")
}

/// Reads clap's own command suggestion off a `ValueEnum` positional
/// rejection (same `jaro` rule as [`best_match`], computed by clap over
/// the [`Command`] variants).
fn clap_command_suggestion(error: &clap::Error) -> Option<String> {
    use clap::error::{ContextKind, ContextValue};
    match error.get(ContextKind::SuggestedValue) {
        Some(ContextValue::String(hit)) => Some(hit.clone()),
        Some(ContextValue::Strings(hits)) => hits.first().cloned(),
        _ => None,
    }
}

/// Maps a clap parse failure back onto [`ArgsError`] so the contract
/// surface never changes: unknown commands (including `ValueEnum`
/// rejections of the command positional) stay unknown commands with
/// typo hints, unknown flags (including `=value` on booleans) stay
/// unknown options, and missing option values stay missing values.
/// `--help`/`-h` and `--version`/`-V` render from the same grammar
/// (issue #203) as [`ArgsError::Help`], never as usage errors.
fn map_clap_error(args: &[String], error: &clap::Error) -> ArgsError {
    use clap::error::ErrorKind;
    match error.kind() {
        ErrorKind::DisplayHelp => {
            let text = match help_command_in(args) {
                Some(command) => render_command_help(command),
                None => render_top_help(),
            };
            ArgsError::Help { text }
        }
        ErrorKind::DisplayVersion => {
            use clap::CommandFactory;
            ArgsError::Help {
                text: Cli::command().render_version().to_string(),
            }
        }
        ErrorKind::UnknownArgument | ErrorKind::TooManyValues => {
            let option = recover_token(args, invalid_token(error));
            let suggestion = clap_suggestion(error).or_else(|| suggest_option(&option));
            ArgsError::UnknownOption { option, suggestion }
        }
        ErrorKind::InvalidValue => {
            let token = invalid_token(error).unwrap_or_default();
            if invalid_value(error).is_none_or(|value| value.is_empty()) {
                ArgsError::MissingValue {
                    option: leading_flag(&token),
                }
            } else if is_command_positional(&token) {
                // The command positional is a `ValueEnum`, so unknown
                // command words fail here, not in `parse`: map them back
                // onto the unknown-command surface with typo hints
                // (issue #199). A lone `-` still reads as an unknown
                // option, exactly like the pre-`ValueEnum` tokenizer did.
                let value = invalid_value(error).unwrap_or(token);
                if value.starts_with('-') {
                    let option = recover_token(args, Some(value.clone()));
                    let suggestion = clap_suggestion(error).or_else(|| suggest_option(&option));
                    ArgsError::UnknownOption { option, suggestion }
                } else {
                    let command = recover_token(args, Some(value.clone()));
                    let suggestion =
                        clap_command_suggestion(error).or_else(|| suggest_command(&command));
                    ArgsError::UnknownCommand {
                        command,
                        suggestion,
                    }
                }
            } else {
                let option = recover_token(args, Some(token));
                let suggestion = clap_suggestion(error).or_else(|| suggest_option(&option));
                ArgsError::UnknownOption { option, suggestion }
            }
        }
        _ => ArgsError::UnknownOption {
            option: error
                .render()
                .to_string()
                .lines()
                .next()
                .unwrap_or("dx")
                .trim()
                .to_owned(),
            suggestion: None,
        },
    }
}

/// Runs the clap tokenizer over `args` (without the executable name).
fn parse_tokens(args: &[String]) -> Result<Cli, ArgsError> {
    Cli::try_parse_from(std::iter::once("dx".to_owned()).chain(args.iter().cloned()))
        .map_err(|error| map_clap_error(args, &error))
}

/// Tokenizes `args` into the clap-classified [`Cli`] plus the verbatim
/// Bazel tail. `dx bazel` owns every token after the command word, so
/// its tail never reaches clap; every other shape parses whole.
fn tokenize(args: &[String]) -> Result<(Cli, Vec<String>), ArgsError> {
    if let Some(at) = split_bazel_verbatim(args) {
        // The prefix holds flags only (the scan stops at the first
        // positional and bails at `--`), so no positionals are lost; the
        // command word itself is the `bazel` token the scan stopped at.
        let mut cli = parse_tokens(&args[..at])?;
        cli.command = Some(Command::Bazel);
        // The first `--` still separates (it is dropped, the rest
        // forwards), exactly like the loop's separator check running
        // before the verbatim arm did.
        let mut bazel_options = Vec::new();
        let mut tail = args[at + 1..].iter();
        for arg in tail.by_ref() {
            if arg == "--" {
                break;
            }
            bazel_options.push(arg.clone());
        }
        bazel_options.extend(tail.cloned());
        return Ok((cli, bazel_options));
    }
    let mut cli = parse_tokens(args)?;
    let bazel_options = std::mem::take(&mut cli.bazel_options);
    Ok((cli, bazel_options))
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

/// Parses a `--min-coverage` value into an integer percent 0-100.
fn parse_min_coverage(value: &str) -> Result<u32, ArgsError> {
    match value.parse::<u32>() {
        Ok(percent) if percent <= 100 => Ok(percent),
        _ => Err(ArgsError::BadMinCoverage {
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
    let (cli, bazel_options) = tokenize(args)?;
    let Cli {
        workspace,
        dry_run,
        quiet,
        verbose,
        output,
        report,
        fail_on,
        min_coverage: min_coverage_name,
        check,
        debug,
        release,
        bazel_clean,
        pin,
        rollback,
        configured,
        command: command_name,
        targets,
        ..
    } = cli;
    if workspace.as_deref().is_some_and(str::is_empty) {
        return Err(ArgsError::MissingValue {
            option: "--workspace".to_owned(),
        });
    }
    if pin.as_deref().is_some_and(str::is_empty) {
        return Err(ArgsError::MissingValue {
            option: "--pin".to_owned(),
        });
    }
    let output_name = output.unwrap_or_else(|| "text".to_owned());
    let fail_on_name = fail_on.unwrap_or_else(|| "warning".to_owned());
    let mut reports = Vec::new();
    for value in &report {
        reports.push(parse_report(value)?);
    }
    let mut min_coverage: Option<u32> = None;
    if let Some(value) = &min_coverage_name {
        min_coverage = Some(parse_min_coverage(value)?);
    }
    let command = command_name.ok_or(ArgsError::MissingCommand)?;
    // `dx bazel` tails never reach this check: the verbatim forwarding
    // above owns every token after the command word.
    if command != Command::Bazel {
        for scope in &targets {
            if scope == "-" {
                return Err(ArgsError::UnknownOption {
                    option: scope.clone(),
                    suggestion: None,
                });
            }
            if scope.is_empty() || scope.starts_with(':') {
                return Err(scope_error(scope));
            }
        }
    }
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
                | dx_setup::ScopeError::NotTargetLabel { value } => scope_error(&value),
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
                return Err(scope_error(scope));
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
        // `update` supports `--output=json` (dry-run planning and the
        // deferred-live error stream `command_started`/`command_finished`
        // like `audit`); `--output=diff` has no patch to emit so it fails
        // fast here, with the shared `supports_diff` gate below as backup.
        if output_name == "diff" {
            return Err(ArgsError::UnsupportedOption {
                command: command.name(),
                option: "--output=diff".to_owned(),
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
                return Err(scope_error(scope));
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
    // `--min-coverage` belongs to `coverage` only: every other command
    // (workflow siblings, quality, umbrellas, managed, adoption) fails
    // fast instead of silently ignoring the threshold.
    if min_coverage.is_some() && command != Command::Coverage {
        return Err(ArgsError::UnsupportedOption {
            command: command.name(),
            option: "--min-coverage".to_owned(),
        });
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
    if command == Command::Run || command == Command::Deploy {
        // O52: `dx run` is a local-only single-target launcher with prose
        // lifecycle on stderr. Machine-owned stdout modes are rejected
        // pre-exec so the application keeps the terminal. `dx deploy`
        // shares the terminal contract: text only, no reports, args
        // after `--` forward verbatim to the program.
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
    // Uniform `--output` contract (issue #200): shared `supports_json` /
    // `supports_diff` gate so no command silently ignores a machine-output
    // request. Commands with their own output arm above (clean, managed,
    // update, `bazel`, `run`) already returned the same error; this gate
    // owns adoption/inspect (only `status` supports JSON, none supports
    // diff), `audit` diff, and workflow `build`/`test`/`coverage` diff.
    // Quality, generate, umbrellas, `audit`/`update` JSON, workflow JSON,
    // and `status` JSON pass through to streaming NDJSON execution.
    if output_name == "json" && !command.supports_json() {
        return Err(ArgsError::UnsupportedOption {
            command: command.name(),
            option: "--output=json".to_owned(),
        });
    }
    if output_name == "diff" && !command.supports_diff() {
        return Err(ArgsError::UnsupportedOption {
            command: command.name(),
            option: "--output=diff".to_owned(),
        });
    }
    // Profile flags (issue #179): `--debug`/`--release` are mutually
    // exclusive and belong to `build`, `run`, `test`, and `deploy`
    // only (every other command fails fast instead of silently
    // ignoring the profile).
    if debug && release {
        return Err(ArgsError::ConflictingProfiles);
    }
    if (debug || release)
        && !matches!(
            command,
            Command::Build | Command::Run | Command::Test | Command::Deploy
        )
    {
        return Err(ArgsError::UnsupportedOption {
            command: command.name(),
            option: if debug {
                "--debug".to_owned()
            } else {
                "--release".to_owned()
            },
        });
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
        debug,
        release,
        workspace,
        dry_run,
        quiet,
        verbose,
        output,
        reports,
        fail_on,
        min_coverage,
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
        let bazel_help = render_command_help(Command::Bazel);
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
