//! Command vocabulary for the `dx` CLI (issue #236).
//!
//! Split from `super` (`args.rs`): owns [`Command`] and its
//! classification helpers (`name`, `parse`, `is_*`, `supports_*`,
//! `describe`). Re-exported through `super` so the public path stays
//! `crate::args::Command`.

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

    pub(crate) fn parse(text: &str) -> Option<Self> {
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
