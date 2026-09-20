//! Within-constraint and Git update semantics.
//!
//! Pure policy pins for the accepted update semantics in the update
//! contract (`docs/cli/commands/audit-update-bazel.md`): selected
//! dependencies move to the newest versions permitted by the declared
//! requirements under the authoritative ecosystem resolver, refreshing
//! standard locks or equivalent resolved files without widening or
//! replacing those requirements. Exact requirement pins remain
//! constraints; versions recorded only in a lockfile may be updated.
//! Prerelease eligibility follows the upstream resolver and project
//! configuration, never a private policy.
//!
//! Git dependencies follow upstream resolution: a declared branch may
//! advance its locked commit, while explicit commit pins and declared
//! tags stay unchanged. Requirements are never rewritten to track
//! another reference or a newer tag, and update never inspects the
//! consumer's Git worktree.
//!
//! Transitive dependencies stay governed by upstream resolution: the
//! command never forces every transitive package to its newest release,
//! and a newer release outside the declared requirements is not an
//! update failure. Resolution failures or unsupported operations are
//! reported, never claimed successful; a successful update is not a
//! guarantee that application code still builds or passes tests.
//!
//! This module classifies over injected requirement descriptors only,
//! so the pins stay deterministic and unit-testable without any
//! resolver. Ecosystem mappings live in [`super::selector`] and upstream
//! operation/report mappings in [`super::backend`].

/// Declared version-requirement shape, as recorded by the project (not
/// inferred by the planner).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VersionRequirement {
    /// Exact pin (e.g. `==1.2.3`): a constraint on resolution, never
    /// widened or replaced by update.
    ExactPin,
    /// Bounded range (e.g. `>=1.2, <2.0`): resolution may move within
    /// it; the range text itself is never rewritten.
    BoundedRange,
    /// No declared requirement: the version is recorded only in the
    /// lockfile and may be updated under upstream resolution.
    LockfileOnly,
}

/// Declared Git-requirement shape, as recorded by the project.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitRequirement {
    /// Declared branch: the locked commit may advance under the
    /// upstream resolver.
    Branch,
    /// Declared tag: the locked commit stays unchanged.
    Tag,
    /// Explicit commit pin: the locked commit stays unchanged.
    Commit,
}

impl VersionRequirement {
    /// Declared requirements are never widened or replaced by update:
    /// they constrain resolution. Pinned here so a future resolver
    /// integration cannot reinterpret update as requirement rewriting.
    pub fn may_be_rewritten(self) -> bool {
        let _ = self;
        false
    }

    /// Whether the locked version may move under upstream resolution
    /// within the declared requirements. Exact pins and bounded ranges
    /// constrain the move; lockfile-only entries may update freely
    /// under the resolver.
    pub fn locked_may_advance(self) -> bool {
        match self {
            VersionRequirement::ExactPin => false,
            VersionRequirement::BoundedRange => true,
            VersionRequirement::LockfileOnly => true,
        }
    }
}

impl GitRequirement {
    /// Only a declared branch may advance its locked commit. Tags and
    /// explicit commit pins stay unchanged; update never rewrites one
    /// reference shape into another.
    pub fn locked_commit_may_advance(self) -> bool {
        match self {
            GitRequirement::Branch => true,
            GitRequirement::Tag | GitRequirement::Commit => false,
        }
    }

    /// Requirement shapes are never rewritten: a branch stays a branch,
    /// a tag stays a tag, a commit pin stays a commit pin.
    pub fn may_be_rewritten(self) -> bool {
        let _ = self;
        false
    }
}

/// Prerelease eligibility follows the upstream resolver and project
/// configuration, never a private `dx` policy. There is no private
/// prerelease rule to configure; this pin exists so a future
/// integration cannot invent one.
pub fn prerelease_follows_upstream() -> bool {
    true
}

/// Transitive dependencies stay governed by upstream resolution: update
/// never forces every transitive package to its newest release
/// regardless of compatibility.
pub fn forces_transitive_newest() -> bool {
    false
}

/// A newer release outside the declared requirements is not an update
/// failure: it is simply out of the within-constraint scope.
pub fn outside_requirements_is_failure() -> bool {
    false
}

/// A successful dependency update carries no build-or-test verdict: the
/// application may still fail to build or pass tests afterwards.
pub fn success_implies_build_passes() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_requirements_are_never_rewritten() {
        for requirement in [
            VersionRequirement::ExactPin,
            VersionRequirement::BoundedRange,
            VersionRequirement::LockfileOnly,
        ] {
            assert!(
                !requirement.may_be_rewritten(),
                "{requirement:?} must constrain, never widen"
            );
        }
        for requirement in [
            GitRequirement::Branch,
            GitRequirement::Tag,
            GitRequirement::Commit,
        ] {
            assert!(
                !requirement.may_be_rewritten(),
                "{requirement:?} must never change reference shape"
            );
        }
    }

    #[test]
    fn lock_advance_follows_requirement_shape() {
        assert!(!VersionRequirement::ExactPin.locked_may_advance());
        assert!(VersionRequirement::BoundedRange.locked_may_advance());
        assert!(VersionRequirement::LockfileOnly.locked_may_advance());
    }

    #[test]
    fn only_branches_advance_locked_commits() {
        assert!(GitRequirement::Branch.locked_commit_may_advance());
        assert!(!GitRequirement::Tag.locked_commit_may_advance());
        assert!(!GitRequirement::Commit.locked_commit_may_advance());
    }

    #[test]
    fn upstream_owns_prerelease_transitives_and_scope() {
        assert!(prerelease_follows_upstream());
        assert!(!forces_transitive_newest());
        assert!(!outside_requirements_is_failure());
        assert!(!success_implies_build_passes());
    }
}
