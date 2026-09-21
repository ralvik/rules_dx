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
//! - npm (`pnpm-lock.yaml` YAML, `package-lock.json` JSON,
//!   `yarn.lock` v1 text): registry entries are assessable;
//!   `link:`/`file:` workspace members are first-party and skipped, not
//!   assessed; git-hosted entries (`git+`, `git://`, `git@`,
//!   `github:`/`gitlab:`/`bitbucket:`/`gist:` shortcuts, `.git` URLs,
//!   host-archive tarballs, or pnpm `resolution: {type: git}` with
//!   `repo`/`commit`) are unsupported revisions (`is_git` incomplete,
//!   never dropped and never clean).
//! - Maven (`third_party/jvm/maven_install.json`, JSON): `artifacts`
//!   carry `group:artifact` plus `version`.
//! - NuGet (`third_party/dotnet/paket.lock`, text): `Name (version)`
//!   lines under the `NUGET` remote section are assessable; `Name
//!   (version)` lines under the `GIT` section are unsupported revisions
//!
//!   identity), reported as `is_git` incomplete, never dropped and never
//!   clean; `HTTP`/`GITHUB` sections and group headers are skipped.
//! - Go (`third_party/go/go.mod`, text): `require` entries (single-line
//!   plus parenthesized blocks, comments stripped) are assessable with
//!   their verbatim `v`-prefixed versions; the main `module` directive is
//!   first-party and skipped, not assessed. `replace` targets resolving
//!   to a filesystem path (no replacement version) are first-party
//!   workspace members and skipped; versioned replacements assess at the
//!   replacement path plus version. `go.sum` carries hashes only and is
//!   never parsed.
//!
//! Private packages have no lockfile auto-detection in V1
//! wont-fix, auditor-owned): a private registry entry is
//! indistinguishable from a public one in lock bytes, so callers mark
//! `is_private` explicitly and matching fails those as incomplete,
//! never clean.
//!
//! License identities per ecosystem: Cargo reads
//! `cargo-bazel-lock.json` (`license` per crate, fallback `UNKNOWN`);
//! npm reads `package-lock.json` `license` fields where present (both
//! `packages:` and legacy `dependencies:` shapes, fallback `UNKNOWN` for
//! pnpm/yarn-only workspaces whose locks carry no license); Maven,
//! NuGet, and Go resolve via the committed `[[inventory]]` table in
//! `licenses.toml` (see [`crate::license_policy::LicenseInventory`]),
//! fallback `UNKNOWN` when uninventoried (denied in `distributed`,
//! inventoried in `internal` per the expression lattice). Notice texts
//! ride per-package `text_present` from the same inventory (absent means
//! no words, fail closed); `missing-notice-text` evaluation lives in
//! [`crate::license_notice`] and is wired into live audit.
//!
//! Dependency evaluation (See: `docs/cli/commands/audit-update-bazel.md#dx-audit`, issue #750):
//! standard crates own each machine format (`cargo-lock` for Cargo.lock,
//! `serde_json` for `package-lock.json`/`maven_install.json`, `yaml_serde`
//! for `pnpm-lock.yaml`, `toml` for Cargo manifests, `semver` for Cargo/Go
//! ordering, `regex` for declarative pnpm/paket splits); yarn v1,
//! paket, and `go.mod` text shapes have no stable crate and stay line
//! parsers with fail-closed git/first-party gates.

use std::sync::OnceLock;

use regex::Regex;

use crate::vuln::LockedPackage;

/// Parse one `Cargo.lock` (TOML) into assessable locked packages for the
/// `cargo` set via the upstream `cargo-lock` crate (RustSec, V1/V2/V3/V4).
/// Registry packages (including sparse registries) become assessable
/// entries; `git` sources become `is_git` incomplete markers via
/// `SourceId::is_git`; path-only workspace members (`None` source) and
/// explicit `path` sources are skipped as first-party. Missing or
/// malformed fields fail closed through the crate's structured errors,
/// never silent skips.
pub fn parse_cargo_lock(text: &str) -> Result<Vec<LockedPackage>, String> {
    let lockfile: cargo_lock::Lockfile = text
        .parse()
        .map_err(|error: cargo_lock::Error| format!("invalid Cargo.lock: {error}"))?;
    if lockfile.packages.is_empty() {
        return Err("invalid Cargo.lock: missing [[package]]".to_owned());
    }
    let mut out = Vec::new();
    for package in &lockfile.packages {
        let name = package.name.as_str().to_owned();
        let version = package.version.to_string();
        if name.trim().is_empty() || version.trim().is_empty() {
            return Err("invalid Cargo.lock: empty package name or version".to_owned());
        }
        let Some(source) = &package.source else {
            // First-party workspace member (e.g. `dx_* 0.0.0`): skip, not
            // an upstream dependency with advisory identity.
            continue;
        };
        if source.is_git() {
            out.push(LockedPackage {
                name,
                version,
                set: "cargo".to_owned(),
                is_git: true,
                is_private: false,
            });
            continue;
        }
        if source.is_path() {
            // Explicit path source: first-party, not assessed.
            continue;
        }
        out.push(LockedPackage {
            name,
            version,
            set: "cargo".to_owned(),
            is_git: false,
            is_private: false,
        });
    }
    out.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    out.dedup_by(|b, a| a.name == b.name && a.version == b.version);
    Ok(out)
}

/// True for npm git-hosted references across all three npm lock
/// shapes (wont-fix, auditor-owned): explicit git markers only, never
/// bare registry versions. Covers `git+` transports, `git://` and
/// `git@` forms, `github:`/`gitlab:`/`bitbucket:`/`gist:` shortcuts,
/// `.git`-suffixed URLs, and host-archive tarballs
/// (`codeload.github.com`, `api.github.com/.../tarball`). A bare `#`
/// fragment alone never counts (registry tarballs carry
/// `#sha512-...` fragments too); git identity comes from the scheme,
/// host, or `repo`/`commit` fields, not the fragment.
pub fn is_npm_git_reference(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("git+") || lower.contains("git://") || lower.contains("git@") {
        return true;
    }
    for prefix in [
        "github:",
        "gitlab:",
        "bitbucket:",
        "gist:",
        "git://",
        "git+",
        "git@",
    ] {
        if lower.starts_with(prefix) {
            return true;
        }
    }
    if lower.contains("codeload.github.com") || lower.contains("/tarball/") {
        return true;
    }
    // `.git` URL path (with optional `#commit` fragment or query).
    let without_fragment = lower.split(['#', '?']).next().unwrap_or(&lower);
    if without_fragment.ends_with(".git") || without_fragment.contains(".git/") {
        return true;
    }
    false
}

/// True for one pnpm `resolution:` value carrying git identity:
/// `type: git`, a `commit` pin, a git-shaped `repo`, or a git-shaped
/// `tarball` (host-archive). Registry `integrity`-only entries stay
/// assessable; `directory:`/`link:` workspace values never reach here
/// (their keys are skipped first-party before this check).
fn pnpm_resolution_is_git(resolution: &yaml_serde::Value) -> bool {
    let mapping = match resolution.as_mapping() {
        Some(mapping) => mapping,
        None => return false,
    };
    let get_str = |key: &str| -> Option<String> {
        mapping.iter().find_map(|(key_value, value)| {
            if key_value.as_str()? == key {
                value.as_str().map(str::to_owned)
            } else {
                None
            }
        })
    };
    if let Some(kind) = get_str("type") {
        if kind.trim().eq_ignore_ascii_case("git") {
            return true;
        }
    }
    // `commit` pins git content (the store key is git-hosted rather
    // than integrity-addressed); `repo` plus `commit` is the canonical
    // pnpm git shape.
    if let Some(commit) = get_str("commit") {
        if !commit.trim().is_empty() {
            return true;
        }
    }
    if let Some(repo) = get_str("repo") {
        if is_npm_git_reference(&repo) {
            return true;
        }
    }
    if let Some(tarball) = get_str("tarball") {
        if is_npm_git_reference(&tarball) {
            return true;
        }
    }
    false
}

/// Parse one `pnpm-lock.yaml` document value's `packages:` mapping into
/// `npm` lock entries. Shared by single- and multi-document lockfiles.
fn pnpm_packages_from_value(value: &yaml_serde::Value, out: &mut Vec<LockedPackage>) {
    let packages = match value.get("packages") {
        None | Some(yaml_serde::Value::Null) => return,
        Some(packages) => packages,
    };
    let mapping = match packages.as_mapping() {
        Some(mapping) => mapping,
        None => return,
    };
    for (key_value, detail) in mapping {
        let key = key_value.as_str().unwrap_or("").trim().to_owned();
        if key.is_empty() {
            continue;
        }
        // `link:` entries appear as `name@link:...` keys; `file:`
        // entries as `name@file:...` keys. Both are first-party
        // workspace members, skipped, not assessed.
        if key.contains("link:") || key.contains("file:") {
            continue;
        }
        if let Some((name, version)) = split_pnpm_key(&key) {
            if name.is_empty() || version.is_empty() {
                continue;
            }
            // Skip workspace `link:`/`file:` versions that slipped
            // through key filtering.
            if version.starts_with("link:") || version.starts_with("file:") {
                continue;
            }
            let is_git = is_npm_git_reference(&key)
                || is_npm_git_reference(&version)
                || pnpm_resolution_is_git(detail);
            out.push(LockedPackage {
                name,
                version,
                set: "npm".to_owned(),
                is_git,
                is_private: false,
            });
        }
    }
}

/// Split one `pnpm-lock.yaml` text into YAML documents on `---`
/// boundaries: pnpm writes a leading `---` env document when config or
/// package-manager dependencies apply, then the project document. A
/// single-document reader silently returns the env graph (plausible
/// packages, no vulnerabilities), so every document is an inventory of
/// its own and all are merged here.
fn split_pnpm_documents(text: &str) -> Vec<String> {
    if !text
        .lines()
        .any(|line| line.trim_start().starts_with("---"))
    {
        return vec![text.to_owned()];
    }
    let mut documents = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        if line.trim() == "---" || line.trim_start().starts_with("--- ") {
            if !current.trim().is_empty() {
                documents.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.trim().is_empty() {
        documents.push(current);
    }
    if documents.is_empty() {
        documents.push(text.to_owned());
    }
    documents
}

/// Parse one `pnpm-lock.yaml` into assessable packages for the `npm` set
/// via `yaml_serde`. External entries under `packages:` shaped
/// `name@version` or `@scope/name@version` become assessable; `link:`/
/// `file:` entries (workspace members) are skipped. Git-hosted entries
/// (git-shaped keys/versions, host-archive tarballs, or
/// `resolution: {type: git}` with `repo`/`commit`) become `is_git`
/// incomplete markers, never dropped and never clean. Versions with
/// peer suffixes (`1.0.0(peer@2.0.0)`) strip the suffix. Multi-document
/// lockfiles (env plus project documents) merge every document's
/// `packages:` map so scanners never report the env graph alone.
pub fn parse_pnpm_lock(text: &str) -> Result<Vec<LockedPackage>, String> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let mut parsed_any = false;
    for document in split_pnpm_documents(text) {
        if document.trim().is_empty() {
            continue;
        }
        let value: yaml_serde::Value = yaml_serde::from_str(&document)
            .map_err(|error| format!("invalid pnpm-lock.yaml: {error}"))?;
        if value.is_null() {
            continue;
        }
        parsed_any = true;
        if value.get("packages").is_some() {
            pnpm_packages_from_value(&value, &mut out);
        } else if value.as_mapping().is_none() {
            return Err("invalid pnpm-lock.yaml: missing packages".to_owned());
        }
    }
    if !parsed_any {
        return Ok(Vec::new());
    }
    // A scalar or sequence document never parses as a mapping with a
    // clear error above; guard the `packages: [...]` shape explicitly
    // for a deterministic message.
    if out.is_empty() {
        let single: Result<yaml_serde::Value, _> = yaml_serde::from_str(text);
        if let Ok(value) = single {
            if let Some(packages) = value.get("packages") {
                if !packages.is_null() && packages.as_mapping().is_none() {
                    return Err("invalid pnpm-lock.yaml: missing packages".to_owned());
                }
            }
        }
    }
    out.sort_by(|a, b| (&a.name, &a.version, a.is_git).cmp(&(&b.name, &b.version, b.is_git)));
    out.dedup_by(|b, a| a.name == b.name && a.version == b.version && a.is_git == b.is_git);
    Ok(out)
}

/// Strip one `node_modules/`-rooted package-lock path to its package
/// name: the segment after the last `node_modules/` (`node_modules/a`
/// to `a`, `node_modules/@scope/name` to `@scope/name`,
/// `node_modules/a/node_modules/b` to `b`). Returns `None` for the
/// root `""` entry and non-`node_modules` keys.
fn package_lock_name(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let last = trimmed.rsplit("node_modules/").next()?.trim();
    if last.is_empty() {
        return None;
    }
    if last.starts_with('@') && !last.contains('/') {
        return None;
    }
    if last.contains('/') && !last.starts_with('@') {
        // Nested non-scope paths still reduce to the final segment;
        // anything with a remaining `/` outside a scope is malformed.
        if !last.starts_with('@') {
            let segment = last.rsplit('/').next()?.trim();
            if segment.is_empty() {
                return None;
            }
            return Some(segment.to_owned());
        }
    }
    if last.contains('/') {
        // Scoped names carry exactly one `/`; deeper nesting reduces to
        // the trailing file segment handled above.
        let parts: Vec<&str> = last.split('/').collect();
        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            return Some(last.to_owned());
        }
        let segment = parts.last()?.trim();
        if segment.is_empty() {
            return None;
        }
        return Some(segment.to_owned());
    }
    Some(last.to_owned())
}

/// Parse one `package-lock.json` (npm v1/v2/v3) into assessable
/// packages for the `npm` set. Registry entries under `packages:`
/// (`node_modules/<name>` with `version`) or legacy `dependencies:`
/// (`<name>` with `version`) become assessable; `link: true` and
/// `file:` entries are first-party workspace members and skipped.
/// Git-hosted entries (git-shaped `version`/`resolved`/`from`) become
/// `is_git` incomplete markers, never dropped and never clean.
pub fn parse_package_lock(text: &str) -> Result<Vec<LockedPackage>, String> {
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|error| format!("invalid package-lock.json: {error}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "invalid package-lock.json: expected object".to_owned())?;
    let mut out = Vec::new();
    if let Some(packages) = object.get("packages").and_then(|value| value.as_object()) {
        for (path, detail) in packages {
            if path.trim().is_empty() {
                continue;
            }
            let Some(name) = package_lock_name(path) else {
                continue;
            };
            let detail = match detail.as_object() {
                Some(detail) => detail,
                None => continue,
            };
            if detail
                .get("link")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
            {
                continue;
            }
            let version = detail
                .get("version")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim()
                .to_owned();
            let resolved = detail
                .get("resolved")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim()
                .to_owned();
            let from = detail
                .get("from")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim()
                .to_owned();
            if version.is_empty() {
                continue;
            }
            if version.starts_with("file:") || version.starts_with("link:") {
                continue;
            }
            if resolved.starts_with("file:") {
                continue;
            }
            let is_git = is_npm_git_reference(&version)
                || is_npm_git_reference(&resolved)
                || is_npm_git_reference(&from);
            out.push(LockedPackage {
                name,
                version,
                set: "npm".to_owned(),
                is_git,
                is_private: false,
            });
        }
    }
    if let Some(dependencies) = object
        .get("dependencies")
        .and_then(|value| value.as_object())
    {
        for (name, detail) in dependencies {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            let detail = match detail.as_object() {
                Some(detail) => detail,
                None => continue,
            };
            if detail
                .get("link")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
            {
                continue;
            }
            let version = detail
                .get("version")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim()
                .to_owned();
            let resolved = detail
                .get("resolved")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim()
                .to_owned();
            let from = detail
                .get("from")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim()
                .to_owned();
            if version.is_empty() {
                continue;
            }
            if version.starts_with("file:") || version.starts_with("link:") {
                continue;
            }
            if resolved.starts_with("file:") {
                continue;
            }
            // Deduplicate against the `packages:` map: same name plus
            // version with the same git disposition is one entry.
            let is_git = is_npm_git_reference(&version)
                || is_npm_git_reference(&resolved)
                || is_npm_git_reference(&from);
            if out.iter().any(|package| {
                package.name == name && package.version == version && package.is_git == is_git
            }) {
                continue;
            }
            out.push(LockedPackage {
                name: name.to_owned(),
                version,
                set: "npm".to_owned(),
                is_git,
                is_private: false,
            });
        }
    }
    out.sort_by(|a, b| (&a.name, &a.version, a.is_git).cmp(&(&b.name, &b.version, b.is_git)));
    out.dedup_by(|b, a| a.name == b.name && a.version == b.version && a.is_git == b.is_git);
    Ok(out)
}

/// Split one yarn v1 stanza header selector (`name@range` or
/// `@scope/name@range`, optionally quoted) into its package name.
/// The range is the requested selector, not the locked version; the
/// locked `version "..."` field carries the assessed identity.
fn split_yarn_selector(selector: &str) -> Option<String> {
    let trimmed = selector.trim().trim_matches('"').trim();
    if trimmed.is_empty() || trimmed.starts_with("__metadata:") {
        return None;
    }
    if let Some((name, _)) = split_pnpm_key(trimmed) {
        // `split_pnpm_key` strips peer suffixes and splits on the last
        // `@`; for selectors the trailing part is the range, so only
        // the name is kept.
        if !name.is_empty() {
            return Some(name);
        }
        return None;
    }
    // Bare names without a range (`"pkg":`) carry no selector `@`;
    // the whole header is the name.
    if !trimmed.contains('@') {
        let name = trimmed.trim_end_matches(':').trim().to_owned();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

/// Read one `  version "..."` / `  resolved "..."` field from a yarn
/// stanza body line: quoted or bare values both parse; trailing
/// comments after the value are ignored.
fn yarn_field(line: &str, key: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with(key) {
        return None;
    }
    let rest = trimmed[key.len()..].trim();
    let rest = rest.strip_prefix(':').unwrap_or(rest).trim();
    if rest.is_empty() {
        return None;
    }
    if let Some(stripped) = rest.strip_prefix('"') {
        let inner = stripped.split('"').next().unwrap_or("").trim();
        if inner.is_empty() {
            return None;
        }
        return Some(inner.to_owned());
    }
    let token = rest.split_whitespace().next().unwrap_or("").trim();
    if token.is_empty() {
        return None;
    }
    Some(token.to_owned())
}

/// Parse one `yarn.lock` (v1 classic text) into assessable packages for
/// the `npm` set. Stanza headers name the selector, the indented
/// `version` field carries the locked version, and `resolved` carries
/// the fetch URL. Registry entries become assessable; `file:`/`link:`/
/// `portal:` entries are first-party and skipped. Git-hosted entries
/// (git-shaped headers, versions, or `resolved` URLs) become `is_git`
/// incomplete markers, never dropped and never clean.
pub fn parse_yarn_lock(text: &str) -> Result<Vec<LockedPackage>, String> {
    let mut out = Vec::new();
    let mut header: Option<String> = None;
    let mut version: Option<String> = None;
    let mut resolved = String::new();
    let flush = |header: &mut Option<String>,
                 version: &mut Option<String>,
                 resolved: &mut String,
                 out: &mut Vec<LockedPackage>| {
        let Some(selector) = header.take() else {
            *version = None;
            resolved.clear();
            return;
        };
        let version_value = version.take().unwrap_or_default();
        let resolved_value = std::mem::take(resolved);
        // First selector names the stanza; comma-separated alternates
        // alias the same locked version.
        let first = selector.split(',').next().unwrap_or("").trim();
        let Some(name) = split_yarn_selector(first) else {
            return;
        };
        if name.is_empty() || version_value.trim().is_empty() {
            return;
        }
        let version_value = version_value.trim().to_owned();
        let resolved_value = resolved_value.trim().to_owned();
        if version_value.starts_with("file:")
            || version_value.starts_with("link:")
            || version_value.starts_with("portal:")
        {
            return;
        }
        if resolved_value.starts_with("file:")
            || resolved_value.starts_with("link:")
            || resolved_value.starts_with("portal:")
        {
            return;
        }
        let is_git = is_npm_git_reference(first)
            || is_npm_git_reference(&version_value)
            || is_npm_git_reference(&resolved_value);
        out.push(LockedPackage {
            name,
            version: version_value,
            set: "npm".to_owned(),
            is_git,
            is_private: false,
        });
    };
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            if header.is_some() && version.is_some() {
                flush(&mut header, &mut version, &mut resolved, &mut out);
            } else if line.trim().is_empty() {
                header = None;
                version = None;
                resolved.clear();
            }
            continue;
        }
        if !line.starts_with([' ', '\t']) {
            // New stanza header: flush the previous stanza when it
            // carried a version, then start the next header.
            if header.is_some() && version.is_some() {
                flush(&mut header, &mut version, &mut resolved, &mut out);
            } else if header.is_some() {
                resolved.clear();
            }
            let selector = line.trim_end_matches(':').trim().to_owned();
            if selector.is_empty() {
                header = None;
                continue;
            }
            header = Some(selector);
            version = None;
            resolved.clear();
            continue;
        }
        if header.is_none() {
            continue;
        }
        if let Some(value) = yarn_field(line, "version") {
            version = Some(value);
        } else if let Some(value) = yarn_field(line, "resolved") {
            resolved = value;
        }
    }
    if header.is_some() && version.is_some() {
        flush(&mut header, &mut version, &mut resolved, &mut out);
    }
    out.sort_by(|a, b| (&a.name, &a.version, a.is_git).cmp(&(&b.name, &b.version, b.is_git)));
    out.dedup_by(|b, a| a.name == b.name && a.version == b.version && a.is_git == b.is_git);
    Ok(out)
}

/// Split one pnpm package key (`name@version` or `@scope/name@version`)
/// into name and version, stripping peer suffixes (`1.0.0(peer)`).
/// Declarative `regex` splits replace the `find`/`rfind`
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
    // Regex-first: scoped `@scope/name@version` and unscoped
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
/// wont-fix); `HTTP`/`GITHUB` sections and group headers are
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

#[path = "locks_go.rs"]
mod locks_go;
#[path = "locks_licenses.rs"]
mod locks_licenses;

pub use locks_go::*;
pub use locks_licenses::*;

#[cfg(test)]
#[path = "locks_tests.rs"]
mod locks_tests;
