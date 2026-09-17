//! Filesystem inventory for `dx clean` (issue #236).
//!
//! Split from `super` (`lib.rs`): owns [`CollectedInventory`] (with
//! [`CollectedInventory::prune_inputs`] and [`CollectedInventory::plan`]),
//! [`walk_filtered`] (ignore-aware workspace walks, issue #223), and
//! [`collect_inventory`] (validating every setup record against the
//! [`dx_setup`] pair identity). Re-exported through `super` so the
//! public paths stay `dx_clean::{CollectedInventory, walk_filtered,
//! collect_inventory}`. Distinct from the `flags` module (frozen flag
//! shapes), the `records` module (setup-record validation), the
//! `planning` module (pure prune selection), and the live/apply/bytes
//! modules.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use dx_setup::{
    GenerationId, CURRENT_LINK_NAME, CURRENT_STAGE_NAME, ENVIRONMENT_LINK_NAME,
    GENERATED_LINK_NAME, SETUPS_DIR_NAME,
};

use super::planning::{plan_prune, CleanPlan, GenerationView, PruneInputs};
use super::records::{validate_record, GenerationKind, SetupRecordView};
use super::CleanError;

/// Owned filesystem inventory behind [`PruneInputs`]: validated setup
/// records, digest-shaped generations, the current selection, and refused
/// unmanaged names. Active (in-use) sets combine caller-provided hexes
/// with the process scan; unknown-live entries prune
/// exactly as the pure plan selects.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CollectedInventory {
    /// Validated setup records (see [`validate_record`]).
    pub records: Vec<SetupRecordView>,
    /// Digest-shaped generations present under the managed roots.
    pub generations: Vec<GenerationView>,
    /// Currently selected setup record hex, if any.
    pub current_hex: Option<String>,
    /// Setup-record hexes still referenced by an active process.
    pub active_setup_hexes: Vec<String>,
    /// Generation hexes still referenced by an active process.
    pub active_generation_hexes: Vec<String>,
    /// Directory names under the managed roots that are not
    /// digest-shaped (or fail record validation): refused, never deleted.
    pub unmanaged_names: Vec<String>,
}

impl CollectedInventory {
    /// Borrows this inventory as the pure planner input.
    pub fn prune_inputs(&self) -> PruneInputs<'_> {
        PruneInputs {
            records: &self.records,
            generations: &self.generations,
            current_hex: self.current_hex.as_deref(),
            active_setup_hexes: &self.active_setup_hexes,
            active_generation_hexes: &self.active_generation_hexes,
            unmanaged_names: &self.unmanaged_names,
        }
    }

    /// Selects the prune set from this inventory.
    pub fn plan(&self) -> CleanPlan {
        plan_prune(self.prune_inputs())
    }
}

/// Reads directory entry names under `dir`, sorted ascending. A missing
/// directory contributes nothing (first selection has no generations
/// yet); any other listing failure reports through [`CleanError`].
///
/// Implemented over [`walkdir::WalkDir`] at depth 1 (issue #223): the
/// managed `.dx` roots stay a direct-children listing with identical
/// semantics to the historical `read_dir` loop (sorted names,
/// non-UTF8 placeholder), while recursive and ignore-aware traversal
/// lives in [`walk_filtered`].
fn entry_names(dir: &Path) -> Result<Vec<String>, CleanError> {
    if !dir.exists() {
        match fs::read_dir(dir) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => {
                return Err(CleanError::Install {
                    reason: format!("cannot list {}: {e}", dir.display()),
                });
            }
            Ok(_) => {}
        }
    }
    let mut names = Vec::new();
    for entry in walkdir::WalkDir::new(dir).max_depth(1).min_depth(1) {
        let entry = entry.map_err(|e| CleanError::Install {
            reason: format!("cannot list {}: {e}", dir.display()),
        })?;
        if let Some(name) = entry.file_name().to_str() {
            names.push(name.to_owned());
        } else {
            // Non-UTF8 names are unmanaged by construction: they
            // can never be digest-shaped. Record a placeholder so
            // the plan refuses something rather than ignoring it.
            names.push("<non-utf8-name>".to_owned());
        }
    }
    names.sort();
    Ok(names)
}

/// Recursively walks `root` honoring `.gitignore` and related ignore
/// files (issue #223), skipping hidden entries and git-ignored paths
/// via the [`ignore`] crate (ripgrep family), with additional
/// caller-supplied glob exclusions via [`globset`].
///
/// `exclude_globs` are gitignore-style globs matched against paths
/// relative to `root` (for example `["*.log", "target/**"]`); invalid
/// globs fail through [`CleanError::Install`]. The returned paths are
/// sorted ascending for deterministic plans. A missing root walks
/// empty; any other traversal failure reports through [`CleanError`].
pub fn walk_filtered(root: &Path, exclude_globs: &[String]) -> Result<Vec<PathBuf>, CleanError> {
    let mut builder = globset::GlobSetBuilder::new();
    for pattern in exclude_globs {
        let glob = globset::Glob::new(pattern).map_err(|e| CleanError::Install {
            reason: format!("invalid exclude glob {pattern:?}: {e}"),
        })?;
        builder.add(glob);
    }
    let excludes = builder.build().map_err(|e| CleanError::Install {
        reason: format!("invalid exclude globs: {e}"),
    })?;
    if fs::read_dir(root).is_err_and(|e| e.kind() == io::ErrorKind::NotFound) {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false)
        .parents(true)
        .build();
    for entry in walker {
        let entry = entry.map_err(|e| CleanError::Install {
            reason: format!("cannot walk {}: {e}", root.display()),
        })?;
        let path = entry.path().to_path_buf();
        if path == root {
            continue;
        }
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if excludes.is_match(relative) {
            continue;
        }
        paths.push(path);
    }
    paths.sort();
    Ok(paths)
}

/// Extracts a generation digest from a record link target: the final path
/// component must be digest-shaped.
fn generation_hex_from_link_target(target: &Path) -> Option<String> {
    target
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|text| GenerationId::new(text).is_ok())
        .map(str::to_owned)
}

/// Collects the clean inventory for `workspace_root`, validating every
/// setup record against the [`dx_setup`] pair identity. Present-but-
/// malformed current state fails closed ([`CleanError::CurrentInvalid`],
/// nothing prunable); digest-spoofed or otherwise invalid records join
/// the refused unmanaged set (never pruned, never adopted).
pub fn collect_inventory(
    workspace_root: &Path,
    active_setup_hexes: &[String],
    active_generation_hexes: &[String],
) -> Result<CollectedInventory, CleanError> {
    if !workspace_root.is_dir() {
        return Err(CleanError::WorkspaceRoot {
            path: workspace_root.to_path_buf(),
        });
    }
    let dx_dir = workspace_root.join(".dx");
    let setups_dir = dx_dir.join(SETUPS_DIR_NAME);
    let mut inventory = CollectedInventory {
        active_setup_hexes: active_setup_hexes.to_vec(),
        active_generation_hexes: active_generation_hexes.to_vec(),
        ..CollectedInventory::default()
    };

    // Current selection: absent selects nothing; anything present but not
    // a digest-shaped symlink target fails closed.
    let current_link = setups_dir.join(CURRENT_LINK_NAME);
    match fs::symlink_metadata(&current_link) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(CleanError::CurrentInvalid {
                reason: format!("cannot inspect {}: {e}", current_link.display()),
            });
        }
        Ok(meta) => {
            if !meta.file_type().is_symlink() {
                return Err(CleanError::CurrentInvalid {
                    reason: format!(
                        "{} is not a symlink; refusing to adopt foreign state",
                        current_link.display()
                    ),
                });
            }
            let target = fs::read_link(&current_link).map_err(|e| CleanError::CurrentInvalid {
                reason: format!("cannot read {}: {e}", current_link.display()),
            })?;
            let hex = target
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_owned();
            if GenerationId::new(&hex).is_err() {
                return Err(CleanError::CurrentInvalid {
                    reason: format!(
                        "{} points at {hex:?}, want a setup digest; refusing digest-spoofed state",
                        current_link.display()
                    ),
                });
            }
            inventory.current_hex = Some(hex);
        }
    }

    // Setup records: digest-shaped names validate against their link
    // targets; anything else (including the staged `current.next` pointer
    // and digest-spoofed records) is refused, never pruned.
    //
    // The current pointer itself resolves through the record directory
    // (setup reads links via `current/<link>`), so record validation must
    // read links relative to each record directory, not the pointer.
    for name in entry_names(&setups_dir)? {
        if name == CURRENT_LINK_NAME || name == CURRENT_STAGE_NAME {
            continue;
        }
        if GenerationId::new(&name).is_err() {
            inventory.unmanaged_names.push(name);
            continue;
        }
        let record = setups_dir.join(&name);
        let environment = fs::read_link(record.join(ENVIRONMENT_LINK_NAME))
            .ok()
            .and_then(|target| generation_hex_from_link_target(&target));
        let generated = fs::read_link(record.join(GENERATED_LINK_NAME))
            .ok()
            .and_then(|target| generation_hex_from_link_target(&target));
        match (environment, generated) {
            (Some(environment_hex), Some(generated_hex)) => {
                match validate_record(&name, &environment_hex, &generated_hex) {
                    Ok(view) => inventory.records.push(view),
                    Err(_) => inventory.unmanaged_names.push(name),
                }
            }
            _ => inventory.unmanaged_names.push(name),
        }
    }

    // Generations: digest-shaped names only; anything else is unmanaged.
    for kind in [GenerationKind::Environment, GenerationKind::Generated] {
        let dir = dx_dir.join(kind.dir_name());
        for name in entry_names(&dir)? {
            if GenerationId::new(&name).is_ok() {
                inventory
                    .generations
                    .push(GenerationView { kind, hex: name });
            } else {
                inventory.unmanaged_names.push(name);
            }
        }
    }

    inventory
        .records
        .sort_by(|left, right| left.hex.cmp(&right.hex));
    inventory.generations.sort_by(|left, right| {
        left.kind
            .dir_name()
            .cmp(right.kind.dir_name())
            .then_with(|| left.hex.cmp(&right.hex))
    });
    inventory.unmanaged_names.sort();
    inventory.unmanaged_names.dedup();
    Ok(inventory)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_filtered_skips_gitignored_and_glob_excluded_files() {
        // Issue #223: recursive walks honor `.gitignore` (via the
        // `ignore` crate) plus caller-supplied `globset` exclusions.
        let scratch = dx_test_scratch::scratch("dx-clean-walk-");
        let root = scratch.path();
        fs::write(root.join(".gitignore"), "ignored.txt\n").expect("gitignore");
        fs::write(root.join("ignored.txt"), "skip me").expect("ignored file");
        fs::write(root.join("kept.txt"), "keep me").expect("kept file");
        fs::write(root.join("skip.log"), "glob me").expect("glob file");
        fs::create_dir_all(root.join("sub")).expect("subdir");
        fs::write(root.join("sub").join("nested.txt"), "nested").expect("nested");

        let walked = walk_filtered(root, &[]).expect("walk");
        let names: Vec<String> = walked
            .iter()
            .filter_map(|path| {
                path.strip_prefix(root)
                    .ok()
                    .and_then(|relative| relative.to_str().map(str::to_owned))
            })
            .collect();
        assert!(
            names.iter().any(|name| name == "kept.txt"),
            "kept file must walk: {names:?}"
        );
        assert!(
            names
                .iter()
                .any(|name| name == "sub/nested.txt" || name == "sub\\nested.txt"),
            "nested file must walk: {names:?}"
        );
        assert!(
            !names.iter().any(|name| name == "ignored.txt"),
            "gitignored file must be skipped: {names:?}"
        );

        let filtered = walk_filtered(root, &["*.log".to_owned()]).expect("filtered walk");
        let filtered_names: Vec<String> = filtered
            .iter()
            .filter_map(|path| {
                path.strip_prefix(root)
                    .ok()
                    .and_then(|relative| relative.to_str().map(str::to_owned))
            })
            .collect();
        assert!(
            !filtered_names.iter().any(|name| name == "skip.log"),
            "glob-excluded file must be skipped: {filtered_names:?}"
        );
        assert!(
            filtered_names.iter().any(|name| name == "kept.txt"),
            "kept file must survive glob filtering: {filtered_names:?}"
        );
        // Deterministic order for plans.
        let mut sorted = filtered.clone();
        sorted.sort();
        assert_eq!(filtered, sorted, "walks must be sorted");
    }

    #[test]
    fn walk_filtered_rejects_invalid_globs_and_walks_missing_empty() {
        let missing = std::env::temp_dir().join("dx-clean-missing-walk-root");
        let _ = fs::remove_dir_all(&missing);
        let empty = walk_filtered(&missing, &[]).expect("missing walks empty");
        assert!(empty.is_empty());
        let scratch = dx_test_scratch::scratch("dx-clean-glob-");
        assert!(matches!(
            walk_filtered(scratch.path(), &["[invalid".to_owned()]),
            Err(CleanError::Install { .. })
        ));
    }
}
