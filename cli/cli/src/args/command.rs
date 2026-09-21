//! Command vocabulary for the `dx` CLI.
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
    Bump,
    Migrate,
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
            Command::Bump => "bump",
            Command::Migrate => "migrate",
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
    /// format, lint, typecheck, and generate (WP4): phases
    /// run in order with stop-on-first-failure under one NDJSON frame.
    pub fn is_umbrella(self) -> bool {
        matches!(self, Command::Check | Command::Fix)
    }

    /// True for the delivered audit/update/bump surfaces (`audit`, `update`,
    /// `bump`): they plan through the `dx_audit`/`dx_update`/`dx_bump`
    /// libraries over family selectors and dependency-set selectors, never
    /// the quality aspect pipeline. Audit is non-mutating with live backends
    /// (Gitleaks secrets, 24h advisory, local vuln matching, SPDX 2.3);
    /// update and bump are mutating without confirmation with live resolver
    /// backends, and bump widens exactly one requirement explicitly.
    /// `migrate` is not audit/update: it plans upgrade rewrites
    /// through `dx_adopt::plan_migrate` over `--from`/`--to` versions
    /// with its own fail-closed execution.
    pub fn is_audit_update(self) -> bool {
        matches!(self, Command::Audit | Command::Update | Command::Bump)
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

    /// True when `--output=json` (NDJSON) is supported.
    /// JSON-capable commands stream one object per line via `write_event`
    /// (never buffer-then-dump). Text-only commands reject `--output=json`
    /// pre-exec with `UnsupportedOption` instead of silently ignoring it:
    /// clean/managed print prose lifecycle, `bazel`/`run`/`deploy` own the terminal
    /// for passthrough applications, and adoption helpers (except
    /// `status`) print local-helper prose or thin query lines.
    /// `update` supports JSON: dry-run planning emits
    /// `command_started`/`command_finished`, while live execution adds
    /// per-set `notice`/`error` events with the same frame.
    /// `bump` supports JSON the same way: dry-run planning emits the
    /// widen summary, live execution adds the widen `notice`/`error`
    ///. `migrate` supports JSON the same way: dry-run
    /// planning emits the manifest plan, live execution fails closed
    /// with `migrate_failed` (no manifests yet).
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
                | Command::Bump
                | Command::Migrate
                | Command::Check
                | Command::Fix
                | Command::Status
        )
    }

    /// True when `--output=diff` emits a unified patch.
    /// Only patch-producing commands accept it (lint, typecheck, format,
    /// generate, check, fix per the output protocol). Every other command
    /// rejects `--output=diff` pre-exec: workflow/audit/update/bump/status
    /// have no patch to emit (empty stdout would mislead), and text-only
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

    /// True for commands that mutate by default (extended by
    /// for `migrate`).
    ///
    /// Mirrors `docs/testing/cli.md#command-registry-and-behavior` plus
    /// `docs/decisions/0005-mutating-operations.md`: lint,
    /// typecheck, format, update, bump, migrate, generate, codegen, env, setup,
    /// init, fix, and hooks apply workspace or managed-state writes
    /// unless a non-mutating mode (`--check`, `--dry-run`) is selected.
    /// `check` stays non-mutating, `clean` mutates managed state only
    /// under its own contract, and workflow/audit/inspect/version
    /// surfaces never mutate workspace sources by default.
    pub fn is_mutating_by_default(self) -> bool {
        matches!(
            self,
            Command::Lint
                | Command::Typecheck
                | Command::Format
                | Command::Update
                | Command::Bump
                | Command::Migrate
                | Command::Generate
                | Command::Codegen
                | Command::Env
                | Command::Setup
                | Command::Init
                | Command::Fix
                | Command::Hooks
        )
    }

    /// One-line summary for `--help` (single source with [`Command::name`];
    /// longer behavior lives in docs/cli/commands/, not duplicated here).
    /// Mutating commands name their default so `--help` plus
    /// [`Command::is_mutating_by_default`] satisfy the
    /// `docs/testing/cli.md` mutating-identification fixture.
    pub fn describe(self) -> &'static str {
        match self {
            Command::Audit => "run security/license audit over resolved scopes (non-mutating; live Gitleaks plus advisory/vuln/SPDX backends)",
            Command::Lint => "run lint analysis over resolved scopes (mutating by default; --check is non-mutating)",
            Command::Typecheck => "run typecheck analysis over resolved scopes (mutating by default; --check is non-mutating)",
            Command::Format => "check or rewrite formatting over resolved scopes (mutating by default; --check is non-mutating)",
            Command::Generate => "emit/sync BUILD files (Gazelle pipeline; mutating by default; --check validates without writes)",
            Command::Build => "run Bazel build over resolved targets",
            Command::Test => "run Bazel test over resolved targets",
            Command::Coverage => "collect LCOV coverage with optional threshold",
            Command::Run => "build and run runnable targets sequentially (explicit labels/patterns; file/dir scopes need exactly one runnable)",
            Command::Deploy => "build and run a single deployable target",
            Command::Check => "run format+lint+typecheck+generate checks in order (non-mutating)",
            Command::Fix => "apply format+lint+typecheck+generate fixes in order (mutating by default; no rerun, run `dx check` to validate)",
            Command::Clean => "prune unselected managed state, never Bazel outputs unless --bazel (no scopes)",
            Command::Update => "update dependencies per set through qualified resolvers (mutating without confirmation; --check is the preset stale gate)",
            Command::Bump => "widen one declared requirement to a new version (explicit; mutating without confirmation)",
            Command::Migrate => "rewrite breaking changes across releases (upgrade-only; mutating by default; --dry-run plans without writes)",
            Command::Codegen => "collect codegen outputs with atomic commit (mutating managed state)",
            Command::Env => "collect the managed development environment (mutating managed state)",
            Command::Setup => "collect setup outputs with atomic commit (mutating managed state)",
            Command::Init => "scaffold dx into a foreign tree (absent-only; mutating by default)",
            Command::Hooks => "manage Git hooks via hermetic Git (mutating by default)",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_names_are_stable() {
        assert_eq!(Command::Audit.name(), "audit");
        assert_eq!(Command::Update.name(), "update");
        assert_eq!(Command::Bump.name(), "bump");
        assert_eq!(Command::Migrate.name(), "migrate");
        assert!(Command::Bump.is_audit_update());
        assert!(Command::Update.is_audit_update());
        assert!(!Command::Migrate.is_audit_update());
        assert!(!Command::Migrate.is_workflow());
        assert!(!Command::Migrate.is_adoption());
        assert!(!Command::Migrate.is_managed());
        assert!(!Command::Migrate.is_umbrella());
        assert!(Command::Migrate.is_mutating_by_default());
        assert!(Command::Migrate.supports_json());
        assert!(!Command::Migrate.supports_diff());
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
            Command::Bump,
            Command::Migrate,
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
    fn final_registry_is_exact_and_rejects_excluded_commands() {
        // The final CLI registry holds
        // exactly the 29 implemented commands (including `deploy` plus
        // `bump` plus `migrate`). `doctor`, `configure`, `docs`, and
        // `new` stay rejected as unknown.
        use clap::ValueEnum;
        let mut got: Vec<&str> = Command::value_variants()
            .iter()
            .map(|command| command.name())
            .collect();
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
            "migrate",
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
        assert_eq!(got, want, "Command registry drifted from the final 29");
        assert_eq!(Command::value_variants().len(), 29);
        for excluded in ["doctor", "configure", "docs", "new", "bogus"] {
            assert_eq!(
                Command::parse(excluded),
                None,
                "{excluded} must stay rejected as unknown"
            );
        }
    }

    #[test]
    fn mutating_by_default_matches_contract_and_help() {
        // `docs/testing/cli.md` mutating
        // identification plus ADR 0005 plus ADR 0018. `check` stays
        // non-mutating; `clean` mutates managed state only under its own
        // contract.
        for command in [
            Command::Lint,
            Command::Typecheck,
            Command::Format,
            Command::Update,
            Command::Bump,
            Command::Migrate,
            Command::Generate,
            Command::Codegen,
            Command::Env,
            Command::Setup,
            Command::Init,
            Command::Fix,
            Command::Hooks,
        ] {
            assert!(
                command.is_mutating_by_default(),
                "{command:?} must be mutating by default"
            );
            assert!(
                command.describe().contains("mutating"),
                "{command:?} help must name its mutating default: {}",
                command.describe()
            );
        }
        for command in [
            Command::Audit,
            Command::Build,
            Command::Test,
            Command::Coverage,
            Command::Run,
            Command::Deploy,
            Command::Check,
            Command::Clean,
            Command::Status,
            Command::Version,
            Command::Watch,
            Command::Owners,
            Command::Deps,
            Command::Why,
            Command::Completion,
            Command::Bazel,
        ] {
            assert!(
                !command.is_mutating_by_default(),
                "{command:?} must stay non-mutating by default"
            );
        }
        assert!(
            Command::Check.describe().contains("non-mutating"),
            "check help must name its non-mutating mode"
        );
        // Diff-capable patch producers stay exactly the six check/fix
        // umbrella members per `docs/testing/cli.md`.
        for command in [
            Command::Lint,
            Command::Typecheck,
            Command::Format,
            Command::Generate,
            Command::Check,
            Command::Fix,
        ] {
            assert!(
                command.supports_diff(),
                "{command:?} must support --output=diff"
            );
        }
        assert!(!Command::Update.supports_diff());
        assert!(!Command::Bump.supports_diff());
        assert!(!Command::Migrate.supports_diff());
    }
}
