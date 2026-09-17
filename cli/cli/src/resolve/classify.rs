//! Scope classification and file-ownership resolution (issue #236).
//!
//! Split from `super` (`resolve.rs`): owns path normalization, the
//! memoized [`PackageCache`], the query-output helpers ([`first_line`],
//! [`parse_owners`]), [`ClassifiedScopes`]/[`classify_scopes`], and
//! [`resolve_file_owners`]. The entry points ([`super::resolve`],
//! [`super::resolve_for_test`], [`super::run_deploy::resolve_run`])
//! stay in `super` and call back in; the query plumbing
//! ([`super::ownership_set_expression`], [`super::run_label_query`])
//! and the test mapping ([`super::test_map::map_owners_to_tests`])
//! stay shared in `super`.

use std::io;
use std::path::{Component, Path};

use super::{ownership_set_expression, run_label_query, QueryRunner, ResolveError};

/// Normalizes a workspace-relative scope path lexically (no filesystem
/// access): drops `.` and empty segments, rejects absolute paths and
/// `..` escapes. An empty result addresses the workspace root.
fn normalize_rel(raw: &str) -> Result<String, ResolveError> {
    if raw.is_empty() {
        return Err(ResolveError::EmptyScope);
    }
    let path = Path::new(raw);
    if path.is_absolute() {
        return Err(ResolveError::OutsideWorkspace {
            scope: raw.to_owned(),
        });
    }
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ResolveError::OutsideWorkspace {
                    scope: raw.to_owned(),
                });
            }
        }
    }
    Ok(parts.join("/"))
}

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

/// Maps a normalized relative directory path to its recursive pattern:
/// the workspace root becomes `//...`, anything else `//path/...`.
/// Recursion is performed by Bazel, never by filesystem traversal.
fn dir_pattern(rel: &str) -> String {
    if rel.is_empty() {
        "//...".to_owned()
    } else {
        format!("//{rel}/...")
    }
}

/// First non-empty stderr line, bounded for diagnostics.
pub(crate) fn first_line(bytes: &[u8]) -> String {
    const LIMIT: usize = 300;
    let text = String::from_utf8_lossy(bytes);
    let line = text.lines().map(str::trim).find(|line| !line.is_empty());
    let line = line.unwrap_or("no Bazel diagnostic");
    if line.len() > LIMIT {
        format!("{}...", &line[..LIMIT])
    } else {
        line.to_owned()
    }
}

/// Parses query stdout into canonical owner labels: trims lines, drops
/// empties, sorts bytewise, and deduplicates.
pub(crate) fn parse_owners(stdout: &[u8], label: &str) -> Result<Vec<String>, ResolveError> {
    let text = std::str::from_utf8(stdout).map_err(|_| ResolveError::QueryFailed {
        label: label.to_owned(),
        detail: "query output is not UTF-8".to_owned(),
    })?;
    let mut owners: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect();
    owners.sort();
    owners.dedup();
    Ok(owners)
}

/// One file scope after classification: the original scope text for
/// diagnostics and the source label through the nearest enclosing
/// package for queries.
pub(crate) struct FileScope {
    /// Original scope positional, for [`ResolveError::NoOwner`].
    pub(crate) scope: String,
    /// Source label addressing the file in query syntax.
    pub(crate) label: String,
}

/// Scope positionals classified once per resolver call and shared by the
/// plain, test, and run paths so they cannot drift apart: labels pass
/// through, directories become recursive patterns without filesystem
/// enumeration, and files carry their source labels for one batched
/// ownership query.
pub(crate) struct ClassifiedScopes {
    /// Main-workspace labels passing through in input order.
    pub(crate) labels: Vec<String>,
    /// File scopes in input order with their source labels.
    pub(crate) files: Vec<FileScope>,
    /// Recursive patterns for directory scopes in input order.
    pub(crate) patterns: Vec<String>,
    /// Original file and directory positionals in input order, for
    /// [`ResolveError::NoRunnable`].
    pub(crate) paths: Vec<String>,
}

/// Classifies every scope positional: main-workspace labels pass
/// through, external and package-relative labels fail, and anything else
/// is a workspace-relative file or directory path. Package-marker walks
/// run only for file scopes through the shared cache, so directory and
/// label scopes never trigger enclosing-package probes.
pub(crate) fn classify_scopes(
    scopes: &[String],
    workspace: &Path,
    cache: &mut PackageCache,
) -> Result<ClassifiedScopes, ResolveError> {
    let mut classified = ClassifiedScopes {
        labels: Vec::new(),
        files: Vec::new(),
        patterns: Vec::new(),
        paths: Vec::new(),
    };
    for raw in scopes {
        if raw.starts_with("//") {
            classified.labels.push(raw.clone());
            continue;
        }
        if raw.starts_with('@') {
            return Err(ResolveError::ExternalScope { scope: raw.clone() });
        }
        if raw.starts_with(':') {
            return Err(ResolveError::RelativeLabel { scope: raw.clone() });
        }
        let rel = normalize_rel(raw)?;
        let entry = workspace.join(&rel);
        let metadata = std::fs::symlink_metadata(&entry).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                ResolveError::PathNotFound { scope: raw.clone() }
            } else {
                ResolveError::QueryFailed {
                    label: cache
                        .file_label(workspace, &rel, raw)
                        .unwrap_or_else(|_| raw.clone()),
                    detail: error.to_string(),
                }
            }
        })?;
        if metadata.is_dir() {
            if rel.chars().any(char::is_control) {
                return Err(ResolveError::UnsupportedName { scope: raw.clone() });
            }
            classified.patterns.push(dir_pattern(&rel));
            classified.paths.push(raw.clone());
        } else if metadata.is_file() {
            if rel.chars().any(char::is_control) {
                return Err(ResolveError::UnsupportedName { scope: raw.clone() });
            }
            let label = cache.file_label(workspace, &rel, raw)?;
            classified.files.push(FileScope {
                scope: raw.clone(),
                label,
            });
            classified.paths.push(raw.clone());
        } else {
            return Err(ResolveError::NotFileOrDir { scope: raw.clone() });
        }
    }
    Ok(classified)
}

/// Resolves every classified file scope to its direct source owners
/// through one bounded `bazel query` invocation over the batched label
/// set. The file content and BUILD text are never read: ownership is a
/// graph fact reported on query stdout. An empty mapping names the first
/// file scope, since per-file attribution is not observable from a union.
pub(crate) fn resolve_file_owners(
    files: &[FileScope],
    workspace: &Path,
    runner: &dyn QueryRunner,
) -> Result<Vec<String>, ResolveError> {
    let labels: Vec<String> = files.iter().map(|file| file.label.clone()).collect();
    let expression = ownership_set_expression(&labels);
    let owners = run_label_query(&expression, workspace, runner)?;
    if owners.is_empty() {
        let first = &files[0];
        return Err(ResolveError::NoOwner {
            file: first.scope.clone(),
            label: first.label.clone(),
        });
    }
    Ok(owners)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::query::quote_label;
    use crate::resolve::{resolve, resolve_for_test, resolve_run};
    use dx_process::Scope;
    use std::cell::RefCell;
    use std::path::PathBuf;

    use crate::resolve::QueryResult;

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
        fn run_query(&self, argv: &[String], cwd: &Path) -> io::Result<QueryResult> {
            self.calls
                .borrow_mut()
                .push((argv.to_vec(), cwd.to_path_buf()));
            Ok(self.outputs.borrow_mut().remove(0))
        }
    }

    /// Query runner that fails the test on any call: directory and label
    /// scopes must resolve without touching Bazel.
    struct NeverQuery;

    // LCOV_EXCL_START - reason: test-only guard; NeverQuery is never called.
    impl QueryRunner for NeverQuery {
        fn run_query(&self, _argv: &[String], _cwd: &Path) -> io::Result<QueryResult> {
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

    /// Test-only one-shot label lookup without a shared cache.
    fn file_label(workspace: &Path, rel: &str, scope: &str) -> Result<String, ResolveError> {
        PackageCache::default().file_label(workspace, rel, scope)
    }

    #[test]
    fn empty_scope_selects_repository() {
        let query = NeverQuery;
        let scratch = temp_workspace("empty");
        let workspace = scratch.path().to_path_buf();
        let got = resolve(&[], &workspace, &query).expect("resolve");
        assert_eq!(got.scope, Scope::Repository);
        assert_eq!(got.targets, scopes(&["//..."]));
    }

    #[test]
    fn label_only_scopes_pass_through_in_order() {
        let query = NeverQuery;
        let scratch = temp_workspace("labels");
        let workspace = scratch.path().to_path_buf();
        let input = scopes(&["//b/...", "//a:one", "@repo//c/..."]);
        let err = resolve(&input, &workspace, &query).expect_err("external must fail");
        assert_eq!(
            err,
            ResolveError::ExternalScope {
                scope: "@repo//c/...".to_owned(),
            }
        );
        let input = scopes(&["//b/...", "//a:one"]);
        let got = resolve(&input, &workspace, &query).expect("resolve");
        assert_eq!(got.scope, Scope::Labels(scopes(&["//b/...", "//a:one"])));
        assert_eq!(got.targets, scopes(&["//b/...", "//a:one"]));
    }

    #[test]
    fn relative_labels_fail_with_guidance() {
        let query = NeverQuery;
        let scratch = temp_workspace("relative");
        let workspace = scratch.path().to_path_buf();
        let err = resolve(&scopes(&[":corpus"]), &workspace, &query).expect_err("relative");
        assert_eq!(
            err,
            ResolveError::RelativeLabel {
                scope: ":corpus".to_owned(),
            }
        );
        assert!(err.to_string().contains("//"));
    }

    #[test]
    fn file_resolves_through_single_rule_constrained_query() {
        let scratch = temp_workspace("file");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("//pkg:lib\n//pkg:lib\n//pkg:extra\n")]);
        let got = resolve(&scopes(&["pkg/a.py"]), &workspace, &query).expect("resolve");
        assert_eq!(
            got.targets,
            scopes(&["//pkg:extra", "//pkg:lib"]),
            "owners are sorted and deduplicated"
        );
        assert_eq!(
            got.scope,
            Scope::ResolvedOwners(scopes(&["//pkg:extra", "//pkg:lib"]))
        );
        let calls = query.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, workspace, "query runs in the workspace");
        assert_eq!(
            calls[0].0,
            scopes(&[
                "bazel",
                "--nohome_rc",
                "--nosystem_rc",
                "query",
                "--",
                "kind('rule', rdeps(//..., set(\"//pkg:a.py\"), 1))",
            ])
        );
    }

    #[test]
    fn file_argv_quotes_spaces_and_special_characters() {
        let scratch = temp_workspace("quoting");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/my file.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("//pkg:lib\n")]);
        resolve(&scopes(&["pkg/my file.py"]), &workspace, &query).expect("resolve");
        let calls = query.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].0.last().expect("expression"),
            "kind('rule', rdeps(//..., set(\"//pkg:my file.py\"), 1))"
        );
        assert_eq!(quote_label("//pkg:a\"b\\c"), "\"//pkg:a\\\"b\\\\c\"");
    }

    #[test]
    fn multiple_files_share_one_bounded_query() {
        let scratch = temp_workspace("batch");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        write(&workspace, "pkg/b.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("//pkg:lib\n//pkg:extra\n")]);
        let got = resolve(&scopes(&["pkg/b.py", "pkg/a.py"]), &workspace, &query).expect("resolve");
        assert_eq!(got.targets, scopes(&["//pkg:extra", "//pkg:lib"]));
        let calls = query.calls();
        assert_eq!(calls.len(), 1, "one bounded query per resolver call");
        assert_eq!(
            calls[0].0.last().expect("expression"),
            "kind('rule', rdeps(//..., set(\"//pkg:a.py\" \"//pkg:b.py\"), 1))"
        );
    }

    #[test]
    fn empty_batch_mapping_names_the_first_file() {
        let scratch = temp_workspace("batch-empty");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        write(&workspace, "pkg/b.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("\n")]);
        let err =
            resolve(&scopes(&["pkg/a.py", "pkg/b.py"]), &workspace, &query).expect_err("orphans");
        assert_eq!(
            err,
            ResolveError::NoOwner {
                file: "pkg/a.py".to_owned(),
                label: "//pkg:a.py".to_owned(),
            }
        );
        assert_eq!(query.calls().len(), 1);
    }

    #[test]
    fn file_after_a_packaged_file_without_package_fails_before_query() {
        let scratch = temp_workspace("batch-no-package");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        write(&workspace, "docs/guide.md", "# guide\n");
        let query = NeverQuery;
        let err = resolve(&scopes(&["pkg/a.py", "docs/guide.md"]), &workspace, &query)
            .expect_err("no package");
        assert_eq!(
            err,
            ResolveError::NotAPackage {
                scope: "docs/guide.md".to_owned(),
            }
        );
    }

    #[test]
    fn root_file_maps_to_root_package_label() {
        let scratch = temp_workspace("root-file");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "BUILD.bazel", "");
        write(&workspace, "top.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("//:lib\n")]);
        let got = resolve(&scopes(&["top.py"]), &workspace, &query).expect("resolve");
        assert_eq!(got.targets, scopes(&["//:lib"]));
        let calls = query.calls();
        assert!(calls[0]
            .0
            .last()
            .expect("expression")
            .contains("\"//:top.py\""));
    }

    #[test]
    fn build_file_text_is_never_consulted() {
        // The BUILD file names no target textually: `srcs` hide behind a
        // glob and a comment points at a decoy owner. Resolution still
        // succeeds because ownership comes only from query stdout.
        let scratch = temp_workspace("build-text");
        let workspace = scratch.path().to_path_buf();
        write(
            &workspace,
            "pkg/BUILD.bazel",
            "# owner: //decoy:not_real\nmy_files = glob([\"*.py\"])\n",
        );
        write(&workspace, "pkg/a.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("//pkg:real\n")]);
        let got = resolve(&scopes(&["pkg/a.py"]), &workspace, &query).expect("resolve");
        assert_eq!(got.targets, scopes(&["//pkg:real"]));
    }

    #[test]
    fn directory_becomes_recursive_pattern_without_query_or_listing() {
        let scratch = temp_workspace("dir");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "src/nested/deep.py", "x = 1\n");
        write(&workspace, "src/top.py", "x = 1\n");
        let query = NeverQuery;
        let got = resolve(&scopes(&["src"]), &workspace, &query).expect("resolve");
        assert_eq!(got.targets, scopes(&["//src/..."]));
        assert_eq!(got.scope, Scope::ResolvedOwners(scopes(&["//src/..."])));
    }

    #[test]
    fn workspace_root_directory_maps_to_repository_pattern() {
        let scratch = temp_workspace("root-dir");
        let workspace = scratch.path().to_path_buf();
        let query = NeverQuery;
        for root in [".", "./"] {
            let got = resolve(&scopes(&[root]), &workspace, &query).expect("resolve");
            assert_eq!(got.targets, scopes(&["//..."]), "root {root}");
        }
    }

    #[test]
    fn mixed_labels_and_paths_merge_sorted_and_deduplicated() {
        let scratch = temp_workspace("mixed");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("//pkg:lib\n//z:z\n")]);
        let got = resolve(&scopes(&["//z:z", "pkg/a.py"]), &workspace, &query).expect("resolve");
        assert_eq!(got.targets, scopes(&["//pkg:lib", "//z:z"]));
        assert_eq!(
            got.scope,
            Scope::ResolvedOwners(scopes(&["//pkg:lib", "//z:z"]))
        );
    }

    #[test]
    fn missing_and_escaping_paths_fail() {
        let query = NeverQuery;
        let scratch = temp_workspace("missing");
        let workspace = scratch.path().to_path_buf();
        assert_eq!(
            resolve(&scopes(&["nope.py"]), &workspace, &query).expect_err("missing"),
            ResolveError::PathNotFound {
                scope: "nope.py".to_owned(),
            }
        );
        for escaping in ["/abs/path.py", "../escape.py", "pkg/../../escape.py"] {
            assert_eq!(
                resolve(&scopes(&[escaping]), &workspace, &query).expect_err("escape"),
                ResolveError::OutsideWorkspace {
                    scope: escaping.to_owned(),
                }
            );
        }
        assert_eq!(
            resolve(&scopes(&[""]), &workspace, &query).expect_err("empty"),
            ResolveError::EmptyScope
        );
    }

    #[test]
    fn non_file_entries_fail() {
        let scratch = temp_workspace("special");
        let workspace = scratch.path().to_path_buf();
        #[cfg(unix)]
        {
            use std::os::unix::net::UnixListener;
            let path = workspace.join("sock");
            let _listener = UnixListener::bind(&path).expect("bind socket");
            let query = NeverQuery;
            assert_eq!(
                resolve(&scopes(&["sock"]), &workspace, &query).expect_err("socket"),
                ResolveError::NotFileOrDir {
                    scope: "sock".to_owned(),
                }
            );
        }
        #[cfg(not(unix))]
        {
            let _ = (&scratch, &workspace);
        }
    }

    #[test]
    fn control_characters_in_names_fail_before_query() {
        let scratch = temp_workspace("control");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "a\nb.py", "x = 1\n");
        let query = NeverQuery;
        assert_eq!(
            resolve(&scopes(&["a\nb.py"]), &workspace, &query).expect_err("control"),
            ResolveError::UnsupportedName {
                scope: "a\nb.py".to_owned(),
            }
        );
    }

    #[test]
    fn ownerless_files_suggest_explicit_labels() {
        let scratch = temp_workspace("no-owner");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/orphan.py", "x = 1\n");
        let query = FakeQuery::new(vec![FakeQuery::ok("\n")]);
        let err = resolve(&scopes(&["pkg/orphan.py"]), &workspace, &query).expect_err("orphan");
        assert_eq!(
            err,
            ResolveError::NoOwner {
                file: "pkg/orphan.py".to_owned(),
                label: "//pkg:orphan.py".to_owned(),
            }
        );
        assert!(err.to_string().contains("explicit target label"));
    }

    #[test]
    fn file_labels_use_the_nearest_enclosing_package() {
        let scratch = temp_workspace("pkg-labels");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "pkg/BUILD.bazel", "");
        write(&workspace, "pkg/a.py", "x = 1\n");
        write(&workspace, "pkg/src/deep/b.py", "x = 1\n");
        write(&workspace, "BUILD.bazel", "");
        write(&workspace, "top.py", "x = 1\n");
        assert_eq!(
            file_label(&workspace, "pkg/a.py", "pkg/a.py").expect("label"),
            "//pkg:a.py"
        );
        assert_eq!(
            file_label(&workspace, "pkg/src/deep/b.py", "pkg/src/deep/b.py").expect("label"),
            "//pkg:src/deep/b.py"
        );
        assert_eq!(
            file_label(&workspace, "top.py", "top.py").expect("label"),
            "//:top.py"
        );
        assert_eq!(dir_pattern(""), "//...");
        assert_eq!(dir_pattern("src"), "//src/...");
        assert_eq!(normalize_rel("./pkg/./a.py").expect("dots"), "pkg/a.py");
        assert_eq!(normalize_rel("pkg//a.py").expect("doubles"), "pkg/a.py");
    }

    #[test]
    fn bare_build_marker_and_nearest_package_win() {
        let scratch = temp_workspace("pkg-markers");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "legacy/BUILD", "");
        write(&workspace, "legacy/a.py", "x = 1\n");
        write(&workspace, "outer/BUILD.bazel", "");
        write(&workspace, "outer/inner/BUILD.bazel", "");
        write(&workspace, "outer/inner/a.py", "x = 1\n");
        write(&workspace, "outer/loose.py", "x = 1\n");
        assert_eq!(
            file_label(&workspace, "legacy/a.py", "legacy/a.py").expect("label"),
            "//legacy:a.py"
        );
        assert_eq!(
            file_label(&workspace, "outer/inner/a.py", "outer/inner/a.py").expect("label"),
            "//outer/inner:a.py"
        );
        assert_eq!(
            file_label(&workspace, "outer/loose.py", "outer/loose.py").expect("label"),
            "//outer:loose.py"
        );
    }

    #[test]
    fn files_without_any_enclosing_package_fail() {
        let scratch = temp_workspace("no-package");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "docs/guide.md", "# guide\n");
        write(&workspace, "other/BUILD.bazel", "");
        let err =
            resolve(&scopes(&["docs/guide.md"]), &workspace, &NeverQuery).expect_err("no package");
        assert_eq!(
            err,
            ResolveError::NotAPackage {
                scope: "docs/guide.md".to_owned(),
            }
        );
        assert!(err.to_string().contains("not a package"), "{err}");
    }

    #[test]
    fn symlinks_are_neither_file_nor_dir() {
        let scratch = temp_workspace("symlink-kind");
        let workspace = scratch.path().to_path_buf();
        write(&workspace, "target.txt", "x\n");
        #[cfg(unix)]
        std::os::unix::fs::symlink(workspace.join("target.txt"), workspace.join("link"))
            .expect("link");
        #[cfg(unix)]
        {
            let query = NeverQuery;
            assert!(matches!(
                resolve(&scopes(&["link"]), &workspace, &query).expect_err("link"),
                ResolveError::NotFileOrDir { .. }
            ));
            assert!(matches!(
                resolve_for_test(&scopes(&["link"]), &workspace, &query).expect_err("link"),
                ResolveError::NotFileOrDir { .. }
            ));
            assert!(matches!(
                resolve_run(&scopes(&["link"]), &workspace, &query).expect_err("link"),
                ResolveError::NotFileOrDir { .. }
            ));
        }
    }
}
