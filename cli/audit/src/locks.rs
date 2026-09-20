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
//! License identities for V1: Cargo reads `cargo-bazel-lock.json`
//! (`license` per crate, fallback `UNKNOWN`); npm/Maven/NuGet/Go report
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
    if without_fragment.ends_with(".git")
        || without_fragment.contains(".git/")
    {
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
    if !text.lines().any(|line| line.trim_start().starts_with("---")) {
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
    out.sort_by(|a, b| {
        (&a.name, &a.version, a.is_git).cmp(&(&b.name, &b.version, b.is_git))
    });
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
            let is_git =
                is_npm_git_reference(&version) || is_npm_git_reference(&resolved) || is_npm_git_reference(&from);
            out.push(LockedPackage {
                name,
                version,
                set: "npm".to_owned(),
                is_git,
                is_private: false,
            });
        }
    }
    if let Some(dependencies) = object.get("dependencies").and_then(|value| value.as_object()) {
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
            let is_git =
                is_npm_git_reference(&version) || is_npm_git_reference(&resolved) || is_npm_git_reference(&from);
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
    out.sort_by(|a, b| {
        (&a.name, &a.version, a.is_git).cmp(&(&b.name, &b.version, b.is_git))
    });
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
    if rest.starts_with('"') {
        let inner = rest[1..].split('"').next().unwrap_or("").trim();
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
    out.sort_by(|a, b| {
        (&a.name, &a.version, a.is_git).cmp(&(&b.name, &b.version, b.is_git))
    });
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

/// Strip one `go.mod` line comment: `//` starts a comment only at the
/// line start or after whitespace, so `https://` inside a directive (or
/// a `//`-free module path) never truncates the requirement itself.
fn strip_go_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut index = 0usize;
    while index + 1 < bytes.len() {
        if bytes[index] == b'/' && bytes[index + 1] == b'/' {
            if index == 0 || bytes[index - 1].is_ascii_whitespace() {
                return line[..index].trim_end();
            }
            // `//` inside a token (never a comment in `go.mod`
            // requirements) stays part of the line.
            index += 2;
            continue;
        }
        index += 1;
    }
    line
}

/// Parse one `replace` right-hand side (`new-path [new-version]`) into
/// its path plus optional version. A missing version means a filesystem
/// target (first-party, skipped); a present version means a versioned
/// replacement (assessed at the replacement identity).
fn split_go_replace_rhs(rhs: &str) -> Option<(String, Option<String>)> {
    let mut tokens = rhs.split_whitespace();
    let path = tokens.next()?.trim().to_owned();
    if path.is_empty() {
        return None;
    }
    let version = tokens.next().map(str::trim).filter(|v| !v.is_empty());
    Some((path, version.map(str::to_owned)))
}

/// Parse one `go.mod` into assessable packages for the `go` set.
/// `require` entries (single-line `require mod vX` plus `require (`
/// blocks, `// indirect` and other comments stripped) become assessable
/// with verbatim `v`-prefixed versions (matching normalizes the prefix
/// in [`crate::vuln`]); the main `module` directive is first-party and
/// skipped. `replace` targets with no replacement version are
/// first-party filesystem paths and skipped; versioned replacements
/// assess at the replacement path plus version. `exclude`, `retract`,
/// `toolchain`, and `go` directives never count. A missing `module`
/// directive fails closed (never an empty clean set from garbage).
pub fn parse_go_mod(text: &str) -> Result<Vec<LockedPackage>, String> {
    let mut has_module = false;
    let mut in_require = false;
    let mut in_replace = false;
    let mut in_skip_block = false;
    let mut requires: Vec<(String, String)> = Vec::new();
    let mut replaces: std::collections::BTreeMap<String, (String, Option<String>)> =
        std::collections::BTreeMap::new();
    for raw in text.lines() {
        let line = strip_go_comment(raw).trim().to_owned();
        if line.is_empty() {
            continue;
        }
        if in_skip_block {
            if line == ")" {
                in_skip_block = false;
            }
            continue;
        }
        if in_require {
            if line == ")" {
                in_require = false;
                continue;
            }
            let mut tokens = line.split_whitespace();
            if let (Some(name), Some(version)) = (tokens.next(), tokens.next()) {
                if !name.is_empty() && !version.is_empty() {
                    requires.push((name.to_owned(), version.to_owned()));
                }
            }
            continue;
        }
        if in_replace {
            if line == ")" {
                in_replace = false;
                continue;
            }
            if let Some((lhs, rhs)) = line.split_once("=>") {
                let old_path = lhs.split_whitespace().next().unwrap_or("").trim();
                if let Some((new_path, new_version)) = split_go_replace_rhs(rhs.trim()) {
                    if !old_path.is_empty() {
                        replaces.insert(old_path.to_owned(), (new_path, new_version));
                    }
                }
            }
            continue;
        }
        if line == ")" {
            return Err("invalid go.mod: unexpected closing paren".to_owned());
        }
        if line.starts_with("module ") || line == "module" {
            has_module = true;
            continue;
        }
        if line == "require (" || line.starts_with("require (") {
            in_require = true;
            continue;
        }
        if line.starts_with("require ") || line.starts_with("require\t") {
            let rest = line["require".len()..].trim();
            let mut tokens = rest.split_whitespace();
            if let (Some(name), Some(version)) = (tokens.next(), tokens.next()) {
                if !name.is_empty() && !version.is_empty() {
                    requires.push((name.to_owned(), version.to_owned()));
                }
            }
            continue;
        }
        if line == "replace (" || line.starts_with("replace (") {
            in_replace = true;
            continue;
        }
        if line.starts_with("replace ") || line.starts_with("replace\t") {
            let rest = line["replace".len()..].trim();
            if let Some((lhs, rhs)) = rest.split_once("=>") {
                let old_path = lhs.split_whitespace().next().unwrap_or("").trim();
                if let Some((new_path, new_version)) = split_go_replace_rhs(rhs.trim()) {
                    if !old_path.is_empty() {
                        replaces.insert(old_path.to_owned(), (new_path, new_version));
                    }
                }
            }
            continue;
        }
        if line == "exclude ("
            || line.starts_with("exclude (")
            || line == "retract ("
            || line.starts_with("retract (")
        {
            in_skip_block = true;
            continue;
        }
        // `go`, `toolchain`, single-line `exclude`/`retract`, and unknown
        // directives carry no dependency identity; skip them.
    }
    if !has_module {
        return Err("invalid go.mod: missing module directive".to_owned());
    }
    let mut out = Vec::new();
    for (name, version) in requires {
        match replaces.get(&name) {
            Some((_, None)) => continue,
            Some((new_path, Some(new_version))) => out.push(LockedPackage {
                name: new_path.clone(),
                version: new_version.clone(),
                set: "go".to_owned(),
                is_git: false,
                is_private: false,
            }),
            None => out.push(LockedPackage {
                name,
                version,
                set: "go".to_owned(),
                is_git: false,
                is_private: false,
            }),
        }
    }
    out.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    out.dedup_by(|b, a| a.name == b.name && a.version == b.version);
    Ok(out)
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
    fn pnpm_lock_git_resolutions_are_incomplete_never_dropped() {
        // Git-hosted entries are unsupported revisions, never
        // silently dropped and never clean.
        let text = "lockfileVersion: '9.0'\npackages:\n  'react@18.2.0':\n    resolution: {integrity: sha512-abc}\n  'git-dep@github:user/repo#abc123':\n    resolution: {repo: 'https://github.com/user/repo.git', commit: abc123}\n  'typed-git@0.0.0':\n    resolution: {type: git, repo: 'https://github.com/user/typed.git', commit: def456}\n  'tarball-git@https://codeload.github.com/user/repo/tar.gz#abc':\n    resolution: {tarball: 'https://codeload.github.com/user/repo/tar.gz#abc'}\n  'local@file:../local':\n    resolution: {directory: ../local}\n";
        let packages = parse_pnpm_lock(text).expect("parses");
        assert!(
            packages
                .iter()
                .any(|package| package.name == "react" && !package.is_git)
        );
        for name in ["git-dep", "typed-git", "tarball-git"] {
            let git = packages
                .iter()
                .find(|package| package.name == name)
                .unwrap_or_else(|| panic!("git entry {name}"));
            assert!(git.is_git, "{name} must be git incomplete");
        }
        assert!(!packages.iter().any(|package| package.name == "local"));
        // Matching maps every git entry to incomplete, never clean.
        let (findings, unassessed) = crate::vuln::match_packages(&packages, &[]);
        assert!(findings.is_empty());
        assert_eq!(unassessed.len(), 3);
        assert!(unassessed.iter().all(|entry| entry.reason == crate::vuln::REASON_GIT));
    }

    #[test]
    fn pnpm_lock_merges_multi_document_env_plus_project() {
        // Two-document lockfiles carry the env graph first and the
        // project graph last; reading only the first document reports
        // plausible packages with no vulnerabilities.
        let text = "---\nlockfileVersion: '9.0'\npackages:\n  'pnpm-bin@1.0.0':\n    resolution: {integrity: sha512-env}\n---\nlockfileVersion: '9.0'\npackages:\n  'react@18.2.0':\n    resolution: {integrity: sha512-abc}\n";
        let packages = parse_pnpm_lock(text).expect("parses");
        assert!(packages.iter().any(|package| package.name == "react"));
        assert!(packages.iter().any(|package| package.name == "pnpm-bin"));
    }

    #[test]
    fn npm_git_reference_markers_are_explicit_only() {
        assert!(is_npm_git_reference("git+https://github.com/user/repo.git"));
        assert!(is_npm_git_reference("github:user/repo#abc123"));
        assert!(is_npm_git_reference("git@github.com:user/repo.git"));
        assert!(is_npm_git_reference(
            "https://codeload.github.com/user/repo/tar.gz#abc"
        ));
        assert!(is_npm_git_reference(
            "https://github.com/user/repo.git#abc123"
        ));
        // Registry versions and tarballs never count: a bare `#`
        // fragment alone is the registry `#sha512-...` shape.
        assert!(!is_npm_git_reference("18.2.0"));
        assert!(!is_npm_git_reference(
            "https://registry.npmjs.org/react/-/react-18.2.0.tgz"
        ));
        assert!(!is_npm_git_reference(""));
    }

    #[test]
    fn package_lock_parses_registry_skips_links_and_flags_git() {
        let text = r#"{"name":"root","lockfileVersion":3,"packages":{"":{"name":"root"},"node_modules/react":{"version":"18.2.0","resolved":"https://registry.npmjs.org/react/-/react-18.2.0.tgz"},"node_modules/@scope/pkg":{"version":"1.0.0","resolved":"https://registry.npmjs.org/@scope/pkg/-/pkg-1.0.0.tgz"},"node_modules/linked":{"version":"1.0.0","link":true},"node_modules/local":{"version":"file:../local"},"node_modules/git-dep":{"version":"github:user/repo#abc123"},"node_modules/git-url":{"version":"1.0.0","resolved":"git+https://github.com/user/repo.git#abc123"}}}"#;
        let packages = parse_package_lock(text).expect("parses");
        assert!(packages
            .iter()
            .any(|package| package.name == "react" && !package.is_git));
        assert!(packages
            .iter()
            .any(|package| package.name == "@scope/pkg" && !package.is_git));
        for name in ["git-dep", "git-url"] {
            let git = packages
                .iter()
                .find(|package| package.name == name)
                .unwrap_or_else(|| panic!("git entry {name}"));
            assert!(git.is_git, "{name} must be git incomplete");
        }
        assert!(!packages.iter().any(|package| package.name == "linked"));
        assert!(!packages.iter().any(|package| package.name == "local"));
        assert!(!packages.iter().any(|package| package.name.is_empty()));
        let (findings, unassessed) = crate::vuln::match_packages(&packages, &[]);
        assert!(findings.is_empty());
        assert_eq!(unassessed.len(), 2);
        assert!(unassessed.iter().all(|entry| entry.reason == crate::vuln::REASON_GIT));
    }

    #[test]
    fn package_lock_legacy_dependencies_shape_flags_git() {
        let text = r#"{"name":"root","lockfileVersion":1,"dependencies":{"react":{"version":"18.2.0","resolved":"https://registry.npmjs.org/react/-/react-18.2.0.tgz"},"git-dep":{"version":"0.0.0","resolved":"git+ssh://git@github.com/user/repo.git#abc123"}}}"#;
        let packages = parse_package_lock(text).expect("parses");
        assert!(packages
            .iter()
            .any(|package| package.name == "react" && !package.is_git));
        let git = packages
            .iter()
            .find(|package| package.name == "git-dep")
            .expect("git entry");
        assert!(git.is_git);
        assert!(parse_package_lock("not json").is_err());
        assert!(parse_package_lock("[1, 2]").is_err());
    }

    #[test]
    fn yarn_lock_parses_registry_skips_file_and_flags_git() {
        let text = "# yarn lockfile v1\n\nreact@^18.0.0:\n  version \"18.2.0\"\n  resolved \"https://registry.yarnpkg.com/react/-/react-18.2.0.tgz#abc\"\n\n\"@scope/pkg@^1.0.0\":\n  version \"1.0.0\"\n  resolved \"https://registry.yarnpkg.com/@scope/pkg/-/pkg-1.0.0.tgz#def\"\n\n\"git-dep@github:user/repo#abc123\":\n  version \"0.0.0\"\n  resolved \"https://github.com/user/repo.git#abc123\"\n\nlocal@file:../local:\n  version \"file:../local\"\n";
        let packages = parse_yarn_lock(text).expect("parses");
        assert!(packages
            .iter()
            .any(|package| package.name == "react" && package.version == "18.2.0" && !package.is_git));
        assert!(packages
            .iter()
            .any(|package| package.name == "@scope/pkg" && !package.is_git));
        let git = packages
            .iter()
            .find(|package| package.name == "git-dep")
            .expect("git entry");
        assert!(git.is_git);
        assert!(!packages.iter().any(|package| package.name == "local"));
        let (findings, unassessed) = crate::vuln::match_packages(&packages, &[]);
        assert!(findings.is_empty());
        assert_eq!(unassessed.len(), 1);
        assert_eq!(unassessed[0].reason, crate::vuln::REASON_GIT);
    }

    #[test]
    fn npm_private_packages_are_incomplete_never_clean() {
        // Private registries are byte-identical to public ones in every
        // npm lock shape, so callers mark `is_private` explicitly and
        // matching fails those as incomplete, never clean.
        let pkgs = vec![
            LockedPackage {
                name: "@internal/pkg".to_owned(),
                version: "1.0.0".to_owned(),
                set: "npm".to_owned(),
                is_git: false,
                is_private: true,
            },
            LockedPackage {
                name: "react".to_owned(),
                version: "18.2.0".to_owned(),
                set: "npm".to_owned(),
                is_git: false,
                is_private: false,
            },
        ];
        let (findings, unassessed) = crate::vuln::match_packages(&pkgs, &[]);
        assert!(findings.is_empty());
        assert_eq!(unassessed.len(), 1);
        assert_eq!(unassessed[0].package, "@internal/pkg");
        assert_eq!(unassessed[0].reason, crate::vuln::REASON_PRIVATE);
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
        // GIT entries are unsupported revisions, never
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
    fn go_mod_parses_require_block_and_single_line_with_comments() {
        let text = "module rules_dx/third_party/go\n\ngo 1.24.12\n\nrequire (\n\tgithub.com/bazelbuild/buildtools v0.0.0-20250930140053-2eb4fccefb52 // indirect\n\tgithub.com/google/go-cmp v0.6.0\n\tgithub.com/pmezard/go-difflib v1.0.0\n)\n\nrequire example.com/single v1.2.3 // indirect\n";
        let packages = parse_go_mod(text).expect("parses");
        assert_eq!(packages.len(), 4);
        assert!(packages
            .iter()
            .any(|package| package.name == "github.com/google/go-cmp"
                && package.version == "v0.6.0"
                && package.set == "go"
                && !package.is_git
                && !package.is_private));
        assert!(packages
            .iter()
            .any(|package| package.name == "github.com/bazelbuild/buildtools"
                && package.version == "v0.0.0-20250930140053-2eb4fccefb52"));
        assert!(packages
            .iter()
            .any(|package| package.name == "example.com/single" && package.version == "v1.2.3"));
        // The main module is first-party, never a dependency.
        assert!(!packages
            .iter()
            .any(|package| package.name == "rules_dx/third_party/go"));
    }

    #[test]
    fn go_mod_replace_paths_skip_while_versioned_replacements_assess() {
        let text = "module example.com/root\n\ngo 1.24.12\n\nrequire (\n\texample.com/local v1.0.0\n\texample.com/forked v1.0.0\n\texample.com/kept v1.0.0\n)\n\nreplace example.com/local => ../local\n\nreplace example.com/forked => example.com/upstream v1.1.0\n";
        let packages = parse_go_mod(text).expect("parses");
        assert!(!packages
            .iter()
            .any(|package| package.name == "example.com/local"));
        let forked = packages
            .iter()
            .find(|package| package.name == "example.com/upstream")
            .expect("versioned replacement assesses");
        assert_eq!(forked.version, "v1.1.0");
        assert!(packages
            .iter()
            .any(|package| package.name == "example.com/kept"));
    }

    #[test]
    fn go_mod_replace_block_paths_skip() {
        let text = "module example.com/root\n\ngo 1.24.12\n\nrequire example.com/local v1.0.0\n\nreplace (\n\texample.com/local => ./local\n)\n";
        let packages = parse_go_mod(text).expect("parses");
        assert!(packages.is_empty());
    }

    #[test]
    fn go_mod_ignores_non_dependency_directives_and_missing_module_fails() {
        let text = "module example.com/root\n\ngo 1.24.12\n\ntoolchain go1.24.12\n\nexclude example.com/bad v1.0.0\n\nretract v1.0.0-bad\n";
        let packages = parse_go_mod(text).expect("parses");
        assert!(packages.is_empty());
        let blocked = "module example.com/root\n\ngo 1.24.12\n\nexclude (\n\texample.com/bad v9.9.9\n)\n\nrequire example.com/kept v1.0.0\n";
        let packages = parse_go_mod(blocked).expect("parses");
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "example.com/kept");
        assert!(parse_go_mod("go 1.24.12\n").is_err());
        assert!(parse_go_mod("").is_err());
        assert!(parse_go_mod("module example.com/root\n)").is_err());
    }

    #[test]
    fn go_mod_comment_stripping_keeps_bare_tokens() {
        assert_eq!(strip_go_comment("// leading comment"), "");
        assert_eq!(
            strip_go_comment("require example.com/mod v1.2.3 // indirect"),
            "require example.com/mod v1.2.3"
        );
        assert_eq!(
            strip_go_comment("module example.com/root"),
            "module example.com/root"
        );
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
