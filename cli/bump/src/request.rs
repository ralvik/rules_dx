//! Single-requirement widen planning for `dx bump` (issue #260).
//!
//! Pure planning over injected selector/version strings only, so widening
//! stays deterministic and unit-testable without a workspace, a Bazel
//! server, registries, or any upstream updater. The explicit operation
//! rewrites exactly one declared requirement in the working copy:
//! `dx bump <selector> <version>` where `<selector>` is `set:package`
//! (e.g. `cargo:anyhow`, `npm:react`, `bazel:rules_rust`,
//! `github-actions:actions/checkout`) and `<version>` is the new exact
//! version (semver for Bazel/Cargo/npm/Go, tag/SHA for GitHub Actions).
//!
//! Never batch: one invocation widens one requirement. Multiple selectors,
//! bare sets, Bazel labels/paths, and empty versions fail closed as usage
//! errors (exit 2), never as partial widens. `dx update` keeps its
//! never-rewrites contract (`dx_update::semantics::may_be_rewritten` stays
//! false); this operation owns the single-requirement rewrite, including
//! exact pins, bounded ranges, and Git tag/commit shapes per the
//! ecosystem mapping in [`super::sets`]. Version shapes validate through
//! upstream [`super::version`] (`semver`, never custom version code).

use super::sets::BumpSet;
use super::version::{self, VersionError, WidenVersion};

/// Planned widen-one-requirement request: which single declared
/// requirement to rewrite to which new version. Spellings preserved
/// verbatim for planning summaries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BumpRequest {
    /// Owning widen set.
    pub set: BumpSet,
    /// Package identity within the set (e.g. `anyhow`, `react`,
    /// `rules_rust`, `actions/checkout`). Verbatim.
    pub package: String,
    /// Validated new version (exact semver or Git tag/commit).
    pub version: WidenVersion,
    /// Raw selector spelling (`set:package`), verbatim.
    pub selector: String,
    /// Raw version spelling, verbatim.
    pub raw_version: String,
}

/// Widen usage errors (exit 2, never a partial widen).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BumpError {
    /// Empty selector or version.
    #[error("empty bump selector or version; expected `dx bump <set:package> <version>`")]
    Empty,
    /// Unknown set or malformed `set:package` shape.
    #[error("unknown bump selector {selector:?}; expected bazel|cargo|npm|go|github-actions as `set:package` (e.g. cargo:anyhow)")]
    UnknownSelector {
        /// Offending spelling.
        selector: String,
    },
    /// Invalid package identity for its set.
    #[error("invalid package {package:?} for set {set}: {reason}")]
    InvalidPackage {
        /// Owning set name.
        set: &'static str,
        /// Offending package spelling.
        package: String,
        /// Why it is invalid.
        reason: &'static str,
    },
    /// Bare set without a package (never batch, never widen a whole set).
    #[error("bump needs one package, not a whole set: {set:?} selects the set; use `{set}:<package> <version>`")]
    BareSet {
        /// Offending set spelling.
        set: String,
    },
    /// Target/label/path passed where `set:package` belongs.
    #[error("bump needs `set:package`, not a label or path: {target:?}")]
    NotAPackage {
        /// Offending spelling.
        target: String,
    },
    /// Declared requirement not found in its manifest (nothing widened).
    #[error("no declared requirement for {package:?} in {manifest} (nothing widened)")]
    NotFound {
        /// Manifest searched.
        manifest: String,
        /// Package sought.
        package: String,
    },
    /// Ambiguous requirement (multiple matches; nothing widened, never
    /// batch).
    #[error("ambiguous requirement for {package:?} in {manifest}: {count} matches (nothing widened; widen one requirement per invocation)")]
    Ambiguous {
        /// Manifest searched.
        manifest: String,
        /// Package sought.
        package: String,
        /// Match count.
        count: usize,
    },
    /// Manifest shape out of v1 widen scope (nothing widened).
    #[error("unsupported manifest shape in {manifest}: {reason} (nothing widened)")]
    UnsupportedManifest {
        /// Manifest searched.
        manifest: String,
        /// Why it is unsupported.
        reason: String,
    },
    /// GitHub Actions tag needs SHA resolution through the upstream
    /// GitHub releases client before the file edit (nothing widened).
    /// Pass the resolved 40/64-char SHA as `<version>` instead.
    #[error("github-actions {package:?} tag {tag:?} needs SHA resolution via the upstream GitHub releases client; pass the resolved SHA as <version> (nothing widened)")]
    NeedsSha {
        /// Action sought (`owner/repo`).
        package: String,
        /// Tag supplied.
        tag: String,
    },
    /// Version-shape failure from upstream validation.
    #[error(transparent)]
    Version(#[from] VersionError),
}

impl BumpRequest {
    /// Plans one widen edit from the two positionals after `dx bump`.
    /// Nothing is probed, fetched, or resolved.
    pub fn parse(selector: &str, version: &str) -> Result<BumpRequest, BumpError> {
        if selector.is_empty() || version.is_empty() {
            return Err(BumpError::Empty);
        }
        // Bare `//...`, labels, and paths are never packages: fail closed
        // before set parsing so `//foo:bar` never reads as `set:package`.
        if is_target_shape(selector) {
            // `MODULE.bazel` and `.bazelversion` are file paths, not
            // selectors: the bump selector is `bazel:<module>` or
            // `bazel:.bazelversion`, never a bare filename.
            return Err(BumpError::NotAPackage {
                target: selector.to_owned(),
            });
        }
        // Bare sets (e.g. `cargo`) select whole sets: widen needs one
        // package, never a whole set.
        if let Some(set) = BumpSet::parse(selector) {
            return Err(BumpError::BareSet {
                set: set.name().to_owned(),
            });
        }
        let (head, tail) = match selector.split_once(':') {
            Some((head, tail)) => (head, tail),
            None => {
                return Err(BumpError::UnknownSelector {
                    selector: selector.to_owned(),
                });
            }
        };
        let set = match BumpSet::parse(head) {
            Some(set) => set,
            None => {
                return Err(BumpError::UnknownSelector {
                    selector: selector.to_owned(),
                });
            }
        };
        if tail.is_empty() {
            return Err(BumpError::InvalidPackage {
                set: set.name(),
                package: tail.to_owned(),
                reason: "package identity is empty",
            });
        }
        // GitHub Actions `owner/repo:tag` never appears here: the colon
        // separates `set:package`, so `github-actions:actions/checkout`
        // carries the slash inside the package. A second colon inside the
        // package (e.g. Maven `group:artifact`) is out of v1 bump scope.
        if tail.contains(':') {
            return Err(BumpError::InvalidPackage {
                set: set.name(),
                package: tail.to_owned(),
                reason: "package identity never contains ':'",
            });
        }
        validate_package(set, tail)?;
        let parsed = version::parse(set, version)?;
        Ok(BumpRequest {
            set,
            package: tail.to_owned(),
            version: parsed,
            selector: selector.to_owned(),
            raw_version: version.to_owned(),
        })
    }

    /// This operation owns the single-requirement rewrite: it widens the
    /// declared requirement to the new version. `dx update` keeps its
    /// never-rewrites contract; bump is the explicit exception. Pinned
    /// here so a future refactor cannot silently merge the two.
    pub fn may_be_rewritten() -> bool {
        true
    }

    /// Whether lock refresh runs resolver-owned after this widen edit
    /// (`dx update <set>` for Cargo/npm/Go) or the set is file-only
    /// (Bazel, GitHub Actions: preset flag-diff review plus build).
    pub fn needs_update_refresh(&self) -> bool {
        self.set.needs_update_refresh()
    }

    /// Workspace-relative manifest owning the declared requirement.
    /// Exactly one requirement in this file widens per invocation.
    pub fn target_manifest(&self) -> &'static str {
        match self.set {
            BumpSet::Bazel => {
                if self.package == ".bazelversion" {
                    ".bazelversion"
                } else {
                    "MODULE.bazel"
                }
            }
            BumpSet::Cargo => "rust/hello/Cargo.toml",
            BumpSet::GithubActions => ".github/workflows/ci.yml",
            BumpSet::Go => "go/go.mod",
            BumpSet::Npm => "package.json",
        }
    }

    /// Human planning summary for `--dry-run` (never argv).
    pub fn summary(&self) -> String {
        let through = if self.needs_update_refresh() {
            format!("then `dx update {}`", self.set.name())
        } else {
            "then preset flag-diff review plus `bazel build //...`".to_owned()
        };
        format!(
            "Widen {} to {} in {} ({through})",
            self.selector,
            self.version.display(),
            self.target_manifest(),
        )
    }

    /// Plans the single-requirement file edit over injected manifest bytes.
    /// Returns the widened file bytes; fails closed (nothing widened) when
    /// the requirement is missing, ambiguous, or in an unsupported shape.
    /// Exactly one requirement widens per invocation (never batch).
    pub fn plan_edit(&self, content: &str) -> Result<String, BumpError> {
        match self.set {
            BumpSet::Bazel => {
                if self.package == ".bazelversion" {
                    plan_bazelversion(content, &self.version)
                } else {
                    plan_module_bazel(content, &self.package, &self.version)
                }
            }
            BumpSet::Cargo => plan_cargo_toml(content, &self.package, &self.version),
            BumpSet::Npm => plan_package_json(content, &self.package, &self.version),
            BumpSet::Go => plan_go_mod(content, &self.package, &self.version),
            BumpSet::GithubActions => plan_github_workflow(content, &self.package, &self.version),
        }
    }
}

/// Thin widen edits below rewrite exactly one quoted version string per
/// invocation, preserving all other bytes. Each helper counts matches and
/// fails closed on zero or multiple (never batch, never guess).
fn plan_bazelversion(content: &str, version: &WidenVersion) -> Result<String, BumpError> {
    let new = match version {
        WidenVersion::Semver(version) => version.to_string(),
        _ => {
            return Err(BumpError::UnsupportedManifest {
                manifest: ".bazelversion".to_owned(),
                reason: "expected exact semver".to_owned(),
            });
        }
    };
    let trimmed = content.trim();
    if trimmed.is_empty() || trimmed.contains('\n') {
        return Err(BumpError::UnsupportedManifest {
            manifest: ".bazelversion".to_owned(),
            reason: "expected one single-line version".to_owned(),
        });
    }
    let newline = content.ends_with('\n');
    if newline {
        Ok(format!("{new}\n"))
    } else {
        Ok(new)
    }
}

fn plan_module_bazel(
    content: &str,
    package: &str,
    version: &WidenVersion,
) -> Result<String, BumpError> {
    let new = match version {
        WidenVersion::Semver(version) => version.to_string(),
        _ => {
            return Err(BumpError::UnsupportedManifest {
                manifest: "MODULE.bazel".to_owned(),
                reason: "expected exact semver".to_owned(),
            });
        }
    };
    let needle = format!("name = \"{package}\"");
    let mut matches = 0usize;
    let mut out = String::with_capacity(content.len());
    for line in content.split_inclusive('\n') {
        if line.contains("bazel_dep(") && line.contains(&needle) {
            // Replace the `version = "old"` attr on this bazel_dep line
            // (never the `name = "..."` attr, which sorts first).
            match replace_version_attr(line, &new) {
                Some(replaced) => {
                    matches += 1;
                    out.push_str(&replaced);
                    continue;
                }
                None => {
                    return Err(BumpError::UnsupportedManifest {
                        manifest: "MODULE.bazel".to_owned(),
                        reason: format!("bazel_dep {package:?} has no quoted version on its line"),
                    });
                }
            }
        }
        out.push_str(line);
    }
    // Handle a final line without trailing newline (split_inclusive still
    // yields it; the loop above already covered it).
    match matches {
        1 => Ok(out),
        0 => Err(BumpError::NotFound {
            manifest: "MODULE.bazel".to_owned(),
            package: package.to_owned(),
        }),
        count => Err(BumpError::Ambiguous {
            manifest: "MODULE.bazel".to_owned(),
            package: package.to_owned(),
            count,
        }),
    }
}

fn plan_cargo_toml(
    content: &str,
    package: &str,
    version: &WidenVersion,
) -> Result<String, BumpError> {
    let new = match version {
        WidenVersion::Semver(version) => version.to_string(),
        _ => {
            return Err(BumpError::UnsupportedManifest {
                manifest: "rust/hello/Cargo.toml".to_owned(),
                reason: "expected exact semver".to_owned(),
            });
        }
    };
    // Format-preserving edit via `toml_edit::DocumentMut`: locate the dep
    // by table key (`package = "old"` or `package = { version = "old" }`
    // or `[dependencies.package] version = "old"`), preserve
    // comments/whitespace/order, keep fail-closed `git`/`path` behavior
    // as explicit typed errors, keep 1-match/0-ambiguous counting.
    let mut doc =
        content
            .parse::<toml_edit::DocumentMut>()
            .map_err(|_| BumpError::UnsupportedManifest {
                manifest: "rust/hello/Cargo.toml".to_owned(),
                reason: "manifest is not valid TOML".to_owned(),
            })?;
    let paths = cargo_dependency_table_paths(&doc);
    let mut matches: Vec<Vec<String>> = Vec::new();
    for path in &paths {
        let Some(table) = cargo_table_at(&doc, path) else {
            continue;
        };
        let Some(item) = table.get(package) else {
            continue;
        };
        match cargo_dep_shape(item) {
            CargoDepShape::Registry => matches.push(path.clone()),
            CargoDepShape::GitOrPath => {
                return Err(BumpError::UnsupportedManifest {
                    manifest: "rust/hello/Cargo.toml".to_owned(),
                    reason: format!(
                        "{package:?} is git/path-shaped; v1 widens registry versions only"
                    ),
                });
            }
            CargoDepShape::Workspace => {
                return Err(BumpError::UnsupportedManifest {
                    manifest: "rust/hello/Cargo.toml".to_owned(),
                    reason: format!(
                        "{package:?} inherits workspace version; v1 widens registry versions only"
                    ),
                });
            }
            CargoDepShape::NoVersion => {
                return Err(BumpError::UnsupportedManifest {
                    manifest: "rust/hello/Cargo.toml".to_owned(),
                    reason: format!("{package:?} has no version to widen"),
                });
            }
        }
    }
    match matches.len() {
        0 => Err(BumpError::NotFound {
            manifest: "rust/hello/Cargo.toml".to_owned(),
            package: package.to_owned(),
        }),
        1 => {
            let table = cargo_table_at_mut(&mut doc, &matches[0]).ok_or_else(|| {
                BumpError::UnsupportedManifest {
                    manifest: "rust/hello/Cargo.toml".to_owned(),
                    reason: format!("{package:?} has no version to widen"),
                }
            })?;
            cargo_set_version(table, package, &new).ok_or_else(|| {
                BumpError::UnsupportedManifest {
                    manifest: "rust/hello/Cargo.toml".to_owned(),
                    reason: format!("{package:?} has no version to widen"),
                }
            })?;
            Ok(doc.to_string())
        }
        count => Err(BumpError::Ambiguous {
            manifest: "rust/hello/Cargo.toml".to_owned(),
            package: package.to_owned(),
            count,
        }),
    }
}

/// Dependency shapes for `plan_cargo_toml`: registry-owned (widenable)
/// versus fail-closed shapes (explicit typed errors, never guessed).
enum CargoDepShape {
    /// `package = "old"`, `package = { version = "old", .. }`, or
    /// `[table.package] version = "old"`.
    Registry,
    /// `git =` / `path =` present (v1 widens registry versions only).
    GitOrPath,
    /// `workspace = true` inheritance (version lives in `[workspace]`).
    Workspace,
    /// No string `version` to widen (table shapes fail closed).
    NoVersion,
}

/// Classifies one dep entry by table key, never by line text.
fn cargo_dep_shape(item: &toml_edit::Item) -> CargoDepShape {
    match item {
        toml_edit::Item::Value(toml_edit::Value::String(_)) => CargoDepShape::Registry,
        toml_edit::Item::Value(toml_edit::Value::InlineTable(table)) => cargo_inline_shape(table),
        toml_edit::Item::Table(table) => cargo_table_shape(table),
        _ => CargoDepShape::NoVersion,
    }
}

/// Classifies `package = { ... }` inline tables.
fn cargo_inline_shape(table: &toml_edit::InlineTable) -> CargoDepShape {
    if table.contains_key("git") || table.contains_key("path") {
        return CargoDepShape::GitOrPath;
    }
    if table
        .get("workspace")
        .is_some_and(|v| v.as_bool() == Some(true))
    {
        return CargoDepShape::Workspace;
    }
    match table.get("version") {
        Some(toml_edit::Value::String(_)) => CargoDepShape::Registry,
        _ => CargoDepShape::NoVersion,
    }
}

/// Classifies `[table.package] ...` tables.
fn cargo_table_shape(table: &toml_edit::Table) -> CargoDepShape {
    if table.contains_key("git") || table.contains_key("path") {
        return CargoDepShape::GitOrPath;
    }
    if table
        .get("workspace")
        .is_some_and(|item| item.as_bool() == Some(true))
    {
        return CargoDepShape::Workspace;
    }
    match table.get("version") {
        Some(toml_edit::Item::Value(toml_edit::Value::String(_))) => CargoDepShape::Registry,
        _ => CargoDepShape::NoVersion,
    }
}

/// All dependency-like tables that may own `package`: top-level
/// `dependencies`/`dev-dependencies`/`build-dependencies`,
/// `workspace.dependencies`, per-target
/// `target.<cfg>.{dependencies,dev-dependencies,build-dependencies}`,
/// and `patch.<source>` (matched by old line surgery, kept here so the
/// rewrite is strictly fewer false `NotFound`s).
fn cargo_dependency_table_paths(doc: &toml_edit::DocumentMut) -> Vec<Vec<String>> {
    let mut paths: Vec<Vec<String>> = Vec::new();
    for name in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if doc.get(name).is_some_and(|item| item.is_table()) {
            paths.push(vec![name.to_owned()]);
        }
    }
    if doc
        .get("workspace")
        .and_then(|item| item.as_table())
        .is_some_and(|workspace| {
            workspace
                .get("dependencies")
                .is_some_and(|item| item.is_table())
        })
    {
        paths.push(vec!["workspace".to_owned(), "dependencies".to_owned()]);
    }
    if let Some(targets) = doc.get("target").and_then(|item| item.as_table()) {
        for (target_name, target_item) in targets.iter() {
            let Some(target_table) = target_item.as_table() else {
                continue;
            };
            for kind in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if target_table.get(kind).is_some_and(|item| item.is_table()) {
                    paths.push(vec![
                        "target".to_owned(),
                        target_name.to_owned(),
                        kind.to_owned(),
                    ]);
                }
            }
        }
    }
    if let Some(patch) = doc.get("patch").and_then(|item| item.as_table()) {
        for (source, _) in patch.iter() {
            paths.push(vec!["patch".to_owned(), source.to_owned()]);
        }
    }
    paths
}

/// Immutable lookup of a dependency-like table by path.
fn cargo_table_at<'a>(
    doc: &'a toml_edit::DocumentMut,
    path: &[String],
) -> Option<&'a toml_edit::Table> {
    let mut item: &toml_edit::Item = doc.as_item();
    for key in path {
        item = item.as_table()?.get(key)?;
    }
    item.as_table()
}

/// Mutable lookup of a dependency-like table by path.
fn cargo_table_at_mut<'a>(
    doc: &'a mut toml_edit::DocumentMut,
    path: &[String],
) -> Option<&'a mut toml_edit::Table> {
    let mut item: &mut toml_edit::Item = doc.as_item_mut();
    for key in path {
        // `as_table_mut` on the current item, then `get_mut` the next key.
        // Split borrows so the mutable chain typechecks.
        let table = item.as_table_mut()?;
        item = table.get_mut(key)?;
    }
    item.as_table_mut()
}

/// Sets the registry version for one dep entry, preserving decor
/// (comments/whitespace) and sibling keys. Returns false when the entry
/// is not registry-shaped (caller already classified it).
fn cargo_set_version(table: &mut toml_edit::Table, package: &str, new: &str) -> bool {
    let Some(item) = table.get_mut(package) else {
        return false;
    };
    match item {
        toml_edit::Item::Value(toml_edit::Value::String(formatted)) => {
            let decor = formatted.decor().clone();
            *formatted = toml_edit::Formatted::new(new.to_owned());
            *formatted.decor_mut() = decor;
            true
        }
        toml_edit::Item::Value(toml_edit::Value::InlineTable(inline)) => {
            let Some(toml_edit::Value::String(formatted)) = inline.get_mut("version") else {
                return false;
            };
            let decor = formatted.decor().clone();
            *formatted = toml_edit::Formatted::new(new.to_owned());
            *formatted.decor_mut() = decor;
            true
        }
        toml_edit::Item::Table(inner) => {
            let Some(toml_edit::Item::Value(toml_edit::Value::String(formatted))) =
                inner.get_mut("version")
            else {
                return false;
            };
            let decor = formatted.decor().clone();
            *formatted = toml_edit::Formatted::new(new.to_owned());
            *formatted.decor_mut() = decor;
            true
        }
        _ => false,
    }
}

fn plan_package_json(
    content: &str,
    package: &str,
    version: &WidenVersion,
) -> Result<String, BumpError> {
    let new = match version {
        WidenVersion::Semver(version) => version.to_string(),
        _ => {
            return Err(BumpError::UnsupportedManifest {
                manifest: "package.json".to_owned(),
                reason: "expected exact semver".to_owned(),
            });
        }
    };
    // Text replacement of `"package": "old"` preserves formatting
    // (no serde_json re-emit, which would reformat the whole file).
    // Validate the file is JSON through the upstream parser first so a
    // corrupt manifest fails closed before any edit.
    let _: serde_json::Value =
        serde_json::from_str(content).map_err(|_| BumpError::UnsupportedManifest {
            manifest: "package.json".to_owned(),
            reason: "manifest is not valid JSON".to_owned(),
        })?;
    let key = format!("\"{package}\"");
    let mut matches = 0usize;
    let mut out = String::with_capacity(content.len());
    for line in content.split_inclusive('\n') {
        if line.contains(&key) && line.contains(':') && line.contains('"') {
            // Only count lines where the key is a JSON object key (quoted
            // package followed by optional whitespace then `:`).
            if let Some(key_at) = line.find(&key) {
                let after = &line[key_at + key.len()..];
                if after.trim_start().starts_with(':') {
                    match replace_first_quoted_version_after_colon(line, &new) {
                        Some(replaced) => {
                            matches += 1;
                            out.push_str(&replaced);
                            continue;
                        }
                        None => {
                            return Err(BumpError::UnsupportedManifest {
                                manifest: "package.json".to_owned(),
                                reason: format!("{package:?} has no quoted version after ':'"),
                            });
                        }
                    }
                }
            }
        }
        out.push_str(line);
    }
    match matches {
        1 => Ok(out),
        0 => Err(BumpError::NotFound {
            manifest: "package.json".to_owned(),
            package: package.to_owned(),
        }),
        count => Err(BumpError::Ambiguous {
            manifest: "package.json".to_owned(),
            package: package.to_owned(),
            count,
        }),
    }
}

fn plan_go_mod(content: &str, package: &str, version: &WidenVersion) -> Result<String, BumpError> {
    let new = match version {
        WidenVersion::Semver(version) => format!("v{version}"),
        _ => {
            return Err(BumpError::UnsupportedManifest {
                manifest: "go/go.mod".to_owned(),
                reason: "expected exact semver".to_owned(),
            });
        }
    };
    let mut matches = 0usize;
    let mut out = String::with_capacity(content.len());
    for line in content.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with('#') {
            out.push_str(line);
            continue;
        }
        // `require example.com/mod v1.2.3` or bare `example.com/mod v1.2.3`
        // inside a require block. Match the module path as a whitespace
        // delimited token to avoid prefix collisions.
        if line_contains_module_token(line, package) && line.contains('v') {
            match replace_go_version_token(line, &new) {
                Some(replaced) => {
                    matches += 1;
                    out.push_str(&replaced);
                    continue;
                }
                None => {
                    return Err(BumpError::UnsupportedManifest {
                        manifest: "go/go.mod".to_owned(),
                        reason: format!("{package:?} has no replaceable v-prefixed version token"),
                    });
                }
            }
        }
        out.push_str(line);
    }
    match matches {
        1 => Ok(out),
        0 => Err(BumpError::NotFound {
            manifest: "go/go.mod".to_owned(),
            package: package.to_owned(),
        }),
        count => Err(BumpError::Ambiguous {
            manifest: "go/go.mod".to_owned(),
            package: package.to_owned(),
            count,
        }),
    }
}

fn line_contains_module_token(line: &str, package: &str) -> bool {
    line.split_whitespace().any(|token| token == package)
}

fn replace_go_version_token(line: &str, new: &str) -> Option<String> {
    // Replace the last `v<digits...>` token (the version) with `new`.
    // Keeps indentation, trailing comments, and newline style intact.
    let mut last_start: Option<usize> = None;
    let mut last_end: Option<usize> = None;
    let bytes = line.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'v' {
            let mut end = index + 1;
            let mut digits = 0usize;
            let mut dots = 0usize;
            while end < bytes.len() {
                let byte = bytes[end];
                if byte.is_ascii_digit() {
                    digits += 1;
                    end += 1;
                } else if byte == b'.' {
                    dots += 1;
                    end += 1;
                } else if byte == b'-' || byte == b'+' {
                    // Prerelease/build suffix: consume until whitespace.
                    end += 1;
                    while end < bytes.len() && !bytes[end].is_ascii_whitespace() {
                        end += 1;
                    }
                    break;
                } else {
                    break;
                }
            }
            if digits > 0 && dots > 0 {
                last_start = Some(index);
                last_end = Some(end);
            }
            index = end.max(index + 1);
        } else {
            index += 1;
        }
    }
    match (last_start, last_end) {
        (Some(start), Some(end)) => {
            let mut replaced = String::with_capacity(line.len());
            replaced.push_str(&line[..start]);
            replaced.push_str(new);
            replaced.push_str(&line[end..]);
            Some(replaced)
        }
        _ => None,
    }
}

fn plan_github_workflow(
    content: &str,
    package: &str,
    version: &WidenVersion,
) -> Result<String, BumpError> {
    match version {
        WidenVersion::GitTag(tag) => Err(BumpError::NeedsSha {
            package: package.to_owned(),
            tag: tag.clone(),
        }),
        WidenVersion::GitCommit(sha) => {
            let needle = format!("{package}@");
            let mut matches = 0usize;
            let mut out = String::with_capacity(content.len());
            for line in content.split_inclusive('\n') {
                if line.contains("uses:") && line.contains(&needle) {
                    match replace_gha_sha(line, &needle, sha) {
                        Some(replaced) => {
                            matches += 1;
                            out.push_str(&replaced);
                            continue;
                        }
                        None => {
                            return Err(BumpError::UnsupportedManifest {
                                manifest: ".github/workflows/ci.yml".to_owned(),
                                reason: format!("{package:?} has no replaceable @SHA pin"),
                            });
                        }
                    }
                }
                out.push_str(line);
            }
            match matches {
                0 => Err(BumpError::NotFound {
                    manifest: ".github/workflows/ci.yml".to_owned(),
                    package: package.to_owned(),
                }),
                1 => Ok(out),
                count => Err(BumpError::Ambiguous {
                    manifest: ".github/workflows/ci.yml".to_owned(),
                    package: package.to_owned(),
                    count,
                }),
            }
        }
        WidenVersion::Semver(_) => Err(BumpError::UnsupportedManifest {
            manifest: ".github/workflows/ci.yml".to_owned(),
            reason: "github-actions pins are tag/SHA-shaped, not semver".to_owned(),
        }),
    }
}

fn replace_gha_sha(line: &str, needle: &str, sha: &str) -> Option<String> {
    let at = line.find(needle)? + needle.len();
    let rest = &line[at..];
    // SHA runs until whitespace or end-of-line.
    let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    let mut replaced = String::with_capacity(line.len());
    replaced.push_str(&line[..at]);
    replaced.push_str(sha);
    replaced.push_str(&rest[end..]);
    Some(replaced)
}

/// Replaces the quoted value of the `version = "old"` attribute on one
/// `bazel_dep(...)` line with `"new"`, preserving the `name` attr and all
/// other bytes.
fn replace_version_attr(line: &str, new: &str) -> Option<String> {
    let version_at = line.find("version")?;
    let after_version = &line[version_at + "version".len()..];
    let eq_rel = after_version.find('=')?;
    let after_eq = &after_version[eq_rel + 1..];
    let quote_rel = after_eq.find('"')?;
    let start = version_at + "version".len() + eq_rel + 1 + quote_rel;
    let after_start = &line[start + 1..];
    let end_rel = after_start.find('"')?;
    let mut replaced = String::with_capacity(line.len());
    replaced.push_str(&line[..=start]);
    replaced.push_str(new);
    replaced.push_str(&after_start[end_rel..]);
    Some(replaced)
}

/// Replaces the quoted version after the first `:` on a JSON line
/// (`"package": "old"` -> `"package": "new"`), preserving spacing.
fn replace_first_quoted_version_after_colon(line: &str, new: &str) -> Option<String> {
    let colon = line.find(':')?;
    let after = &line[colon + 1..];
    let start_rel = after.find('"')?;
    let start = colon + 1 + start_rel;
    let after_start = &line[start + 1..];
    let end_rel = after_start.find('"')?;
    let mut replaced = String::with_capacity(line.len());
    replaced.push_str(&line[..=start]);
    replaced.push_str(new);
    replaced.push_str(&after_start[end_rel..]);
    Some(replaced)
}

/// True for Bazel labels/patterns and file/dir paths (never `set:package`).
fn is_target_shape(text: &str) -> bool {
    // `set:package` never starts with `/`/`@` and never contains `/`
    // except inside GitHub Actions `owner/repo` packages (which still
    // start with `github-actions:`/`gha:`). Labels/paths do.
    if text.starts_with("//") || text.starts_with('@') {
        return true;
    }
    // Bare filenames/paths owned by bump manifests are still not
    // selectors: `package.json`, `MODULE.bazel`, `.bazelversion`, and any
    // `a/b` path without a known `set:` prefix fail as NotAPackage, not
    // as UnknownSelector, so the operator learns the `set:package` shape.
    if text.contains('/') && !text.contains(':') {
        return true;
    }
    if text.contains('/')
        && BumpSet::parse(text.split_once(':').map_or("", |(head, _)| head)).is_none()
    {
        return true;
    }
    text == "..."
        || text == "MODULE.bazel"
        || text == ".bazelversion"
        || text == "package.json"
        || text == "Cargo.toml"
}

/// Validates an ecosystem package identity (upstream-native, no versions).
fn validate_package(set: BumpSet, package: &str) -> Result<(), BumpError> {
    let invalid = |reason: &'static str| BumpError::InvalidPackage {
        set: set.name(),
        package: package.to_owned(),
        reason,
    };
    match set {
        BumpSet::Bazel => {
            if package == ".bazelversion" {
                return Ok(());
            }
            if package.is_empty()
                || !package
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
            {
                return Err(invalid(
                    "bazel modules use [A-Za-z0-9_.-] only (or .bazelversion)",
                ));
            }
            Ok(())
        }
        BumpSet::Cargo => {
            if package.is_empty()
                || !package
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err(invalid("cargo crate names use [A-Za-z0-9_-] only"));
            }
            Ok(())
        }
        BumpSet::Npm => {
            if package.is_empty() || package.contains(':') || package.contains(' ') {
                return Err(invalid("npm package names never contain ':' or spaces"));
            }
            if let Some(rest) = package.strip_prefix('@') {
                let (scope, slash, name) = match rest.find('/') {
                    Some(idx) => (&rest[..idx], true, &rest[idx + 1..]),
                    None => ("", false, ""),
                };
                let _ = slash;
                if scope.is_empty() || name.is_empty() || !slash {
                    return Err(invalid("scoped npm names are @scope/name"));
                }
                if !scope
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
                    || !name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
                {
                    return Err(invalid("npm scope/name use [A-Za-z0-9_.-] only"));
                }
                Ok(())
            } else {
                if package.contains('/') {
                    return Err(invalid("unscoped npm names never contain '/'"));
                }
                if !package
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
                {
                    return Err(invalid("npm names use [A-Za-z0-9_.-] only"));
                }
                Ok(())
            }
        }
        BumpSet::Go => {
            if package.is_empty()
                || package.contains(':')
                || package.contains(' ')
                || package.starts_with('/')
                || package.ends_with('/')
                || package.contains("//")
            {
                return Err(invalid("go module paths never contain ':' or spaces"));
            }
            if !package.chars().all(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_' | '~' | '+')
            }) {
                return Err(invalid("go module paths use [A-Za-z0-9/_.-~+] only"));
            }
            Ok(())
        }
        BumpSet::GithubActions => {
            let (owner, repo) = match package.split_once('/') {
                Some((owner, repo)) => (owner, repo),
                None => {
                    return Err(invalid("github-actions identities are owner/repo"));
                }
            };
            if owner.is_empty()
                || repo.is_empty()
                || repo.contains('/')
                || repo.contains(' ')
                || owner.contains(' ')
            {
                return Err(invalid("github-actions identities are owner/repo"));
            }
            let valid = |text: &str| {
                text.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
            };
            if !valid(owner) || !valid(repo) {
                return Err(invalid("github-actions owner/repo use [A-Za-z0-9_.-] only"));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_requirement_shapes() {
        let bump = BumpRequest::parse("cargo:anyhow", "1.2.3").expect("cargo");
        assert_eq!(bump.set, BumpSet::Cargo);
        assert_eq!(bump.package, "anyhow");
        assert_eq!(bump.target_manifest(), "rust/hello/Cargo.toml");
        assert!(bump.needs_update_refresh());

        let bump = BumpRequest::parse("npm:react", "1.2.3").expect("npm");
        assert_eq!(bump.target_manifest(), "package.json");

        let bump = BumpRequest::parse("npm:@astrojs/compiler", "1.2.3").expect("scoped");
        assert_eq!(bump.package, "@astrojs/compiler");

        let bump = BumpRequest::parse("bazel:rules_rust", "0.74.0").expect("bazel module");
        assert_eq!(bump.target_manifest(), "MODULE.bazel");
        assert!(!bump.needs_update_refresh());

        let bump = BumpRequest::parse("bazel:.bazelversion", "9.2.0").expect("bazelversion");
        assert_eq!(bump.target_manifest(), ".bazelversion");

        let bump = BumpRequest::parse("github-actions:actions/checkout", "v4").expect("gha");
        assert_eq!(bump.set, BumpSet::GithubActions);
        assert!(!bump.needs_update_refresh());

        let bump = BumpRequest::parse("go:example.com/mod", "1.2.3").expect("go");
        assert_eq!(bump.target_manifest(), "go/go.mod");
    }

    #[test]
    fn aliases_resolve_to_canonical_sets() {
        let bump = BumpRequest::parse("gomod:example.com/mod", "1.2.3").expect("gomod");
        assert_eq!(bump.set, BumpSet::Go);
        let bump = BumpRequest::parse("gha:actions/checkout", "v4").expect("gha");
        assert_eq!(bump.set, BumpSet::GithubActions);
    }

    #[test]
    fn widen_owns_rewrite_while_update_never_does() {
        assert!(BumpRequest::may_be_rewritten());
    }

    #[test]
    fn bare_sets_targets_and_unknown_fail_closed() {
        assert!(matches!(
            BumpRequest::parse("cargo", "1.2.3"),
            Err(BumpError::BareSet { .. })
        ));
        assert!(matches!(
            BumpRequest::parse("//rust/hello:hello", "1.2.3"),
            Err(BumpError::NotAPackage { .. })
        ));
        assert!(matches!(
            BumpRequest::parse("rust/hello/Cargo.toml", "1.2.3"),
            Err(BumpError::NotAPackage { .. })
        ));
        assert!(matches!(
            BumpRequest::parse("crates", "1.2.3"),
            Err(BumpError::UnknownSelector { .. })
        ));
        assert!(matches!(
            BumpRequest::parse("cargo:", "1.2.3"),
            Err(BumpError::InvalidPackage { .. })
        ));
        assert!(matches!(
            BumpRequest::parse("", "1.2.3"),
            Err(BumpError::Empty)
        ));
        assert!(matches!(
            BumpRequest::parse("cargo:anyhow", ""),
            Err(BumpError::Empty)
        ));
        assert!(matches!(
            BumpRequest::parse("cargo:bad name", "1.2.3"),
            Err(BumpError::InvalidPackage { .. })
        ));
        assert!(matches!(
            BumpRequest::parse("maven:junit:junit", "1.2.3"),
            Err(BumpError::UnknownSelector { .. })
        ));
    }

    #[test]
    fn invalid_versions_fail_without_widen() {
        assert!(matches!(
            BumpRequest::parse("cargo:anyhow", "not-a-version!!!"),
            Err(BumpError::Version(_))
        ));
        assert!(matches!(
            BumpRequest::parse("github-actions:actions/checkout", "bad tag"),
            Err(BumpError::Version(_))
        ));
    }

    #[test]
    fn summary_names_selector_version_and_next_step() {
        let bump = BumpRequest::parse("cargo:anyhow", "1.2.3").expect("cargo");
        let summary = bump.summary();
        assert!(summary.contains("cargo:anyhow"), "{summary}");
        assert!(summary.contains("1.2.3"), "{summary}");
        assert!(summary.contains("dx update cargo"), "{summary}");
        let bump = BumpRequest::parse("bazel:rules_rust", "0.74.0").expect("bazel");
        assert!(bump.summary().contains("flag-diff"), "{summary}");
    }

    #[test]
    fn edits_rewrite_exactly_one_requirement() {
        // `.bazelversion`: single-line replace, newline preserved.
        let bump = BumpRequest::parse("bazel:.bazelversion", "9.3.0").expect("bazelversion");
        assert_eq!(bump.plan_edit("9.2.0\n").expect("edit"), "9.3.0\n");
        assert_eq!(bump.plan_edit("9.2.0").expect("edit"), "9.3.0");
        assert!(matches!(
            bump.plan_edit("9.2.0\n9.3.0\n"),
            Err(BumpError::UnsupportedManifest { .. })
        ));

        // `MODULE.bazel`: one `bazel_dep` line rewrites, others preserved.
        let module = "bazel_dep(name = \"rules_rust\", version = \"0.74.0\")\n\
                      bazel_dep(name = \"gazelle\", version = \"0.52.2\")\n";
        let bump = BumpRequest::parse("bazel:rules_rust", "0.75.0").expect("module");
        let widened = bump.plan_edit(module).expect("edit");
        assert!(
            widened.contains("name = \"rules_rust\", version = \"0.75.0\""),
            "{widened}"
        );
        assert!(
            widened.contains("name = \"gazelle\", version = \"0.52.2\""),
            "{widened}"
        );
        assert!(matches!(
            bump.plan_edit("bazel_dep(name = \"other\", version = \"1.0.0\")\n"),
            Err(BumpError::NotFound { .. })
        ));

        // `Cargo.toml`: `package = "old"` rewrites once.
        let cargo = "[dependencies]\nanyhow = \"1\"\nserde = \"1\"\n";
        let bump = BumpRequest::parse("cargo:anyhow", "1.2.3").expect("cargo");
        let widened = bump.plan_edit(cargo).expect("edit");
        assert!(widened.contains("anyhow = \"1.2.3\""), "{widened}");
        assert!(widened.contains("serde = \"1\""), "{widened}");

        // `package.json`: `"package": "old"` rewrites once, formatting kept.
        let npm = "{\n  \"devDependencies\": {\n    \"jest\": \"30.2.0\",\n    \"vue\": \"3.5.42\"\n  }\n}\n";
        let bump = BumpRequest::parse("npm:jest", "30.3.0").expect("npm");
        let widened = bump.plan_edit(npm).expect("edit");
        assert!(widened.contains("\"jest\": \"30.3.0\""), "{widened}");
        assert!(widened.contains("\"vue\": \"3.5.42\""), "{widened}");
        assert!(matches!(
            bump.plan_edit("{\n}\n"),
            Err(BumpError::NotFound { .. })
        ));

        // `go.mod`: `require <mod> vX` rewrites once.
        let gomod = "module example.com/root\n\nrequire example.com/mod v1.2.3\n";
        let bump = BumpRequest::parse("go:example.com/mod", "1.3.0").expect("go");
        let widened = bump.plan_edit(gomod).expect("edit");
        assert!(widened.contains("example.com/mod v1.3.0"), "{widened}");
    }

    #[test]
    fn cargo_toml_preserves_format_and_fails_closed() {
        let bump = BumpRequest::parse("cargo:anyhow", "1.2.3").expect("cargo");

        // Inline table keeps sibling keys, comments, and order.
        let cargo = "[dependencies]\nanyhow = { version = \"1\", features = [\"derive\"] } # keep\nserde = \"1\"\n";
        let widened = bump.plan_edit(cargo).expect("inline");
        assert!(
            widened.contains("anyhow = { version = \"1.2.3\""),
            "{widened}"
        );
        assert!(widened.contains("features = [\"derive\"]"), "{widened}");
        assert!(widened.contains("# keep"), "{widened}");
        assert!(widened.contains("serde = \"1\""), "{widened}");

        // `[dependencies.package]` table form widens `version`.
        let cargo = "[dependencies.anyhow]\nversion = \"1\"\nfeatures = [\"derive\"]\n";
        let widened = bump.plan_edit(cargo).expect("table");
        assert!(widened.contains("version = \"1.2.3\""), "{widened}");
        assert!(widened.contains("features ="), "{widened}");

        // Single `dev-dependencies` entry widens.
        let cargo = "[dev-dependencies]\nanyhow = \"1\"\n";
        let widened = bump.plan_edit(cargo).expect("dev");
        assert!(widened.contains("anyhow = \"1.2.3\""), "{widened}");

        // Same crate in two tables is ambiguous (never batch).
        let cargo = "[dependencies]\nanyhow = \"1\"\n[dev-dependencies]\nanyhow = \"1\"\n";
        assert!(matches!(
            bump.plan_edit(cargo),
            Err(BumpError::Ambiguous { count: 2, .. })
        ));

        // Git/path shapes fail closed as unsupported.
        let cargo =
            "[dependencies]\nanyhow = { git = \"https://example.com/repo\", tag = \"v1\" }\n";
        assert!(matches!(
            bump.plan_edit(cargo),
            Err(BumpError::UnsupportedManifest { .. })
        ));
        let cargo = "[dependencies]\nanyhow = { path = \"../anyhow\" }\n";
        assert!(matches!(
            bump.plan_edit(cargo),
            Err(BumpError::UnsupportedManifest { .. })
        ));

        // Workspace inheritance and missing versions fail closed.
        let cargo = "[dependencies]\nanyhow = { workspace = true }\n";
        assert!(matches!(
            bump.plan_edit(cargo),
            Err(BumpError::UnsupportedManifest { .. })
        ));
        let cargo = "[dependencies]\nanyhow = { optional = true }\n";
        assert!(matches!(
            bump.plan_edit(cargo),
            Err(BumpError::UnsupportedManifest { .. })
        ));

        // Invalid TOML and missing deps fail closed without widening.
        assert!(matches!(
            bump.plan_edit("[dependencies\nanyhow = "),
            Err(BumpError::UnsupportedManifest { .. })
        ));
        assert!(matches!(
            bump.plan_edit("[dependencies]\nserde = \"1\"\n"),
            Err(BumpError::NotFound { .. })
        ));
    }

    #[test]
    fn github_actions_needs_sha_for_tags() {
        let sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let workflow = format!("      - uses: actions/checkout@{old} # v7\n      - uses: actions/cache@bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb # v6\n", old = "3d3c42e5aac5ba805825da76410c181273ba90b1");
        let bump = BumpRequest::parse("github-actions:actions/checkout", &sha).expect("sha");
        let widened = bump.plan_edit(&workflow).expect("edit");
        assert!(
            widened.contains(&format!("actions/checkout@{sha}")),
            "{widened}"
        );
        assert!(widened.contains("actions/cache@bbbb"), "{widened}");

        let tag = BumpRequest::parse("github-actions:actions/checkout", "v5").expect("tag");
        assert!(matches!(
            tag.plan_edit(&workflow),
            Err(BumpError::NeedsSha { .. })
        ));
    }
}
