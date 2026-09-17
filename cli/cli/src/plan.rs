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
pub mod managed;
pub mod quality;
pub mod run_deploy;
pub mod run_temp;

pub use generate::{
    generate_scope_elements, generate_scope_json, generate_traversal_dirs, plan_generate,
    GenerateScopeElement, GENERATE_CHECK_TARGET, GENERATE_ENV_INTENDED, GENERATE_ENV_MODE,
    GENERATE_ENV_SCOPE, GENERATE_TARGET,
};
pub use managed::{plan_bazel, plan_managed};
pub(crate) use quality::workflow_scope_labels;
pub use quality::{
    plan_build, plan_workflow, protected_flags, required_options, spec, workflow_options,
    workflow_protected, CommandSpec, WorkflowVerb,
};
pub use run_deploy::{plan_deploy_build, plan_deploy_run, plan_run, plan_run_targets};
pub use run_temp::{bep_path, create_run_temp_dir, intended_path, run_nonce};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_policy_matches_committed_default() {
        assert_eq!(WORKSPACE_POLICY_LABEL, "//dx:config");
        assert_eq!(
            workspace_flag(),
            "--@rules_dx//config:workspace=//dx:config"
        );
    }
}
