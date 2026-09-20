//! Scope entry points for resolution.
//!
//! Split from `super` (`resolve.rs`): owns [`resolve`] and
//! [`resolve_for_test`], which compose classification
//! ([`super::classify`]) with ownership and test-mapping queries
//! ([`super::query`], [`super::test_map`]). The classification,
//! query-plumbing, run/deploy, and test-mapping domains live in
//! sibling submodules; shared types stay in `super` and every public
//! path is preserved via re-exports.

use std::path::Path;

use dx_process::Scope;

use super::{
    classify_scopes, map_owners_to_tests, resolve_file_owners, PackageCache, QueryRunner,
    ResolveError, ResolvedScope,
};

/// Resolves explicit scope positionals into exact Bazel targets.
///
/// Empty input selects the repository scope (`//...`). Label-only input
/// passes through in order. Once any file or directory resolves, every
/// target is canonicalized, deduplicated, and bytewise sorted under
/// [`Scope::ResolvedOwners`].
pub fn resolve(
    scopes: &[String],
    workspace: &Path,
    runner: &dyn QueryRunner,
) -> Result<ResolvedScope, ResolveError> {
    if scopes.is_empty() {
        return Ok(ResolvedScope {
            scope: Scope::Repository,
            targets: vec!["//...".to_owned()],
        });
    }
    let mut cache = PackageCache::default();
    let classified = classify_scopes(scopes, workspace, &mut cache)?;
    if classified.files.is_empty() && classified.patterns.is_empty() {
        return Ok(ResolvedScope {
            scope: Scope::Labels(classified.labels.clone()),
            targets: classified.labels,
        });
    }
    let mut targets = classified.labels;
    if !classified.files.is_empty() {
        targets.extend(resolve_file_owners(&classified.files, workspace, runner)?);
    }
    targets.extend(classified.patterns);
    targets.sort();
    targets.dedup();
    Ok(ResolvedScope {
        scope: Scope::ResolvedOwners(targets.clone()),
        targets,
    })
}

/// Resolves test/coverage scope: labels, patterns, and directories pass
/// through, while file scopes map through every direct owner to all
/// transitive reverse-dependent tests.
///
/// Returns the exact Bazel targets for `bazel test` / `bazel coverage`.
/// File owners with no reverse-dependent test are
/// [`ResolveError::NoTests`], never silent success. Directory scopes
/// stay recursive patterns for Bazel to expand; label scopes pass
/// through in order. With no file scope, this matches [`resolve`].
pub fn resolve_for_test(
    scopes: &[String],
    workspace: &Path,
    runner: &dyn QueryRunner,
) -> Result<ResolvedScope, ResolveError> {
    if scopes.is_empty() {
        return Ok(ResolvedScope {
            scope: Scope::Repository,
            targets: vec!["//...".to_owned()],
        });
    }
    let mut cache = PackageCache::default();
    let classified = classify_scopes(scopes, workspace, &mut cache)?;
    if classified.files.is_empty() {
        if classified.patterns.is_empty() {
            return Ok(ResolvedScope {
                scope: Scope::Labels(classified.labels.clone()),
                targets: classified.labels,
            });
        }
        let mut targets = classified.labels;
        targets.extend(classified.patterns);
        targets.sort();
        targets.dedup();
        return Ok(ResolvedScope {
            scope: Scope::ResolvedOwners(targets.clone()),
            targets,
        });
    }
    let file_owners = resolve_file_owners(&classified.files, workspace, runner)?;
    let tests = map_owners_to_tests(&file_owners, workspace, runner)?;
    let mut targets = classified.labels;
    targets.extend(classified.patterns);
    targets.extend(tests);
    targets.sort();
    targets.dedup();
    Ok(ResolvedScope {
        scope: Scope::ResolvedOwners(targets.clone()),
        targets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::{QueryResult, QueryRunner};
    use std::cell::RefCell;
    use std::path::PathBuf;

    /// Scripted query runner: records argv/cwd and replays canned
    /// outputs per expression in call order.
    struct FakeQuery {
        calls: RefCell<Vec<(Vec<String>, PathBuf)>>,
        outputs: RefCell<Vec<QueryResult>>,
    }

    impl FakeQuery {
        fn new(outputs: Vec<QueryResult>) -> Self {
            FakeQuery {
                calls: RefCell::new(Vec::new()),
                outputs: RefCell::new(outputs),
            }
        }

        fn ok(lines: &str) -> QueryResult {
            QueryResult {
                code: Some(0),
                stdout: lines.as_bytes().to_vec(),
                stderr: Vec::new(),
            }
        }

        fn calls(&self) -> Vec<(Vec<String>, PathBuf)> {
            self.calls.borrow().clone()
        }
    }

    impl QueryRunner for FakeQuery {
        fn run_query(&self, argv: &[String], cwd: &Path) -> std::io::Result<QueryResult> {
            self.calls
                .borrow_mut()
                .push((argv.to_vec(), cwd.to_path_buf()));
            Ok(self.outputs.borrow_mut().remove(0))
        }
    }

    /// Query runner that fails the test on any call: directory and
    /// label scopes must resolve without touching Bazel.
    struct NeverQuery;

    // LCOV_EXCL_START - reason: test-only guard; NeverQuery is never called.
    impl QueryRunner for NeverQuery {
        fn run_query(&self, _argv: &[String], _cwd: &Path) -> std::io::Result<QueryResult> {
            panic!("directory and label scopes must not run queries");
        }
    }
    // LCOV_EXCL_STOP - reason: end of NeverQuery exclusion.

    fn scopes(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    fn temp_workspace(name: &str) -> dx_test_scratch::TempDir {
        dx_test_scratch::scratch(&format!("dx-resolve-test-{name}-"))
    }

    fn write(workspace: &Path, rel: &str, text: &str) {
        let full = workspace.join(rel);
        std::fs::create_dir_all(full.parent().expect("parent")).expect("parent dir");
        std::fs::write(full, text).expect("write file");
    }

    #[test]
    fn test_scope_maps_files_to_tests_and_keeps_patterns() {
        let scratch = temp_workspace("test-scope");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        let query = FakeQuery::new(vec![
            FakeQuery::ok("//pkg:lib\n"),
            FakeQuery::ok("//pkg:unit\n"),
        ]);
        let got = resolve_for_test(&scopes(&["pkg/a.py", "//other/..."]), &workspace, &query)
            .expect("resolve");
        assert_eq!(
            got.targets,
            scopes(&["//other/...", "//pkg:unit"]),
            "owners are replaced by mapped tests"
        );
        assert_eq!(
            query.calls().len(),
            2,
            "one ownership query plus one mapping"
        );
    }

    #[test]
    fn test_scope_without_files_matches_plain_resolve() {
        let scratch = temp_workspace("test-scope-labels");
        let workspace = scratch.path().to_path_buf();
        let query = NeverQuery;
        let got = resolve_for_test(&scopes(&["//a:one", "//b/..."]), &workspace, &query)
            .expect("resolve");
        assert_eq!(got.targets, scopes(&["//a:one", "//b/..."]));
        assert_eq!(got.scope, Scope::Labels(scopes(&["//a:one", "//b/..."])));
    }

    #[test]
    fn test_scope_dir_only_resolves_without_query() {
        let scratch = temp_workspace("test-scope-dir");
        let workspace = scratch.path().to_path_buf();
        std::fs::create_dir_all(workspace.join("app")).expect("dir");
        let query = NeverQuery;
        let got = resolve_for_test(&scopes(&["app"]), &workspace, &query).expect("dir");
        assert_eq!(got.targets, scopes(&["//app/..."]));
    }
}
