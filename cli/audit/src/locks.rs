use std::sync::OnceLock;

use regex::Regex;

use crate::vuln::LockedPackage;

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
    let without_fragment = lower.split(['#', '?']).next().unwrap_or(&lower);
    if without_fragment.ends_with(".git") || without_fragment.contains(".git/") {
        return true;
    }
    false
}

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
        if key.contains("link:") || key.contains("file:") {
            continue;
        }
        if let Some((name, version)) = split_pnpm_key(&key) {
            if name.is_empty() || version.is_empty() {
                continue;
            }
            if version.starts_with("link:") || version.starts_with("file:") {
                continue;
            }
            let is_git = is_npm_git_reference(&key)
                || is_npm_git_reference(&version)
                || detail.get("resolution").is_some_and(pnpm_resolution_is_git);
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
        if !last.starts_with('@') {
            let segment = last.rsplit('/').next()?.trim();
            if segment.is_empty() {
                return None;
            }
            return Some(segment.to_owned());
        }
    }
    if last.contains('/') {
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

fn split_yarn_selector(selector: &str) -> Option<String> {
    let trimmed = selector.trim().trim_matches('"').trim();
    if trimmed.is_empty() || trimmed.starts_with("__metadata:") {
        return None;
    }
    if let Some((name, _)) = split_pnpm_key(trimmed) {
        if !name.is_empty() {
            return Some(name);
        }
        return None;
    }
    if !trimmed.contains('@') {
        let name = trimmed.trim_end_matches(':').trim().to_owned();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

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

fn scoped_pnpm_re() -> Option<&'static Regex> {
    static RE: OnceLock<Regex> = OnceLock::new();
    if let Some(compiled) = RE.get() {
        return Some(compiled);
    }
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
    match Regex::new(r"^(?P<name>.+)@(?P<version>[^@]+)$") {
        Ok(compiled) => {
            let _ = RE.set(compiled);
            RE.get()
        }
        Err(_) => None,
    }
}

fn split_pnpm_key(key: &str) -> Option<(String, String)> {
    let base = key
        .split_once('(')
        .map(|(stem, _)| stem)
        .unwrap_or(key)
        .trim();
    if base.is_empty() {
        return None;
    }
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
