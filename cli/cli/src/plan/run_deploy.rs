//! `dx run` / `dx deploy` Bazel planning (issue #236 resolve/plan/run/deploy
//! unscramble, handoff from #237).
//!
//! Split from `super` (`plan.rs`): owns [`plan_run_targets`], [`plan_run`],
//! [`plan_deploy_build`], and [`plan_deploy_run`]. Re-exported through
//! `super` so the public paths stay `crate::plan::{plan_run,
//! plan_deploy_build, plan_deploy_run}`. Shares [`super::BuildPlan`] and
//! [`super::workspace_flag`] with the quality/workflow planning in `super`;
//! scope resolution for these plans lives in
//! [`crate::resolve::run_deploy`], carved in the same unscramble.

use super::{workspace_flag, BuildPlan};

/// Builds the exact `bazel run` argv for the resolved runnable targets.
/// `targets` carries one label on the single-runnable path and several
/// on the label-only multi-target path, which `bazel run` rejects with
/// its own diagnostic. `app_args` are the verbatim application
/// arguments after `--`: they are never validated as Bazel options and
/// forward after a `--` separator. Required options are the canonical
/// workspace policy plus the `--config=dx_*` profile pin (always
/// explicit, including the `dx_dev` default); there is no BEP stream,
/// no `keep_going`, and no user Bazel options on this path. This is the
/// single shared builder behind [`plan_run`] and the multi-target
/// dispatch so the launcher, startup options, and workspace policy
/// cannot drift.
pub fn plan_run_targets(
    targets: &[String],
    app_args: &[String],
    profile: crate::args::Profile,
) -> BuildPlan {
    use dx_process::{launcher_argv0, WORKFLOW_STARTUP_OPTS};

    let mut argv =
        Vec::with_capacity(WORKFLOW_STARTUP_OPTS.len() + 4 + targets.len() + app_args.len());
    argv.push(launcher_argv0().to_owned());
    argv.extend(WORKFLOW_STARTUP_OPTS.iter().map(ToString::to_string));
    argv.push("run".to_owned());
    argv.push(workspace_flag());
    argv.push(profile.config_flag());
    argv.extend(targets.iter().cloned());
    if !app_args.is_empty() {
        argv.push("--".to_owned());
        argv.extend(app_args.iter().cloned());
    }
    let summary = format!("Running run for {}", targets.join(" "));
    BuildPlan { argv, summary }
}

/// Builds the exact `bazel run` argv for one resolved runnable target.
///
/// `target` is the single runnable label from [`crate::resolve`] (file/dir
/// scopes) or label/pattern passthrough. `app_args` are the verbatim
/// application arguments after `--`: they are never validated as Bazel
/// options and forward after a `--` separator. Required options are the
/// canonical workspace policy plus the `--config=dx_*` profile pin;
/// there is no BEP stream, no `keep_going`, and no user Bazel options on
/// this path.
pub fn plan_run(target: &str, app_args: &[String], profile: crate::args::Profile) -> BuildPlan {
    plan_run_targets(&[target.to_owned()], app_args, profile)
}

/// Builds the exact `bazel build` argv for one resolved deploy label.
/// Required options are the canonical workspace policy plus the
/// `--config=dx_*` profile pin (always explicit, including the
/// `dx_release` deploy default); there is no BEP stream, no
/// `keep_going`, and no user Bazel options on this path. Shares the
/// launcher, startup options, and workspace policy with [`plan_run`].
pub fn plan_deploy_build(label: &str, profile: crate::args::Profile) -> BuildPlan {
    use dx_process::{launcher_argv0, WORKFLOW_STARTUP_OPTS};

    let mut argv = Vec::with_capacity(WORKFLOW_STARTUP_OPTS.len() + 4);
    argv.push(launcher_argv0().to_owned());
    argv.extend(WORKFLOW_STARTUP_OPTS.iter().map(ToString::to_string));
    argv.push("build".to_owned());
    argv.push(workspace_flag());
    argv.push(profile.config_flag());
    argv.push(label.to_owned());
    let summary = format!("Running deploy build for {label}");
    BuildPlan { argv, summary }
}

/// Builds the exact `bazel run` argv for one resolved deploy label.
/// `app_args` are the verbatim deploy-program arguments after `--`:
/// they are never validated as Bazel options and forward after a `--`
/// separator. Required options mirror [`plan_deploy_build`]; the caller
/// sets `DX_PROFILE` on the run environment. Shares the single-run
/// shape with [`plan_run`] so the launcher and policy cannot drift.
pub fn plan_deploy_run(
    label: &str,
    app_args: &[String],
    profile: crate::args::Profile,
) -> BuildPlan {
    let plan = plan_run(label, app_args, profile);
    let summary = format!("Running deploy run for {label}");
    BuildPlan {
        argv: plan.argv,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Profile;

    fn options(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn run_targets_share_single_builder() {
        let single = plan_run("//app:bin", &options(&["--port=8080"]), Profile::Dev);
        let multi = plan_run_targets(
            &options(&["//app:bin"]),
            &options(&["--port=8080"]),
            Profile::Dev,
        );
        assert_eq!(single, multi);
        let joined = plan_run_targets(&options(&["//a:one", "//b:two"]), &[], Profile::Dev);
        assert_eq!(
            joined.argv.last(),
            Some(&"//b:two".to_owned()),
            "{joined:?}"
        );
        assert!(joined.summary.contains("//a:one //b:two"));
    }

    #[test]
    fn run_plan_forwards_app_args_verbatim() {
        let plan = plan_run("//app:bin", &options(&["--port=8080"]), Profile::Dev);
        assert_eq!(
            plan.argv,
            options(&[
                "bazel",
                "--nohome_rc",
                "--nosystem_rc",
                "run",
                "--@rules_dx//config:workspace=//dx:config",
                "--config=dx_dev",
                "//app:bin",
                "--",
                "--port=8080",
            ])
        );
        assert!(plan.summary.contains("//app:bin"));
        let bare = plan_run("//app:bin", &[], Profile::Dev);
        assert!(!bare.argv.contains(&"--".to_owned()));
    }

    #[test]
    fn run_profile_pins_config_flag() {
        for (profile, flag) in [
            (Profile::Debug, "--config=dx_debug"),
            (Profile::Dev, "--config=dx_dev"),
            (Profile::Release, "--config=dx_release"),
        ] {
            let plan = plan_run("//app:bin", &[], profile);
            let argv: Vec<&str> = plan.argv.iter().map(String::as_str).collect();
            assert_eq!(
                argv[..6],
                [
                    "bazel",
                    "--nohome_rc",
                    "--nosystem_rc",
                    "run",
                    "--@rules_dx//config:workspace=//dx:config",
                    flag,
                ],
                "{profile:?}: {plan:?}"
            );
        }
    }
}
