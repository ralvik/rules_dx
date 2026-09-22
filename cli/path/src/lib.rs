//! Workspace-relative path shape checks shared by every shard validator.
//!
//! Contract: `docs/architecture/README.md`.
//!
//! The empty → absolute → backslash → empty-component → `.` → `..`
//! ladder lives here only (sole owner). Every shard validator
//! (`quality/result`, `generation/result`, `codegen_shard`, `docs/ir`,
//! `env_shard`, `dx_output`, `dx_apply/validators`, `dx_update/manifest`)
//! is a thin wrapper around [`classify`]: it keeps its own error payload
//! and, where its contract needs different wording or allowed rungs, its
//! own message map, but never its own shape checks.
//! Check order is pinned: the first problem in ladder order wins.
//!
//! Dependency evaluation (stays hand-rolled): the ladder is a
//! workspace-relative shape classifier with pinned order and machine-readable
//! [`PathProblem`], not lexical normalization (`path-clean`) nor a UTF-8 path
//! type (`camino`). Adopting those crates would add supply-chain review,
//! lockfile churn, and `MODULE.bazel` manifests for zero behavior gain while
//! changing classifier semantics, so the hand-rolled ladder stays.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

/// One workspace-relative path shape violation, in pinned check order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathProblem {
    Empty,
    Absolute,
    Backslash,
    EmptyComponent,
    Dot,
    DotDot,
}

impl PathProblem {
    /// Canonical human-readable reason for one ladder rung.
    ///
    /// Sole message owner for the `quality/result` + `dx_output` wording;
    /// other contracts keep their own wording (or allowed rungs) and only
    /// reuse [`classify`] for order, with pinned message tests proving
    /// parity or documenting the intentional difference.
    pub fn reason(self) -> &'static str {
        match self {
            PathProblem::Empty => "path must be non-empty",
            PathProblem::Absolute => "path must be workspace-relative, not absolute",
            PathProblem::Backslash => "path must use forward slashes",
            PathProblem::EmptyComponent => "path must have no empty component",
            PathProblem::Dot => "path must have no '.' component",
            PathProblem::DotDot => "path must have no '..' component",
        }
    }
}

/// Classifies the first shape violation in pinned ladder order, or `None`
/// when the path is a well-formed workspace-relative path.
pub fn classify(path: &str) -> Option<PathProblem> {
    if path.is_empty() {
        return Some(PathProblem::Empty);
    }
    if path.starts_with('/') {
        return Some(PathProblem::Absolute);
    }
    if path.contains('\\') {
        return Some(PathProblem::Backslash);
    }
    if path.split('/').any(str::is_empty) {
        return Some(PathProblem::EmptyComponent);
    }
    if path.split('/').any(|component| component == ".") {
        return Some(PathProblem::Dot);
    }
    if path.split('/').any(|component| component == "..") {
        return Some(PathProblem::DotDot);
    }
    None
}

/// Canonical reason for the first ladder violation, or `None` when valid.
///
/// Thin-wrapper shortcut for contracts using the canonical wording
/// (`quality/result`, `dx_output`); other contracts map [`classify`]
/// to their own messages instead.
pub fn reject_reason(path: &str) -> Option<&'static str> {
    classify(path).map(PathProblem::reason)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_workspace_paths_pass() {
        for path in ["src/lib.rs", "a", "a/b/c", "a..b", "a.b/c", ".hidden/x"] {
            assert_eq!(classify(path), None, "path rejected: {path:?}");
        }
    }

    #[test]
    fn each_ladder_rung_classifies() {
        for (path, problem) in [
            ("", PathProblem::Empty),
            ("/absolute", PathProblem::Absolute),
            ("back\\slash", PathProblem::Backslash),
            ("a//b", PathProblem::EmptyComponent),
            ("trailing/", PathProblem::EmptyComponent),
            ("/leading", PathProblem::Absolute),
            ("a/./b", PathProblem::Dot),
            (".", PathProblem::Dot),
            ("a/../b", PathProblem::DotDot),
            ("..", PathProblem::DotDot),
        ] {
            assert_eq!(classify(path), Some(problem), "path: {path:?}");
        }
    }

    #[test]
    fn first_problem_in_ladder_order_wins() {
        // Absolute beats everything below it; backslash beats empty
        // components and dot segments.
        assert_eq!(classify("/a//b"), Some(PathProblem::Absolute));
        assert_eq!(classify("a\\//b"), Some(PathProblem::Backslash));
        assert_eq!(classify("a//./b"), Some(PathProblem::EmptyComponent));
        assert_eq!(classify("a/./../b"), Some(PathProblem::Dot));
    }

    #[test]
    fn canonical_reasons_are_pinned() {
        // Sole message owner for the canonical wording: wrappers using
        // `reason`/`reject_reason` inherit these strings verbatim.
        for (problem, reason) in [
            (PathProblem::Empty, "path must be non-empty"),
            (
                PathProblem::Absolute,
                "path must be workspace-relative, not absolute",
            ),
            (PathProblem::Backslash, "path must use forward slashes"),
            (
                PathProblem::EmptyComponent,
                "path must have no empty component",
            ),
            (PathProblem::Dot, "path must have no '.' component"),
            (PathProblem::DotDot, "path must have no '..' component"),
        ] {
            assert_eq!(problem.reason(), reason, "rung: {problem:?}");
        }
    }

    #[test]
    fn reject_reason_maps_ladder_in_order() {
        for (path, reason) in [
            ("", "path must be non-empty"),
            ("/absolute", "path must be workspace-relative, not absolute"),
            ("back\\slash", "path must use forward slashes"),
            ("a//b", "path must have no empty component"),
            ("a/./b", "path must have no '.' component"),
            ("a/../b", "path must have no '..' component"),
        ] {
            assert_eq!(reject_reason(path), Some(reason), "path: {path:?}");
        }
        assert_eq!(reject_reason("src/lib.rs"), None);
        // First problem in ladder order wins, including for messages.
        assert_eq!(
            reject_reason("/a//b"),
            Some("path must be workspace-relative, not absolute")
        );
        assert_eq!(
            reject_reason("a/./../b"),
            Some("path must have no '.' component")
        );
    }
}
