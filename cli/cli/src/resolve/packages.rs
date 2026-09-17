//! Memoized Bazel package-marker probes (issue #236).
//!
//! Split from [`super::classify`] (`classify.rs`): owns the
//! memoized [`PackageCache`] plus its [`PACKAGE_FILES`] marker table.
//! Re-exported through `super` so the public path stays
//! `crate::resolve::PackageCache` via the re-exports below. Shares
//! [`super::ResolveError`] with the classification
//! ([`super::classify`]), entry ([`super::entry`]), and run/deploy
//! ([`super::run_deploy`]) domains; classification calls back in via
//! [`PackageCache::file_label`] and every resolver call owns one cache.

use std::path::Path;

use super::ResolveError;

/// Package markers: a directory is a Bazel package when it holds one.
/// Only existence is probed; contents are never read.
const PACKAGE_FILES: [&str; 2] = ["BUILD.bazel", "BUILD"];

/// Memoized package-marker probes for one resolver call. Only ancestor
/// directories of input files are ever probed: label and directory
/// scopes never trigger package walks, and each directory's marker
/// existence is probed at most once no matter how many files share the
/// enclosing package.
#[derive(Default)]
pub(crate) struct PackageCache {
    /// Directory ("" for the workspace root) to marker presence.
    is_package: std::collections::HashMap<String, bool>,
}

impl PackageCache {
    /// Reports whether `dir` holds a package marker, probing once.
    fn is_package(&mut self, workspace: &Path, dir: &str) -> bool {
        if let Some(hit) = self.is_package.get(dir) {
            return *hit;
        }
        let base = if dir.is_empty() {
            workspace.to_path_buf()
        } else {
            workspace.join(dir)
        };
        let found = PACKAGE_FILES
            .iter()
            .any(|marker| std::fs::symlink_metadata(base.join(marker)).is_ok());
        self.is_package.insert(dir.to_owned(), found);
        found
    }

    /// Finds the nearest enclosing Bazel package for a normalized relative
    /// file path by walking from the parent directory up to the workspace
    /// root. Returns `(package, path_in_package)`, where the root package
    /// is `""`. Returns `None` when no directory in the chain holds a
    /// package marker.
    fn enclosing(&mut self, workspace: &Path, rel: &str) -> Option<(String, String)> {
        let mut dir = match rel.rfind('/') {
            Some(index) => &rel[..index],
            None => "",
        };
        loop {
            if self.is_package(workspace, dir) {
                let in_package = if dir.is_empty() {
                    rel.to_owned()
                } else {
                    rel[dir.len() + 1..].to_owned()
                };
                return Some((dir.to_owned(), in_package));
            }
            if dir.is_empty() {
                return None;
            }
            dir = match dir.rfind('/') {
                Some(index) => &dir[..index],
                None => "",
            };
        }
    }

    /// Maps a normalized relative file path to its source label through
    /// the nearest enclosing package (`pkg/src/deep/a.py` to
    /// `//pkg:src/deep/a.py`, root files to `//:file`). Files with no
    /// enclosing package are [`ResolveError::NotAPackage`], never an
    /// invalid label: Bazel file labels require a package.
    pub(crate) fn file_label(
        &mut self,
        workspace: &Path,
        rel: &str,
        scope: &str,
    ) -> Result<String, ResolveError> {
        match self.enclosing(workspace, rel) {
            Some((package, in_package)) => {
                if package.is_empty() {
                    Ok(format!("//:{in_package}"))
                } else {
                    Ok(format!("//{package}:{in_package}"))
                }
            }
            None => Err(ResolveError::NotAPackage {
                scope: scope.to_owned(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_workspace(name: &str) -> dx_test_scratch::TempDir {
        dx_test_scratch::scratch(&format!("dx-resolve-packages-{name}-"))
    }

    fn write(workspace: &Path, rel: &str, text: &str) {
        let full = workspace.join(rel);
        std::fs::create_dir_all(full.parent().expect("parent")).expect("parent dir");
        std::fs::write(full, text).expect("write file");
    }

    #[test]
    fn file_label_uses_nearest_enclosing_package() {
        let scratch = temp_workspace("nearest");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/sub/BUILD.bazel", "");
        let mut cache = PackageCache::default();
        assert_eq!(
            cache
                .file_label(&workspace, "pkg/sub/a.py", "pkg/sub/a.py")
                .expect("label"),
            "//pkg/sub:a.py",
        );
        assert_eq!(
            cache
                .file_label(&workspace, "pkg/a.py", "pkg/a.py")
                .expect("label"),
            "//pkg:a.py",
        );
    }

    #[test]
    fn file_label_maps_root_and_reports_missing_package() {
        let scratch = temp_workspace("root-missing");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "BUILD.bazel", "");
        write(&workspace, "top.py", "x = 1\n");
        let mut cache = PackageCache::default();
        assert_eq!(
            cache
                .file_label(&workspace, "top.py", "top.py")
                .expect("label"),
            "//:top.py",
        );
        let scratch = temp_workspace("missing");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "pkg/a.py", "x = 1\n");
        let mut cache = PackageCache::default();
        assert!(matches!(
            cache.file_label(&workspace, "pkg/a.py", "pkg/a.py"),
            Err(ResolveError::NotAPackage { .. })
        ));
    }
}
