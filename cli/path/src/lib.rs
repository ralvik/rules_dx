//! Workspace-relative path shape checks shared by every shard validator.
//!
//! Contract: `docs/architecture/README.md`.
//!
//! The empty → absolute → backslash → empty-component →
//! `.` → `..` ladder was copy-pasted across `quality/result`,
//! `codegen_shard`, `docs/ir`, `env_shard`, and
//! `dx_apply/validators`, each with its own messages. The classifier here
//! reports the machine-readable [`PathProblem`] only; callers keep their
//! own error payloads and messages, so adopting it is behavior-preserving.
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
}
