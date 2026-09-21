//! Hermetic lockfile-consistency and declared-dependency usage checker.
//!
//! Owning contract: `docs/quality/quality-testing.md` (dependency-check truth
//! table, offline, non-mutating, exception/category/obsolete clauses).
//!
//! Why a direct port: the checker must keep CLI args, exit codes, and
//! diagnostic sentences stable for CI callers while running on all platforms
//! as a single native binary with no interpreter startup.
//! See: `tools/depcheck/BUILD.bazel` (rust targets).

#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Dependency declaration from a manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepInfo {
    pub spec: String,
    pub category: String,
    pub optional: bool,
    pub platform: bool,
    pub raw: String,
    pub peer: bool,
    pub sha256: String,
}

/// Narrow dependency-scoped exception with an explanatory reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exception {
    pub raw: String,
    pub reason: String,
}

/// Usage flags for one declaration across the owning scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Usage {
    pub src: bool,
    pub test: bool,
    pub build: bool,
}

/// Supported ecosystems (CLI `--ecosystem` values).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ecosystem {
    Rust,
    Python,
    Js,
    Ts,
    Go,
    Java,
    Kotlin,
    Scala,
    Csharp,
    Fsharp,
    Cc,
}

impl Ecosystem {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "rust" => Some(Self::Rust),
            "python" => Some(Self::Python),
            "js" => Some(Self::Js),
            "ts" => Some(Self::Ts),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "kotlin" => Some(Self::Kotlin),
            "scala" => Some(Self::Scala),
            "csharp" => Some(Self::Csharp),
            "fsharp" => Some(Self::Fsharp),
            "cc" => Some(Self::Cc),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::Js => "js",
            Self::Ts => "ts",
            Self::Go => "go",
            Self::Java => "java",
            Self::Kotlin => "kotlin",
            Self::Scala => "scala",
            Self::Csharp => "csharp",
            Self::Fsharp => "fsharp",
            Self::Cc => "cc",
        }
    }
}

pub fn normalize_py(name: &str) -> String {
    name.to_lowercase().replace(['-', '.'], "_")
}

pub fn normalize_js(name: &str) -> String {
    name.to_lowercase()
}

pub fn normalize_rs(name: &str) -> String {
    name.to_lowercase().replace('-', "_")
}

pub fn normalize_go(name: &str) -> String {
    name.to_lowercase()
}

pub fn normalize_jvm(name: &str) -> String {
    name.to_lowercase()
}

pub fn normalize_dotnet(name: &str) -> String {
    name.to_lowercase()
}

pub fn normalize_cc(name: &str) -> String {
    name.to_lowercase().replace('-', "_")
}

fn split_version_parts(value: &str, count: usize) -> Vec<String> {
    value
        .split(['.', '-'])
        .take(count)
        .map(|s| s.to_owned())
        .collect()
}

fn version_tuple(value: &str) -> Vec<i64> {
    let base = value.split('+').next().unwrap_or(value);
    base.split(['.', '-'])
        .take(3)
        .map(|part| {
            let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse::<i64>().unwrap_or(0)
        })
        .collect()
}

fn is_full_version(text: &str) -> bool {
    let mut parts = text.split(['.', '-']);
    let (Some(a), Some(b), Some(c)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    !a.is_empty()
        && !b.is_empty()
        && !c.is_empty()
        && a.chars().all(|c| c.is_ascii_digit())
        && b.chars().all(|c| c.is_ascii_digit())
        && c.chars().next().is_some_and(|c| c.is_ascii_digit())
}

/// Minimal semver compatibility for fixtures (no registry query).
pub fn satisfies(spec: &str, locked: &str) -> bool {
    let mut s = spec
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .trim()
        .to_owned();
    if let Some(first) = s.split(';').next() {
        s = first.trim().to_owned();
    }
    if s.len() > 1
        && s.starts_with('v')
        && s[1..].chars().next().is_some_and(|c| c.is_ascii_digit())
    {
        s = s[1..].to_owned();
    }
    let locked_trim = locked.trim().to_owned();
    let locked_norm = if locked_trim.len() > 1
        && locked_trim.starts_with('v')
        && locked_trim[1..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
    {
        locked_trim[1..].to_owned()
    } else {
        locked_trim
    };
    let mut exact = false;
    if let Some(rest) = s.strip_prefix("==") {
        s = rest.trim().to_owned();
        exact = true;
    } else if let Some(rest) = s.strip_prefix('=') {
        s = rest.trim().to_owned();
        exact = true;
    } else if let Some(rest) = s.strip_prefix('^') {
        s = rest.trim().to_owned();
    } else if let Some(rest) = s.strip_prefix('~') {
        s = rest.trim().to_owned();
        let lv = locked_norm
            .split('+')
            .next()
            .unwrap_or(&locked_norm)
            .to_owned();
        let sp = split_version_parts(&s, 2);
        let lp = split_version_parts(&lv, 2);
        if sp.len() == 2 && lp.len() == 2 {
            let parse_pair = |pair: &[String]| -> Option<(i64, i64)> {
                let a = pair[0].parse::<i64>().ok()?;
                let b = pair[1].parse::<i64>().ok()?;
                Some((a, b))
            };
            if let (Some(a), Some(b)) = (parse_pair(&sp), parse_pair(&lp)) {
                return a == b;
            }
            return s == lv;
        }
        return s == lv;
    } else if s.starts_with(">=") || s.starts_with("<=") {
        let op = s[..2].to_owned();
        let mut rest = s[2..].trim().to_owned();
        if let Some(first) = rest.split(',').next() {
            rest = first.trim().to_owned();
        }
        let locked_t = version_tuple(&locked_norm);
        let want_t = version_tuple(&rest);
        if op == ">=" {
            return locked_t >= want_t;
        }
        return locked_t <= want_t;
    }
    if s == "*" || s.is_empty() {
        return true;
    }
    if exact {
        return locked_norm.trim() == s;
    }
    if is_full_version(&s) {
        return locked_norm.trim() == s;
    }
    let smajor = s.split('.').next().unwrap_or("").trim().to_owned();
    let lmajor = locked_norm
        .split('.')
        .next()
        .unwrap_or("")
        .trim()
        .to_owned();
    if !smajor.is_empty() && smajor.chars().all(|c| c.is_ascii_digit()) {
        return smajor == lmajor;
    }
    smajor == lmajor
}

fn toml_string(value: &toml::Value) -> Option<String> {
    match value {
        toml::Value::String(s) => Some(s.clone()),
        toml::Value::Integer(i) => Some(i.to_string()),
        toml::Value::Float(f) => Some(f.to_string()),
        toml::Value::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
}

fn toml_bool(value: &toml::Value) -> bool {
    value.as_bool().unwrap_or(false)
}

fn parse_toml_file(path: &Path) -> Result<toml::Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("unreadable manifest: {e}"))?;
    toml::from_str(&text).map_err(|e| format!("unreadable manifest: {e}"))
}

/// Parse Rust `Cargo.toml` declarations.
pub fn parse_rust_manifest(path: &Path) -> Result<BTreeMap<String, DepInfo>, String> {
    let data = parse_toml_file(path)?;
    let mut deps = BTreeMap::new();
    for (cat, key) in [
        ("prod", "dependencies"),
        ("dev", "dev-dependencies"),
        ("build", "build-dependencies"),
    ] {
        if let Some(tbl) = data.get(key).and_then(|v| v.as_table()) {
            for (name, val) in tbl {
                let (spec, optional) = match val {
                    toml::Value::String(s) => (s.clone(), false),
                    toml::Value::Table(t) => {
                        let spec = t
                            .get("version")
                            .and_then(toml_string)
                            .unwrap_or_else(|| "*".to_owned());
                        let optional = t.get("optional").map(toml_bool).unwrap_or(false);
                        (spec, optional)
                    }
                    _ => ("*".to_owned(), false),
                };
                deps.insert(
                    name.to_lowercase(),
                    DepInfo {
                        spec,
                        category: cat.to_owned(),
                        optional,
                        platform: false,
                        raw: name.clone(),
                        peer: false,
                        sha256: String::new(),
                    },
                );
            }
        }
    }
    if let Some(target) = data.get("target").and_then(|v| v.as_table()) {
        for tval in target.values() {
            let Some(tmap) = tval.as_table() else {
                continue;
            };
            for sub in ["dependencies", "dev-dependencies", "build-dependencies"] {
                let Some(tbl) = tmap.get(sub).and_then(|v| v.as_table()) else {
                    continue;
                };
                let cat = if sub == "dependencies" {
                    "prod"
                } else if sub.starts_with("dev") {
                    "dev"
                } else {
                    "build"
                };
                for (name, val) in tbl {
                    let spec = match val {
                        toml::Value::String(s) => s.clone(),
                        toml::Value::Table(t) => t
                            .get("version")
                            .and_then(toml_string)
                            .unwrap_or_else(|| "*".to_owned()),
                        _ => "*".to_owned(),
                    };
                    deps.insert(
                        name.to_lowercase(),
                        DepInfo {
                            spec,
                            category: cat.to_owned(),
                            optional: false,
                            platform: true,
                            raw: name.clone(),
                            peer: false,
                            sha256: String::new(),
                        },
                    );
                }
            }
        }
    }
    Ok(deps)
}

/// Parse Rust `Cargo.lock` packages.
pub fn parse_rust_lock(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("unreadable lock: {e}"))?;
    let data: toml::Value = toml::from_str(&text).map_err(|e| format!("unreadable lock: {e}"))?;
    let mut pkgs = BTreeMap::new();
    if let Some(list) = data.get("package").and_then(|v| v.as_array()) {
        for item in list {
            let name = item
                .get("name")
                .and_then(toml_string)
                .unwrap_or_default()
                .to_lowercase();
            let ver = item
                .get("version")
                .and_then(toml_string)
                .unwrap_or_default();
            if !name.is_empty() {
                pkgs.insert(name, ver);
            }
        }
    }
    Ok(pkgs)
}

fn python_spec_from_rest(rest: &str) -> String {
    let mut spec = String::new();
    for ch in rest.chars() {
        if matches!(
            ch,
            '=' | '<' | '>' | '^' | '~' | '!' | '.' | ',' | '*' | ' ' | '\t'
        ) || ch.is_ascii_digit()
        {
            spec.push(ch);
        } else {
            break;
        }
    }
    let trimmed = spec.trim().to_owned();
    if trimmed.is_empty() {
        "*".to_owned()
    } else {
        trimmed
    }
}

fn python_raw_name(item: &str) -> Option<(String, String)> {
    let item = item.trim();
    if item.is_empty() {
        return None;
    }
    let mut end = 0usize;
    for (idx, ch) in item.char_indices() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '[' | ']') {
            end = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    let mut head = item[..end].to_owned();
    // Strip extras `[extra]` from the head for the raw name.
    if let Some(bracket) = head.find('[') {
        head = head[..bracket].to_owned();
    }
    if head.is_empty() {
        return None;
    }
    let rest = item[end..].trim().to_owned();
    Some((head, rest))
}

/// Parse Python `pyproject.toml` declarations.
pub fn parse_python_manifest(path: &Path) -> Result<BTreeMap<String, DepInfo>, String> {
    let data = parse_toml_file(path)?;
    let mut deps = BTreeMap::new();
    if let Some(proj) = data.get("project").and_then(|v| v.as_table()) {
        if let Some(list) = proj.get("dependencies").and_then(|v| v.as_array()) {
            for item in list {
                let Some(text) = item.as_str() else { continue };
                let Some((rawname, rest)) = python_raw_name(text) else {
                    continue;
                };
                let marker_platform = rest.contains("sys_platform")
                    || rest.contains("sys-platform")
                    || rest.contains("platform_system")
                    || rest.contains("os_name");
                let spec = python_spec_from_rest(&rest);
                deps.insert(
                    normalize_py(&rawname),
                    DepInfo {
                        spec,
                        category: "prod".to_owned(),
                        optional: false,
                        platform: marker_platform,
                        raw: rawname,
                        peer: false,
                        sha256: String::new(),
                    },
                );
            }
        }
        if let Some(opt) = proj.get("optional-dependencies").and_then(|v| v.as_table()) {
            for items in opt.values() {
                let Some(list) = items.as_array() else {
                    continue;
                };
                for item in list {
                    let Some(text) = item.as_str() else { continue };
                    let Some((rawname, rest)) = python_raw_name(text) else {
                        continue;
                    };
                    let spec = python_spec_from_rest(&rest);
                    deps.insert(
                        normalize_py(&rawname),
                        DepInfo {
                            spec,
                            category: "dev".to_owned(),
                            optional: true,
                            platform: false,
                            raw: rawname,
                            peer: false,
                            sha256: String::new(),
                        },
                    );
                }
            }
        }
    }
    if let Some(groups) = data.get("dependency-groups").and_then(|v| v.as_table()) {
        for items in groups.values() {
            let Some(list) = items.as_array() else {
                continue;
            };
            for item in list {
                let Some(text) = item.as_str() else { continue };
                let Some((rawname, rest)) = python_raw_name(text) else {
                    continue;
                };
                let spec = python_spec_from_rest(&rest);
                deps.insert(
                    normalize_py(&rawname),
                    DepInfo {
                        spec,
                        category: "dev".to_owned(),
                        optional: false,
                        platform: false,
                        raw: rawname,
                        peer: false,
                        sha256: String::new(),
                    },
                );
            }
        }
    }
    Ok(deps)
}

/// Parse Python `uv.lock` packages.
pub fn parse_python_lock(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("unreadable lock: {e}"))?;
    let data: toml::Value = toml::from_str(&text).map_err(|e| format!("unreadable lock: {e}"))?;
    let mut pkgs = BTreeMap::new();
    if let Some(list) = data.get("package").and_then(|v| v.as_array()) {
        for item in list {
            let name = item.get("name").and_then(toml_string).unwrap_or_default();
            let ver = item
                .get("version")
                .and_then(toml_string)
                .unwrap_or_default();
            if !name.is_empty() {
                pkgs.insert(normalize_py(&name), ver);
            }
        }
    }
    if pkgs.is_empty() {
        let re = regex::Regex::new(r#"name\s*=\s*"([^"]+)"\s*\n\s*version\s*=\s*"([^"]+)""#)
            .map_err(|e| format!("unreadable lock: {e}"))?;
        for caps in re.captures_iter(&text) {
            pkgs.insert(normalize_py(&caps[1]), caps[2].to_owned());
        }
    }
    Ok(pkgs)
}

/// Parse JS/TS `package.json` declarations.
pub fn parse_js_manifest(path: &Path) -> Result<BTreeMap<String, DepInfo>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("unreadable manifest: {e}"))?;
    let data: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("unreadable manifest: {e}"))?;
    let mut deps = BTreeMap::new();
    if let Some(map) = data.get("dependencies").and_then(|v| v.as_object()) {
        for (name, spec) in map {
            deps.insert(
                normalize_js(name),
                DepInfo {
                    spec: spec.as_str().unwrap_or("").to_owned(),
                    category: "prod".to_owned(),
                    optional: false,
                    platform: false,
                    raw: name.clone(),
                    peer: false,
                    sha256: String::new(),
                },
            );
        }
    }
    if let Some(map) = data.get("devDependencies").and_then(|v| v.as_object()) {
        for (name, spec) in map {
            deps.insert(
                normalize_js(name),
                DepInfo {
                    spec: spec.as_str().unwrap_or("").to_owned(),
                    category: "dev".to_owned(),
                    optional: false,
                    platform: false,
                    raw: name.clone(),
                    peer: false,
                    sha256: String::new(),
                },
            );
        }
    }
    if let Some(map) = data.get("optionalDependencies").and_then(|v| v.as_object()) {
        for (name, spec) in map {
            deps.insert(
                normalize_js(name),
                DepInfo {
                    spec: spec.as_str().unwrap_or("").to_owned(),
                    category: "prod".to_owned(),
                    optional: true,
                    platform: true,
                    raw: name.clone(),
                    peer: false,
                    sha256: String::new(),
                },
            );
        }
    }
    if let Some(map) = data.get("peerDependencies").and_then(|v| v.as_object()) {
        for (name, spec) in map {
            deps.insert(
                normalize_js(name),
                DepInfo {
                    spec: spec.as_str().unwrap_or("").to_owned(),
                    category: "prod".to_owned(),
                    optional: false,
                    platform: false,
                    raw: name.clone(),
                    peer: true,
                    sha256: String::new(),
                },
            );
        }
    }
    Ok(deps)
}

/// Minimal `pnpm-lock.yaml` parser for fixtures (no yaml dep).
pub fn parse_pnpm_lock(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("unreadable lock: {e}"))?;
    let section_re =
        regex::Regex::new(r"^\S+:\s*$").map_err(|e| format!("unreadable lock: {e}"))?;
    let mut pkgs = BTreeMap::new();
    let mut in_packages = false;
    for line in text.lines() {
        if line.starts_with("packages:") && line.trim() == "packages:" {
            in_packages = true;
            continue;
        }
        if in_packages
            && !line.starts_with(' ')
            && !line.starts_with('\t')
            && line.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && !line.trim_start().starts_with('\'')
            && !line.trim_start().starts_with('"')
            && section_re.is_match(line)
        {
            in_packages = false;
            continue;
        }
        if !in_packages {
            continue;
        }
        let trimmed = line.trim().trim_matches(|c| c == '\'' || c == '"');
        let Some(colon) = trimmed.find(':') else {
            continue;
        };
        let raw = trimmed[..colon]
            .trim()
            .trim_matches(|c| c == '\'' || c == '"');
        let Some(at) = raw.rfind('@') else {
            continue;
        };
        if at == 0 {
            continue;
        }
        let name = raw[..at]
            .trim()
            .trim_matches(|c| c == '\'' || c == '"' || c == ' ');
        let mut ver = raw[at + 1..]
            .split(':')
            .next()
            .unwrap_or("")
            .trim()
            .to_owned();
        if let Some(paren) = ver.find('(') {
            ver = ver[..paren].to_owned();
        }
        ver = ver
            .trim_matches(|c| c == '\'' || c == '"' || c == ' ' || c == '(' || c == ')')
            .to_owned();
        if name.is_empty() || ver.is_empty() {
            continue;
        }
        pkgs.insert(normalize_js(name), ver);
    }
    Ok(pkgs)
}

/// Parse `go.mod` requires (fixture subset with depcheck markers).
pub fn parse_go_manifest(path: &Path) -> Result<BTreeMap<String, DepInfo>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("unreadable manifest: {e}"))?;
    let mut deps = BTreeMap::new();
    let mut in_require = false;
    let single_re = regex::Regex::new(r"^require\s+(\S+)\s+(\S+)(.*)$")
        .map_err(|e| format!("unreadable manifest: {e}"))?;
    let block_re = regex::Regex::new(r"^(\S+)\s+(\S+)(.*)$")
        .map_err(|e| format!("unreadable manifest: {e}"))?;
    let optional_re = regex::Regex::new(r"(?i)//\s*optional\b")
        .map_err(|e| format!("unreadable manifest: {e}"))?;
    let platform_re = regex::Regex::new(r"(?i)//\s*platform\b")
        .map_err(|e| format!("unreadable manifest: {e}"))?;
    let test_re =
        regex::Regex::new(r"(?i)//\s*test\b").map_err(|e| format!("unreadable manifest: {e}"))?;
    for rawline in text.lines() {
        let line = rawline.trim().to_owned();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("//") && !line.starts_with("// depcheck") {
            if line.starts_with("module ") || line.starts_with("go ") {
                continue;
            }
            if line.starts_with("//") {
                continue;
            }
        }
        if line.starts_with("require (") {
            in_require = true;
            continue;
        }
        if in_require && line == ")" {
            in_require = false;
            continue;
        }
        let caps = if line.starts_with("require ") && !in_require {
            single_re.captures(&line)
        } else if in_require {
            block_re.captures(&line)
        } else {
            None
        };
        let Some(caps) = caps else { continue };
        let module = caps[1].to_owned();
        let ver = caps[2].to_owned();
        let rest = caps
            .get(3)
            .map(|m| m.as_str().to_owned())
            .unwrap_or_default();
        let mut category = "prod".to_owned();
        let mut optional = false;
        let mut platform = false;
        let low = rest.to_lowercase();
        if low.contains("depcheck:test") {
            category = "dev".to_owned();
        }
        if low.contains("depcheck:optional") {
            optional = true;
        }
        if low.contains("depcheck:platform") {
            platform = true;
        }
        if optional_re.is_match(&rest) {
            optional = true;
        }
        if platform_re.is_match(&rest) {
            platform = true;
        }
        if test_re.is_match(&rest) {
            category = "dev".to_owned();
        }
        deps.insert(
            normalize_go(&module),
            DepInfo {
                spec: ver,
                category,
                optional,
                platform,
                raw: module,
                peer: false,
                sha256: String::new(),
            },
        );
    }
    Ok(deps)
}

/// Parse `go.sum` (native lock).
pub fn parse_go_lock(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("unreadable lock: {e}"))?;
    let mut pkgs = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let module = parts[0].to_owned();
        let mut ver = parts[1].to_owned();
        if let Some(stripped) = ver.strip_suffix("/go.mod") {
            ver = stripped.to_owned();
        }
        let key = normalize_go(&module);
        pkgs.entry(key).or_insert(ver);
    }
    Ok(pkgs)
}

/// Parse `jvm_deps.toml` fixture manifest.
pub fn parse_jvm_manifest(path: &Path) -> Result<BTreeMap<String, DepInfo>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("unreadable manifest: {e}"))?;
    let data: toml::Value =
        toml::from_str(&text).map_err(|e| format!("unreadable manifest: {e}"))?;
    let mut deps = BTreeMap::new();
    let items = data
        .get("dep")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for item in items {
        let grp = item
            .get("group")
            .and_then(toml_string)
            .unwrap_or_default()
            .trim()
            .to_owned();
        let art = item
            .get("artifact")
            .and_then(toml_string)
            .unwrap_or_default()
            .trim()
            .to_owned();
        let ver = item
            .get("version")
            .and_then(toml_string)
            .unwrap_or_else(|| "*".to_owned())
            .trim()
            .to_owned();
        if grp.is_empty() || art.is_empty() {
            return Err("jvm dep entry without group/artifact".to_owned());
        }
        let scope = item
            .get("scope")
            .and_then(toml_string)
            .unwrap_or_else(|| "compile".to_owned())
            .trim()
            .to_lowercase();
        let category = if scope == "test" || scope == "dev" {
            "dev"
        } else {
            "prod"
        };
        let key = normalize_jvm(&format!("{grp}:{art}"));
        deps.insert(
            key,
            DepInfo {
                spec: ver,
                category: category.to_owned(),
                optional: item.get("optional").map(toml_bool).unwrap_or(false),
                platform: item.get("platform").map(toml_bool).unwrap_or(false),
                raw: format!("{grp}:{art}"),
                peer: false,
                sha256: String::new(),
            },
        );
    }
    Ok(deps)
}

/// Parse `maven_install.json` (native lock).
pub fn parse_jvm_lock(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("unreadable lock: {e}"))?;
    let data: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("unreadable lock: {e}"))?;
    let mut pkgs = BTreeMap::new();
    if let Some(arts) = data.get("artifacts").and_then(|v| v.as_object()) {
        for (coord, info) in arts {
            let ver = info
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            pkgs.insert(normalize_jvm(coord), ver);
        }
    }
    Ok(pkgs)
}

/// Parse `paket.dependencies` (native Paket subset).
pub fn parse_dotnet_manifest(path: &Path) -> Result<BTreeMap<String, DepInfo>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("unreadable manifest: {e}"))?;
    let mut deps = BTreeMap::new();
    let mut group = "Main".to_owned();
    let group_re = regex::Regex::new(r"(?i)^group\s+(\S+)")
        .map_err(|e| format!("unreadable manifest: {e}"))?;
    let nuget_re = regex::Regex::new(r"(?i)^nuget\s+(\S+)\s+(\S+)(.*)$")
        .map_err(|e| format!("unreadable manifest: {e}"))?;
    let optional_re = regex::Regex::new(r"(?i)//\s*optional\b")
        .map_err(|e| format!("unreadable manifest: {e}"))?;
    let platform_re = regex::Regex::new(r"(?i)//\s*platform\b")
        .map_err(|e| format!("unreadable manifest: {e}"))?;
    for rawline in text.lines() {
        let line = rawline.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let low = line.to_lowercase();
        if low.starts_with("source ") || low.starts_with("framework:") {
            continue;
        }
        if let Some(caps) = group_re.captures(line) {
            group = caps[1].to_owned();
            continue;
        }
        let Some(caps) = nuget_re.captures(line) else {
            continue;
        };
        let name = caps[1].to_owned();
        let ver = caps[2].to_owned();
        let rest = caps.get(3).map(|m| m.as_str()).unwrap_or("");
        let category = if ["main", "prod", "compile"].contains(&group.to_lowercase().as_str()) {
            "prod"
        } else {
            "dev"
        };
        deps.insert(
            normalize_dotnet(&name),
            DepInfo {
                spec: ver,
                category: category.to_owned(),
                optional: optional_re.is_match(rest),
                platform: platform_re.is_match(rest),
                raw: name,
                peer: false,
                sha256: String::new(),
            },
        );
    }
    Ok(deps)
}

/// Parse `paket.lock` (native).
pub fn parse_dotnet_lock(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("unreadable lock: {e}"))?;
    let re = regex::Regex::new(r"^\s*([A-Za-z0-9_.\-]+)\s+\(([^)]+)\)")
        .map_err(|e| format!("unreadable lock: {e}"))?;
    let mut pkgs = BTreeMap::new();
    for line in text.lines() {
        if let Some(caps) = re.captures(line) {
            pkgs.insert(normalize_dotnet(&caps[1]), caps[2].trim().to_owned());
        }
    }
    Ok(pkgs)
}

/// Parse `cc_deps.toml` fixture manifest (sha256 authority).
pub fn parse_cc_manifest(path: &Path) -> Result<BTreeMap<String, DepInfo>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("unreadable manifest: {e}"))?;
    let data: toml::Value =
        toml::from_str(&text).map_err(|e| format!("unreadable manifest: {e}"))?;
    let mut deps = BTreeMap::new();
    let items = data
        .get("dep")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for item in items {
        let name = item
            .get("name")
            .and_then(toml_string)
            .unwrap_or_default()
            .trim()
            .to_owned();
        let ver = item
            .get("version")
            .and_then(toml_string)
            .unwrap_or_default()
            .trim()
            .to_owned();
        let sha = item
            .get("sha256")
            .and_then(toml_string)
            .unwrap_or_default()
            .trim()
            .to_owned();
        if name.is_empty() {
            return Err("cc dep entry without name".to_owned());
        }
        let scope = item
            .get("scope")
            .and_then(toml_string)
            .unwrap_or_else(|| "prod".to_owned())
            .trim()
            .to_lowercase();
        let category = if scope == "test" || scope == "dev" {
            "dev"
        } else {
            "prod"
        };
        deps.insert(
            normalize_cc(&name),
            DepInfo {
                spec: if ver.is_empty() { "*".to_owned() } else { ver },
                category: category.to_owned(),
                optional: item.get("optional").map(toml_bool).unwrap_or(false),
                platform: item.get("platform").map(toml_bool).unwrap_or(false),
                sha256: sha,
                raw: name,
                peer: false,
            },
        );
    }
    Ok(deps)
}

/// Parse `cc_lock.json`.
pub fn parse_cc_lock(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("unreadable lock: {e}"))?;
    let data: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("unreadable lock: {e}"))?;
    let mut pkgs = BTreeMap::new();
    if let Some(map) = data.get("packages").and_then(|v| v.as_object()) {
        for (name, info) in map {
            let ver = info
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            pkgs.insert(normalize_cc(name), ver);
        }
    }
    Ok(pkgs)
}

/// CC lock sha256 map (hash authority side channel).
pub fn parse_cc_lock_sha(path: &Path) -> BTreeMap<String, String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let Ok(data): Result<serde_json::Value, _> = serde_json::from_str(&text) else {
        return BTreeMap::new();
    };
    let mut out = BTreeMap::new();
    if let Some(map) = data.get("packages").and_then(|v| v.as_object()) {
        for (name, info) in map {
            let sha = info
                .get("sha256")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            out.insert(normalize_cc(name), sha);
        }
    }
    out
}

/// Parse `depcheck_exceptions.toml` (raw key uses lower plus dash-to-underscore).
pub fn parse_exceptions(path: Option<&Path>) -> Result<BTreeMap<String, Exception>, String> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    if !path.exists() {
        return Err(format!("exceptions file missing: {}", path.display()));
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("unreadable exceptions: {e}"))?;
    let data: toml::Value =
        toml::from_str(&text).map_err(|e| format!("unreadable exceptions: {e}"))?;
    let mut out = BTreeMap::new();
    let items = data
        .get("exception")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for item in items {
        let name = item
            .get("dependency")
            .and_then(toml_string)
            .unwrap_or_default()
            .trim()
            .to_owned();
        let reason = item
            .get("reason")
            .and_then(toml_string)
            .unwrap_or_default()
            .trim()
            .to_owned();
        if name.is_empty() {
            return Err("exception entry without dependency".to_owned());
        }
        out.insert(
            name.to_lowercase().replace('-', "_"),
            Exception { raw: name, reason },
        );
    }
    Ok(out)
}

fn normalize_exception_key(eco: Ecosystem, raw: &str) -> String {
    match eco {
        Ecosystem::Python => normalize_py(raw),
        Ecosystem::Rust => raw.to_lowercase(),
        Ecosystem::Go => normalize_go(raw),
        Ecosystem::Java | Ecosystem::Kotlin | Ecosystem::Scala => normalize_jvm(raw),
        Ecosystem::Csharp | Ecosystem::Fsharp => normalize_dotnet(raw),
        Ecosystem::Cc => normalize_cc(raw),
        Ecosystem::Js | Ecosystem::Ts => normalize_js(raw),
    }
}

fn collect_sources(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut children: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            children.push(entry.path());
        }
        children.sort();
        for child in children.into_iter().rev() {
            if child.is_dir() {
                stack.push(child);
            } else if child.is_file() {
                out.push(child);
            }
        }
    }
    out.sort();
    out
}

/// Whether a path is a test file for the ecosystem.
pub fn is_test_file(eco: Ecosystem, path: &Path) -> bool {
    let s = path.to_string_lossy().replace('\\', "/");
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_owned();
    match eco {
        Ecosystem::Rust => {
            s.contains("/tests/") || s.ends_with("_test.rs") || name.contains("test")
        }
        Ecosystem::Python => {
            name.starts_with("test_") || name.ends_with("_test.py") || s.contains("/tests/")
        }
        Ecosystem::Go => {
            name.ends_with("_test.go") || s.contains("/tests/") || s.contains("/test/")
        }
        Ecosystem::Java | Ecosystem::Kotlin | Ecosystem::Scala => {
            name.contains("Test") || s.to_lowercase().contains("/test/") || s.contains("/tests/")
        }
        Ecosystem::Csharp | Ecosystem::Fsharp => {
            name.contains("Test") || s.to_lowercase().contains("/test/") || s.contains("/tests/")
        }
        Ecosystem::Cc => {
            name.to_lowercase().contains("test")
                || s.to_lowercase().contains("/test/")
                || s.contains("/tests/")
        }
        Ecosystem::Js | Ecosystem::Ts => {
            name.ends_with(".test.js")
                || name.ends_with(".test.ts")
                || s.contains("/__tests__/")
                || s.contains("/tests/")
        }
    }
}

fn source_suffixes() -> BTreeSet<&'static str> {
    [
        ".rs", ".py", ".js", ".ts", ".mjs", ".cjs", ".jsx", ".tsx", ".go", ".java", ".kt", ".kts",
        ".scala", ".cs", ".fs", ".fsi", ".fsx", ".cc", ".cpp", ".cxx", ".c", ".h", ".hpp",
    ]
    .into_iter()
    .collect()
}

fn skip_names() -> BTreeSet<&'static str> {
    [
        "Cargo.toml",
        "Cargo.lock",
        "pyproject.toml",
        "uv.lock",
        "package.json",
        "pnpm-lock.yaml",
        "go.mod",
        "go.sum",
        "jvm_deps.toml",
        "maven_install.json",
        "paket.dependencies",
        "paket.lock",
        "cc_deps.toml",
        "cc_lock.json",
        "depcheck_exceptions.toml",
    ]
    .into_iter()
    .collect()
}

fn regex_escape(text: &str) -> String {
    regex::escape(text)
}

/// Usage across the owning scope (all sources under `sources_root`).
pub fn find_usages(
    eco: Ecosystem,
    sources_root: &Path,
    dep_names: &[String],
) -> Result<BTreeMap<String, Usage>, String> {
    let mut out: BTreeMap<String, Usage> = dep_names
        .iter()
        .map(|k| (k.clone(), Usage::default()))
        .collect();
    let files = collect_sources(sources_root);
    let skips = skip_names();
    let suffixes = source_suffixes();
    let mut texts: Vec<(PathBuf, String)> = Vec::new();
    for file in files {
        let Some(fname) = file.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if skips.contains(fname) {
            continue;
        }
        let has_suffix = suffixes.iter().any(|s| fname.ends_with(s));
        // Python skips non-source extensions entirely (manifests/locks are
        // not sources); mirror by requiring a known source suffix.
        let ext = Path::new(fname)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let dotted = format!(".{ext}");
        if !has_suffix && !suffixes.contains(dotted.as_str()) {
            // Also accept files like `hello.go` via suffix check above;
            // anything else is not a source.
            let mut matched = false;
            for suffix in suffixes.iter() {
                if fname.ends_with(suffix) {
                    matched = true;
                    break;
                }
            }
            if !matched {
                continue;
            }
        }
        let Ok(text) = std::fs::read(&file) else {
            continue;
        };
        let text = String::from_utf8_lossy(&text).into_owned();
        texts.push((file, text));
    }
    for dep in dep_names {
        let patterns: Vec<String> = match eco {
            Ecosystem::Rust => {
                let cname = dep.replace('-', "_");
                vec![
                    format!(r"\buse\s+{}\b", regex_escape(&cname)),
                    format!(r"\bextern\s+crate\s+{}\b", regex_escape(&cname)),
                    format!(r"\b{}\s*::", regex_escape(&cname)),
                ]
            }
            Ecosystem::Python => {
                let raw_dash = dep.replace('_', "-");
                vec![
                    format!(r"(?m)^\s*import\s+{}\b", regex_escape(dep)),
                    format!(r"(?m)^\s*from\s+{}\b", regex_escape(dep)),
                    format!(r"(?m)^\s*import\s+{}\b", regex_escape(&raw_dash)),
                    format!(r"(?m)^\s*from\s+{}\b", regex_escape(&raw_dash)),
                ]
            }
            Ecosystem::Go => vec![
                format!(r#"import\s+(?:\(\s*)?["']{}["']"#, regex_escape(dep)),
                format!(r#"["']{}(?:/[^"']*)?["']"#, regex_escape(dep)),
            ],
            Ecosystem::Java | Ecosystem::Kotlin | Ecosystem::Scala => {
                let art = dep.split(':').next_back().unwrap_or(dep).to_owned();
                let art_dash = art.replace(['-', '.'], "_");
                vec![
                    format!(r"(?m)^\s*import\s+.*{}\b", regex_escape(&art)),
                    format!(r"(?m)^\s*import\s+.*{}\b", regex_escape(&art_dash)),
                ]
            }
            Ecosystem::Csharp | Ecosystem::Fsharp => {
                let base = dep.split('.').next_back().unwrap_or(dep).to_owned();
                vec![
                    format!(r"(?m)^\s*(using|open)\s+.*{}\b", regex_escape(dep)),
                    format!(r"(?m)^\s*(using|open)\s+.*{}\b", regex_escape(&base)),
                ]
            }
            Ecosystem::Cc => {
                let cname = dep.replace('-', "_");
                vec![
                    format!(r#"#\s*include\s+[<"'].*{}.*[>"']"#, regex_escape(dep)),
                    format!(r#"#\s*include\s+[<"'].*{}.*[>"']"#, regex_escape(&cname)),
                ]
            }
            Ecosystem::Js | Ecosystem::Ts => vec![
                format!(r#"from\s+['"]{}['"]"#, regex_escape(dep)),
                format!(r#"require\(\s*['"]{}['"]\s*\)"#, regex_escape(dep)),
                format!(r#"import\(\s*['"]{}['"]\s*\)"#, regex_escape(dep)),
            ],
        };
        let mut compiled = Vec::new();
        for pat in patterns {
            let re = if matches!(eco, Ecosystem::Csharp | Ecosystem::Fsharp) {
                regex::Regex::new(&format!("(?i){pat}"))
            } else {
                regex::Regex::new(&pat)
            }
            .map_err(|e| format!("unreadable sources: {e}"))?;
            compiled.push(re);
        }
        for (path, text) in &texts {
            if !compiled.iter().any(|re| re.is_match(text)) {
                continue;
            }
            let entry = out.get_mut(dep);
            let Some(entry) = entry else { continue };
            if path.file_name().and_then(|n| n.to_str()) == Some("build.rs") {
                entry.build = true;
            } else if is_test_file(eco, path) {
                entry.test = true;
            } else {
                entry.src = true;
            }
        }
    }
    Ok(out)
}

fn load_manifest(eco: Ecosystem, manifest: &Path) -> Result<BTreeMap<String, DepInfo>, String> {
    match eco {
        Ecosystem::Rust => parse_rust_manifest(manifest),
        Ecosystem::Python => parse_python_manifest(manifest),
        Ecosystem::Js | Ecosystem::Ts => parse_js_manifest(manifest),
        Ecosystem::Go => parse_go_manifest(manifest),
        Ecosystem::Java | Ecosystem::Kotlin | Ecosystem::Scala => parse_jvm_manifest(manifest),
        Ecosystem::Csharp | Ecosystem::Fsharp => parse_dotnet_manifest(manifest),
        Ecosystem::Cc => parse_cc_manifest(manifest),
    }
}

fn load_lock(eco: Ecosystem, lock: &Path) -> Result<BTreeMap<String, String>, String> {
    match eco {
        Ecosystem::Rust => parse_rust_lock(lock),
        Ecosystem::Python => parse_python_lock(lock),
        Ecosystem::Js | Ecosystem::Ts => parse_pnpm_lock(lock),
        Ecosystem::Go => parse_go_lock(lock),
        Ecosystem::Java | Ecosystem::Kotlin | Ecosystem::Scala => parse_jvm_lock(lock),
        Ecosystem::Csharp | Ecosystem::Fsharp => parse_dotnet_lock(lock),
        Ecosystem::Cc => parse_cc_lock(lock),
    }
}

/// Verify manifest versus lock (offline, non-mutating). Returns exit code.
pub fn cmd_consistency(
    eco: Ecosystem,
    manifest: &Path,
    lock: &Path,
    stdout: &mut dyn std::fmt::Write,
    stderr: &mut dyn std::fmt::Write,
) -> i32 {
    if !manifest.exists() {
        let _ = writeln!(
            stderr,
            "depcheck: ERROR: manifest missing: {}",
            manifest.display()
        );
        return 2;
    }
    if !lock.exists() {
        let _ = writeln!(
            stderr,
            "depcheck: ERROR: lock missing: {} (declare lock inputs, do not skip)",
            lock.display()
        );
        return 2;
    }
    let mut deps = match load_manifest(eco, manifest) {
        Ok(deps) => deps,
        Err(err) => {
            let _ = writeln!(stderr, "depcheck: ERROR: {err}");
            return 2;
        }
    };
    if matches!(eco, Ecosystem::Js | Ecosystem::Ts) {
        deps.retain(|_, v| !v.peer);
    }
    let pkgs = match load_lock(eco, lock) {
        Ok(pkgs) => pkgs,
        Err(err) => {
            let _ = writeln!(stderr, "depcheck: ERROR: {err}");
            return 2;
        }
    };
    let mut failures = Vec::new();
    let cc_lock_sha = if eco == Ecosystem::Cc {
        parse_cc_lock_sha(lock)
    } else {
        BTreeMap::new()
    };
    for (name, info) in &deps {
        let mut locked = pkgs.get(name).cloned();
        if locked.is_none() {
            let alt = if name.contains('_') {
                name.replace('_', "-")
            } else {
                name.replace('-', "_")
            };
            locked = pkgs.get(&alt).cloned();
        }
        let Some(locked) = locked else {
            failures.push(format!(
                "missing lock entry for '{}' (manifest requires {})",
                info.raw, info.spec
            ));
            continue;
        };
        if !satisfies(&info.spec, &locked) {
            failures.push(format!(
                "stale lock entry for '{}': manifest requires {}, lock has {locked}",
                info.raw, info.spec
            ));
        } else if eco == Ecosystem::Cc {
            let want_sha = info.sha256.trim().to_owned();
            let got_sha = cc_lock_sha
                .get(name)
                .cloned()
                .unwrap_or_default()
                .trim()
                .to_owned();
            if want_sha.is_empty() {
                failures.push(format!(
                    "missing sha256 for '{}' (every http_archive carries sha256/integrity)",
                    info.raw
                ));
            } else if want_sha != got_sha {
                failures.push(format!(
                    "stale sha256 for '{}': manifest has {want_sha}, lock has {got_sha}",
                    info.raw
                ));
            }
        }
    }
    if !failures.is_empty() {
        for failure in &failures {
            let _ = writeln!(stderr, "depcheck: FAIL: consistency: {failure}");
        }
        return 1;
    }
    let _ = writeln!(
        stdout,
        "depcheck: OK: consistency: {} declarations match lock ({})",
        deps.len(),
        eco.name()
    );
    0
}

/// Verify declared deps are used in the owning scope. Returns exit code.
pub fn cmd_usage(
    eco: Ecosystem,
    manifest: &Path,
    sources: &Path,
    exceptions: Option<&Path>,
    stdout: &mut dyn std::fmt::Write,
    stderr: &mut dyn std::fmt::Write,
) -> i32 {
    if !manifest.exists() {
        let _ = writeln!(
            stderr,
            "depcheck: ERROR: manifest missing: {}",
            manifest.display()
        );
        return 2;
    }
    if !sources.exists() {
        let _ = writeln!(
            stderr,
            "depcheck: ERROR: sources missing: {}",
            sources.display()
        );
        return 2;
    }
    let mut deps = match load_manifest(eco, manifest) {
        Ok(deps) => deps,
        Err(err) => {
            let _ = writeln!(stderr, "depcheck: ERROR: {err}");
            return 2;
        }
    };
    if matches!(eco, Ecosystem::Js | Ecosystem::Ts) {
        deps.retain(|_, v| !v.peer);
    }
    let exc = match parse_exceptions(exceptions) {
        Ok(exc) => exc,
        Err(err) => {
            let _ = writeln!(stderr, "depcheck: ERROR: {err}");
            return 2;
        }
    };
    let mut norm_exc: BTreeMap<String, Exception> = BTreeMap::new();
    for value in exc.values() {
        norm_exc.insert(normalize_exception_key(eco, &value.raw), value.clone());
    }
    let dep_names: Vec<String> = deps.keys().cloned().collect();
    let usages = match find_usages(eco, sources, &dep_names) {
        Ok(usages) => usages,
        Err(err) => {
            let _ = writeln!(stderr, "depcheck: ERROR: {err}");
            return 2;
        }
    };
    let mut failures = Vec::new();
    for (key, value) in &norm_exc {
        if !deps.contains_key(key) {
            failures.push(format!(
                "obsolete exception for removed dependency '{}' (report only, not deleted)",
                value.raw
            ));
        }
    }
    for (name, info) in &deps {
        let usage = usages.get(name).copied().unwrap_or_default();
        let used_any = usage.src || usage.test || usage.build;
        let has_exc = norm_exc.contains_key(name);
        let reason = norm_exc
            .get(name)
            .map(|v| v.reason.clone())
            .unwrap_or_default();
        if has_exc && reason.is_empty() {
            failures.push(format!(
                "exception for '{}' missing reason (each exception needs an explanatory reason)",
                info.raw
            ));
            continue;
        }
        if has_exc && used_any {
            failures.push(format!(
                "obsolete exception for '{}' (usage recognized; remove the exception)",
                info.raw
            ));
            continue;
        }
        if has_exc && !used_any {
            continue;
        }
        if used_any {
            if info.category == "prod" && !usage.src && !usage.build && usage.test {
                failures.push(format!(
                    "category error: production declaration '{}' used only by tests (move to dev)",
                    info.raw
                ));
                continue;
            }
            if eco == Ecosystem::Rust
                && info.category == "prod"
                && !usage.src
                && usage.build
                && !usage.test
            {
                failures.push(format!(
                    "category error: production declaration '{}' used only by build tooling (move to build-dependencies)",
                    info.raw
                ));
                continue;
            }
            continue;
        }
        if info.optional || info.platform {
            let kind = if info.optional {
                "optional"
            } else {
                "platform-specific"
            };
            failures.push(format!(
                "unused {kind} declaration '{}' (optional/platform is not proof of usage)",
                info.raw
            ));
        } else {
            failures.push(format!("unused declaration '{}'", info.raw));
        }
    }
    if !failures.is_empty() {
        for failure in &failures {
            let _ = writeln!(stderr, "depcheck: FAIL: usage: {failure}");
        }
        return 1;
    }
    let _ = writeln!(
        stdout,
        "depcheck: OK: usage: {} declarations used in owning scope ({})",
        deps.len(),
        eco.name()
    );
    0
}

#[cfg(test)]
mod tests;
