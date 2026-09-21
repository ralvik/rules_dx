//! Consumer `.bazelrc` preset fragment rendering.
//!
//! Single source for the vendored execution preset consumed via
//! `dx update` (regenerate) and `dx update --check` (stale gate).
//! The rendered bytes must stay byte-identical to
//! `tools/bazelrc/src/lib.rs::render_fragment` (the repository's own
//! `tools/bazelrc/preset.bazelrc` is the snapshot proving parity via
//! `//:preset_parity_test`); a drift in either inventory fails the
//! parity test, not silently.
//!
//! Version and consumer provenance live in the pin constants plus
//! `preset_tests.bzl` file checks, not in the fragment header (minimal
//! `GENERATED` plus regenerate command only).
//! See: `tools/bazelrc/preset_tests.bzl`.

use std::collections::BTreeSet;
use std::path::Path;

/// Bazel pin tracked by the preset (`PRESET_BAZEL_VERSION` in
/// `tools/bazelrc/src/lib.rs`; must equal `.bazelversion`).
pub const PRESET_BAZEL_VERSION: &str = "9.2.0";

/// Reviewed upstream-derived execution flags (mirrors `UPSTREAM_FLAGS`).
const UPSTREAM_FLAGS: [&str; 3] = [
    "common --enable_bzlmod",
    "build --verbose_failures",
    "test --test_output=errors",
];

/// Owned coverage flags (mirrors `EXTRA_PRESETS["coverage"]`).
/// Why no `COVERAGE_GCOV_PATH` pin: coverage tools resolve per-host via
/// the C++ toolchain, never an ambient `/usr/bin/*` path.
/// See: `tools/bazelrc/src/lib.rs`.
const COVERAGE_FLAGS: [&str; 4] = [
    "coverage --test_env=GENERATE_LLVM_LCOV=1",
    "coverage --combined_report=lcov",
    "coverage --test_tag_filters=-no-coverage",
    "coverage --instrumentation_filter=^//",
];

/// Owned build profiles (mirrors `BUILD_PROFILES`).
const BUILD_PROFILES: [&str; 3] = [
    "build:dx_debug --compilation_mode=dbg",
    "build:dx_dev --compilation_mode=fastbuild",
    "build:dx_release --compilation_mode=opt",
];

/// Renders the preset fragment byte-identical to `tools/bazelrc/src/lib.rs`.
pub fn render_preset_fragment() -> String {
    let mut lines = vec![
        "# Vendored Bazel execution preset -- GENERATED, do not edit.".to_owned(),
        "# Regenerate: `bazel run //tools/bazelrc:preset.update`.".to_owned(),
    ];
    lines.extend(UPSTREAM_FLAGS.iter().map(|s| (*s).to_owned()));
    lines.push("# Owned extra_presets group: coverage.".to_owned());
    lines.extend(COVERAGE_FLAGS.iter().map(|s| (*s).to_owned()));
    lines.push(
        "# Owned build profiles (issue #177; See: docs/decisions/0021-build-profiles.md)."
            .to_owned(),
    );
    lines.extend(BUILD_PROFILES.iter().map(|s| (*s).to_owned()));
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Flag lines owned by the preset (non-comment, non-empty).
fn rendered_flag_lines(rendered: &str) -> BTreeSet<String> {
    rendered
        .lines()
        .filter_map(|line| {
            if line.is_empty() || line.starts_with('#') {
                None
            } else {
                Some(line.to_owned())
            }
        })
        .collect()
}

/// Root `.bazelrc` lines duplicating preset flag lines.
///
/// Project overrides stay explicit only for non-preset flags; `import`,
/// `try-import`, comments, and blanks are ignored (mirrors
/// `_owned_collisions`).
pub fn owned_collisions_in_content(root_content: &str, rendered: &str) -> Vec<String> {
    let rendered_lines = rendered_flag_lines(rendered);
    let mut collisions = Vec::new();
    for raw in root_content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("import ") || line.starts_with("try-import ") {
            continue;
        }
        if rendered_lines.contains(line) {
            collisions.push(line.to_owned());
        }
    }
    collisions.sort();
    collisions.dedup();
    collisions
}

/// Preset file locations under a workspace root.
pub fn preset_paths(workspace: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    (
        workspace.join(".bazelrc"),
        workspace.join("tools/bazelrc/preset.bazelrc"),
    )
}

/// Preset freshness failure.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PresetError {
    /// Root `.bazelrc` duplicates preset-owned lines.
    #[error("root .bazelrc duplicates preset lines; reconcile (remove owned duplicates, keep project overrides explicit): {lines}")]
    OwnedCollision {
        /// Duplicated lines, sorted.
        lines: String,
    },
    /// Fragment is missing or differs from the rendered inventory.
    #[error("preset stale: {detail}")]
    Stale {
        /// Human detail plus unified diff.
        detail: String,
    },
    /// Fragment cannot be written.
    #[error("cannot write {path}: {detail}")]
    Unwritable {
        /// Workspace-relative path.
        path: String,
        /// I/O detail.
        detail: String,
    },
}

/// Simple unified diff (checked-in vs regenerated) for flag-diff review.
fn unified_diff(checked_in: &str, regenerated: &str) -> String {
    let old: Vec<&str> = checked_in.lines().collect();
    let new: Vec<&str> = regenerated.lines().collect();
    let mut out = vec!["--- checked-in".to_owned(), "+++ regenerated".to_owned()];
    let max = old.len().max(new.len());
    for i in 0..max {
        let o = old.get(i).copied().unwrap_or("");
        let n = new.get(i).copied().unwrap_or("");
        if o != n {
            if i < old.len() {
                out.push(format!("-{o}"));
            }
            if i < new.len() {
                out.push(format!("+{n}"));
            }
        }
    }
    out.join("\n")
}

/// Checks preset freshness without mutating (for `dx update --check`).
///
/// Returns `Ok(())` when clean, `Err(PresetError::Stale)` when the
/// fragment is missing or differs, `Err(PresetError::OwnedCollision)`
/// when the root duplicates preset lines.
pub fn check_preset(workspace: &Path) -> Result<(), PresetError> {
    let (root_path, fragment_path) = preset_paths(workspace);
    let rendered = render_preset_fragment();
    let rendered_lines = rendered_flag_lines(&rendered);
    // Collision gate first (mirrors `tools/bazelrc/src/lib.rs` ordering).
    if let Ok(root_content) = std::fs::read_to_string(&root_path) {
        let mut collisions = Vec::new();
        for raw in root_content.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with("import ") || line.starts_with("try-import ") {
                continue;
            }
            if rendered_lines.contains(line) {
                collisions.push(line.to_owned());
            }
        }
        collisions.sort();
        collisions.dedup();
        if !collisions.is_empty() {
            return Err(PresetError::OwnedCollision {
                lines: collisions.join(", "),
            });
        }
    }
    let checked_in = std::fs::read_to_string(&fragment_path).map_err(|e| PresetError::Stale {
        detail: format!(
            "tools/bazelrc/preset.bazelrc is stale (missing); run `dx update` to regenerate ({e})"
        ),
    })?;
    if checked_in == rendered {
        Ok(())
    } else {
        let diff = unified_diff(&checked_in, &rendered);
        Err(PresetError::Stale {
            detail: format!(
                "tools/bazelrc/preset.bazelrc is stale; run `dx update` to regenerate\n{diff}"
            ),
        })
    }
}

/// Regenerates the preset fragment atomically (for `dx update`).
///
/// Preserves project overrides (`/.bazelrc` untouched) and `user.bazelrc`
/// (still `try-import`ed last); fails closed on owned collisions.
/// Creates `tools/bazelrc/` when absent (consumer workspaces).
pub fn update_preset(workspace: &Path) -> Result<(), PresetError> {
    let (root_path, fragment_path) = preset_paths(workspace);
    let rendered = render_preset_fragment();
    let rendered_lines = rendered_flag_lines(&rendered);
    if let Ok(root_content) = std::fs::read_to_string(&root_path) {
        let mut collisions = Vec::new();
        for raw in root_content.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with("import ") || line.starts_with("try-import ") {
                continue;
            }
            if rendered_lines.contains(line) {
                collisions.push(line.to_owned());
            }
        }
        collisions.sort();
        collisions.dedup();
        if !collisions.is_empty() {
            return Err(PresetError::OwnedCollision {
                lines: collisions.join(", "),
            });
        }
    }
    if let Some(parent) = fragment_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| PresetError::Unwritable {
            path: "tools/bazelrc/preset.bazelrc".to_owned(),
            detail: e.to_string(),
        })?;
    }
    dx_atomic_fs::write_atomic(&fragment_path, rendered.as_bytes()).map_err(|e| {
        PresetError::Unwritable {
            path: "tools/bazelrc/preset.bazelrc".to_owned(),
            detail: e.to_string(),
        }
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragment_matches_preset_inventory() {
        let rendered = render_preset_fragment();
        assert!(rendered.contains("GENERATED, do not edit"));
        assert!(rendered.contains("# Regenerate: `bazel run //tools/bazelrc:preset.update`."));
        assert!(!rendered.contains("Version-matched to Bazel"));
        assert!(!rendered.contains("Consumer refresh:"));
        assert!(!rendered.contains("Upstream-derived flags"));
        // Exact inventory counts (mirrors `tools/bazelrc/src/lib.rs` len pins).
        assert_eq!(UPSTREAM_FLAGS.len(), 3);
        assert_eq!(COVERAGE_FLAGS.len(), 4);
        assert_eq!(BUILD_PROFILES.len(), 3);
        // Flags present.
        for flag in UPSTREAM_FLAGS
            .iter()
            .chain(COVERAGE_FLAGS.iter())
            .chain(BUILD_PROFILES.iter())
        {
            assert!(rendered.contains(flag), "missing {flag}");
        }
        // Trailing newline, no extra blank line.
        assert!(rendered.ends_with('\n'));
        assert!(!rendered.ends_with("\n\n"));
    }

    #[test]
    fn dx_stamp_tracks_single_version() {
        // No new pin file: the stamp must equal the delivered version.
        assert_eq!(crate::version::DX_VERSION, "0.0.0");
        assert_eq!(PRESET_BAZEL_VERSION, "9.2.0");
    }

    #[test]
    fn collisions_ignore_imports_comments_and_blanks() {
        let rendered = render_preset_fragment();
        let root = "# comment\n\nimport %workspace%/tools/bazelrc/preset.bazelrc\ntry-import %workspace%/user.bazelrc\nbuild --verbose_failures\n";
        let collisions = owned_collisions_in_content(root, &rendered);
        assert_eq!(collisions, vec!["build --verbose_failures".to_owned()]);
        let clean = "import %workspace%/tools/bazelrc/preset.bazelrc\nbuild --@rules_rust//rust/settings:extra_rustc_flags=--deny=warnings\n";
        assert!(owned_collisions_in_content(clean, &rendered).is_empty());
    }

    #[test]
    fn check_and_update_round_trip_in_scratch() {
        let scratch = dx_test_scratch::scratch("dx-preset-");
        let root = scratch.path().to_path_buf();
        // Minimal consumer workspace: MODULE marker, root .bazelrc with import.
        std::fs::write(root.join("MODULE.bazel"), "").expect("module");
        std::fs::write(
            root.join(".bazelrc"),
            "import %workspace%/tools/bazelrc/preset.bazelrc\ntry-import %workspace%/user.bazelrc\n",
        )
        .expect("bazelrc");
        // Missing fragment is stale.
        assert!(matches!(
            check_preset(&root),
            Err(PresetError::Stale { .. })
        ));
        update_preset(&root).expect("update creates fragment");
        assert_eq!(check_preset(&root), Ok(()));
        // Dirty fragment is stale with a diff.
        let (_, fragment) = preset_paths(&root);
        std::fs::write(&fragment, "# dirty\n").expect("dirty");
        match check_preset(&root) {
            Err(PresetError::Stale { detail }) => {
                assert!(detail.contains("stale"));
                assert!(detail.contains("checked-in"));
            }
            other => panic!("want Stale, got {other:?}"),
        }
        update_preset(&root).expect("update fixes dirty");
        assert_eq!(check_preset(&root), Ok(()));
        // Duplicates fail closed on both paths.
        std::fs::write(
            root.join(".bazelrc"),
            "import %workspace%/tools/bazelrc/preset.bazelrc\ncommon --enable_bzlmod\n",
        )
        .expect("collision");
        assert!(matches!(
            check_preset(&root),
            Err(PresetError::OwnedCollision { .. })
        ));
        assert!(matches!(
            update_preset(&root),
            Err(PresetError::OwnedCollision { .. })
        ));
        scratch.close().expect("cleanup");
    }
}
