//! Shared schema-version helpers for versioned shard protocols (#72).
//!
//! `quality/result`, `generation/result`, `documentation/ir`, and `dx/output`
//! all version their wire form with `schema_major = 1` / `schema_minor = 0`.
//! Major bumps are breaking (decode must fail); minor bumps are
//! forward-compatible (newer minors decode when their bytes satisfy the
//! current rules). [`SchemaVersion`] carries a decoded `(major, minor)` pair;
//! [`check_major`] enforces the breaking axis only, so callers keep their own
//! `UnsupportedMajor`-style payloads and messages.

/// Frozen schema major accepted by every versioned shard protocol.
pub const SCHEMA_MAJOR: u32 = 1;
/// Schema minor the crates were written against. Newer minors decode when
/// their bytes satisfy the current rules.
pub const SCHEMA_MINOR: u32 = 0;

/// One decoded `(schema_major, schema_minor)` pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaVersion {
    /// Decoded `schema_major` wire field.
    pub major: u32,
    /// Decoded `schema_minor` wire field.
    pub minor: u32,
}

impl SchemaVersion {
    /// The current `(1, 0)` version every shard is written against.
    pub const CURRENT: Self = Self {
        major: SCHEMA_MAJOR,
        minor: SCHEMA_MINOR,
    };

    /// Builds one version pair from decoded wire fields.
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Rejects breaking majors; minor is forward-compatible and always passes.
    ///
    /// Returns the offending major on failure so callers can keep their own
    /// error payloads.
    pub fn check_major(self) -> Result<(), u32> {
        check_major(self.major)
    }
}

/// Rejects breaking majors; minor is forward-compatible and never checked.
///
/// Returns the offending major on failure so callers can keep their own
/// error payloads.
pub fn check_major(found: u32) -> Result<(), u32> {
    if found != SCHEMA_MAJOR {
        return Err(found);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_matches_frozen_constants() {
        assert_eq!(SchemaVersion::CURRENT.major, SCHEMA_MAJOR);
        assert_eq!(SchemaVersion::CURRENT.minor, SCHEMA_MINOR);
        assert_eq!(SchemaVersion::CURRENT, SchemaVersion::new(1, 0));
    }

    #[test]
    fn matching_major_passes_regardless_of_minor() {
        assert_eq!(check_major(1), Ok(()));
        assert_eq!(SchemaVersion::new(1, 0).check_major(), Ok(()));
        // Newer minors are forward-compatible.
        assert_eq!(SchemaVersion::new(1, 7).check_major(), Ok(()));
    }

    #[test]
    fn mismatching_major_fails_with_found() {
        assert_eq!(check_major(0), Err(0));
        assert_eq!(check_major(2), Err(2));
        assert_eq!(SchemaVersion::new(2, 0).check_major(), Err(2));
    }
}
