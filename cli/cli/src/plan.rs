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
pub mod quality;
pub mod run_deploy;

pub use generate::{
    generate_scope_elements, generate_scope_json, generate_traversal_dirs, plan_generate,
    GenerateScopeElement, GENERATE_CHECK_TARGET, GENERATE_ENV_INTENDED, GENERATE_ENV_MODE,
    GENERATE_ENV_SCOPE, GENERATE_TARGET,
};
pub(crate) use quality::workflow_scope_labels;
pub use quality::{
    plan_build, plan_workflow, protected_flags, required_options, spec, workflow_options,
    workflow_protected, CommandSpec, WorkflowVerb,
};
pub use run_deploy::{plan_deploy_build, plan_deploy_run, plan_run, plan_run_targets};

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::args::Command;
use dx_process::{build_workflow_argv, ForwardError, ProtectedFlag};

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

/// Planned Bazel execution for a quality command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildPlan {
    /// Exact `build` argv: launcher, startup options, `build`, required
    /// options, forwarded user options, repository scope.
    pub argv: Vec<String>,
    /// Human operation display, e.g. `Running lint analysis for //...`.
    pub summary: String,
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

    fn options(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
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
