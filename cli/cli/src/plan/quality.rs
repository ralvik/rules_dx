//! Quality Bazel planning (issue #236).
//!
//! Split from `super` (`plan.rs`): owns the static command registry
//! ([`CommandSpec`]/[`spec`]), the quality required/protected option
//! helpers, [`plan_build`], and the shared scope-label helper
//! ([`workflow_scope_labels`]). The `build`/`test`/`coverage` workflow
//! planning ([`super::workflow::WorkflowVerb`],
//! [`super::workflow::plan_workflow`]) lives in the [`super::workflow`]
//! domain submodule. Re-exported through `super` so the
//! public paths stay `crate::plan::{spec, plan_build, ...}`. Shares the
//! policy leaves
//! ([`super::workspace_flag`], [`super::BuildPlan`], flag constants)
//! with the generate/run-deploy/managed/workflow planning; scope resolution for
//! these plans lives in `crate::resolve`.

use dx_process::{
    build_workflow_argv, describe_scope, operation_summary, ForwardError, ProtectedFlag, Scope,
};

use super::{
    workspace_flag, BuildPlan, BEP_FLAG_NAME, CLIPPY_DIAGNOSTICS_FLAG, KEEP_GOING_FLAG,
    OUTPUT_GROUP, RUSTC_DIAGNOSTICS_FLAG, VALIDATE_FLAG,
};
use crate::args::Command;
use crate::resolve::ResolvedScope;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{parse, ArgsError};
    use crate::plan::WorkflowVerb;
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
}
