#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Command {
    Security,
    License,
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
    New,
    Upgrade,
    Hooks,
    Status,
    Version,
    Watch,
    Owners,
    Deps,
    Why,
    Completion,
    Docs,
    Bazel,
}

impl Command {
    pub fn pipe_list() -> String {
        use clap::ValueEnum;
        Self::value_variants()
            .iter()
            .map(|command| command.name())
            .collect::<Vec<_>>()
            .join("|")
    }

    pub fn scope_policy(self) -> &'static str {
        match self {
            Command::Security
            | Command::License
            | Command::Lint
            | Command::Typecheck
            | Command::Format
            | Command::Build
            | Command::Test
            | Command::Coverage
            | Command::Check
            | Command::Fix
            | Command::Migrate => "default-//...",
            Command::Generate | Command::Docs => "default-repo",
            Command::Codegen | Command::Env | Command::Setup => "default-repo|exact-label",
            Command::Deploy => "require-label",
            Command::Update => "selector-default-all",
            Command::Bump => "require-selector+version",
            Command::Run
            | Command::Hooks
            | Command::Watch
            | Command::Owners
            | Command::Deps
            | Command::Completion
            | Command::New => "require",
            Command::Why => "require-file+label",
            Command::Clean | Command::Status | Command::Version | Command::Upgrade => "reject",
            Command::Init => "optional-name",
            Command::Bazel => "passthrough",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Command::Security => "security",
            Command::License => "license",
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
            Command::New => "new",
            Command::Upgrade => "upgrade",
            Command::Hooks => "hooks",
            Command::Status => "status",
            Command::Version => "version",
            Command::Watch => "watch",
            Command::Owners => "owners",
            Command::Deps => "deps",
            Command::Why => "why",
            Command::Completion => "completion",
            Command::Docs => "docs",
            Command::Bazel => "bazel",
        }
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        use clap::ValueEnum;
        Self::from_str(text, false).ok()
    }

    pub fn is_workflow(self) -> bool {
        matches!(
            self,
            Command::Build | Command::Test | Command::Coverage | Command::Run | Command::Deploy
        )
    }

    pub fn is_umbrella(self) -> bool {
        matches!(self, Command::Check | Command::Fix)
    }

    pub fn is_audit_update(self) -> bool {
        matches!(
            self,
            Command::Security | Command::License | Command::Update | Command::Bump
        )
    }

    pub fn is_managed(self) -> bool {
        matches!(self, Command::Codegen | Command::Env | Command::Setup)
    }

    pub fn is_adoption(self) -> bool {
        matches!(
            self,
            Command::Init
                | Command::New
                | Command::Upgrade
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

    pub fn supports_json(self) -> bool {
        matches!(
            self,
            Command::Security
                | Command::License
                | Command::Lint
                | Command::Typecheck
                | Command::Format
                | Command::Generate
                | Command::Build
                | Command::Test
                | Command::Coverage
                | Command::Run
                | Command::Update
                | Command::Bump
                | Command::Migrate
                | Command::Upgrade
                | Command::Check
                | Command::Fix
                | Command::Clean
                | Command::Codegen
                | Command::Env
                | Command::Setup
                | Command::Status
                | Command::Version
                | Command::Owners
                | Command::Deps
                | Command::Why
                | Command::Docs
        )
    }

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

    pub fn supports_here(self) -> bool {
        matches!(
            self,
            Command::Security
                | Command::License
                | Command::Lint
                | Command::Typecheck
                | Command::Format
                | Command::Generate
                | Command::Build
                | Command::Test
                | Command::Coverage
                | Command::Check
                | Command::Fix
                | Command::Docs
        )
    }

    pub fn supports_offline(self) -> bool {
        matches!(
            self,
            Command::Security | Command::License | Command::Update | Command::Bump
        )
    }

    pub fn is_mutating_by_default(self) -> bool {
        matches!(
            self,
            Command::Lint
                | Command::Typecheck
                | Command::Format
                | Command::Update
                | Command::Bump
                | Command::Migrate
                | Command::New
                | Command::Upgrade
                | Command::Generate
                | Command::Codegen
                | Command::Env
                | Command::Setup
                | Command::Init
                | Command::Fix
                | Command::Hooks
        )
    }

    pub fn describe(self) -> &'static str {
        match self {
            Command::Security => "run security audit over resolved scopes (non-mutating; live Gitleaks plus advisory/vuln backends)",
            Command::License => "run license audit over resolved scopes (non-mutating; live license-policy plus SPDX backend)",
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
            Command::New => "scaffold a minimal qualified project for one language (absent-only; mutating by default)",
            Command::Upgrade => "one-shot pin+migrate+setup composition with recovery pointer (mutating by default; --dry-run plans without writes)",
            Command::Hooks => "manage Git hooks via hermetic Git (mutating by default)",
            Command::Status => "report workspace and target status",
            Command::Version => "report version and pin drift",
            Command::Watch => "watch for changes and rebuild (local only)",
            Command::Owners => "query owners of files via Bazel query",
            Command::Deps => "query dependencies of targets",
            Command::Why => "explain why a target depends on another",
            Command::Completion => "emit shell completions from the CLI grammar (bash|zsh|fish|powershell; --check verifies without writing)",
            Command::Docs => "build, check, and serve the unified documentation site (non-mutating; --check validates without rendering)",
            Command::Bazel => "forward raw arguments to the Bazel launcher",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_names_are_stable() {
        assert_eq!(Command::Security.name(), "security");
        assert_eq!(Command::License.name(), "license");
        assert_eq!(Command::Update.name(), "update");
        assert_eq!(Command::Bump.name(), "bump");
        assert_eq!(Command::Migrate.name(), "migrate");
        assert_eq!(Command::New.name(), "new");
        assert_eq!(Command::Upgrade.name(), "upgrade");
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
        assert!(Command::New.is_adoption());
        assert!(Command::Upgrade.is_adoption());
        assert!(Command::New.is_mutating_by_default());
        assert!(Command::Upgrade.is_mutating_by_default());
        assert!(!Command::New.supports_json());
        assert!(Command::Upgrade.supports_json());
        assert!(!Command::New.supports_diff());
        assert!(!Command::Upgrade.supports_diff());
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
        assert_eq!(Command::Docs.name(), "docs");
        assert!(!Command::Docs.is_workflow());
        assert!(!Command::Docs.is_adoption());
        assert!(!Command::Docs.is_managed());
        assert!(!Command::Docs.is_umbrella());
        assert!(!Command::Docs.is_audit_update());
        assert!(!Command::Docs.is_mutating_by_default());
        assert!(Command::Docs.supports_json());
        assert!(!Command::Docs.supports_diff());
        assert!(Command::Docs.supports_here());
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
            Command::Security,
            Command::License,
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
            Command::New,
            Command::Upgrade,
            Command::Hooks,
            Command::Status,
            Command::Version,
            Command::Watch,
            Command::Owners,
            Command::Deps,
            Command::Why,
            Command::Completion,
            Command::Docs,
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
        // exactly the 33 implemented commands (including `deploy` plus
        // `bump` plus `migrate` plus `new` plus `upgrade` plus `docs`).
        // `doctor` and `configure` stay rejected as unknown.
        use clap::ValueEnum;
        let mut got: Vec<&str> = Command::value_variants()
            .iter()
            .map(|command| command.name())
            .collect();
        got.sort_unstable();
        let mut want = vec![
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
            "license",
            "lint",
            "migrate",
            "new",
            "owners",
            "run",
            "security",
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
        assert_eq!(got, want, "Command registry drifted from the final 33");
        assert_eq!(Command::value_variants().len(), 33);
        for excluded in ["doctor", "configure", "bogus"] {
            assert_eq!(
                Command::parse(excluded),
                None,
                "{excluded} must stay rejected as unknown"
            );
        }
    }

    #[test]
    fn mutating_by_default_matches_contract_and_help() {
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
            Command::New,
            Command::Upgrade,
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
            Command::Security,
            Command::License,
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
            Command::Docs,
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

    #[test]
    fn offline_is_audit_update_bump_only() {
        // (`--frozen` alias) forces no fetches on security/license/update/bump only.
        for command in [
            Command::Security,
            Command::License,
            Command::Update,
            Command::Bump,
        ] {
            assert!(
                command.supports_offline(),
                "{command:?} must support --offline"
            );
        }
        for command in [
            Command::Lint,
            Command::Build,
            Command::Clean,
            Command::Codegen,
            Command::Status,
            Command::Docs,
            Command::Bazel,
            Command::Migrate,
        ] {
            assert!(
                !command.supports_offline(),
                "{command:?} must reject --offline"
            );
        }
    }

    #[test]
    fn fallback_usage_registry_is_single_sourced() {
        use clap::ValueEnum;
        let list = Command::pipe_list();
        assert_eq!(
            Command::value_variants().len(),
            33,
            "registry width changed; update scope matrix plus fallbacks"
        );
        let missing_text = super::super::error::ArgsError::MissingCommand.to_string();
        let unknown_text = super::super::error::ArgsError::UnknownCommand {
            command: "bogus".to_owned(),
            suggestion: None,
        }
        .to_string();
        assert!(
            missing_text.contains(&list),
            "ArgsError::MissingCommand drifted from pipe_list: {missing_text}"
        );
        assert!(
            unknown_text.contains(&list),
            "ArgsError::UnknownCommand drifted from pipe_list: {unknown_text}"
        );
        assert_eq!(
            list,
            "security|license|lint|typecheck|format|generate|build|test|coverage|run|deploy|check|fix|clean|update|bump|migrate|codegen|env|setup|init|new|upgrade|hooks|status|version|watch|owners|deps|why|completion|docs|bazel",
            "pipe_list order must match declaration order"
        );
    }

    #[test]
    fn scope_defaults_partition_covers_all_commands() {
        use clap::ValueEnum;
        assert_eq!(Command::value_variants().len(), 33);
        for command in Command::value_variants() {
            let policy = command.scope_policy();
            assert!(
                [
                    "default-//...",
                    "default-repo",
                    "default-repo|exact-label",
                    "require",
                    "require-file+label",
                    "require-label",
                    "require-selector+version",
                    "selector-default-all",
                    "reject",
                    "optional-name",
                    "passthrough",
                ]
                .contains(&policy),
                "{command:?} has unknown scope policy {policy}"
            );
            if command.supports_here() {
                assert!(
                    policy.starts_with("default-"),
                    "{command:?} supports --here so policy must be default-*: {policy}"
                );
            }
            if command.is_managed() {
                assert_eq!(
                    policy, "default-repo|exact-label",
                    "{command:?} managed policy must stay exact-label"
                );
            }
            if command.is_adoption()
                && matches!(
                    command,
                    Command::Status | Command::Version | Command::Upgrade | Command::Completion
                )
            {
                assert!(
                    policy == "reject" || policy == "require",
                    "{command:?} adoption scope must be reject/require: {policy}"
                );
            }
        }
        assert_eq!(Command::Run.scope_policy(), "require");
        assert_eq!(Command::Deploy.scope_policy(), "require-label");
        assert_eq!(Command::Clean.scope_policy(), "reject");
        assert_eq!(Command::Bump.scope_policy(), "require-selector+version");
        assert_eq!(Command::Update.scope_policy(), "selector-default-all");
        assert_eq!(Command::Migrate.scope_policy(), "default-//...");
        assert_eq!(Command::Bazel.scope_policy(), "passthrough");
    }

    #[test]
    fn value_sets_and_man_parity_are_pinned() {
        use clap::ValueEnum;
        // the grammar uses `String`): the error texts name the sets, and
        // the shell list stays exact. `man/dx.1` renders from the same
        // `cli_command` grammar as `--help`, so parity holds by
        // construction and is pinned here.
        let bad_output = super::super::error::ArgsError::BadOutput {
            value: "yaml".to_owned(),
        }
        .to_string();
        assert!(
            bad_output.contains("text|diff|json"),
            "output value set drifted: {bad_output}"
        );
        let bad_fail = super::super::error::ArgsError::BadFailOn {
            value: "never".to_owned(),
        }
        .to_string();
        assert!(
            bad_fail.contains("info|warning|error"),
            "fail-on value set drifted: {bad_fail}"
        );
        let bad_color = super::super::error::ArgsError::BadColor {
            value: "bright".to_owned(),
        }
        .to_string();
        assert!(
            bad_color.contains("auto|always|never"),
            "color value set drifted: {bad_color}"
        );
        assert_eq!(
            super::super::completion::COMPLETION_SHELLS,
            &["bash", "zsh", "fish", "powershell"],
            "shell value set changed; update docs plus fixtures"
        );
        let man = clap_mangen::Man::new(super::super::grammar::cli_command());
        let mut buffer = Vec::new();
        man.render(&mut buffer).expect("man renders");
        let text = String::from_utf8(buffer).expect("man utf8");
        for command in Command::value_variants() {
            assert!(
                text.contains(command.name()),
                "man missing command {}",
                command.name()
            );
        }
        // Man escapes dashes (`\-\-output`), so assert the stems plus the
        // alias token: same grammar as `--help`, parity by construction.
        for stem in ["output", "fail", "here", "cwd", "color", "host", "open"] {
            assert!(text.contains(stem), "man missing flag stem {stem}");
        }
    }
}
