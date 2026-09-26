#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VersionRequirement {
    ExactPin,
    BoundedRange,
    LockfileOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitRequirement {
    Branch,
    Tag,
    Commit,
}

impl VersionRequirement {
    pub fn may_be_rewritten(self) -> bool {
        let _ = self;
        false
    }

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
    pub fn locked_commit_may_advance(self) -> bool {
        match self {
            GitRequirement::Branch => true,
            GitRequirement::Tag | GitRequirement::Commit => false,
        }
    }

    pub fn may_be_rewritten(self) -> bool {
        let _ = self;
        false
    }
}

pub fn prerelease_follows_upstream() -> bool {
    true
}

pub fn forces_transitive_newest() -> bool {
    false
}

pub fn outside_requirements_is_failure() -> bool {
    false
}

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
