//! Lockfile readers for `dx audit` live execution (issue #18).
//!
//! Pure parsing over injected lockfile text, per the audit contract
//! (`docs/cli/commands/audit-update-bazel.md#dx-audit`): target-scoped
//! dependency audits select the targets' owning dependency sets and audit
//! their complete standard locks, including dependencies not used by those
//! particular targets. Shared sets are audited once; unrelated sets are
//! never included merely because they share a repository.
//!
//! V1 coverage mirrors `dx_update::sets` (Cargo, npm, Maven, NuGet, Go):
//! - Cargo (`rust/hello/Cargo.lock`, TOML): registry packages are
//!   assessable; `git+` sources are unsupported revisions (incomplete,
//!   never clean); path-only workspace members (no `source`, version
//!   `0.0.0`) are first-party and skipped, not assessed.
//! - npm (`pnpm-lock.yaml`, YAML): external `packages:` entries are
//!   assessable via minimal line parsing (no YAML dependency); `link:`
//!   entries are workspace members and skipped.
//! - Maven (`third_party/jvm/maven_install.json`, JSON): `artifacts`
//!   carry `group:artifact` plus `version`.
//! - NuGet (`third_party/dotnet/paket.lock`, text): `Name (version)`
//!   lines under the `NUGET` remote section.
//! - Go: empty set (no `go.mod`), no-op success.
//!
//! License identities for V1: Cargo reads `cargo-bazel-lock.json`
//! (`license` per crate, fallback `UNKNOWN`); npm/Maven/NuGet report
//! `UNKNOWN` (denied in `distributed`, inventoried in `internal` per the
//! expression lattice) pending per-ecosystem qualification. Notice texts
//! are treated as present for known licenses in V1 (collection via
//! declared Bazel inputs lands later); `missing-notice-text` stays pinned
//! by unit tests in `license_notice`.

use crate::vuln::LockedPackage;

/// Parse one `Cargo.lock` (TOML) into assessable locked packages for the
/// `cargo` set. Registry packages become assessable entries; `git+`
/// sources become `is_git` incomplete markers; path-only workspace members
/// (no `source`) are skipped as first-party.
pub fn parse_cargo_lock(text: &str) -> Result<Vec<LockedPackage>, String> {
    let value: toml::Value =
        toml::from_str(text).map_err(|error| format!("invalid Cargo.lock: {error}"))?;
    let packages = value
        .get("package")
        .and_then(|value| value.as_array())
        .ok_or_else(|| "invalid Cargo.lock: missing [[package]]".to_owned())?;
    let mut out = Vec::new();
    for package in packages {
        let name = package
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let version = package
            .get("version")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        if name.is_empty() || version.is_empty() {
            continue;
        }
        let source = package
            .get("source")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_owned();
        if source.is_empty() {
            // First-party workspace member (e.g. `dx_* 0.0.0`): skip, not
            // an upstream dependency with advisory identity.
            continue;
        }
        if source.contains("git+") {
            out.push(LockedPackage {
                name: name.to_owned(),
                version: version.to_owned(),
                set: "cargo".to_owned(),
                is_git: true,
                is_private: false,
            });
            continue;
        }
        out.push(LockedPackage {
            name: name.to_owned(),
            version: version.to_owned(),
            set: "cargo".to_owned(),
            is_git: false,
            is_private: false,
        });
    }
    out.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    out.dedup_by(|b, a| a.name == b.name && a.version == b.version);
    Ok(out)
}

/// Parse one `pnpm-lock.yaml` into assessable packages for the `npm` set
/// via minimal line parsing (no YAML dependency). External entries under
/// `packages:` shaped `'name@version':` or `'@scope/name@version':`
/// become assessable; `link:` entries (workspace members) are skipped.
/// Versions with peer suffixes (`1.0.0(peer@2.0.0)`) strip the suffix.
pub fn parse_pnpm_lock(text: &str) -> Result<Vec<LockedPackage>, String> {
    let mut out = Vec::new();
    let mut in_packages = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "packages:" {
            in_packages = true;
            continue;
        }
        if in_packages {
            // Top-level keys are two-space indented; deeper keys (e.g.
            // `resolution:`) are four-space+ and skipped. A new top-level
            // section (no indent) ends the packages block.
            if !line.starts_with(' ') {
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    break;
                }
                continue;
            }
            if line.starts_with("    ") || line.starts_with('\t') {
                continue;
            }
            if !(line.starts_with("  ") && trimmed.starts_with('\'') || trimmed.starts_with('"')) {
                // Allow both quoted and bare keys, but only two-space keys.
                if !(line.starts_with("  ") && !trimmed.is_empty() && !trimmed.contains(':')) {
                    // Heuristic: two-space lines with a colon are package keys.
                    if !(line.starts_with("  ") && trimmed.contains(':')) {
                        continue;
                    }
                }
            }
            // Extract the quoted key: `'...':` or `"...":`.
            let key = extract_quoted_key(trimmed).unwrap_or_else(|| {
                trimmed
                    .split_once(':')
                    .map(|(key, _)| key.trim().to_owned())
                    .unwrap_or_default()
            });
            if key.is_empty() {
                continue;
            }
            // Skip importers and link entries encoded in the value line?
            // `link:` entries appear as `name@link:...` keys; skip them.
            if key.contains("link:") {
                continue;
            }
            if let Some((name, version)) = split_pnpm_key(&key) {
                if name.is_empty() || version.is_empty() {
                    continue;
                }
                // Skip workspace `link:` versions that slipped through.
                if version.starts_with("link:") {
                    continue;
                }
                out.push(LockedPackage {
                    name,
                    version,
                    set: "npm".to_owned(),
                    is_git: false,
                    is_private: false,
                });
            }
        }
    }
    out.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    out.dedup_by(|b, a| a.name == b.name && a.version == b.version);
    Ok(out)
}

fn extract_quoted_key(trimmed: &str) -> Option<String> {
    for quote in ['\'', '"'] {
        if trimmed.starts_with(quote) {
            if let Some(end) = trimmed[1..].find(quote) {
                return Some(trimmed[1..1 + end].to_owned());
            }
        }
    }
    None
}

/// Split one pnpm package key (`name@version` or `@scope/name@version`)
/// into name and version, stripping peer suffixes (`1.0.0(peer)`).
fn split_pnpm_key(key: &str) -> Option<(String, String)> {
    // Scoped: `@scope/name@version`; unscoped: `name@version`.
    let (name, version) = if key.starts_with('@') {
        let slash = key.find('/')?;
        let rest = &key[slash + 1..];
        let at = rest.rfind('@')?;
        let scope = &key[..slash];
        let base = &rest[..at];
        let version = &rest[at + 1..];
        (format!("{scope}/{base}"), version.to_owned())
    } else {
        let at = key.rfind('@')?;
        (key[..at].to_owned(), key[at + 1..].to_owned())
    };
    let version = version
        .split_once('(')
        .map(|(base, _)| base)
        .unwrap_or(&version)
        .trim()
        .to_owned();
    if name.is_empty() || version.is_empty() {
        return None;
    }
    Some((name, version))
}

/// Parse one `maven_install.json` into assessable packages for the
/// `maven` set. `artifacts` keys are `group:artifact` with `version`
/// inside; both are required.
pub fn parse_maven_install(text: &str) -> Result<Vec<LockedPackage>, String> {
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|error| format!("invalid maven_install.json: {error}"))?;
    let artifacts = value
        .get("artifacts")
        .and_then(|value| value.as_object())
        .ok_or_else(|| "invalid maven_install.json: missing artifacts".to_owned())?;
    let mut out = Vec::new();
    for (key, detail) in artifacts {
        let version = detail
            .get("version")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        if key.trim().is_empty() || version.is_empty() {
            continue;
        }
        // Key is `group:artifact`; keep verbatim for `maven:group:artifact`
        // identity (see `dx_update::selector`).
        out.push(LockedPackage {
            name: key.clone(),
            version: version.to_owned(),
            set: "maven".to_owned(),
            is_git: false,
            is_private: false,
        });
    }
    out.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    Ok(out)
}

/// Parse one `paket.lock` into assessable packages for the `nuget` set.
/// Only `Name (version)` lines under the `NUGET` remote section count;
/// `HTTP`/`GITHUB` sections and group headers are skipped.
pub fn parse_paket_lock(text: &str) -> Result<Vec<LockedPackage>, String> {
    let mut out = Vec::new();
    let mut in_nuget = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "NUGET" {
            in_nuget = true;
            continue;
        }
        if trimmed == "HTTP" || trimmed == "GITHUB" || trimmed == "GIT" {
            in_nuget = false;
            continue;
        }
        if !in_nuget {
            continue;
        }
        // Package lines are indented `Name (version)`; remote lines are
        // `remote: ...` and group headers are `GROUP ...`.
        if trimmed.is_empty()
            || trimmed.starts_with("remote:")
            || trimmed.starts_with("GROUP")
            || trimmed.starts_with("RESTRICTION:")
        {
            continue;
        }
        if let Some((name, version)) = split_paket_line(trimmed) {
            out.push(LockedPackage {
                name,
                version,
                set: "nuget".to_owned(),
                is_git: false,
                is_private: false,
            });
        }
    }
    out.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    Ok(out)
}

fn split_paket_line(trimmed: &str) -> Option<(String, String)> {
    let open = trimmed.rfind('(')?;
    let close = trimmed.rfind(')')?;
    if close < open {
        return None;
    }
    let name = trimmed[..open].trim().to_owned();
    let version = trimmed[open + 1..close].trim().to_owned();
    if name.is_empty() || version.is_empty() || name.contains(' ') && name.contains(':') {
        // Guard against non-package lines; names never contain `remote:`.
        if name.contains("remote") {
            return None;
        }
    }
    if name.is_empty() || version.is_empty() {
        return None;
    }
    Some((name, version))
}

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
    let mut by_name_version = std::collections::BTreeMap::new();
    if let Some(packages_map) = value.get("packages").and_then(|value| value.as_object()) {
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

/// License identities for non-Cargo sets in V1: `UNKNOWN` (denied in
/// `distributed`, inventoried in `internal`), pending per-ecosystem
/// qualification.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_lock_splits_registry_git_and_first_party() {
        let text = r#"
[[package]]
name = "serde"
version = "1.0.100"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "abc"

[[package]]
name = "git-dep"
version = "0.1.0"
source = "git+https://github.com/example/git-dep#abc123"

[[package]]
name = "dx_audit"
version = "0.0.0"
"#;
        let packages = parse_cargo_lock(text).expect("parses");
        assert_eq!(packages.len(), 2);
        let serde = packages
            .iter()
            .find(|package| package.name == "serde")
            .expect("serde");
        assert!(!serde.is_git && !serde.is_private);
        let git = packages
            .iter()
            .find(|package| package.name == "git-dep")
            .expect("git");
        assert!(git.is_git);
        assert!(!packages.iter().any(|package| package.name == "dx_audit"));
    }

    #[test]
    fn pnpm_lock_parses_external_and_skips_links() {
        let text = "lockfileVersion: '9.0'\n\npackages:\n\n  'react@18.2.0':\n    resolution: {integrity: sha512-abc}\n\n  '@astrojs/compiler@4.0.0':\n    resolution: {integrity: sha512-def}\n\n  'my-workspace@link:.':\n    resolution: {directory: .}\n";
        let packages = parse_pnpm_lock(text).expect("parses");
        assert!(packages
            .iter()
            .any(|package| package.name == "react" && package.version == "18.2.0"));
        assert!(packages
            .iter()
            .any(|package| package.name == "@astrojs/compiler" && package.version == "4.0.0"));
        assert!(!packages.iter().any(|package| package.name.contains("link")));
    }

    #[test]
    fn maven_install_parses_artifacts() {
        let text = r#"{"artifacts": {"junit:junit": {"version": "4.13.2"}, "com.google.guava:guava": {"version": "32.0.0"}}}"#;
        let packages = parse_maven_install(text).expect("parses");
        assert_eq!(packages.len(), 2);
        assert!(packages
            .iter()
            .any(|package| package.name == "junit:junit" && package.version == "4.13.2"));
    }

    #[test]
    fn paket_lock_parses_nuget_section_only() {
        let text = "NUGET\n  remote: https://api.nuget.org/v3/index.json\n    FSharp.Core (10.1.201)\n    My.Pkg (1.2.3)\nHTTP\n  remote: https://example.com\n    Other (9.9.9)\n";
        let packages = parse_paket_lock(text).expect("parses");
        assert_eq!(packages.len(), 2);
        assert!(packages.iter().any(|package| package.name == "FSharp.Core"));
        assert!(!packages.iter().any(|package| package.name == "Other"));
    }

    #[test]
    fn cargo_licenses_prefer_bazel_license_or_unknown() {
        let cargo_lock = r#"
[[package]]
name = "serde"
version = "1.0.100"
source = "registry+https://github.com/rust-lang/crates.io-index"
"#;
        let packages = parse_cargo_lock(cargo_lock).expect("locks");
        let bazel = r#"{"packages": {"serde 1.0.100": {"license": "MIT OR Apache-2.0"}}}"#;
        let licensed = cargo_licenses(bazel, &packages);
        assert_eq!(licensed.len(), 1);
        assert_eq!(licensed[0].license, "MIT OR Apache-2.0");
        let missing = cargo_licenses("not json", &packages);
        assert_eq!(missing[0].license, "UNKNOWN");
    }

    #[test]
    fn unknown_licenses_mark_all_unknown() {
        let packages = vec![LockedPackage {
            name: "react".to_owned(),
            version: "18.2.0".to_owned(),
            set: "npm".to_owned(),
            is_git: false,
            is_private: false,
        }];
        let licensed = unknown_licenses(&packages, "npm");
        assert_eq!(licensed[0].license, "UNKNOWN");
    }
}
