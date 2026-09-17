//! Quality command planning (M07 WP1+WP3, M08 WP1+WP4).
//!
//! Contract: `docs/cli/cli-contract.md` (protected flags, canonical
//! workspace policy, operation display) and
//! `docs/cli/commands/quality.md` (command behavior). Every quality
//! command runs the repository scope (`//...`) unless explicit scope
//! positionals resolve to a narrower scope; labels pass through in
//! order while file and directory scopes resolve to owning targets
//! through [`crate::resolve`].
//!
//! Run/deploy planning (`plan_run`, `plan_run_targets`,
//! `plan_deploy_build`, `plan_deploy_run`) lives in the
//! [`run_deploy`](self::run_deploy) domain submodule (issue #236
//! resolve/plan/run/deploy unscramble, handoff from issue #237); the
//! public path stays `crate::plan::{...}` via the re-exports below.
//! Scope resolution for these plans lives in
//! [`crate::resolve::run_deploy`], carved in the same unscramble.

pub mod generate;
pub mod run_deploy;

pub use generate::{
    generate_scope_elements, generate_scope_json, generate_traversal_dirs, plan_generate,
    GenerateScopeElement, GENERATE_CHECK_TARGET, GENERATE_ENV_INTENDED, GENERATE_ENV_MODE,
    GENERATE_ENV_SCOPE, GENERATE_TARGET,
};
pub use run_deploy::{plan_deploy_build, plan_deploy_run, plan_run, plan_run_targets};

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::args::Command;
use crate::resolve::ResolvedScope;
use dx_process::{
    build_workflow_argv, describe_scope, operation_summary, ForwardError, ProtectedFlag, Scope,
};

/// Static command registry entry: capability, Bazel aspects, and supported
/// standard-report formats.
///
/// `settings` carries upstream build-setting flags the command's aspects
/// require as workflow mechanism (never user policy): `dx` always sets
/// them and rejects every user override, mirroring the workspace and
/// validate flags.
/// `typecheck` selects the real typecheck aspect (M12 WP3 wires the rustc
/// stage over the rust class); families without a typecheck selection
/// resolve to no stages, so the command stays a silent no-op there per
/// `docs/cli/commands/quality.md`. Lint and typecheck export normalized
/// findings as SARIF 2.1.0; format has no initial standard report per
/// `docs/cli/standard-reports.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    pub command: Command,
    pub capability: &'static str,
    pub aspects: &'static [&'static str],
    pub reports: &'static [&'static str],
    pub settings: &'static [&'static str],
}

/// Returns the registry entry for `command`. Quality commands select
/// capability aspects and SARIF reports; workflow commands run Bazel
/// verbs directly with Bazel-owned status, so they select no aspects.
/// `test` normalizes `test.xml` artifacts into one JUnit document and
/// `coverage` normalizes `coverage.dat` artifacts into one LCOV document;
/// `build` and `run` have no standard report. `generate` runs the
/// canonical `//dx:generate` Gazelle runner with no aspects and no
/// standard report: per-command result transport is pending O13, so
/// machine-readable changes and mutations stay absent.
pub fn spec(command: Command) -> CommandSpec {
    match command {
        Command::Lint => CommandSpec {
            command,
            capability: "lint",
            aspects: &["//quality:real_aspects.bzl%real_lint_aspect"],
            reports: &["sarif"],
            settings: &[CLIPPY_DIAGNOSTICS_FLAG],
        },
        Command::Typecheck => CommandSpec {
            command,
            capability: "typecheck",
            aspects: &["//quality:real_aspects.bzl%real_typecheck_aspect"],
            reports: &["sarif"],
            settings: &[RUSTC_DIAGNOSTICS_FLAG],
        },
        Command::Format => CommandSpec {
            command,
            capability: "format",
            aspects: &["//quality:real_aspects.bzl%real_format_aspect"],
            reports: &[],
            settings: &[],
        },
        Command::Generate => CommandSpec {
            command,
            capability: "generate",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        Command::Build => CommandSpec {
            command,
            capability: "build",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        Command::Test => CommandSpec {
            command,
            capability: "test",
            aspects: &[],
            reports: &["junit"],
            settings: &[],
        },
        Command::Coverage => CommandSpec {
            command,
            capability: "coverage",
            aspects: &[],
            reports: &["lcov"],
            settings: &[],
        },
        Command::Run => CommandSpec {
            command,
            capability: "run",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        Command::Deploy => CommandSpec {
            command,
            capability: "deploy",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        // Sequential umbrellas (M10 WP4, O59) never build Bazel
        // invocations of their own; phases reuse their registries
        // verbatim. The umbrella routes SARIF requests to the
        // SARIF-capable phases (lint, typecheck) and merges runs.
        Command::Check => CommandSpec {
            command,
            capability: "check",
            aspects: &[],
            reports: &["sarif"],
            settings: &[],
        },
        Command::Fix => CommandSpec {
            command,
            capability: "fix",
            aspects: &[],
            reports: &["sarif"],
            settings: &[],
        },
        // Explicit managed-state cleanup (M25 WP5, O60): no Bazel
        // invocation of its own for the prune itself (filesystem
        // inventory plus the shared commit lock in `dx_clean`); the
        // optional `bazel clean` forward is planned at execution.
        Command::Clean => CommandSpec {
            command,
            capability: "clean",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        // Managed environment/codegen/setup selections (M25 WP5): one
        // Bazel collection request behind a canonical selection plus
        // generation commit, never the quality aspect pipeline and no
        // standard reports. The capability names the selecting command.
        Command::Codegen | Command::Env | Command::Setup => CommandSpec {
            command,
            capability: command.name(),
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        // Delivered adoption/inspect surfaces (M30b): local helpers or
        // thin query forwarding, never the quality aspect pipeline.
        Command::Init
        | Command::Hooks
        | Command::Status
        | Command::Version
        | Command::Watch
        | Command::Owners
        | Command::Deps
        | Command::Why
        | Command::Completion => CommandSpec {
            command,
            capability: "adoption",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        // Audit/update planning surfaces (M26 WP1/WP2 slice 1): family
        // selection and dependency-set selectors plan through the
        // `dx_audit`/`dx_update` libraries, never the quality aspect
        // pipeline. Audit exports SARIF through the shared report
        // contract; update has no standard report until O12 backend
        // mappings land.
        Command::Audit => CommandSpec {
            command,
            capability: "audit",
            aspects: &[],
            reports: &["sarif"],
            settings: &[],
        },
        Command::Update => CommandSpec {
            command,
            capability: "update",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        // Raw launcher passthrough (M26 WP4 helper surface): no
        // aspects, no reports, no scope resolution; planned at
        // execution as launcher plus forwarded arguments.
        Command::Bazel => CommandSpec {
            command,
            capability: "bazel",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
    }
}

/// Canonical workspace policy label selected by every quality command.
/// Matches the committed consumer `.bazelrc` default while staying
/// independent from ignored `user.bazelrc`: startup options suppress
/// home and system rc files, the workspace `.bazelrc` remains in effect.
pub const WORKSPACE_POLICY_LABEL: &str = "//dx:config";

/// Canonical workspace policy flag, spelled identically in this
/// repository (where `@rules_dx` resolves to the local module) and in
/// consumer workspaces (where it resolves to the pinned module).
pub fn workspace_flag() -> String {
    format!("--@rules_dx//config:workspace={WORKSPACE_POLICY_LABEL}")
}

/// Validation stays off on the quality path: aspects report through
/// `dx_results` instead of failing the build action.
pub const VALIDATE_FLAG: &str = "--@rules_dx//config:validate=false";

/// Upstream Clippy diagnostics capture (#47): the real lint aspect
/// requires `rust_clippy_aspect`, which writes the authoritative
/// `.clippy.diagnostics` file only when this setting is set. `dx lint`
/// always sets it; the runner parses the file instead of spawning
/// Clippy, so dependency context, edition, and crate type match the
/// real build. Users cannot override it: without the file the clippy
/// stage reports no findings.
pub const CLIPPY_DIAGNOSTICS_FLAG: &str =
    "--@rules_rust//rust/settings:clippy_output_diagnostics=true";

/// Upstream rustc diagnostics capture (#48): the real typecheck aspect
/// reads the authoritative `.rustc-output` file from the `rustc_output`
/// output group, which every Rust rule emits only when this setting is
/// set. `dx typecheck` always sets it; the runner parses the file
/// instead of spawning rustc, so dependency context, edition, and crate
/// type match the real build. Users cannot override it: without the
/// file the rustc stage reports no findings. A crate that fails to
/// compile produces no diagnostics file, so hard type errors fail the
/// `dx typecheck` build itself (with the compiler error visible) rather
/// than arriving as findings.
pub const RUSTC_DIAGNOSTICS_FLAG: &str =
    "--@rules_rust//rust/settings:rustc_output_diagnostics=true";

/// Output group carrying the `QualityResult` protos.
pub const OUTPUT_GROUP: &str = "dx_results";

/// Results from every target are required; failures surface as findings.
pub const KEEP_GOING_FLAG: &str = "--keep_going";

/// Bare Bazel flag name carrying the build-event JSON stream.
pub const BEP_FLAG_NAME: &str = "build_event_json_file";

/// Required workflow options in argv order, placed after `build` and
/// before user options by [`build_workflow_argv`]. Command settings
/// (upstream build-setting flags the aspects require) travel last.
/// [`protected_flags`] locates `keep_going` by value, so this order is
/// a display choice, not a positional contract.
pub fn required_options(entry: &CommandSpec, bep_path: &str) -> Vec<String> {
    let mut options = vec![
        format!("--aspects={}", entry.aspects.join(",")),
        format!("--output_groups={OUTPUT_GROUP}"),
        workspace_flag(),
        VALIDATE_FLAG.to_owned(),
        KEEP_GOING_FLAG.to_owned(),
        format!("--{BEP_FLAG_NAME}={bep_path}"),
    ];
    options.extend(entry.settings.iter().map(ToString::to_string));
    options
}

/// Bare setting name for a required `--name=value` option: strips the
/// leading dashes and any `=value`, so
/// `--@rules_rust//rust/settings:clippy_output_diagnostics=true`
/// protects `@rules_rust//rust/settings:clippy_output_diagnostics`.
/// Fails when `option` is not a `--name[=value]` workflow option
/// instead of silently protecting a wrong name.
fn setting_name(option: &str) -> Result<String, ForwardError> {
    let bare = option
        .strip_prefix("--")
        .ok_or_else(|| ForwardError::InvalidSetting {
            option: option.to_owned(),
        })?;
    let name = bare.split('=').next().unwrap_or("");
    if name.is_empty() {
        return Err(ForwardError::InvalidSetting {
            option: option.to_owned(),
        });
    }
    Ok(name.to_owned())
}

/// Protected workflow flags derived from [`required_options`]. Aspect,
/// output-group, workspace, and validate options reject every user
/// override; `keep_going` accepts repetition of the required value only
/// (located by value in `required`, never by position),
/// and `nokeep_going` is always rejected; command settings reject every
/// user override like the other mechanism flags. The BEP stream has no required
/// value because the CLI chooses a fresh path per run; the user spelling
/// is rejected so collection always observes the actual build.
/// Fails when the required `keep_going` entry or a command setting is
/// malformed instead of panicking or hiding the miswiring.
pub fn protected_flags(
    required: &[String],
    settings: &[&str],
) -> Result<Vec<ProtectedFlag>, ForwardError> {
    let keep_going = required
        .iter()
        .find(|option| option.as_str() == KEEP_GOING_FLAG)
        .cloned()
        .ok_or_else(|| ForwardError::InvalidRequiredOption {
            flag: "keep_going".to_owned(),
        })?;
    let mut flags = vec![
        ProtectedFlag {
            name: "aspects".to_owned(),
            required: None,
        },
        ProtectedFlag {
            name: "output_groups".to_owned(),
            required: None,
        },
        ProtectedFlag {
            name: "@rules_dx//config:workspace".to_owned(),
            required: None,
        },
        ProtectedFlag {
            name: "@rules_dx//config:validate".to_owned(),
            required: None,
        },
        ProtectedFlag {
            name: "keep_going".to_owned(),
            required: Some(keep_going),
        },
        ProtectedFlag {
            name: "nokeep_going".to_owned(),
            required: None,
        },
        ProtectedFlag {
            name: BEP_FLAG_NAME.to_owned(),
            required: None,
        },
    ];
    for setting in settings {
        flags.push(ProtectedFlag {
            name: setting_name(setting)?,
            required: None,
        });
    }
    Ok(flags)
}

/// Planned Bazel execution for a quality command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildPlan {
    /// Exact `build` argv: launcher, startup options, `build`, required
    /// options, forwarded user options, repository scope.
    pub argv: Vec<String>,
    /// Human operation display, e.g. `Running lint analysis for //...`.
    pub summary: String,
}

/// Resolved scope with its exact Bazel labels. Empty targets select
/// the repository scope (`//...`); every planner shares this fallback
/// so scope handling cannot drift between commands.
pub(crate) fn workflow_scope_labels(resolved: &ResolvedScope) -> (Scope, Vec<String>) {
    if resolved.targets.is_empty() {
        (Scope::Repository, vec![describe_scope(&Scope::Repository)])
    } else {
        (resolved.scope.clone(), resolved.targets.clone())
    }
}

/// Builds the exact workflow argv for `command` over a resolved scope.
/// `resolved.targets` supplies the exact Bazel targets (empty selects
/// the repository scope `//...`) and `resolved.scope` renders the
/// operation summary. `bep_path` receives the build-event JSON stream
/// the CLI collects with `dx_bep`. Fails before execution when user
/// options conflict with required workflow policy.
pub fn plan_build(
    command: Command,
    resolved: &ResolvedScope,
    bazel_options: &[String],
    bep_path: &str,
) -> Result<BuildPlan, ForwardError> {
    let entry = spec(command);
    let required = required_options(&entry, bep_path);
    let protected = protected_flags(&required, entry.settings)?;
    let (scope, labels) = workflow_scope_labels(resolved);
    let argv = build_workflow_argv("build", bazel_options, &required, &protected, &labels)?;
    let summary = operation_summary(command.name(), "analysis", &scope);
    Ok(BuildPlan { argv, summary })
}

/// Bazel verb behind a workflow command (`build`, `test`, `coverage`, `run`).
/// The verb selects the Bazel command line; required workflow policy is
/// identical across verbs except for the BEP stream, which only
/// `test` and `coverage` collect report artifacts from. `deploy` plans
/// its own build+run argv pair ([`plan_deploy_build`]/[`plan_deploy_run`])
/// and never maps to a single verb.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowVerb {
    Build,
    Test,
    Coverage,
    Run,
}

impl WorkflowVerb {
    /// Maps a workflow command to its verb. Returns `None` for quality
    /// commands, which plan through [`plan_build`] instead, and for
    /// `deploy`, which plans a build+run pair instead of one verb.
    pub fn of(command: Command) -> Option<Self> {
        match command {
            Command::Build => Some(WorkflowVerb::Build),
            Command::Test => Some(WorkflowVerb::Test),
            Command::Coverage => Some(WorkflowVerb::Coverage),
            Command::Run => Some(WorkflowVerb::Run),
            Command::Deploy => None,
            Command::Lint | Command::Typecheck | Command::Format | Command::Generate => None,
            Command::Check | Command::Fix | Command::Clean => None,
            // Managed selections plan their own collection argv
            // ([`plan_managed`]), never a fixed workflow verb.
            Command::Codegen | Command::Env | Command::Setup => None,
            // Audit/update plan through `dx_audit`/`dx_update`,
            // never a fixed workflow verb.
            Command::Audit | Command::Update => None,
            // Raw launcher passthrough plans its own argv (launcher
            // plus forwarded arguments), never a fixed workflow verb.
            Command::Bazel => None,
            Command::Init
            | Command::Hooks
            | Command::Status
            | Command::Version
            | Command::Watch
            | Command::Owners
            | Command::Deps
            | Command::Why
            | Command::Completion => None,
        }
    }

    /// Stable Bazel command name.
    pub fn name(self) -> &'static str {
        match self {
            WorkflowVerb::Build => "build",
            WorkflowVerb::Test => "test",
            WorkflowVerb::Coverage => "coverage",
            WorkflowVerb::Run => "run",
        }
    }

    /// True when the verb collects report artifacts from a BEP stream.
    /// `build` and `run` have no standard report, so they plan no BEP flag.
    pub fn collects_reports(self) -> bool {
        matches!(self, WorkflowVerb::Test | WorkflowVerb::Coverage)
    }
}

/// Required workflow options in argv order: canonical workspace policy,
/// the build-profile config, full-target collection, and — for
/// report-collecting verbs — the BEP stream path. Coverage additionally
/// requires `--combined_report=lcov` so Bazel emits LCOV tracefiles.
/// Aspects, output groups, and validation stay off this path: Bazel owns
/// the workflow status. Fail-fast is the default: no `--keep_going` is
/// forced; an explicit user `--keep_going` (or `--nocancel`
/// equivalents) forwards via `bazel_options`.
///
/// `profile` is `Some` for `build`/`test` (always an explicit
/// `--config=dx_*`, including the `dx_dev` default) and `None` for
/// `coverage`, which has no profile flags in issue #179 scope: its argv
/// is unchanged and bare Bazel behavior already equals `fastbuild`.
pub const COVERAGE_COMBINED_REPORT_FLAG: &str = "--combined_report=lcov";

pub fn workflow_options(
    verb: WorkflowVerb,
    bep_path: Option<&str>,
    profile: Option<crate::args::Profile>,
) -> Vec<String> {
    let mut required = vec![workspace_flag()];
    if let Some(profile) = profile {
        required.push(profile.config_flag());
    }
    if verb == WorkflowVerb::Coverage {
        required.push(COVERAGE_COMBINED_REPORT_FLAG.to_owned());
    }
    if let Some(path) = bep_path {
        required.push(format!("--{BEP_FLAG_NAME}={path}"));
    }
    required
}

/// Protected workflow flags: workspace and BEP reject every user
/// override; coverage `combined_report` accepts repetition of the
/// required value only. The build-profile `--config=dx_*` (issue #179)
/// accepts repetition of the required value only, so an explicit user
/// `--config` that conflicts with the resolved profile fails before
/// execution instead of silently overriding it.
pub fn workflow_protected(
    verb: WorkflowVerb,
    profile: Option<crate::args::Profile>,
) -> Vec<ProtectedFlag> {
    let mut protected = vec![ProtectedFlag {
        name: "@rules_dx//config:workspace".to_owned(),
        required: None,
    }];
    if let Some(profile) = profile {
        protected.push(ProtectedFlag {
            name: "config".to_owned(),
            required: Some(profile.config_flag()),
        });
    }
    if verb == WorkflowVerb::Coverage {
        protected.push(ProtectedFlag {
            name: "combined_report".to_owned(),
            required: Some(COVERAGE_COMBINED_REPORT_FLAG.to_owned()),
        });
    }
    protected.push(ProtectedFlag {
        name: BEP_FLAG_NAME.to_owned(),
        required: None,
    });
    protected
}

/// Builds the exact workflow argv for a `build`, `test`, `coverage`, or
/// `run` command over a resolved scope. `resolved.targets` supplies the exact
/// Bazel targets (empty selects the repository scope `//...`) and
/// `resolved.scope` renders the operation summary. `bep_path` carries
/// the build-event JSON stream for report-collecting verbs and must be
/// `None` for `build` and `run`. `profile` carries the `--config=dx_*`
/// pin for `build`/`test` (always `Some`, including the default) and
/// must be `None` for `coverage` (no profile flags). Fails before
/// execution when user options conflict with required workflow policy.
pub fn plan_workflow(
    verb: WorkflowVerb,
    resolved: &ResolvedScope,
    bazel_options: &[String],
    bep_path: Option<&str>,
    profile: Option<crate::args::Profile>,
) -> Result<BuildPlan, ForwardError> {
    let required = workflow_options(verb, bep_path, profile);
    let protected = workflow_protected(verb, profile);
    let (scope, labels) = workflow_scope_labels(resolved);
    let argv = build_workflow_argv(verb.name(), bazel_options, &required, &protected, &labels)?;
    // Workflow verbs are self-describing (`Running build for ...`):
    // no phase noun applies.
    let summary = format!("Running {} for {}", verb.name(), describe_scope(&scope));
    Ok(BuildPlan { argv, summary })
}

/// Builds the exact raw launcher argv for `dx bazel`: the launcher
/// followed by the verbatim forwarded arguments, per
/// `docs/cli/commands/audit-update-bazel.md` ("arguments unchanged").
/// No startup options, no workspace policy, no protected flags, no
/// scope resolution, no reports: unlike the quality and workflow
/// paths, the escape hatch applies no rc suppression, so the user's
/// home and system rc files behave exactly as they do under a direct
/// `bazel` invocation from the same workspace.
pub fn plan_bazel(forwarded: &[String]) -> BuildPlan {
    let mut argv = Vec::with_capacity(1 + forwarded.len());
    argv.push(dx_process::launcher_argv0().to_owned());
    argv.extend(forwarded.iter().cloned());
    let summary = if forwarded.is_empty() {
        "Running bazel".to_owned()
    } else {
        format!("Running bazel {}", forwarded.join(" "))
    };
    BuildPlan { argv, summary }
}

/// Builds the exact `bazel build` argv for a managed
/// environment/codegen/setup selection (M25 WP5) over a validated setup
/// scope: the command's collection roots with its collecting aspects and
/// private output groups, plus the canonical workspace policy and the
/// BEP stream path the CLI collects with `dx_bep`. Root computation
/// delegates to each command's own planning library so the WP4 (O34)
/// root benchmark flows through unchanged; user options after `--`
/// forward after the required policy. Fails before execution when user
/// options conflict with required collection policy. The caller owns
/// scope validation ([`dx_setup::resolve_scope`]); `command` must be
/// managed (`codegen`, `env`, `setup`) and any other command fails with
/// [`ForwardError::UnsupportedCommand`] instead of panicking.
pub fn plan_managed(
    command: Command,
    scope: &dx_setup::SetupScope,
    bazel_options: &[String],
    bep_path: &str,
) -> Result<BuildPlan, ForwardError> {
    let unsupported = || ForwardError::UnsupportedCommand {
        command: command.name().to_owned(),
    };
    if !command.is_managed() {
        return Err(unsupported());
    }
    let (roots, aspects, output_groups) = match command {
        Command::Codegen => {
            let scope = match scope {
                dx_setup::SetupScope::Repository => dx_codegen::CodegenScope::Repository,
                dx_setup::SetupScope::Exact(label) => {
                    dx_codegen::CodegenScope::Exact(label.clone())
                }
            };
            (
                dx_codegen::scope_targets(&scope),
                vec![dx_codegen::CODEGEN_ASPECT.to_owned()],
                vec![dx_codegen::OUTPUT_GROUP.to_owned()],
            )
        }
        Command::Env => {
            let scope = match scope {
                dx_setup::SetupScope::Repository => dx_env_plan::EnvScope::Repository,
                dx_setup::SetupScope::Exact(label) => dx_env_plan::EnvScope::Exact(label.clone()),
            };
            (
                dx_env_plan::scope_targets(&scope),
                vec![dx_env_plan::ENV_ASPECT.to_owned()],
                vec![dx_env_plan::OUTPUT_GROUP.to_owned()],
            )
        }
        Command::Setup => {
            let request = dx_setup::plan_request(scope);
            (request.roots, request.aspects, request.output_groups)
        }
        _ => return Err(unsupported()),
    };
    let mut required = Vec::with_capacity(aspects.len() + output_groups.len() + 2);
    for aspect in &aspects {
        required.push(format!("--aspects={aspect}"));
    }
    for group in &output_groups {
        required.push(format!("--output_groups={group}"));
    }
    required.push(workspace_flag());
    required.push(format!("--{BEP_FLAG_NAME}={bep_path}"));
    let protected = vec![
        ProtectedFlag {
            name: "aspects".to_owned(),
            required: None,
        },
        ProtectedFlag {
            name: "output_groups".to_owned(),
            required: None,
        },
        ProtectedFlag {
            name: "@rules_dx//config:workspace".to_owned(),
            required: None,
        },
        ProtectedFlag {
            name: BEP_FLAG_NAME.to_owned(),
            required: None,
        },
    ];
    let argv = build_workflow_argv("build", bazel_options, &required, &protected, &roots)?;
    let display = match scope {
        dx_setup::SetupScope::Repository => "//...",
        dx_setup::SetupScope::Exact(label) => label,
    };
    let summary = format!("Running {} for {}", command.name(), display);
    Ok(BuildPlan { argv, summary })
}

/// BEP stream destination under `temp_dir`, unique per process
/// invocation. `nonce` distinguishes repeated runs inside one process
/// (tests, retries); production callers pass a per-run counter.
pub fn bep_path(temp_dir: &Path, pid: u32, nonce: u64) -> PathBuf {
    temp_dir.join(format!("dx-bep-{pid}-{nonce}.json"))
}

/// Intended-manifest destination under `temp_dir`, unique per process
/// invocation like [`bep_path`]. The wrapper passes it as
/// [`GENERATE_ENV_INTENDED`] so the Gazelle extension witnesses its
/// exact BUILD changes there for the finalizer.
pub fn intended_path(temp_dir: &Path, pid: u32, nonce: u64) -> PathBuf {
    temp_dir.join(format!("dx-generate-{pid}-{nonce}.json"))
}

/// Per-process run counter feeding [`run_nonce`]. `Relaxed` suffices:
/// no happens-before edge is needed, only atomicity — every fetch
/// yields a distinct value even under concurrent callers.
///
/// Cross-process uniqueness comes from the [`tempfile`] directory itself
/// (exclusive create with a random suffix); the nonce only needs to be
/// unique within this process because BEP/intended files live inside the
/// unique directory.
static RUN_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Fresh nonce for one launcher run (issue #93).
///
/// A process-local monotonic counter: every call in this process yields a
/// distinct value. Cross-process collisions are harmless — each run owns a
/// unique [`tempfile::TempDir`], so identical nonces in different processes
/// name files in different directories.
pub fn run_nonce() -> u64 {
    RUN_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Creates a fresh unique run directory and returns it with the nonce
/// the caller must forward as [`crate::exec::Env::nonce`] so BEP and
/// intended-manifest paths share the run's uniqueness.
///
/// Uses [`tempfile::Builder`] with prefix `dx-run-`: exclusive create plus
/// internal retry closes the PID-recycle collision window that hand-rolled
/// `create_dir` loops used to cover. The returned [`tempfile::TempDir`]
/// auto-cleans on drop; callers that need a removal warning should call
/// [`tempfile::TempDir::close`] explicitly.
pub fn create_run_temp_dir(base: &Path) -> std::io::Result<(tempfile::TempDir, u64)> {
    let dir = tempfile::Builder::new()
        .prefix("dx-run-")
        .tempdir_in(base)?;
    let nonce = run_nonce();
    Ok((dir, nonce))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{parse, ArgsError, Profile};

    fn options(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    fn resolved(targets: &[&str]) -> ResolvedScope {
        ResolvedScope {
            scope: if targets.is_empty() {
                Scope::Repository
            } else {
                Scope::Labels(options(targets))
            },
            targets: options(targets),
        }
    }

    #[test]
    fn registry_entries_match_command_contract() {
        let lint = spec(Command::Lint);
        assert_eq!(lint.capability, "lint");
        assert_eq!(
            lint.aspects,
            &["//quality:real_aspects.bzl%real_lint_aspect"]
        );
        assert_eq!(lint.reports, &["sarif"]);
        assert_eq!(lint.settings, &[CLIPPY_DIAGNOSTICS_FLAG]);
        let typecheck = spec(Command::Typecheck);
        assert_eq!(typecheck.capability, "typecheck");
        assert_eq!(
            typecheck.aspects,
            &["//quality:real_aspects.bzl%real_typecheck_aspect"]
        );
        assert_eq!(typecheck.reports, &["sarif"]);
        assert_eq!(typecheck.settings, &[RUSTC_DIAGNOSTICS_FLAG]);
        let format = spec(Command::Format);
        assert_eq!(format.capability, "format");
        assert_eq!(
            format.aspects,
            &["//quality:real_aspects.bzl%real_format_aspect"]
        );
        assert!(format.reports.is_empty());
        let build = spec(Command::Build);
        assert!(build.aspects.is_empty());
        assert!(build.reports.is_empty());
        let test = spec(Command::Test);
        assert_eq!(test.reports, &["junit"]);
        let coverage = spec(Command::Coverage);
        assert_eq!(coverage.reports, &["lcov"]);
        let run = spec(Command::Run);
        assert_eq!(run.capability, "run");
        assert!(run.aspects.is_empty());
        assert!(run.reports.is_empty());
        let generate = spec(Command::Generate);
        assert_eq!(generate.capability, "generate");
        assert!(generate.aspects.is_empty());
        assert!(generate.reports.is_empty());
        assert_eq!(WorkflowVerb::of(Command::Generate), None);
        assert_eq!(WorkflowVerb::of(Command::Run), Some(WorkflowVerb::Run));
        assert_eq!(WorkflowVerb::of(Command::Lint), None);
        assert_eq!(WorkflowVerb::of(Command::Typecheck), None);
        assert_eq!(WorkflowVerb::of(Command::Format), None);
        assert_eq!(WorkflowVerb::of(Command::Check), None);
        assert_eq!(WorkflowVerb::of(Command::Fix), None);
        let clean = spec(Command::Clean);
        assert_eq!(clean.capability, "clean");
        assert!(clean.aspects.is_empty());
        assert!(clean.reports.is_empty());
        assert_eq!(WorkflowVerb::of(Command::Clean), None);
        let bazel = spec(Command::Bazel);
        assert_eq!(bazel.capability, "bazel");
        assert!(bazel.aspects.is_empty());
        assert!(bazel.reports.is_empty());
        assert_eq!(WorkflowVerb::of(Command::Bazel), None);
        for command in [Command::Codegen, Command::Env, Command::Setup] {
            let entry = spec(command);
            assert_eq!(entry.capability, command.name());
            assert!(entry.aspects.is_empty());
            assert!(entry.reports.is_empty());
            assert_eq!(WorkflowVerb::of(command), None);
            assert!(command.is_managed());
        }
        let audit = spec(Command::Audit);
        assert_eq!(audit.capability, "audit");
        assert!(audit.aspects.is_empty());
        assert_eq!(audit.reports, &["sarif"]);
        assert_eq!(WorkflowVerb::of(Command::Audit), None);
        assert!(Command::Audit.is_audit_update());
        let update = spec(Command::Update);
        assert_eq!(update.capability, "update");
        assert!(update.aspects.is_empty());
        assert!(update.reports.is_empty());
        assert_eq!(WorkflowVerb::of(Command::Update), None);
        assert!(Command::Update.is_audit_update());
        assert_eq!(WorkflowVerb::Run.name(), "run");
        assert!(!WorkflowVerb::Run.collects_reports());
    }

    #[test]
    fn managed_plan_builds_collection_request() {
        use dx_setup::SetupScope;

        let plan = plan_managed(
            Command::Codegen,
            &SetupScope::Repository,
            &options(&["--jobs=4"]),
            "/tmp/bep.json",
        )
        .expect("plan");
        assert_eq!(
            plan.argv,
            options(&[
                "bazel",
                "--nohome_rc",
                "--nosystem_rc",
                "build",
                "--aspects=//generation:codegen.bzl%dx_codegen_plan_aspect",
                "--output_groups=dx_codegen_plans",
                "--@rules_dx//config:workspace=//dx:config",
                "--build_event_json_file=/tmp/bep.json",
                "--jobs=4",
                "//dx:codegen",
            ])
        );
        assert_eq!(plan.summary, "Running codegen for //...");
        let plan = plan_managed(Command::Env, &SetupScope::Repository, &[], "/tmp/bep.json")
            .expect("plan");
        assert!(plan
            .argv
            .iter()
            .any(|arg| arg == "--aspects=//env:plan.bzl%dx_env_plan_aspect"));
        assert!(plan
            .argv
            .iter()
            .any(|arg| arg == "--output_groups=dx_env_plans"));
        assert_eq!(plan.argv.last(), Some(&"//dx:env".to_owned()));
        assert_eq!(plan.summary, "Running env for //...");
        let plan = plan_managed(
            Command::Setup,
            &SetupScope::Repository,
            &[],
            "/tmp/bep.json",
        )
        .expect("plan");
        assert_eq!(
            plan.argv
                .iter()
                .filter(|arg| arg.starts_with("--aspects="))
                .count(),
            2,
            "setup requests both collecting aspects: {plan:?}"
        );
        assert_eq!(
            plan.argv
                .iter()
                .filter(|arg| arg.starts_with("--output_groups="))
                .count(),
            2,
            "setup requests both output groups: {plan:?}"
        );
        assert!(plan.argv.contains(&"//dx:codegen".to_owned()));
        assert!(plan.argv.contains(&"//dx:env".to_owned()));
        assert_eq!(plan.summary, "Running setup for //...");
    }

    #[test]
    fn managed_plan_exact_scope_selects_label() {
        use dx_setup::SetupScope;

        for command in [Command::Codegen, Command::Env, Command::Setup] {
            let scope = SetupScope::Exact("//a:one".to_owned());
            let plan = plan_managed(command, &scope, &[], "/tmp/bep.json").expect("plan");
            assert_eq!(plan.argv.last(), Some(&"//a:one".to_owned()), "{command:?}");
            assert_eq!(
                plan.summary,
                format!("Running {} for //a:one", command.name())
            );
        }
    }

    #[test]
    fn managed_plan_rejects_policy_conflicts_and_startup_options() {
        use dx_setup::SetupScope;

        for conflicting in [
            "--aspects=//other.bzl%aspect",
            "--output_groups=other",
            "--@rules_dx//config:workspace=//other:config",
            "--build_event_json_file=/tmp/other.json",
        ] {
            let err = plan_managed(
                Command::Setup,
                &SetupScope::Repository,
                &options(&[conflicting]),
                "/tmp/bep.json",
            )
            .expect_err("conflict must fail");
            assert!(
                matches!(err, ForwardError::ConflictingOption { .. }),
                "{conflicting} produced {err:?}"
            );
        }
        let err = plan_managed(
            Command::Codegen,
            &SetupScope::Repository,
            &options(&["--home_rc"]),
            "/tmp/bep.json",
        )
        .expect_err("startup option must fail");
        assert!(matches!(err, ForwardError::StartupOption { .. }));
    }

    #[test]
    fn protected_flags_find_keep_going_by_value() {
        let entry = spec(Command::Lint);
        let required = required_options(&entry, "/tmp/bep.json");
        let flags = protected_flags(&required, entry.settings).expect("flags");
        let keep_going = flags
            .iter()
            .find(|flag| flag.name == "keep_going")
            .expect("keep_going guard");
        assert_eq!(keep_going.required, Some(KEEP_GOING_FLAG.to_owned()));
    }

    #[test]
    fn protected_flags_reject_miswired_required_and_settings() {
        let entry = spec(Command::Lint);
        let mut required = required_options(&entry, "/tmp/bep.json");
        required.retain(|option| option.as_str() != KEEP_GOING_FLAG);
        let err = protected_flags(&required, entry.settings).expect_err("missing keep_going");
        assert!(
            matches!(err, ForwardError::InvalidRequiredOption { .. }),
            "missing keep_going produced {err:?}"
        );
        let required = required_options(&entry, "/tmp/bep.json");
        let err = protected_flags(&required, &["clippy_output_diagnostics=true"])
            .expect_err("malformed setting");
        assert!(
            matches!(err, ForwardError::InvalidSetting { .. }),
            "malformed setting produced {err:?}"
        );
    }

    #[test]
    fn managed_plan_rejects_unmanaged_commands() {
        use dx_setup::SetupScope;

        for command in [Command::Build, Command::Lint, Command::Run] {
            let err = plan_managed(command, &SetupScope::Repository, &[], "/tmp/bep.json")
                .expect_err("unmanaged command must fail");
            assert!(
                matches!(err, ForwardError::UnsupportedCommand { .. }),
                "{command:?} produced {err:?}"
            );
        }
    }

    #[test]
    fn bazel_plan_forwards_arguments_verbatim() {
        let plan = plan_bazel(&options(&["build", "//...", "--jobs=4"]));
        assert_eq!(
            plan.argv,
            options(&["bazel", "build", "//...", "--jobs=4",])
        );
        assert!(plan.summary.contains("build //..."));
        let bare = plan_bazel(&[]);
        assert_eq!(bare.argv, options(&["bazel"]));
    }

    #[test]
    fn workspace_policy_matches_committed_default() {
        assert_eq!(WORKSPACE_POLICY_LABEL, "//dx:config");
        assert_eq!(
            workspace_flag(),
            "--@rules_dx//config:workspace=//dx:config"
        );
    }

    #[test]
    fn lint_plan_argv_places_required_before_user_options() {
        let plan = plan_build(
            Command::Lint,
            &resolved(&[]),
            &options(&["--jobs=4"]),
            "/tmp/bep.json",
        )
        .expect("plan");
        let argv: Vec<&str> = plan.argv.iter().map(String::as_str).collect();
        assert_eq!(
            argv[..7],
            [
                "bazel",
                "--nohome_rc",
                "--nosystem_rc",
                "build",
                "--aspects=//quality:real_aspects.bzl%real_lint_aspect",
                "--output_groups=dx_results",
                "--@rules_dx//config:workspace=//dx:config",
            ]
        );
        assert_eq!(
            argv[7..],
            [
                "--@rules_dx//config:validate=false",
                "--keep_going",
                "--build_event_json_file=/tmp/bep.json",
                "--@rules_rust//rust/settings:clippy_output_diagnostics=true",
                "--jobs=4",
                "//...",
            ]
        );
        assert_eq!(plan.summary, "Running lint analysis for //...");
    }

    #[test]
    fn lint_plan_enables_upstream_clippy_diagnostics() {
        let plan = plan_build(Command::Lint, &resolved(&[]), &[], "/tmp/bep.json").expect("plan");
        assert!(
            plan.argv.iter().any(|arg| arg == CLIPPY_DIAGNOSTICS_FLAG),
            "lint sets the upstream diagnostics setting: {plan:?}"
        );
        for command in [Command::Typecheck, Command::Format] {
            let plan = plan_build(command, &resolved(&[]), &[], "/tmp/bep.json").expect("plan");
            assert!(
                plan.argv.iter().all(|arg| arg != CLIPPY_DIAGNOSTICS_FLAG),
                "{command:?} leaves the clippy setting off: {plan:?}"
            );
        }
    }

    #[test]
    fn typecheck_plan_enables_upstream_rustc_diagnostics() {
        let plan =
            plan_build(Command::Typecheck, &resolved(&[]), &[], "/tmp/bep.json").expect("plan");
        assert!(
            plan.argv.iter().any(|arg| arg == RUSTC_DIAGNOSTICS_FLAG),
            "typecheck sets the upstream diagnostics setting: {plan:?}"
        );
        for command in [Command::Lint, Command::Format] {
            let plan = plan_build(command, &resolved(&[]), &[], "/tmp/bep.json").expect("plan");
            assert!(
                plan.argv.iter().all(|arg| arg != RUSTC_DIAGNOSTICS_FLAG),
                "{command:?} leaves the rustc setting off: {plan:?}"
            );
        }
    }

    #[test]
    fn explicit_targets_replace_repository_scope() {
        let plan = plan_build(
            Command::Lint,
            &resolved(&["//a:one", "//b/..."]),
            &[],
            "/tmp/bep.json",
        )
        .expect("plan");
        let argv: Vec<&str> = plan.argv.iter().map(String::as_str).collect();
        assert_eq!(argv[argv.len() - 2..], ["//a:one", "//b/..."]);
        assert_eq!(plan.summary, "Running lint analysis for //a:one //b/...");
    }

    #[test]
    fn resolved_owners_render_sorted_owners_in_summary() {
        let scope = ResolvedScope {
            scope: Scope::ResolvedOwners(options(&["//a:a", "//b:b"])),
            targets: options(&["//a:a", "//b:b"]),
        };
        let plan = plan_build(Command::Lint, &scope, &[], "/tmp/bep.json").expect("plan");
        let argv: Vec<&str> = plan.argv.iter().map(String::as_str).collect();
        assert_eq!(argv[argv.len() - 2..], ["//a:a", "//b:b"]);
        assert_eq!(plan.summary, "Running lint analysis for //a:a //b:b");
    }

    #[test]
    fn typecheck_plan_carries_typecheck_aspect_and_format_summary() {
        let plan =
            plan_build(Command::Typecheck, &resolved(&[]), &[], "/tmp/bep.json").expect("plan");
        assert!(
            plan.argv
                .iter()
                .any(|arg| arg == "--aspects=//quality:real_aspects.bzl%real_typecheck_aspect"),
            "typecheck selects the real typecheck aspect: {plan:?}"
        );
        assert_eq!(plan.summary, "Running typecheck analysis for //...");
        let plan = plan_build(Command::Format, &resolved(&[]), &[], "/tmp/bep.json").expect("plan");
        assert_eq!(plan.summary, "Running format analysis for //...");
    }

    #[test]
    fn keep_going_repetition_is_accepted() {
        let plan = plan_build(
            Command::Lint,
            &resolved(&[]),
            &options(&["--keep_going"]),
            "/tmp/bep.json",
        )
        .expect("plan");
        assert!(plan.argv.iter().any(|arg| arg == "--keep_going"));
    }

    #[test]
    fn conflicting_workflow_options_fail_before_execution() {
        for conflicting in [
            "--aspects=//other.bzl%aspect",
            "--output_groups=other",
            "--@rules_dx//config:workspace=//other:config",
            "--@rules_dx//config:validate=true",
            "--nokeep_going",
            "--build_event_json_file=/tmp/other.json",
            "--@rules_rust//rust/settings:clippy_output_diagnostics=false",
            "--@rules_rust//rust/settings:clippy_output_diagnostics",
        ] {
            let err = plan_build(
                Command::Lint,
                &resolved(&[]),
                &options(&[conflicting]),
                "/tmp/bep.json",
            )
            .expect_err("conflict must fail");
            assert!(
                matches!(err, ForwardError::ConflictingOption { .. }),
                "{conflicting} produced {err:?}"
            );
        }
        for conflicting in [
            "--@rules_rust//rust/settings:rustc_output_diagnostics=false",
            "--@rules_rust//rust/settings:rustc_output_diagnostics",
        ] {
            let err = plan_build(
                Command::Typecheck,
                &resolved(&[]),
                &options(&[conflicting]),
                "/tmp/bep.json",
            )
            .expect_err("conflict must fail");
            assert!(
                matches!(err, ForwardError::ConflictingOption { .. }),
                "{conflicting} produced {err:?}"
            );
        }
    }

    #[test]
    fn startup_options_are_rejected_as_command_options() {
        let err = plan_build(
            Command::Lint,
            &resolved(&[]),
            &options(&["--home_rc"]),
            "/tmp/bep.json",
        )
        .expect_err("startup option must fail");
        assert!(matches!(err, ForwardError::StartupOption { .. }));
    }

    #[test]
    fn unsupported_report_format_fails_with_command_registry() {
        let invocation = parse(&options(&["lint", "--report=junit=out.xml"])).expect("parse");
        let entry = spec(invocation.command);
        assert!(
            !entry
                .reports
                .contains(&invocation.reports[0].format.as_str()),
            "junit is not a lint report: {invocation:?}"
        );
        let invocation = parse(&options(&["format", "--report=sarif=out.sarif"])).expect("parse");
        let entry = spec(invocation.command);
        assert!(
            !entry
                .reports
                .contains(&invocation.reports[0].format.as_str()),
            "format has no standard report: {invocation:?}"
        );
        assert_eq!(
            parse(&options(&["lint", "--report=sarif"])),
            Err(ArgsError::BadReport {
                value: "sarif".to_owned(),
            })
        );
    }

    #[test]
    fn workflow_plan_defaults_to_fail_fast_without_forced_keep_going() {
        for (verb, profile) in [
            (WorkflowVerb::Build, Some(Profile::Dev)),
            (WorkflowVerb::Test, Some(Profile::Dev)),
            (WorkflowVerb::Coverage, None),
        ] {
            let plan = plan_workflow(verb, &resolved(&[]), &[], None, profile).expect("plan");
            assert!(
                !plan.argv.iter().any(|arg| arg == "--keep_going"),
                "{verb:?} must not force keep_going: {plan:?}"
            );
        }
        let plan = plan_workflow(
            WorkflowVerb::Test,
            &resolved(&[]),
            &[],
            None,
            Some(Profile::Dev),
        )
        .expect("plan");
        let argv: Vec<&str> = plan.argv.iter().map(String::as_str).collect();
        assert_eq!(
            argv[..6],
            [
                "bazel",
                "--nohome_rc",
                "--nosystem_rc",
                "test",
                "--@rules_dx//config:workspace=//dx:config",
                "--config=dx_dev",
            ]
        );
        assert_eq!(argv[6..], ["//..."]);
    }

    #[test]
    fn workflow_plan_forwards_explicit_keep_going() {
        let plan = plan_workflow(
            WorkflowVerb::Test,
            &resolved(&[]),
            &options(&["--keep_going"]),
            None,
            Some(Profile::Dev),
        )
        .expect("plan");
        assert!(plan.argv.iter().any(|arg| arg == "--keep_going"));
    }

    #[test]
    fn coverage_plan_requires_combined_lcov_report() {
        let plan =
            plan_workflow(WorkflowVerb::Coverage, &resolved(&[]), &[], None, None).expect("plan");
        assert!(plan
            .argv
            .iter()
            .any(|arg| arg == COVERAGE_COMBINED_REPORT_FLAG));
        let repeated = plan_workflow(
            WorkflowVerb::Coverage,
            &resolved(&[]),
            &options(&[COVERAGE_COMBINED_REPORT_FLAG]),
            None,
            None,
        )
        .expect("repeated required flag is accepted");
        assert!(repeated
            .argv
            .iter()
            .any(|arg| arg == COVERAGE_COMBINED_REPORT_FLAG));
        let err = plan_workflow(
            WorkflowVerb::Coverage,
            &resolved(&[]),
            &options(&["--combined_report=json"]),
            None,
            None,
        )
        .expect_err("conflicting combined_report must fail");
        assert!(
            matches!(err, ForwardError::ConflictingOption { .. }),
            "got {err:?}"
        );
        let err = plan_workflow(
            WorkflowVerb::Test,
            &resolved(&[]),
            &options(&["--build_event_json_file=/tmp/other.json"]),
            Some("/tmp/bep.json"),
            Some(Profile::Dev),
        )
        .expect_err("BEP override must fail");
        assert!(
            matches!(err, ForwardError::ConflictingOption { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn workflow_profile_pins_config_flag_in_order() {
        // Fixture pinning the issue #179 mapping: flag profiles select
        // their `--config=dx_*` right after the workspace policy; the
        // bare default is the explicit `dx_dev`.
        for (profile, flag) in [
            (Profile::Debug, "--config=dx_debug"),
            (Profile::Dev, "--config=dx_dev"),
            (Profile::Release, "--config=dx_release"),
        ] {
            let plan = plan_workflow(
                WorkflowVerb::Build,
                &resolved(&[]),
                &[],
                None,
                Some(profile),
            )
            .expect("plan");
            let argv: Vec<&str> = plan.argv.iter().map(String::as_str).collect();
            assert_eq!(
                argv[..6],
                [
                    "bazel",
                    "--nohome_rc",
                    "--nosystem_rc",
                    "build",
                    "--@rules_dx//config:workspace=//dx:config",
                    flag,
                ],
                "{profile:?}: {plan:?}"
            );
        }
        // Coverage carries no profile pin (no flags in #179 scope).
        let plan =
            plan_workflow(WorkflowVerb::Coverage, &resolved(&[]), &[], None, None).expect("plan");
        assert!(
            !plan.argv.iter().any(|arg| arg.starts_with("--config=")),
            "coverage argv is unchanged: {plan:?}"
        );
        // Repeating the required profile value is accepted and
        // canonicalized; a conflicting `--config` fails before
        // execution instead of silently overriding the profile.
        let repeated = plan_workflow(
            WorkflowVerb::Build,
            &resolved(&[]),
            &options(&["--config=dx_dev"]),
            None,
            Some(Profile::Dev),
        )
        .expect("repeated required config is accepted");
        assert!(repeated.argv.contains(&"--config=dx_dev".to_owned()));
        let err = plan_workflow(
            WorkflowVerb::Build,
            &resolved(&[]),
            &options(&["--config=dx_release"]),
            None,
            Some(Profile::Dev),
        )
        .expect_err("conflicting config must fail");
        assert!(
            matches!(err, ForwardError::ConflictingOption { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn bep_paths_are_unique_per_run() {
        let dir = Path::new("/tmp/dx");
        assert_eq!(
            bep_path(dir, 42, 0),
            PathBuf::from("/tmp/dx/dx-bep-42-0.json")
        );
        assert_ne!(bep_path(dir, 42, 0), bep_path(dir, 42, 1));
        assert_ne!(bep_path(dir, 42, 0), bep_path(dir, 43, 0));
    }

    #[test]
    fn run_nonces_are_unique_in_process() {
        use std::collections::HashSet;
        let seen: HashSet<u64> = (0..512).map(|_| run_nonce()).collect();
        assert_eq!(seen.len(), 512, "counter-backed nonces must not repeat");
    }

    #[test]
    fn create_run_temp_dir_is_unique_and_real() {
        let base = std::env::temp_dir();
        let (first, first_nonce) = create_run_temp_dir(&base).expect("first run dir");
        let (second, second_nonce) = create_run_temp_dir(&base).expect("second run dir");
        assert_ne!(
            first.path(),
            second.path(),
            "concurrent runs must never share a directory"
        );
        assert_ne!(first_nonce, second_nonce);
        assert!(first.path().is_dir() && second.path().is_dir());
        assert_eq!(first.path().parent(), Some(base.as_path()));
        assert_eq!(second.path().parent(), Some(base.as_path()));
        for dir in [&first, &second] {
            let name = dir
                .path()
                .file_name()
                .expect("run dir has a file name")
                .to_string_lossy();
            assert!(
                name.starts_with("dx-run-"),
                "run dir {name:?} must carry the dx-run- prefix"
            );
        }
        // `TempDir` auto-cleans on drop; explicit close asserts removal works.
        first.close().expect("cleanup first");
        second.close().expect("cleanup second");
    }
}
