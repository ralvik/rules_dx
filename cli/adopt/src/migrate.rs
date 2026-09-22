//! Migration planning for `dx migrate`.
//!
//! Split from `super` (`lib.rs`): owns `migrate_is_major_bump`,
//! `migrate_is_upgrade`, `migrate_manifest_name`,
//! `migrate_manifest_name_full`, `MigratePlan`, and `plan_migrate`.
//! Re-exported through `super` so the public path stays
//! `dx_adopt::{migrate_is_major_bump, migrate_is_upgrade,
//! migrate_manifest_name, migrate_manifest_name_full, MigratePlan,
//! plan_migrate}`.

use super::AdoptError;

/// Whether a `dx migrate` version pair is a major-release bump.
///
/// Major bumps are the coarse subset of upgrades: both versions parse
/// as Cargo-flavor semver, differ, and the target major exceeds the
/// source major. Kept so major-hop manifests stay addressable; the
/// planning gate itself is [`migrate_is_upgrade`].
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

/// Whether a `dx migrate` version pair is any upgrade.
///
/// The migrator accepts breaking-ish rewrites over the generation
/// edit-manifest pattern for any upgrading pair: both versions parse
/// as Cargo-flavor semver and the target exceeds the source semver
/// (`to > from`). Minor/patch upgrades qualify alongside major hops
/// (See: `docs/cli/commands/migrate.md`, issue #671); downgrades, equal versions, and non-semver text
/// never qualify.
pub fn migrate_is_upgrade(from: &str, to: &str) -> bool {
    let from_v = match semver::Version::parse(from) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let to_v = match semver::Version::parse(to) {
        Ok(v) => v,
        Err(_) => return false,
    };
    to_v > from_v
}

/// Manifest selection for `dx migrate` major hops.
///
/// One manifest per major-release hop, named after the major versions
/// so selection is mechanical: `migrate-v<from_major>-to-v<to_major>.json`.
/// The manifest carries generation edit-manifest records (create/modify
/// with digests and byte-range replacements); the migrator applies them
/// through the same write-outcome/completion reporting as `dx generate`.
/// Callers must validate via [`migrate_is_upgrade`] first, then branch
/// on [`migrate_is_major_bump`] for this name versus
/// [`migrate_manifest_name_full`].
pub fn migrate_manifest_name(from_major: u64, to_major: u64) -> String {
    format!("migrate-v{from_major}-to-v{to_major}.json")
}

/// Manifest selection for `dx migrate` minor/patch upgrades.
///
/// One manifest per full version pair, named mechanically after the
/// full versions so strict/config rollouts stay addressable without a
/// major bump: `migrate-v<from>-to-v<to>.json` (for example
/// `migrate-v1.2.3-to-v1.3.0.json`). Same edit-manifest records as
/// [`migrate_manifest_name`]. Callers must validate via
/// [`migrate_is_upgrade`] first.
pub fn migrate_manifest_name_full(from: &str, to: &str) -> String {
    format!("migrate-v{from}-to-v{to}.json")
}

/// One planned migration: the validated version pair plus
/// the selected manifest name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigratePlan {
    /// Source version (inclusive, already installed).
    pub from: String,
    /// Target version.
    pub to: String,
    /// Selected manifest (see [`migrate_manifest_name`] plus
    /// [`migrate_manifest_name_full`]).
    pub manifest: String,
}

/// Plan one `dx migrate` invocation.
///
/// Syntax (live successor to closed):
/// `dx migrate --from <version> --to <version> [scope ...]`. Both
/// versions are Cargo-flavor semver; the pair must be an upgrade
/// (see [`migrate_is_upgrade`]). Major bumps select one manifest per
/// major hop (see [`migrate_manifest_name`]); minor/patch upgrades
/// select one manifest per full version pair (see
/// [`migrate_manifest_name_full`]). Scope selection reuses
/// generation scope resolution verbatim (empty scope refreshes
/// `//...`); external scopes are rejected like workflow commands.
/// With no breaking-change manifests published yet (module at `0.0.0`,
/// no releases cut), planning succeeds but execution fails closed
/// (`migrate_failed`) until the first manifest lands —
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
    if !migrate_is_upgrade(from, to) {
        return Err(AdoptError::MigrateNotUpgrade {
            from: from.to_owned(),
            to: to.to_owned(),
        });
    }
    let manifest = if to_v.major > from_v.major {
        migrate_manifest_name(from_v.major, to_v.major)
    } else {
        migrate_manifest_name_full(from, to)
    };
    Ok(MigratePlan {
        from: from.to_owned(),
        to: to.to_owned(),
        manifest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_upgrade_gate_accepts_any_upgrade() {
        // (issue #671, ADR 0025; See: `docs/cli/commands/migrate.md`): upgrade-only gate. Major, minor,
        // and patch upgrades qualify; downgrades, equal versions,
        // and non-semver never qualify.
        assert!(migrate_is_upgrade("1.2.3", "2.0.0"));
        assert!(migrate_is_upgrade("1.2.3", "1.3.0"));
        assert!(migrate_is_upgrade("1.2.3", "1.2.4"));
        assert!(migrate_is_upgrade("1.9.9", "2.0.0-alpha.1"));
        assert!(migrate_is_upgrade("0.0.0", "1.0.0"));
        assert!(migrate_is_upgrade("0.0.0", "0.1.0"));
        assert!(migrate_is_upgrade("1.0.0-alpha", "1.0.0"));
        assert!(!migrate_is_upgrade("2.0.0", "1.0.0"));
        assert!(!migrate_is_upgrade("1.2.3", "1.2.3"));
        assert!(!migrate_is_upgrade("abc", "2.0.0"));
        assert!(!migrate_is_upgrade("1.2.3", ""));
        // Major bumps stay the coarse subset for major-hop manifests.
        assert!(migrate_is_major_bump("1.2.3", "2.0.0"));
        assert!(migrate_is_major_bump("1.9.9", "2.0.0-alpha.1"));
        assert!(migrate_is_major_bump("0.0.0", "1.0.0"));
        assert!(!migrate_is_major_bump("1.2.3", "1.3.0"));
        assert!(!migrate_is_major_bump("1.2.3", "1.2.4"));
        assert_eq!(migrate_manifest_name(1, 2), "migrate-v1-to-v2.json");
        assert_eq!(
            migrate_manifest_name_full("1.2.3", "1.3.0"),
            "migrate-v1.2.3-to-v1.3.0.json"
        );
        let plan = plan_migrate("1.2.3", "2.0.0").expect("major bump plans");
        assert_eq!(plan.manifest, "migrate-v1-to-v2.json");
        let minor = plan_migrate("1.2.3", "1.3.0").expect("minor plans");
        assert_eq!(minor.manifest, "migrate-v1.2.3-to-v1.3.0.json");
        let patch = plan_migrate("1.2.3", "1.2.4").expect("patch plans");
        assert_eq!(patch.manifest, "migrate-v1.2.3-to-v1.2.4.json");
        assert!(plan_migrate("2.0.0", "1.0.0").is_err());
        assert!(plan_migrate("", "2.0.0").is_err());
        assert!(plan_migrate("abc", "2.0.0").is_err());
        // Typed errors render stably for CLI diagnostics.
        assert_eq!(
            plan_migrate("2.0.0", "1.0.0").unwrap_err().to_string(),
            "migrate is upgrade-only: 2.0.0 -> 1.0.0"
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
        // same upgrade gate, and every rejection renders stably for CLI
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
            "migrate is upgrade-only: 2.0.0 -> 1.0.0"
        );
        assert_eq!(
            plan_migrate("1.2.3", "1.2.3").unwrap_err().to_string(),
            "migrate is upgrade-only: 1.2.3 -> 1.2.3"
        );
    }

    #[test]
    fn migrate_prerelease_and_build_metadata_table() {
        // See: `docs/cli/commands/migrate.md`.
        // Prerelease and build metadata ride the same `to > from` upgrade
        // gate (Cargo-flavor semver via the `semver` crate, which orders
        // build metadata for a total order). Multi-major jumps select one
        // manifest (`v1-to-v3`), never a chain.
        assert!(migrate_is_upgrade("1.9.9", "2.0.0-alpha.1"));
        assert!(migrate_is_upgrade("1.0.0-alpha", "1.0.0"));
        assert!(migrate_is_upgrade("1.0.0+build.1", "1.0.1"));
        assert!(migrate_is_upgrade("1.0.0+build.1", "2.0.0"));
        assert!(migrate_is_upgrade("1.0.0+build.1", "1.0.0+build.2"));
        assert!(migrate_is_upgrade("1.0.0", "1.0.0+build.1"));
        assert!(!migrate_is_upgrade("2.0.0-alpha.1", "1.9.9"));
        let skip = plan_migrate("1.0.0", "3.0.0").expect("multi-hop plans single");
        assert_eq!(skip.manifest, "migrate-v1-to-v3.json");
    }
}
