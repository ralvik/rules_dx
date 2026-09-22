//! `dx generate` Bazel planning (resolve/plan/run/deploy unscramble).
//!
//! Split from `super` (`plan.rs`): owns the canonical Gazelle runner
//! targets, the private-protocol environment names, [`GenerateScopeElement`]
//! plus the scope-projection helpers, and [`plan_generate`]. Re-exported
//! through `super` so the public paths stay
//! `crate::plan::{GENERATE_TARGET, plan_generate, generate_scope_json,
//! ...}`. Shares [`super::BuildPlan`], [`super::workspace_flag`], and
//! [`super::workflow_scope_labels`] with the quality/workflow planning in
//! `super`.

use dx_process::{build_workflow_argv, describe_scope, ForwardError, ProtectedFlag};

use super::{workflow_scope_labels, workspace_flag, BuildPlan};
use crate::resolve::ResolvedScope;

/// Canonical Gazelle runners behind `dx generate`: the repo-wide
/// `update` entrypoint from WP1, plus the non-mutating check
/// entrypoint (dispatch). The check target encodes upstream
/// `-mode diff` in canonical-target wiring: Gazelle computes the same
/// rewrite, witnesses the same intended manifest through
/// `AfterResolvingDeps`, writes no workspace file, and exits nonzero
/// when changes exist. Scoped runs keep the mode target and narrow the
/// traversal through positional arguments plus the resolved scope
/// manifest (`DX_GENERATE_SCOPE`) from WP1.
pub const GENERATE_TARGET: &str = "//dx:generate";
/// Non-mutating Gazelle entrypoint for `dx generate --check`: identical
/// language wiring with upstream `-mode diff`.
pub const GENERATE_CHECK_TARGET: &str = "//dx:generate_check";

/// Private protocol environment the execution wrapper sets on the
/// Gazelle run (WP1, dispatch). Names mirror the extension
/// side (`gazelle/dispatch/manifest.go`); the CLI never reads them back.
pub const GENERATE_ENV_INTENDED: &str = "DX_GENERATE_INTENDED";
/// JSON list of `{"element","dirs"}` scope elements, see
/// [`generate_scope_json`].
pub const GENERATE_ENV_SCOPE: &str = "DX_GENERATE_SCOPE";
/// `"check"` for `--check`, `"default"` otherwise.
pub const GENERATE_ENV_MODE: &str = "DX_GENERATE_MODE";

/// One resolved scope element for the versioned intended-manifest
/// contract (`DX_GENERATE_SCOPE`, WP1): the owning Bazel target plus
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
/// scope through [`workflow_scope_labels`], shared with the other
/// planners.
pub fn generate_scope_elements(resolved: &ResolvedScope) -> Vec<GenerateScopeElement> {
    let (_, labels) = workflow_scope_labels(resolved);
    labels
        .iter()
        .map(|target| GenerateScopeElement {
            element: target.clone(),
            dirs: vec![generate_traversal_dir(target)],
        })
        .collect()
}

/// Renders the manifest scope value (`DX_GENERATE_SCOPE`, WP1): a
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
/// scope, or `bazel run //dx:generate_check` when `check` holds. Only
/// the canonical workspace policy is required; user options after `--`
/// forward as `run` command options before the target. The resolved
/// traversal directories from [`generate_traversal_dirs`] forward after
/// a second `--` separator as Gazelle positional arguments; the
/// repository root needs no traversal arguments because Gazelle
/// already walks the whole workspace. Fails before execution when user
/// options conflict with required policy. There is no BEP stream:
/// Gazelle owns its output and exit status, and the versioned intended
/// manifest carries per-command results.
pub fn plan_generate(
    resolved: &ResolvedScope,
    bazel_options: &[String],
    check: bool,
) -> Result<BuildPlan, ForwardError> {
    let required = vec![workspace_flag()];
    let protected = vec![ProtectedFlag {
        name: "@rules_dx//config:workspace".to_owned(),
        required: None,
    }];
    let target = if check {
        GENERATE_CHECK_TARGET
    } else {
        GENERATE_TARGET
    };
    let mut argv = build_workflow_argv(
        "run",
        bazel_options,
        &required,
        &protected,
        &[target.to_owned()],
    )?;
    let dirs = generate_traversal_dirs(resolved);
    let root_only = dirs.len() == 1 && dirs.first().is_some_and(String::is_empty);
    if !dirs.is_empty() && !root_only {
        argv.push("--".to_owned());
        argv.extend(dirs);
    }
    let (scope, _) = workflow_scope_labels(resolved);
    // `run` verbs are self-describing (`Running generate for ...`):
    // no phase noun applies.
    let summary = format!("Running generate for {}", describe_scope(&scope));
    Ok(BuildPlan { argv, summary })
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn generate_plan_runs_canonical_runner_repo_wide() {
        assert_eq!(GENERATE_TARGET, "//dx:generate");
        assert_eq!(GENERATE_CHECK_TARGET, "//dx:generate_check");
        let plan = plan_generate(&resolved(&[]), &options(&["--jobs=4"]), false).expect("plan");
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
        let bare = plan_generate(&resolved(&[]), &[], false).expect("plan");
        assert_eq!(
            bare.argv.last(),
            Some(&GENERATE_TARGET.to_owned()),
            "{bare:?}"
        );
    }

    #[test]
    fn generate_plan_check_selects_non_mutating_runner() {
        let plan = plan_generate(&resolved(&[]), &[], true).expect("plan");
        assert_eq!(
            plan.argv,
            options(&[
                "bazel",
                "--nohome_rc",
                "--nosystem_rc",
                "run",
                "--@rules_dx//config:workspace=//dx:config",
                "//dx:generate_check",
            ])
        );
        assert_eq!(plan.summary, "Running generate for //...");
        let scoped = plan_generate(&resolved(&["//a:one"]), &[], true).expect("plan");
        assert_eq!(
            scoped.argv.last(),
            Some(&"a".to_owned()),
            "check keeps scoped traversal: {scoped:?}"
        );
        assert!(
            scoped.argv.contains(&GENERATE_CHECK_TARGET.to_owned()),
            "{scoped:?}"
        );
    }

    #[test]
    fn generate_plan_rejects_policy_conflicts_and_startup_options() {
        let err = plan_generate(
            &resolved(&[]),
            &options(&["--@rules_dx//config:workspace=//other:config"]),
            false,
        )
        .expect_err("workspace override must fail");
        assert!(
            matches!(err, ForwardError::ConflictingOption { .. }),
            "got {err:?}"
        );
        let err = plan_generate(&resolved(&[]), &options(&["--home_rc"]), false)
            .expect_err("startup option must fail");
        assert!(matches!(err, ForwardError::StartupOption { .. }));
    }

    #[test]
    fn generate_plan_forwards_scoped_traversal_dirs() {
        let plan = plan_generate(&resolved(&["//b/...", "//a:one"]), &[], false).expect("plan");
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
        let plan = plan_generate(&resolved(&["//:foo"]), &[], false).expect("plan");
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
}
