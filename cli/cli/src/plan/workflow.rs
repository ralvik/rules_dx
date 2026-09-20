//! Build/test/coverage workflow Bazel planning.
//!
//! Split from [`super::quality`]: owns the `build`/`test`/`coverage`/`run`
//! workflow planning ([`WorkflowVerb`], [`workflow_options`],
//! [`workflow_protected`], [`plan_workflow`],
//! [`COVERAGE_COMBINED_REPORT_FLAG`]). Re-exported through `super` so the
//! public paths stay `crate::plan::{plan_workflow, WorkflowVerb, ...}`.
//! Shares [`super::BuildPlan`], [`super::workspace_flag`],
//! [`super::BEP_FLAG_NAME`], and [`super::workflow_scope_labels`] with the
//! quality planning in [`super::quality`]; scope resolution for these
//! plans lives in `crate::resolve`.

use dx_process::{build_workflow_argv, describe_scope, ForwardError, ProtectedFlag};

use super::{workflow_scope_labels, workspace_flag, BuildPlan, BEP_FLAG_NAME};
use crate::args::Command;
use crate::resolve::ResolvedScope;

/// Bazel verb behind a workflow command (`build`, `test`, `coverage`, `run`).
/// The verb selects the Bazel command line; required workflow policy is
/// identical across verbs except for the BEP stream, which only
/// `test` and `coverage` collect report artifacts from. `deploy` plans
/// its own build+run argv pair ([`super::run_deploy::plan_deploy_build`]/[`super::run_deploy::plan_deploy_run`])
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
    /// commands, which plan through [`super::quality::plan_build`] instead, and for
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
            // ([`super::managed::plan_managed`]), never a fixed workflow verb.
            Command::Codegen | Command::Env | Command::Setup => None,
            // Audit/update/bump plan through `dx_audit`/`dx_update`/`dx_bump`,
            // never a fixed workflow verb.
            Command::Audit | Command::Update | Command::Bump => None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Profile;
    use dx_process::Scope;

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
}
