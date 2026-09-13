//! Combined setup request planning for the `dx` CLI (M25 WP3 slice 1).
//!
//! Contract: `docs/cli/commands/environment-codegen-setup.md` (`dx setup`
//! scope: no argument prepares repository-wide codegen and environment
//! generations, one exact target label prepares that target, paths/
//! patterns/multiple labels/profiles/language selectors rejected) and
//! `docs/environments/managed-state.md` (one Bazel request with the union
//! of required roots, both aspects, both output groups, and one BEP
//! stream; codegen and env never invoke each other).
//!
//! This crate owns scope resolution and the combined request plan only:
//! which Bazel roots to build with which aspects and output groups in the
//! single setup request. Bare `dx setup` builds the two canonical
//! repository selections (`//dx:codegen` plus `//dx:env`); an exact label
//! builds that one label with both aspects and both output groups applied,
//! where provider applicability bounds each aspect to targets with a
//! nonempty effective stage subset. Staging, validation, carry-forward,
//! the commit lock, and the atomic `.dx/setups/current` replacement land
//! in later WP3 slices; no filesystem mutation happens here.

/// Codegen collecting aspect applied in the combined request. Matches
/// `dx_codegen_plan_aspect` in `//generation:codegen.bzl` and
/// `CODEGEN_ASPECT` in `dx_codegen`.
pub const CODEGEN_ASPECT: &str = "//generation:codegen.bzl%dx_codegen_plan_aspect";

/// Environment collecting aspect applied in the combined request.
/// Matches `dx_env_plan_aspect` in `//env:plan.bzl` and `ENV_ASPECT` in
/// `dx_env_plan`.
pub const ENV_ASPECT: &str = "//env:plan.bzl%dx_env_plan_aspect";

/// Private output group carrying collected codegen shards plus every
/// referenced generated artifact. Matches
/// `DX_CODEGEN_PLAN_OUTPUT_GROUP` in `//generation:codegen.bzl` and
/// `OUTPUT_GROUP` in `dx_codegen`.
pub const CODEGEN_OUTPUT_GROUP: &str = "dx_codegen_plans";

/// Private output group carrying collected env shards plus every
/// referenced artifact. Matches `DX_ENV_PLAN_OUTPUT_GROUP` in
/// `//env:plan.bzl` and `OUTPUT_GROUP` in `dx_env_plan`.
pub const ENV_OUTPUT_GROUP: &str = "dx_env_plans";

/// Canonical repository-wide codegen selection built by bare `dx setup`.
/// Matches `REPOSITORY_TARGET` in `dx_codegen`. The effective Bazel roots
/// behind this label stay provisional pending the WP4 (O34) root
/// benchmark; this crate only owns the selection identity.
pub const CODEGEN_REPOSITORY_TARGET: &str = "//dx:codegen";

/// Canonical repository-wide env selection built by bare `dx setup`.
/// Matches `REPOSITORY_TARGET` in `dx_env_plan`.
pub const ENV_REPOSITORY_TARGET: &str = "//dx:env";

/// Selected setup scope: the whole repository or one exact target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupScope {
    Repository,
    Exact(String),
}

/// Exact-target scope failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeError {
    /// More than one positional target: setup selects at most one.
    MultipleTargets { count: usize },
    /// A target pattern (`...`, `*`, `?`): patterns never select setup.
    TargetPattern { value: String },
    /// Anything that is not an exact target label: paths, directories,
    /// profiles/flags, and language selectors.
    NotTargetLabel { value: String },
}

impl std::fmt::Display for ScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ScopeError {}

/// Resolves the `dx setup` positional scope: empty selects the
/// repository (both canonical selections), one exact `//` or `@` label
/// selects its configured closure, and anything else fails before
/// execution. A compatible target whose closure contributes no shards
/// selects empty sides downstream (with carry-forward or managed empty
/// generations), never a failure here.
pub fn resolve_scope(targets: &[String]) -> Result<SetupScope, ScopeError> {
    match targets {
        [] => Ok(SetupScope::Repository),
        [single] => {
            if single.contains("...") || single.contains('*') || single.contains('?') {
                Err(ScopeError::TargetPattern {
                    value: single.clone(),
                })
            } else if single.starts_with("//") || single.starts_with('@') {
                Ok(SetupScope::Exact(single.clone()))
            } else {
                Err(ScopeError::NotTargetLabel {
                    value: single.clone(),
                })
            }
        }
        _ => Err(ScopeError::MultipleTargets {
            count: targets.len(),
        }),
    }
}

/// Bazel labels to build for `scope`: both canonical repository
/// selections, or the one exact label. The combined request builds this
/// root set once; Bazel deduplicates common configured closures and
/// actions across the two aspects.
pub fn scope_targets(scope: &SetupScope) -> Vec<String> {
    match scope {
        SetupScope::Repository => vec![
            CODEGEN_REPOSITORY_TARGET.to_owned(),
            ENV_REPOSITORY_TARGET.to_owned(),
        ],
        SetupScope::Exact(label) => vec![label.clone()],
    }
}

/// Collecting aspects applied together in the single setup request: the
/// codegen plan aspect plus the env plan aspect. Codegen and env never
/// invoke each other; each aspect stays bounded by provider
/// applicability to targets with a nonempty effective stage subset.
pub fn request_aspects() -> Vec<String> {
    vec![CODEGEN_ASPECT.to_owned(), ENV_ASPECT.to_owned()]
}

/// Output groups requested together in the single setup request: the
/// codegen plan group plus the env plan group, collected from one BEP
/// stream. No separate env/codegen Bazel commands or output bases are
/// used.
pub fn request_output_groups() -> Vec<String> {
    vec![CODEGEN_OUTPUT_GROUP.to_owned(), ENV_OUTPUT_GROUP.to_owned()]
}

/// The planned single Bazel request behind `dx setup`: the union of
/// required roots with both collecting aspects and both output groups.
/// Later slices execute this request, split the one BEP stream by output
/// group into the codegen and env collectors, and commit both sides (or
/// neither) through the atomic current-pointer replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupRequest {
    pub roots: Vec<String>,
    pub aspects: Vec<String>,
    pub output_groups: Vec<String>,
}

/// Plans the single combined Bazel request for `scope`.
pub fn plan_request(scope: &SetupScope) -> SetupRequest {
    SetupRequest {
        roots: scope_targets(scope),
        aspects: request_aspects(),
        output_groups: request_output_groups(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_identities_match_codegen_and_env() {
        assert_eq!(
            CODEGEN_ASPECT,
            "//generation:codegen.bzl%dx_codegen_plan_aspect"
        );
        assert_eq!(ENV_ASPECT, "//env:plan.bzl%dx_env_plan_aspect");
        assert_eq!(CODEGEN_OUTPUT_GROUP, "dx_codegen_plans");
        assert_eq!(ENV_OUTPUT_GROUP, "dx_env_plans");
        assert_eq!(CODEGEN_REPOSITORY_TARGET, "//dx:codegen");
        assert_eq!(ENV_REPOSITORY_TARGET, "//dx:env");
    }

    #[test]
    fn empty_scope_selects_both_repository_targets() {
        assert_eq!(resolve_scope(&[]), Ok(SetupScope::Repository));
        assert_eq!(
            scope_targets(&SetupScope::Repository),
            vec!["//dx:codegen", "//dx:env"]
        );
    }

    #[test]
    fn exact_labels_pass_through() {
        for label in ["//app:server", "@rules_dx//generation:result_proto_rs"] {
            let scope = resolve_scope(&[label.to_owned()]).expect("exact label");
            assert_eq!(scope, SetupScope::Exact(label.to_owned()));
            assert_eq!(scope_targets(&scope), vec![label.to_owned()]);
        }
    }

    #[test]
    fn scope_rejects_non_exact_inputs() {
        assert_eq!(
            resolve_scope(&["//a:x".to_owned(), "//b:y".to_owned()]),
            Err(ScopeError::MultipleTargets { count: 2 })
        );
        for pattern in ["//...", "//app/...", "//app:*", "@repo//pkg:all?"] {
            assert_eq!(
                resolve_scope(&[pattern.to_owned()]),
                Err(ScopeError::TargetPattern {
                    value: pattern.to_owned(),
                }),
                "pattern {pattern:?} must fail"
            );
        }
        for other in ["app/server", "gen", "rust", "--profile=fast", ":relative"] {
            assert_eq!(
                resolve_scope(&[other.to_owned()]),
                Err(ScopeError::NotTargetLabel {
                    value: other.to_owned(),
                }),
                "non-label {other:?} must fail"
            );
        }
    }

    #[test]
    fn scope_errors_display() {
        assert!(ScopeError::MultipleTargets { count: 2 }
            .to_string()
            .contains("MultipleTargets"));
    }

    #[test]
    fn repository_request_unions_both_sides() {
        let request = plan_request(&SetupScope::Repository);
        assert_eq!(request.roots, vec!["//dx:codegen", "//dx:env"]);
        assert_eq!(
            request.aspects,
            vec![
                "//generation:codegen.bzl%dx_codegen_plan_aspect",
                "//env:plan.bzl%dx_env_plan_aspect",
            ]
        );
        assert_eq!(
            request.output_groups,
            vec!["dx_codegen_plans", "dx_env_plans"]
        );
    }

    #[test]
    fn exact_request_applies_both_sides_to_one_root() {
        let request = plan_request(&SetupScope::Exact("//app:server".to_owned()));
        assert_eq!(request.roots, vec!["//app:server"]);
        assert_eq!(request.aspects.len(), 2);
        assert_eq!(request.output_groups.len(), 2);
    }

    #[test]
    fn request_sides_are_deterministic() {
        let first = plan_request(&SetupScope::Repository);
        let second = plan_request(&SetupScope::Repository);
        assert_eq!(first, second);
    }
}
