//! Quality command planning (M07 WP1+WP3, M08 WP1+WP4).
//!
//! Contract: `docs/cli/cli-contract.md` (protected flags, canonical
//! workspace policy, operation display) and
//! `docs/cli/commands/quality.md` (command behavior). Every quality
//! command runs the repository scope (`//...`) unless explicit scope
//! positionals resolve to a narrower scope; labels pass through in
//! order while file and directory scopes resolve to owning targets
//! through [`crate::resolve`].

use std::path::{Path, PathBuf};

use crate::args::Command;
use crate::resolve::ResolvedScope;
use dx_process::{
    build_workflow_argv, describe_scope, operation_summary, ForwardError, ProtectedFlag, Scope,
};

/// Static command registry entry: capability, Bazel aspects, and supported
/// standard-report formats.
///
/// `typecheck` selects no aspects in M07 because no typecheck adapter
/// applies yet; it resolves scope and policy, then succeeds as a silent
/// no-op per `docs/cli/commands/quality.md`. Lint and typecheck export
/// normalized findings as SARIF 2.1.0; format has no initial standard
/// report per `docs/cli/standard-reports.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    pub command: Command,
    pub capability: &'static str,
    pub aspects: &'static [&'static str],
    pub reports: &'static [&'static str],
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
        },
        Command::Typecheck => CommandSpec {
            command,
            capability: "typecheck",
            aspects: &[],
            reports: &["sarif"],
        },
        Command::Format => CommandSpec {
            command,
            capability: "format",
            aspects: &["//quality:real_aspects.bzl%real_format_aspect"],
            reports: &[],
        },
        Command::Generate => CommandSpec {
            command,
            capability: "generate",
            aspects: &[],
            reports: &[],
        },
        Command::Build => CommandSpec {
            command,
            capability: "build",
            aspects: &[],
            reports: &[],
        },
        Command::Test => CommandSpec {
            command,
            capability: "test",
            aspects: &[],
            reports: &["junit"],
        },
        Command::Coverage => CommandSpec {
            command,
            capability: "coverage",
            aspects: &[],
            reports: &["lcov"],
        },
        Command::Run => CommandSpec {
            command,
            capability: "run",
            aspects: &[],
            reports: &[],
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

/// Output group carrying the `QualityResult` protos.
pub const OUTPUT_GROUP: &str = "dx_results";

/// Results from every target are required; failures surface as findings.
pub const KEEP_GOING_FLAG: &str = "--keep_going";

/// Bare Bazel flag name carrying the build-event JSON stream.
pub const BEP_FLAG_NAME: &str = "build_event_json_file";

/// Required workflow options in argv order, placed after `build` and
/// before user options by [`build_workflow_argv`].
pub fn required_options(aspects: &[&str], bep_path: &str) -> Vec<String> {
    vec![
        format!("--aspects={}", aspects.join(",")),
        format!("--output_groups={OUTPUT_GROUP}"),
        workspace_flag(),
        VALIDATE_FLAG.to_owned(),
        KEEP_GOING_FLAG.to_owned(),
        format!("--{BEP_FLAG_NAME}={bep_path}"),
    ]
}

/// Protected workflow flags derived from [`required_options`]. Aspect,
/// output-group, workspace, and validate options reject every user
/// override; `keep_going` accepts repetition of the required value only,
/// and `nokeep_going` is always rejected. The BEP stream has no required
/// value because the CLI chooses a fresh path per run; the user spelling
/// is rejected so collection always observes the actual build.
pub fn protected_flags(required: &[String]) -> Vec<ProtectedFlag> {
    vec![
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
            required: Some(required[4].clone()),
        },
        ProtectedFlag {
            name: "nokeep_going".to_owned(),
            required: None,
        },
        ProtectedFlag {
            name: BEP_FLAG_NAME.to_owned(),
            required: None,
        },
    ]
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
    let required = required_options(entry.aspects, bep_path);
    let protected = protected_flags(&required);
    let scope = if resolved.targets.is_empty() {
        Scope::Repository
    } else {
        resolved.scope.clone()
    };
    let labels = if resolved.targets.is_empty() {
        vec![describe_scope(&Scope::Repository)]
    } else {
        resolved.targets.clone()
    };
    let argv = build_workflow_argv("build", bazel_options, &required, &protected, &labels)?;
    let summary = operation_summary(command.name(), "analysis", &scope);
    Ok(BuildPlan { argv, summary })
}

/// Bazel verb behind a workflow command (`build`, `test`, `coverage`, `run`).
/// The verb selects the Bazel command line; required workflow policy is
/// identical across verbs except for the BEP stream, which only
/// `test` and `coverage` collect report artifacts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowVerb {
    Build,
    Test,
    Coverage,
    Run,
}

impl WorkflowVerb {
    /// Maps a workflow command to its verb. Returns `None` for quality
    /// commands, which plan through [`plan_build`] instead.
    pub fn of(command: Command) -> Option<Self> {
        match command {
            Command::Build => Some(WorkflowVerb::Build),
            Command::Test => Some(WorkflowVerb::Test),
            Command::Coverage => Some(WorkflowVerb::Coverage),
            Command::Run => Some(WorkflowVerb::Run),
            Command::Lint | Command::Typecheck | Command::Format | Command::Generate => None,
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
/// full-target collection, and — for report-collecting verbs — the BEP
/// stream path. Coverage additionally requires `--combined_report=lcov`
/// so Bazel emits LCOV tracefiles. Aspects, output groups, and validation
/// stay off this path: Bazel owns the workflow status. Fail-fast is the
/// default: no `--keep_going` is forced; an explicit user `--keep_going`
/// (or `--nocancel` equivalents) forwards via `bazel_options`.
pub const COVERAGE_COMBINED_REPORT_FLAG: &str = "--combined_report=lcov";

pub fn workflow_options(verb: WorkflowVerb, bep_path: Option<&str>) -> Vec<String> {
    let mut required = vec![workspace_flag()];
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
/// required value only.
pub fn workflow_protected(verb: WorkflowVerb) -> Vec<ProtectedFlag> {
    let mut protected = vec![ProtectedFlag {
        name: "@rules_dx//config:workspace".to_owned(),
        required: None,
    }];
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
/// `None` for `build` and `run`. Fails before execution when user options
/// conflict with required workflow policy.
pub fn plan_workflow(
    verb: WorkflowVerb,
    resolved: &ResolvedScope,
    bazel_options: &[String],
    bep_path: Option<&str>,
) -> Result<BuildPlan, ForwardError> {
    let required = workflow_options(verb, bep_path);
    let protected = workflow_protected(verb);
    let scope = if resolved.targets.is_empty() {
        Scope::Repository
    } else {
        resolved.scope.clone()
    };
    let labels = if resolved.targets.is_empty() {
        vec![describe_scope(&Scope::Repository)]
    } else {
        resolved.targets.clone()
    };
    let argv = build_workflow_argv(verb.name(), bazel_options, &required, &protected, &labels)?;
    // Workflow verbs are self-describing (`Running build for ...`):
    // no phase noun applies.
    let summary = format!("Running {} for {}", verb.name(), describe_scope(&scope));
    Ok(BuildPlan { argv, summary })
}

/// Builds the exact `bazel run` argv for one resolved runnable target.
///
/// `target` is the single runnable label from [`crate::resolve`] (file/dir
/// scopes) or label/pattern passthrough. `app_args` are the verbatim
/// application arguments after `--`: they are never validated as Bazel
/// options and forward after a `--` separator. Only the canonical
/// workspace policy is required; there is no BEP stream, no `keep_going`,
/// and no user Bazel options on this path.
pub fn plan_run(target: &str, app_args: &[String]) -> BuildPlan {
    use dx_process::{launcher_argv0, WORKFLOW_STARTUP_OPTS};

    let mut argv = Vec::with_capacity(WORKFLOW_STARTUP_OPTS.len() + 4 + app_args.len());
    argv.push(launcher_argv0().to_owned());
    argv.extend(WORKFLOW_STARTUP_OPTS.iter().map(ToString::to_string));
    argv.push("run".to_owned());
    argv.push(workspace_flag());
    argv.push(target.to_owned());
    if !app_args.is_empty() {
        argv.push("--".to_owned());
        argv.extend(app_args.iter().cloned());
    }
    let summary = format!("Running run for {target}");
    BuildPlan { argv, summary }
}

/// Canonical Gazelle runner behind `dx generate`: the repo-wide
/// `update` entrypoint from M10 WP1. Scoped runs keep this target and
/// narrow the traversal through positional arguments plus the resolved
/// scope manifest (`DX_GENERATE_SCOPE`) from M10 WP1; per-command result
/// projection stays out of this plan pending O13.
pub const GENERATE_TARGET: &str = "//dx:generate";

/// One resolved scope element for the versioned intended-manifest
/// contract (`DX_GENERATE_SCOPE`, M10 WP1): the owning Bazel target plus
/// the workspace-relative directories the runner must traverse for it.
/// `""` is the workspace root, selected by the repository scope
/// (`//...`); every other directory is relative without a leading `./`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateScopeElement {
    /// Owning resolved target, in `resolved.targets` order.
    pub element: String,
    /// Traversal directories for `element`; exactly one today because
    /// every resolved target owns a single package directory.
    pub dirs: Vec<String>,
}

/// Maps one resolved target to its workspace-relative traversal
/// directory: recursive patterns traverse their own directory, labels
/// traverse their package directory, and the repository scope (`//...`)
/// traverses the root (`""`). Root-package labels (`//:foo`) traverse
/// the root as well.
fn generate_traversal_dir(target: &str) -> String {
    if target == "//..." {
        return String::new();
    }
    let Some(rest) = target.strip_prefix("//") else {
        // Unreachable through `resolve`, which rejects external scopes
        // before planning: stay repo-wide rather than deriving a
        // narrower traversal from a foreign label.
        return String::new();
    };
    if let Some(dir) = rest.strip_suffix("/...") {
        return dir.to_owned();
    }
    match rest.split_once(':') {
        Some((package, _)) => package.to_owned(),
        None => rest.to_owned(),
    }
}

/// Projects a resolved scope onto manifest scope elements in
/// `resolved.targets` order. Empty targets fall back to the repository
/// scope, mirroring [`plan_workflow`].
pub fn generate_scope_elements(resolved: &ResolvedScope) -> Vec<GenerateScopeElement> {
    if resolved.targets.is_empty() {
        return vec![GenerateScopeElement {
            element: describe_scope(&Scope::Repository),
            dirs: vec![String::new()],
        }];
    }
    resolved
        .targets
        .iter()
        .map(|target| GenerateScopeElement {
            element: target.clone(),
            dirs: vec![generate_traversal_dir(target)],
        })
        .collect()
}

/// Renders the manifest scope value (`DX_GENERATE_SCOPE`, M10 WP1): a
/// JSON array of `{"element","dirs"}` objects in
/// [`generate_scope_elements`] order.
pub fn generate_scope_json(resolved: &ResolvedScope) -> String {
    let items: Vec<serde_json::Value> = generate_scope_elements(resolved)
        .iter()
        .map(|element| {
            serde_json::json!({"element": element.element.clone(), "dirs": element.dirs.clone()})
        })
        .collect();
    serde_json::Value::Array(items).to_string()
}

/// Collects the sorted, deduplicated traversal directories for a
/// resolved scope. The repository root (`""`) only appears alone: any
/// narrower directory implies a scoped run.
pub fn generate_traversal_dirs(resolved: &ResolvedScope) -> Vec<String> {
    let mut dirs: Vec<String> = generate_scope_elements(resolved)
        .into_iter()
        .flat_map(|element| element.dirs)
        .collect();
    dirs.sort();
    dirs.dedup();
    dirs
}

/// Builds the exact `bazel run //dx:generate` argv for a resolved
/// scope. Only the canonical workspace policy is required; user options
/// after `--` forward as `run` command options before the target. The
/// resolved traversal directories from [`generate_traversal_dirs`]
/// forward after a second `--` separator as Gazelle positional
/// arguments; the repository root needs no traversal arguments because
/// Gazelle already walks the whole workspace. Fails before execution
/// when user options conflict with required policy. There is no BEP
/// stream: Gazelle owns its output and exit status, and per-command
/// result transport is pending O13.
pub fn plan_generate(
    resolved: &ResolvedScope,
    bazel_options: &[String],
) -> Result<BuildPlan, ForwardError> {
    let required = vec![workspace_flag()];
    let protected = vec![ProtectedFlag {
        name: "@rules_dx//config:workspace".to_owned(),
        required: None,
    }];
    let mut argv = build_workflow_argv(
        "run",
        bazel_options,
        &required,
        &protected,
        &[GENERATE_TARGET.to_owned()],
    )?;
    let dirs = generate_traversal_dirs(resolved);
    let root_only = dirs.len() == 1 && dirs.first().is_some_and(String::is_empty);
    if !dirs.is_empty() && !root_only {
        argv.push("--".to_owned());
        argv.extend(dirs);
    }
    let scope = if resolved.targets.is_empty() {
        Scope::Repository
    } else {
        resolved.scope.clone()
    };
    // `run` verbs are self-describing (`Running generate for ...`):
    // no phase noun applies.
    let summary = format!("Running generate for {}", describe_scope(&scope));
    Ok(BuildPlan { argv, summary })
}

/// BEP stream destination under `temp_dir`, unique per process
/// invocation. `nonce` distinguishes repeated runs inside one process
/// (tests, retries); production callers pass a per-run counter.
pub fn bep_path(temp_dir: &Path, pid: u32, nonce: u64) -> PathBuf {
    temp_dir.join(format!("dx-bep-{pid}-{nonce}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{parse, ArgsError};

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
        let typecheck = spec(Command::Typecheck);
        assert_eq!(typecheck.capability, "typecheck");
        assert!(typecheck.aspects.is_empty());
        assert_eq!(typecheck.reports, &["sarif"]);
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
        assert_eq!(WorkflowVerb::Run.name(), "run");
        assert!(!WorkflowVerb::Run.collects_reports());
    }

    #[test]
    fn run_plan_forwards_app_args_verbatim() {
        let plan = plan_run("//app:bin", &options(&["--port=8080"]));
        assert_eq!(
            plan.argv,
            options(&[
                "bazel",
                "--nohome_rc",
                "--nosystem_rc",
                "run",
                "--@rules_dx//config:workspace=//dx:config",
                "//app:bin",
                "--",
                "--port=8080",
            ])
        );
        assert!(plan.summary.contains("//app:bin"));
        let bare = plan_run("//app:bin", &[]);
        assert!(!bare.argv.contains(&"--".to_owned()));
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
    fn generate_plan_runs_canonical_runner_repo_wide() {
        assert_eq!(GENERATE_TARGET, "//dx:generate");
        let plan = plan_generate(&resolved(&[]), &options(&["--jobs=4"])).expect("plan");
        assert_eq!(
            plan.argv,
            options(&[
                "bazel",
                "--nohome_rc",
                "--nosystem_rc",
                "run",
                "--@rules_dx//config:workspace=//dx:config",
                "--jobs=4",
                "//dx:generate",
            ])
        );
        assert_eq!(plan.summary, "Running generate for //...");
        let bare = plan_generate(&resolved(&[]), &[]).expect("plan");
        assert_eq!(
            bare.argv.last(),
            Some(&GENERATE_TARGET.to_owned()),
            "{bare:?}"
        );
    }

    #[test]
    fn generate_plan_rejects_policy_conflicts_and_startup_options() {
        let err = plan_generate(
            &resolved(&[]),
            &options(&["--@rules_dx//config:workspace=//other:config"]),
        )
        .expect_err("workspace override must fail");
        assert!(
            matches!(err, ForwardError::ConflictingOption { .. }),
            "got {err:?}"
        );
        let err = plan_generate(&resolved(&[]), &options(&["--home_rc"]))
            .expect_err("startup option must fail");
        assert!(matches!(err, ForwardError::StartupOption { .. }));
    }

    #[test]
    fn generate_plan_forwards_scoped_traversal_dirs() {
        let plan = plan_generate(&resolved(&["//b/...", "//a:one"]), &[]).expect("plan");
        assert_eq!(
            plan.argv,
            options(&[
                "bazel",
                "--nohome_rc",
                "--nosystem_rc",
                "run",
                "--@rules_dx//config:workspace=//dx:config",
                "//dx:generate",
                "--",
                "a",
                "b",
            ])
        );
        assert_eq!(plan.summary, "Running generate for //b/... //a:one");
    }

    #[test]
    fn generate_plan_root_package_label_stays_repo_wide() {
        let plan = plan_generate(&resolved(&["//:foo"]), &[]).expect("plan");
        assert!(
            !plan.argv.contains(&"--".to_owned()),
            "root traversal needs no positional arguments: {plan:?}"
        );
        assert_eq!(plan.summary, "Running generate for //:foo");
    }

    #[test]
    fn generate_scope_json_matches_manifest_contract() {
        let json = generate_scope_json(&resolved(&["//a:one", "//b/..."]));
        assert_eq!(
            json,
            r#"[{"dirs":["a"],"element":"//a:one"},{"dirs":["b"],"element":"//b/..."}]"#
        );
        assert_eq!(
            generate_scope_json(&resolved(&[])),
            r#"[{"dirs":[""],"element":"//..."}]"#
        );
    }

    #[test]
    fn generate_traversal_dirs_sort_dedup_and_fall_back() {
        assert_eq!(
            generate_traversal_dirs(&resolved(&["//b/...", "//b/...", "//a:one"])),
            options(&["a", "b"])
        );
        assert_eq!(generate_traversal_dirs(&resolved(&[])), vec![String::new()]);
        // Foreign labels are unreachable through `resolve` (external
        // scopes fail before planning): traversal stays repo-wide.
        assert_eq!(
            generate_traversal_dirs(&resolved(&["@ext//pkg/..."])),
            vec![String::new()]
        );
        assert_eq!(
            generate_traversal_dirs(&resolved(&["//a"])),
            options(&["a"])
        );
        let resolved_owners = ResolvedScope {
            scope: Scope::ResolvedOwners(options(&["//a:one"])),
            targets: options(&["//a:one"]),
        };
        assert_eq!(
            generate_scope_elements(&resolved_owners),
            &[GenerateScopeElement {
                element: "//a:one".to_owned(),
                dirs: vec!["a".to_owned()],
            }]
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
                "--jobs=4",
                "//...",
            ]
        );
        assert_eq!(plan.summary, "Running lint analysis for //...");
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
    fn typecheck_plan_carries_empty_aspects_and_format_summary() {
        let plan =
            plan_build(Command::Typecheck, &resolved(&[]), &[], "/tmp/bep.json").expect("plan");
        assert!(
            plan.argv.iter().any(|arg| arg == "--aspects="),
            "empty aspect list still declares the flag: {plan:?}"
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
        for verb in [
            WorkflowVerb::Build,
            WorkflowVerb::Test,
            WorkflowVerb::Coverage,
        ] {
            let plan = plan_workflow(verb, &resolved(&[]), &[], None).expect("plan");
            assert!(
                !plan.argv.iter().any(|arg| arg == "--keep_going"),
                "{verb:?} must not force keep_going: {plan:?}"
            );
        }
        let plan = plan_workflow(WorkflowVerb::Test, &resolved(&[]), &[], None).expect("plan");
        let argv: Vec<&str> = plan.argv.iter().map(String::as_str).collect();
        assert_eq!(
            argv[..5],
            [
                "bazel",
                "--nohome_rc",
                "--nosystem_rc",
                "test",
                "--@rules_dx//config:workspace=//dx:config",
            ]
        );
        assert_eq!(argv[5..], ["//..."]);
    }

    #[test]
    fn workflow_plan_forwards_explicit_keep_going() {
        let plan = plan_workflow(
            WorkflowVerb::Test,
            &resolved(&[]),
            &options(&["--keep_going"]),
            None,
        )
        .expect("plan");
        assert!(plan.argv.iter().any(|arg| arg == "--keep_going"));
    }

    #[test]
    fn coverage_plan_requires_combined_lcov_report() {
        let plan = plan_workflow(WorkflowVerb::Coverage, &resolved(&[]), &[], None).expect("plan");
        assert!(plan
            .argv
            .iter()
            .any(|arg| arg == COVERAGE_COMBINED_REPORT_FLAG));
        let repeated = plan_workflow(
            WorkflowVerb::Coverage,
            &resolved(&[]),
            &options(&[COVERAGE_COMBINED_REPORT_FLAG]),
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
        )
        .expect_err("BEP override must fail");
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
}
