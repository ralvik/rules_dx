//! Pure `dx update` selection planning.
//!
//! Contract: `docs/cli/commands/audit-update-bazel.md#dx-update`.
//!
//! This crate owns the update command surface with live resolver
//! integration: bare selection means every supported dependency set,
//! explicit selectors narrow to sets/packages/targets, and the command is
//! mutating without a confirmation prompt. It plans over injected argument
//! strings only, so selection stays deterministic and unit-testable without
//! a workspace, a Bazel server, or any upstream updater.
//!
//! Frozen command shape (`docs/cli/commands/audit-update-bazel.md`):
//! `dx update [selector ...]`. With no selection every supported
//! dependency set in the repository updates, independent of the current
//! working directory and never via a CLI filesystem scan; set discovery
//! uses the approved set registry derived from the Bazel integrations.
//! V1 supports dependency-set selection, individual packages within
//! selected sets (executed for npm, reported unsupported for other sets
//! rather than silently widened), and Bazel label/pattern/file/dir
//! selectors resolved to owning sets. Transitive changes stay permitted
//! under upstream resolver semantics.
//!
//! Selector syntax and ecosystem package-identity mappings live in
//! [`selector`], the five-set registry in [`sets`], and resolver-owned
//! backend operations in [`backend`]. Independent-set
//! continuation/blocked-dependent execution semantics live in [`outcome`],
//! and aggregate exit-status selection over its reports lives in
//! [`report`]. Per-set commit boundary plus manual recovery planning
//! lives in [`recovery`]. Within-constraint and Git handling live in [`semantics`].

#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

pub mod backend;
pub mod outcome;
pub mod recovery;
pub mod report;
pub mod selector;
pub mod semantics;
pub mod sets;

/// Planned update request: which dependency-set/package selectors the
/// resolver must satisfy. Exact syntax lives in [`selector`]; the
/// spellings here are preserved verbatim for planning summaries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateRequest {
    /// Selector arguments after `dx update`, verbatim. Empty means
    /// every supported repository dependency set.
    pub selectors: Vec<String>,
}

/// Effective update selection: all supported sets, or exactly the
/// named selectors. Set discovery itself is resolver-owned; this only
/// distinguishes the bare repository-wide run from a narrowed one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateSelection {
    /// No selectors: update all supported dependency sets, independent
    /// of the current working directory.
    AllSets,
    /// Explicit selectors (sets and/or packages within sets), verbatim.
    Selected(Vec<String>),
}

impl UpdateRequest {
    /// Plan an update request from the arguments after `dx update`.
    /// Every argument is a selector; nothing is probed or resolved.
    pub fn plan(args: &[String]) -> UpdateRequest {
        UpdateRequest {
            selectors: args.to_vec(),
        }
    }

    /// Effective selection: bare invocations cover all supported sets.
    pub fn selection(&self) -> UpdateSelection {
        if self.selectors.is_empty() {
            UpdateSelection::AllSets
        } else {
            UpdateSelection::Selected(self.selectors.clone())
        }
    }

    /// Update applies immediately: invoking it authorizes application
    /// without an interactive confirmation prompt or a separate
    /// acceptance flag, in terminal and noninteractive use alike.
    /// License-consent and mutation-safety requirements still apply.
    pub fn requires_confirmation() -> bool {
        false
    }

    /// Update mutates: it refreshes standard locks or equivalent
    /// resolved dependency files through the upstream resolver, within
    /// declared requirements. It never widens requirements and owns no
    /// private resolver or lockfile.
    pub fn is_mutating() -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().copied().map(str::to_owned).collect()
    }

    #[test]
    fn bare_update_selects_all_sets() {
        let request = UpdateRequest::plan(&[]);
        assert!(request.selectors.is_empty());
        assert_eq!(request.selection(), UpdateSelection::AllSets);
    }

    #[test]
    fn selectors_pass_through_verbatim() {
        let request = UpdateRequest::plan(&args(&["cargo-lock", "npm-root:react"]));
        assert_eq!(
            request.selection(),
            UpdateSelection::Selected(vec!["cargo-lock".to_owned(), "npm-root:react".to_owned(),])
        );
    }

    #[test]
    fn single_selector_narrows_without_rewriting() {
        let request = UpdateRequest::plan(&args(&["cargo-lock"]));
        assert_eq!(
            request.selection(),
            UpdateSelection::Selected(vec!["cargo-lock".to_owned()])
        );
    }

    #[test]
    fn update_applies_without_confirmation_and_mutates() {
        assert!(!UpdateRequest::requires_confirmation());
        assert!(UpdateRequest::is_mutating());
    }
}
