//! Cargo manifest widen edits for `dx bump` (split from `request.rs`).
//!
//! `plan_cargo_toml` plus the `cargo_*` table-shape helpers. No
//! behavior change: moved verbatim.

use super::*;

pub(super) fn plan_cargo_toml(
    content: &str,
    package: &str,
    version: &WidenVersion,
) -> Result<String, BumpError> {
    let new = match version {
        WidenVersion::Semver(version) => version.to_string(),
        _ => {
            return Err(BumpError::UnsupportedManifest {
                manifest: "rust/tests/fixtures/hello/Cargo.toml".to_owned(),
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
                manifest: "rust/tests/fixtures/hello/Cargo.toml".to_owned(),
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
                    manifest: "rust/tests/fixtures/hello/Cargo.toml".to_owned(),
                    reason: format!(
                        "{package:?} is git/path-shaped; v1 widens registry versions only"
                    ),
                });
            }
            CargoDepShape::Workspace => {
                return Err(BumpError::UnsupportedManifest {
                    manifest: "rust/tests/fixtures/hello/Cargo.toml".to_owned(),
                    reason: format!(
                        "{package:?} inherits workspace version; v1 widens registry versions only"
                    ),
                });
            }
            CargoDepShape::NoVersion => {
                return Err(BumpError::UnsupportedManifest {
                    manifest: "rust/tests/fixtures/hello/Cargo.toml".to_owned(),
                    reason: format!("{package:?} has no version to widen"),
                });
            }
        }
    }
    match matches.len() {
        0 => Err(BumpError::NotFound {
            manifest: "rust/tests/fixtures/hello/Cargo.toml".to_owned(),
            package: package.to_owned(),
        }),
        1 => {
            let table = cargo_table_at_mut(&mut doc, &matches[0]).ok_or_else(|| {
                BumpError::UnsupportedManifest {
                    manifest: "rust/tests/fixtures/hello/Cargo.toml".to_owned(),
                    reason: format!("{package:?} has no version to widen"),
                }
            })?;
            cargo_set_version(table, package, &new).ok_or_else(|| {
                BumpError::UnsupportedManifest {
                    manifest: "rust/tests/fixtures/hello/Cargo.toml".to_owned(),
                    reason: format!("{package:?} has no version to widen"),
                }
            })?;
            Ok(doc.to_string())
        }
        count => Err(BumpError::Ambiguous {
            manifest: "rust/tests/fixtures/hello/Cargo.toml".to_owned(),
            package: package.to_owned(),
            count,
        }),
    }
}

/// Dependency shapes for `plan_cargo_toml`: registry-owned (widenable)
/// versus fail-closed shapes (explicit typed errors, never guessed).
pub(super) enum CargoDepShape {
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
pub(super) fn cargo_dep_shape(item: &toml_edit::Item) -> CargoDepShape {
    match item {
        toml_edit::Item::Value(toml_edit::Value::String(_)) => CargoDepShape::Registry,
        toml_edit::Item::Value(toml_edit::Value::InlineTable(table)) => cargo_inline_shape(table),
        toml_edit::Item::Table(table) => cargo_table_shape(table),
        _ => CargoDepShape::NoVersion,
    }
}

/// Classifies `package = { ... }` inline tables.
pub(super) fn cargo_inline_shape(table: &toml_edit::InlineTable) -> CargoDepShape {
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
pub(super) fn cargo_table_shape(table: &toml_edit::Table) -> CargoDepShape {
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
pub(super) fn cargo_dependency_table_paths(doc: &toml_edit::DocumentMut) -> Vec<Vec<String>> {
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
pub(super) fn cargo_table_at<'a>(
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
pub(super) fn cargo_table_at_mut<'a>(
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
pub(super) fn cargo_set_version(table: &mut toml_edit::Table, package: &str, new: &str) -> bool {
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
