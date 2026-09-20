//! Major-release migration planning for `dx migrate`.
//!
//! Split from `super` (`lib.rs`): owns `migrate_is_major_bump`,
//! `migrate_manifest_name`, `MigratePlan`, and `plan_migrate`.
//! Re-exported through `super` so the public path stays
//! `dx_adopt::{migrate_is_major_bump, migrate_manifest_name, MigratePlan,
//! plan_migrate}`.

use super::AdoptError;

/// Whether a `dx migrate` version pair is a major-release bump.
///
/// The migrator is major-release-only breaking-change rewrites over the
/// generation edit-manifest pattern: both versions must parse as
/// Cargo-flavor semver, differ, and the target major must exceed the
/// source major. Minor/patch-only bumps, downgrades, and non-semver
/// text never qualify — they run through `dx generate`, not `migrate`.
pub fn migrate_is_major_bump(from: &str, to: &str) -> bool {
    let from_v = match semver::Version::parse(from) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let to_v = match semver::Version::parse(to) {
        Ok(v) => v,
        Err(_) => return false,
    };
    if from_v == to_v {
        return false;
    }
    to_v.major > from_v.major
}

/// Manifest selection for `dx migrate`.
///
/// One manifest per major-release hop, named after the major versions
/// so selection is mechanical: `migrate-v<from_major>-to-v<to_major>.json`.
/// The manifest carries generation edit-manifest records (create/modify
/// with digests and byte-range replacements); the migrator applies them
/// through the same write-outcome/completion reporting as `dx generate`.
/// Callers must validate via [`migrate_is_major_bump`] first.
pub fn migrate_manifest_name(from_major: u64, to_major: u64) -> String {
    format!("migrate-v{from_major}-to-v{to_major}.json")
}

/// One planned migration: the validated version pair plus
/// the selected manifest name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigratePlan {
    /// Source version (inclusive, already installed).
    pub from: String,
    /// Target major-release version.
    pub to: String,
    /// Selected manifest (see [`migrate_manifest_name`]).
    pub manifest: String,
}

/// Plan one `dx migrate` invocation.
///
/// Syntax (live successor to closed):
/// `dx migrate --from <version> --to <version> [scope ...]`. Both
/// versions are Cargo-flavor semver; the pair must be a major-release
/// bump (see [`migrate_is_major_bump`]). Scope selection reuses
/// generation scope resolution verbatim (empty scope refreshes
/// `//...`); external scopes are rejected like workflow commands.
/// With no breaking-change manifests published yet (module at `0.0.0`,
/// no releases cut), planning succeeds but execution fails closed
/// (`migrate_failed`) until the first major-release manifest lands —
/// the same fail-closed discipline as `audit_failed` (`dx audit` plus
/// `dx update` execute live, and).
pub fn plan_migrate(from: &str, to: &str) -> Result<MigratePlan, AdoptError> {
    if from.is_empty() || to.is_empty() {
        return Err(AdoptError::MigrateVersions {
            detail: "from and to must both be set".to_owned(),
        });
    }
    let from_v = semver::Version::parse(from).map_err(|_| AdoptError::MigrateVersions {
        detail: format!("invalid from version: {from}"),
    })?;
    let to_v = semver::Version::parse(to).map_err(|_| AdoptError::MigrateVersions {
        detail: format!("invalid to version: {to}"),
    })?;
    if !migrate_is_major_bump(from, to) {
        return Err(AdoptError::MigrateNotMajor {
            from: from.to_owned(),
            to: to.to_owned(),
        });
    }
    Ok(MigratePlan {
        from: from.to_owned(),
        to: to.to_owned(),
        manifest: migrate_manifest_name(from_v.major, to_v.major),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_is_major_release_only() {
        // (live successor to closed): major-release-only
        // gate. Minor/patch bumps, downgrades, equal versions, and
        // non-semver never qualify — they run through `dx generate`,
        // not `migrate`.
        assert!(migrate_is_major_bump("1.2.3", "2.0.0"));
        assert!(migrate_is_major_bump("1.9.9", "2.0.0-alpha.1"));
        assert!(migrate_is_major_bump("0.0.0", "1.0.0"));
        assert!(!migrate_is_major_bump("1.2.3", "1.3.0"));
        assert!(!migrate_is_major_bump("1.2.3", "1.2.4"));
        assert!(!migrate_is_major_bump("2.0.0", "1.0.0"));
        assert!(!migrate_is_major_bump("1.2.3", "1.2.3"));
        assert!(!migrate_is_major_bump("abc", "2.0.0"));
        assert!(!migrate_is_major_bump("1.2.3", ""));
        assert_eq!(migrate_manifest_name(1, 2), "migrate-v1-to-v2.json");
        let plan = plan_migrate("1.2.3", "2.0.0").expect("major bump plans");
        assert_eq!(plan.manifest, "migrate-v1-to-v2.json");
        assert!(plan_migrate("1.2.3", "1.3.0").is_err());
        assert!(plan_migrate("", "2.0.0").is_err());
        assert!(plan_migrate("abc", "2.0.0").is_err());
        // Typed errors render stably for CLI diagnostics.
        assert_eq!(
            plan_migrate("1.0.0", "1.1.0").unwrap_err().to_string(),
            "migrate is major-release-only: 1.0.0 -> 1.1.0"
        );
    }

    #[test]
    fn migrate_manifest_selection_is_mechanical_per_major_hop() {
        // One manifest per major hop, named after the major
        // versions so selection is mechanical.
        assert_eq!(migrate_manifest_name(0, 1), "migrate-v0-to-v1.json");
        assert_eq!(migrate_manifest_name(1, 2), "migrate-v1-to-v2.json");
        assert_eq!(migrate_manifest_name(2, 3), "migrate-v2-to-v3.json");
        assert_eq!(migrate_manifest_name(1, 3), "migrate-v1-to-v3.json");
        assert_eq!(migrate_manifest_name(10, 11), "migrate-v10-to-v11.json");
        // Planning selects the manifest from the parsed majors,
        // preserving the full version strings.
        let plan = plan_migrate("1.2.3", "2.0.0").expect("plan");
        assert_eq!(plan.from, "1.2.3");
        assert_eq!(plan.to, "2.0.0");
        assert_eq!(plan.manifest, "migrate-v1-to-v2.json");
        let hop = plan_migrate("0.9.0", "1.0.0").expect("0 to 1 plans");
        assert_eq!(hop.manifest, "migrate-v0-to-v1.json");
        let skip = plan_migrate("1.0.0", "3.0.0").expect("multi-hop plans");
        assert_eq!(skip.manifest, "migrate-v1-to-v3.json");
    }

    #[test]
    fn migrate_syntax_pins_semver_prerelease_and_errors() {
        // `dx migrate --from <version> --to <version>` takes
        // Cargo-flavor semver only; prerelease/build metadata ride the
        // same major gate, and every rejection renders stably for CLI
        // usage diagnostics (exit 2).
        assert!(migrate_is_major_bump("1.0.0", "2.0.0-alpha.1"));
        assert!(migrate_is_major_bump("1.0.0+build.1", "2.0.0"));
        assert!(migrate_is_major_bump("1.0.0-alpha", "2.0.0"));
        assert!(!migrate_is_major_bump("1.0.0-alpha", "1.0.0"));
        assert!(!migrate_is_major_bump("1.0.0+build.1", "1.0.1"));
        assert!(!migrate_is_major_bump("2.0.0-alpha.1", "1.9.9"));
        assert!(!migrate_is_major_bump("1.0", "2.0.0"));
        assert!(!migrate_is_major_bump("1.0.0", "2.0"));
        assert_eq!(
            plan_migrate("", "2.0.0").unwrap_err().to_string(),
            "migrate needs distinct versions: from and to must both be set"
        );
        assert_eq!(
            plan_migrate("1.2.3", "").unwrap_err().to_string(),
            "migrate needs distinct versions: from and to must both be set"
        );
        assert_eq!(
            plan_migrate("abc", "2.0.0").unwrap_err().to_string(),
            "migrate needs distinct versions: invalid from version: abc"
        );
        assert_eq!(
            plan_migrate("1.2.3", "xyz").unwrap_err().to_string(),
            "migrate needs distinct versions: invalid to version: xyz"
        );
        assert_eq!(
            plan_migrate("2.0.0", "1.0.0").unwrap_err().to_string(),
            "migrate is major-release-only: 2.0.0 -> 1.0.0"
        );
        assert_eq!(
            plan_migrate("1.2.3", "1.2.3").unwrap_err().to_string(),
            "migrate is major-release-only: 1.2.3 -> 1.2.3"
        );
    }
}
