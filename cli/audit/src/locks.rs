//! Lockfile readers for `dx audit` live execution.
//!
//! Pure parsing over injected lockfile text, per the audit contract
//! (`docs/cli/commands/audit-update-bazel.md#dx-audit`): target-scoped
//! dependency audits select the targets' owning dependency sets and audit
//! their complete standard locks, including dependencies not used by those
//! particular targets. Shared sets are audited once; unrelated sets are
//! never included merely because they share a repository.
//!
//! V1 coverage mirrors `dx_update::sets` (Cargo, npm, Maven, NuGet, Go):
//! - Cargo (`rust/tests/fixtures/hello/Cargo.lock`, TOML): registry packages are
//!   assessable; `git+` sources are unsupported revisions (incomplete,
//!   never clean); path-only workspace members (no `source`, version
//!   `0.0.0`) are first-party and skipped, not assessed.
//! - npm (`pnpm-lock.yaml`, YAML): external `packages:` entries are
//!   assessable via `yaml_serde`; `link:` entries are workspace members
//!   and skipped.
//! - Maven (`third_party/jvm/maven_install.json`, JSON): `artifacts`
//!   carry `group:artifact` plus `version`.
//! - NuGet (`third_party/dotnet/paket.lock`, text): `Name (version)`
//!   lines under the `NUGET` remote section are assessable; `Name
//!   (version)` lines under the `GIT` section are unsupported revisions
//!   (issue #584 wont-fix, auditor-owned: SHA carries no OSV version
//!   identity), reported as `is_git` incomplete, never dropped and never
//!   clean; `HTTP`/`GITHUB` sections and group headers are skipped.
//! - Go: empty set (no `go.mod`), no-op success.
//!
//! Private packages have no lockfile auto-detection in V1 (issue #584
//! wont-fix, auditor-owned): a private registry entry is
//! indistinguishable from a public one in lock bytes, so callers mark
//! `is_private` explicitly and matching fails those as incomplete,
//! never clean.
//!
//! License identities for V1: Cargo reads `cargo-bazel-lock.json`
//! (`license` per crate, fallback `UNKNOWN`); npm/Maven/NuGet report
//! `UNKNOWN` (denied in `distributed`, inventoried in `internal` per the
//! expression lattice) pending per-ecosystem qualification. Notice texts
//! are treated as present for known licenses in V1 (collection via
//! declared Bazel inputs lands later); `missing-notice-text` stays pinned
//! by unit tests in `license_notice`.

use std::sync::OnceLock;

use regex::Regex;

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
/// via `yaml_serde`. External entries under `packages:` shaped
/// `name@version` or `@scope/name@version` become assessable; `link:`
/// entries (workspace members) are skipped. Versions with peer suffixes
/// (`1.0.0(peer@2.0.0)`) strip the suffix.
pub fn parse_pnpm_lock(text: &str) -> Result<Vec<LockedPackage>, String> {
    let value: yaml_serde::Value =
        yaml_serde::from_str(text).map_err(|error| format!("invalid pnpm-lock.yaml: {error}"))?;
    if value.is_null() {
        return Ok(Vec::new());
    }
    let packages = match value.get("packages") {
        None | Some(yaml_serde::Value::Null) => return Ok(Vec::new()),
        Some(packages) => packages,
    };
    let mapping = packages
        .as_mapping()
        .ok_or_else(|| "invalid pnpm-lock.yaml: missing packages".to_owned())?;
    let mut out = Vec::new();
    for (key_value, _) in mapping {
        let key = key_value.as_str().unwrap_or("").trim().to_owned();
        if key.is_empty() {
            continue;
        }
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
    out.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    out.dedup_by(|b, a| a.name == b.name && a.version == b.version);
    Ok(out)
}

/// Split one pnpm package key (`name@version` or `@scope/name@version`)
/// into name and version, stripping peer suffixes (`1.0.0(peer)`).
/// Declarative `regex` splits (issue #397) replace the `find`/`rfind`
/// `@` heuristics; peer-suffix stripping stays a textual `split_once`
/// because it is a single delimiter, not a character class.
fn scoped_pnpm_re() -> Option<&'static Regex> {
    static RE: OnceLock<Regex> = OnceLock::new();
    if let Some(compiled) = RE.get() {
        return Some(compiled);
    }
    // Last-`@` split (mirrors the historical `rfind`): scope has no `/`,
    // name takes up to the final `@`, version carries no `@`.
    match Regex::new(r"^@(?P<scope>[^/]+)/(?P<name>.+)@(?P<version>[^@]+)$") {
        Ok(compiled) => {
            let _ = RE.set(compiled);
            RE.get()
        }
        Err(_) => None,
    }
}

fn unscoped_pnpm_re() -> Option<&'static Regex> {
    static RE: OnceLock<Regex> = OnceLock::new();
    if let Some(compiled) = RE.get() {
        return Some(compiled);
    }
    // Last-`@` split for `name@version`.
    match Regex::new(r"^(?P<name>.+)@(?P<version>[^@]+)$") {
        Ok(compiled) => {
            let _ = RE.set(compiled);
            RE.get()
        }
        Err(_) => None,
    }
}

fn split_pnpm_key(key: &str) -> Option<(String, String)> {
    // Strip peer suffixes first: `jest@30.2.0(@types/node@22.20.2)` must
    // split on the version `@`, not the peer `@`.
    let base = key
        .split_once('(')
        .map(|(stem, _)| stem)
        .unwrap_or(key)
        .trim();
    if base.is_empty() {
        return None;
    }
    // Regex-first (issue #397): scoped `@scope/name@version` and unscoped
    // `name@version` via declarative captures. Falls back to the
    // `find`/`rfind` heuristics when a static pattern fails to compile.
    if base.starts_with('@') {
        if let Some(re) = scoped_pnpm_re() {
            if let Some(caps) = re.captures(base) {
                let name = format!("@{}/{}", &caps["scope"], &caps["name"])
                    .trim()
                    .to_owned();
                let version = caps["version"].trim().to_owned();
                if !name.is_empty() && !version.is_empty() {
                    return Some((name, version));
                }
                return None;
            }
            return split_pnpm_scoped_fallback(base);
        }
        return split_pnpm_scoped_fallback(base);
    }
    if let Some(re) = unscoped_pnpm_re() {
        if let Some(caps) = re.captures(base) {
            let name = caps["name"].trim().to_owned();
            let version = caps["version"].trim().to_owned();
            if !name.is_empty() && !version.is_empty() {
                return Some((name, version));
            }
            return None;
        }
        return None;
    }
    split_pnpm_unscoped_fallback(base)
}

fn split_pnpm_scoped_fallback(base: &str) -> Option<(String, String)> {
    let slash = base.find('/')?;
    let rest = &base[slash + 1..];
    let at = rest.rfind('@')?;
    let scope = &base[..slash];
    let name_base = &rest[..at];
    let version = &rest[at + 1..];
    let name = format!("{scope}/{name_base}").trim().to_owned();
    let version = version.trim().to_owned();
    if name.is_empty() || version.is_empty() {
        return None;
    }
    Some((name, version))
}

fn split_pnpm_unscoped_fallback(base: &str) -> Option<(String, String)> {
    let at = base.rfind('@')?;
    let name = base[..at].trim().to_owned();
    let version = base[at + 1..].trim().to_owned();
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
/// Only `Name (version)` lines under the `NUGET` remote section count as
/// assessable; `Name (version)` lines under the `GIT` section count as
/// unsupported git revisions (`is_git` incomplete, never clean, issue
/// #584 wont-fix); `HTTP`/`GITHUB` sections and group headers are
/// skipped.
pub fn parse_paket_lock(text: &str) -> Result<Vec<LockedPackage>, String> {
    let mut out = Vec::new();
    let mut in_nuget = false;
    let mut in_git = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "NUGET" {
            in_nuget = true;
            in_git = false;
            continue;
        }
        if trimmed == "GIT" {
            in_nuget = false;
            in_git = true;
            continue;
        }
        if trimmed == "HTTP" || trimmed == "GITHUB" {
            in_nuget = false;
            in_git = false;
            continue;
        }
        if !in_nuget && !in_git {
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
                is_git: in_git,
                is_private: false,
            });
        }
    }
    out.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    Ok(out)
}

fn paket_line_re() -> Option<&'static Regex> {
    static RE: OnceLock<Regex> = OnceLock::new();
    if let Some(compiled) = RE.get() {
        return Some(compiled);
    }
    // Greedy name up to the last `(` (mirrors `rfind`), version with no
    // parens, trailing bytes after `)` ignored like the historical slice.
    match Regex::new(r"^(?P<name>.+)\((?P<version>[^()]+)\)") {
        Ok(compiled) => {
            let _ = RE.set(compiled);
            RE.get()
        }
        Err(_) => None,
    }
}

fn split_paket_line(trimmed: &str) -> Option<(String, String)> {
    if let Some(re) = paket_line_re() {
        let caps = re.captures(trimmed)?;
        let name = caps["name"].trim().to_owned();
        let version = caps["version"].trim().to_owned();
        if name.is_empty() || version.is_empty() {
            return None;
        }
        // Historical guard: `remote:`-shaped names never count, even when
        // the paren shape matches.
        if name.contains("remote") {
            return None;
        }
        return Some((name, version));
    }
    split_paket_line_fallback(trimmed)
}

fn split_paket_line_fallback(trimmed: &str) -> Option<(String, String)> {
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
    fn pnpm_lock_handles_quoted_bare_scoped_peer_and_link_keys() {
        let text = "lockfileVersion: '9.0'\npackages:\n  'react@18.2.0':\n    resolution: {integrity: sha512-abc}\n  \"lodash@4.17.21\":\n    resolution: {integrity: sha512-def}\n  acorn@8.18.0:\n    resolution: {integrity: sha512-ghi}\n  '@babel/core@7.29.7':\n    resolution: {integrity: sha512-jkl}\n  'jest@30.2.0(@types/node@22.20.2)':\n    resolution: {integrity: sha512-mno}\n  'my-workspace@link:.':\n    resolution: {directory: .}\n  some-pkg@link:../some-pkg:\n    resolution: {directory: ../some-pkg}\n";
        let packages = parse_pnpm_lock(text).expect("parses");
        assert!(packages
            .iter()
            .any(|package| package.name == "react" && package.version == "18.2.0"));
        assert!(packages
            .iter()
            .any(|package| package.name == "lodash" && package.version == "4.17.21"));
        assert!(packages
            .iter()
            .any(|package| package.name == "acorn" && package.version == "8.18.0"));
        assert!(packages
            .iter()
            .any(|package| package.name == "@babel/core" && package.version == "7.29.7"));
        assert!(packages
            .iter()
            .any(|package| package.name == "jest" && package.version == "30.2.0"));
        assert!(!packages.iter().any(|package| package.name.contains("link")));
        assert!(!packages
            .iter()
            .any(|package| package.version.contains("link:")));
        assert_eq!(packages.len(), 5);
    }

    #[test]
    fn pnpm_lock_missing_packages_is_empty_and_invalid_fails() {
        let missing = "lockfileVersion: '9.0'\nimporters:\n  .:\n    specifier: 1.0.0\n";
        let packages = parse_pnpm_lock(missing).expect("missing packages is empty");
        assert!(packages.is_empty());
        let empty_packages = "packages: {}\n";
        let packages = parse_pnpm_lock(empty_packages).expect("empty packages is empty");
        assert!(packages.is_empty());
        assert!(parse_pnpm_lock("packages: [unclosed").is_err());
        assert!(parse_pnpm_lock("packages: ['a', 'b']").is_err());
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
        assert!(packages.iter().all(|package| !package.is_git));
    }

    #[test]
    fn paket_lock_git_section_is_incomplete_never_dropped() {
        // Issue #584: GIT entries are unsupported revisions, never
        // silently dropped and never clean.
        let text = "NUGET\n  remote: https://api.nuget.org/v3/index.json\n    FSharp.Core (10.1.201)\nGIT\n  remote: https://github.com/example/lib.git\n    Git.Lib (1.0.0)\nHTTP\n  remote: https://example.com\n    Other (9.9.9)\n";
        let packages = parse_paket_lock(text).expect("parses");
        assert_eq!(packages.len(), 2);
        let assessable = packages
            .iter()
            .find(|package| package.name == "FSharp.Core")
            .expect("nuget entry");
        assert!(!assessable.is_git);
        let git = packages
            .iter()
            .find(|package| package.name == "Git.Lib")
            .expect("git entry");
        assert!(git.is_git);
        assert!(!packages.iter().any(|package| package.name == "Other"));
        // Matching maps the GIT entry to incomplete, never clean.
        let (findings, unassessed) = crate::vuln::match_packages(&packages, &[]);
        assert!(findings.is_empty());
        assert_eq!(unassessed.len(), 1);
        assert_eq!(unassessed[0].package, "Git.Lib");
        assert_eq!(unassessed[0].reason, crate::vuln::REASON_GIT);
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

    #[test]
    fn regex_pnpm_scoped_peer_and_dash_boundaries() {
        // Scoped peer suffix strips to the base version.
        assert_eq!(
            split_pnpm_key("@babel/core@7.29.7(@babel/types@7.0.0)"),
            Some(("@babel/core".to_owned(), "7.29.7".to_owned()))
        );
        // Dashes are literal: `my-jest` never collides with `jest`.
        assert_eq!(
            split_pnpm_key("my-jest@30.2.0"),
            Some(("my-jest".to_owned(), "30.2.0".to_owned()))
        );
        assert_eq!(
            split_pnpm_key("jest@30.2.0"),
            Some(("jest".to_owned(), "30.2.0".to_owned()))
        );
        // `link:` versions stay skipped by the caller; the splitter itself
        // still surfaces them so the filter owns the policy.
        assert_eq!(
            split_pnpm_key("some-pkg@link:../some-pkg"),
            Some(("some-pkg".to_owned(), "link:../some-pkg".to_owned()))
        );
    }

    #[test]
    fn regex_paket_line_keeps_greedy_paren_and_remote_guard() {
        assert_eq!(
            split_paket_line("My.Pkg (1.2.3)"),
            Some(("My.Pkg".to_owned(), "1.2.3".to_owned()))
        );
        // Trailing bytes after `)` are ignored like the historical slice.
        assert_eq!(
            split_paket_line("My.Pkg (1.2.3) extra"),
            Some(("My.Pkg".to_owned(), "1.2.3".to_owned()))
        );
        // `remote:`-shaped names never count.
        assert!(split_paket_line("remote: foo (1.2.3)").is_none());
        assert!(split_paket_line("no-parens-here").is_none());
    }
}
