//! License inventory (split from `locks.rs`). No behavior change.

use super::*;

/// One package license identity for the license family.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LicensedPackage {
    /// Package name in its owning set.
    pub name: String,
    /// Locked version.
    pub version: String,
    /// Owning set.
    pub set: String,
    /// SPDX expression text or `UNKNOWN` when unidentified.
    pub license: String,
}

/// Extract Cargo license identities from one `cargo-bazel-lock.json`
/// document (JSON with per-crate `license`). Only crates present in
/// `packages` (the assessable lock contents) are returned; workspace
/// members without upstream identity fall back to `UNKNOWN` only when
/// explicitly listed (they are otherwise skipped as first-party).
pub fn cargo_licenses(cargo_bazel_text: &str, packages: &[LockedPackage]) -> Vec<LicensedPackage> {
    let value: serde_json::Value = match serde_json::from_str(cargo_bazel_text) {
        Ok(value) => value,
        Err(_) => {
            return packages
                .iter()
                .filter(|package| package.set == "cargo")
                .map(|package| LicensedPackage {
                    name: package.name.clone(),
                    version: package.version.clone(),
                    set: package.set.clone(),
                    license: "UNKNOWN".to_owned(),
                })
                .collect();
        }
    };
    // `cargo-bazel-lock.json` keys are `name version` (e.g. `serde 1.0.229`).
    // Real files use `crates` (per-crate `license`); unit fixtures use
    // `packages`. Accept both so SBOM stays accurate on dogfood.
    let mut by_name_version = std::collections::BTreeMap::new();
    for key in ["crates", "packages"] {
        if let Some(packages_map) = value.get(key).and_then(|value| value.as_object()) {
            for (key, detail) in packages_map {
                let license = detail
                    .get("license")
                    .and_then(|value| value.as_str())
                    .unwrap_or("UNKNOWN")
                    .trim();
                let license = if license.is_empty() {
                    "UNKNOWN"
                } else {
                    license
                };
                by_name_version.insert(key.clone(), license.to_owned());
            }
        }
    }
    packages
        .iter()
        .filter(|package| package.set == "cargo")
        .map(|package| {
            let key = format!("{} {}", package.name, package.version);
            let license = by_name_version
                .get(&key)
                .cloned()
                .unwrap_or_else(|| "UNKNOWN".to_owned());
            LicensedPackage {
                name: package.name.clone(),
                version: package.version.clone(),
                set: package.set.clone(),
                license,
            }
        })
        .collect()
}

/// License identities for non-Cargo sets without a qualified source:
/// `UNKNOWN` (denied in `distributed`, inventoried in `internal`).
/// Prefer [`npm_licenses`] for npm (reads `package-lock.json` where
/// present) and [`inventory_licenses`] for Maven/NuGet/Go plus npm
/// fallback (reads the committed `[[inventory]]` table); this stays as
/// the fail-closed fallback when neither source identifies a package.
pub fn unknown_licenses(packages: &[LockedPackage], set: &str) -> Vec<LicensedPackage> {
    packages
        .iter()
        .filter(|package| package.set == set)
        .map(|package| LicensedPackage {
            name: package.name.clone(),
            version: package.version.clone(),
            set: package.set.clone(),
            license: "UNKNOWN".to_owned(),
        })
        .collect()
}

/// Extract npm license identities from one `package-lock.json` document
/// (JSON with per-package `license`). Both the `packages:`
/// (`node_modules/<name>` with `version` plus `license`) and the legacy
/// `dependencies:` (`<name>` with `version` plus `license`) shapes are
/// read; entries without a non-empty string `license` fall back to
/// `UNKNOWN`. Only packages present in `packages` (the assessable lock
/// contents) are returned. `pnpm-lock.yaml` and `yarn.lock` carry no
/// license field, so pnpm/yarn-only workspaces without a sibling
/// `package-lock.json` stay `UNKNOWN` (fail closed) unless the
/// `[[inventory]]` table identifies them via [`inventory_licenses`].
pub fn npm_licenses(package_lock_text: &str, packages: &[LockedPackage]) -> Vec<LicensedPackage> {
    let mut by_name_version = std::collections::BTreeMap::new();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(package_lock_text) {
        if let Some(object) = value.as_object() {
            if let Some(detail_map) = object.get("packages").and_then(|v| v.as_object()) {
                for (path, detail) in detail_map {
                    let Some(name) = package_lock_name(path) else {
                        continue;
                    };
                    let detail = match detail.as_object() {
                        Some(detail) => detail,
                        None => continue,
                    };
                    let version = detail
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim();
                    if version.is_empty()
                        || version.starts_with("file:")
                        || version.starts_with("link:")
                    {
                        continue;
                    }
                    let license = detail
                        .get("license")
                        .and_then(|v| v.as_str())
                        .unwrap_or("UNKNOWN")
                        .trim();
                    let license = if license.is_empty() {
                        "UNKNOWN"
                    } else {
                        license
                    };
                    by_name_version.insert((name, version.to_owned()), license.to_owned());
                }
            }
            if let Some(detail_map) = object.get("dependencies").and_then(|v| v.as_object()) {
                for (name, detail) in detail_map {
                    let name = name.trim();
                    if name.is_empty() {
                        continue;
                    }
                    let detail = match detail.as_object() {
                        Some(detail) => detail,
                        None => continue,
                    };
                    let version = detail
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim();
                    if version.is_empty() {
                        continue;
                    }
                    let license = detail
                        .get("license")
                        .and_then(|v| v.as_str())
                        .unwrap_or("UNKNOWN")
                        .trim();
                    let license = if license.is_empty() {
                        "UNKNOWN"
                    } else {
                        license
                    };
                    by_name_version
                        .entry((name.to_owned(), version.to_owned()))
                        .or_insert_with(|| license.to_owned());
                }
            }
        }
    }
    packages
        .iter()
        .filter(|package| package.set == "npm")
        .map(|package| {
            let license = by_name_version
                .get(&(package.name.clone(), package.version.clone()))
                .cloned()
                .unwrap_or_else(|| "UNKNOWN".to_owned());
            LicensedPackage {
                name: package.name.clone(),
                version: package.version.clone(),
                set: package.set.clone(),
                license,
            }
        })
        .collect()
}

/// Resolve license identities via the committed `[[inventory]]` table
/// (see [`crate::license_policy::LicenseInventory`]): the first entry
/// matching package plus set whose `versions` scope contains the locked
/// version (via [`crate::vuln::version_affected`], upstream semantics per
/// set) supplies the license; unmatched packages fall back to `UNKNOWN`
/// (fail closed). Out-of-range versions never inherit an entry. Callers
/// pass the full inventory and the owning set; only that set's entries
/// are considered.
pub fn inventory_licenses(
    packages: &[LockedPackage],
    inventory: &[crate::license_policy::LicenseInventory],
    set: &str,
) -> Vec<LicensedPackage> {
    packages
        .iter()
        .filter(|package| package.set == set)
        .map(|package| {
            let mut license = "UNKNOWN".to_owned();
            for entry in inventory {
                if entry.set != set || entry.package != package.name {
                    continue;
                }
                if crate::vuln::version_affected(set, &entry.versions, &package.version) {
                    license = entry.license.clone();
                    break;
                }
            }
            LicensedPackage {
                name: package.name.clone(),
                version: package.version.clone(),
                set: package.set.clone(),
                license,
            }
        })
        .collect()
}

/// Whether the committed inventory records `LICENSE*`/`NOTICE*` words
/// for one locked package: true only when an entry matches package plus
/// set with an in-scope version and `text_present`. Absent or
/// out-of-range entries mean no words (fail closed in `distributed` when
/// the license requires reproduction).
pub fn inventory_text_present(
    package: &LockedPackage,
    inventory: &[crate::license_policy::LicenseInventory],
) -> bool {
    for entry in inventory {
        if entry.set != package.set || entry.package != package.name {
            continue;
        }
        if crate::vuln::version_affected(&package.set, &entry.versions, &package.version) {
            return entry.text_present;
        }
    }
    false
}
