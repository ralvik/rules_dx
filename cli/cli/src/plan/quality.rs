//! Quality Bazel planning (issue #236).
//!
//! Split from `super` (`plan.rs`): owns the quality required/protected
//! option helpers, [`plan_build`], and the shared scope-label helper
//! ([`workflow_scope_labels`]). The static command registry
//! ([`super::registry::CommandSpec`], [`super::registry::spec`]) lives
//! in the [`super::registry`] domain submodule; the `build`/`test`/
//! `coverage` workflow planning ([`super::workflow::WorkflowVerb`],
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
    registry::{spec, CommandSpec},
    workspace_flag, BuildPlan, BEP_FLAG_NAME, KEEP_GOING_FLAG, OUTPUT_GROUP, VALIDATE_FLAG,
};
use crate::args::Command;
use crate::resolve::ResolvedScope;

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
    use crate::plan::{CLIPPY_DIAGNOSTICS_FLAG, RUSTC_DIAGNOSTICS_FLAG};
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
