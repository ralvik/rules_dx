//! Complete unified-diff renderer for validated workspace changes (M06 WP2).
//!
//! Contract: `docs/cli/output-protocol.md`, Diff Output section. The
//! renderer takes validated file changes with exact original and candidate
//! bytes and emits one concatenated unified diff in normalized path UTF-8
//! byte order. It computes presentation-only line hunks in memory with three
//! context lines; it never touches the workspace, invokes an external
//! `diff`, or uses patch fuzz.
//!
//! Canonical `dx` hunk form: headers always carry explicit `start,length`
//! counts (`@@ -os,ol +ns,nl @@`), existing files use `--- a/<path>` and
//! `+++ b/<path>` headers, created files use `--- /dev/null`, and a final
//! line without a line-feed byte gets the conventional
//! `\ No newline at end of file` marker. An invocation with no changes
//! writes no bytes. Paths containing a tab, carriage return, or line feed
//! cannot be represented unambiguously and fail before any output.

/// Whether a patch entry modifies an existing file or creates a new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchKind {
    Modify,
    Create,
}

/// One validated file change to render.
///
/// `original` is the exact analyzed bytes (`""` for [`PatchKind::Create`]);
/// `candidate` is the exact proposed bytes. Both must be valid UTF-8; UTF-8
/// shape is validated upstream when change events are constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilePatch<'a> {
    pub path: &'a str,
    pub kind: PatchKind,
    pub original: &'a str,
    pub candidate: &'a str,
}

/// Diff rendering failure (issue #211 slice, #221 follow-up).
///
/// Every variant renders human-readable via `Display` for CLI
/// operational diagnostics; binaries render via `to_string()`, never
/// Rust `Debug`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DiffError {
    /// A path cannot be represented unambiguously in unified form.
    #[error("unrepresentable path {path:?}: paths with tab, carriage return, or line feed cannot be rendered")]
    UnrepresentablePath { path: String },
    /// The same path was supplied twice.
    #[error("duplicate path {path:?}")]
    DuplicatePath { path: String },
    /// A create entry carries original bytes.
    #[error("create {path:?} carries original bytes")]
    CreateWithOriginal { path: String },
    /// A modify entry whose candidate is byte-identical to its original.
    #[error("no change for {path:?}: candidate is identical to original")]
    NoopPatch { path: String },
}

/// Number of context lines around each hunk.
const CONTEXT: usize = 3;

fn check_path(path: &str) -> Result<(), DiffError> {
    if path.contains(['\t', '\r', '\n']) {
        return Err(DiffError::UnrepresentablePath {
            path: path.to_owned(),
        });
    }
    Ok(())
}

/// Renders one concatenated unified diff for `files`.
///
/// Files are emitted in path UTF-8 byte order regardless of input order.
/// Duplicate paths, unrepresentable paths, creates with original bytes, and
/// byte-identical modify candidates fail; an empty input renders `""`.
pub fn render_patch(files: &[FilePatch<'_>]) -> Result<String, DiffError> {
    let mut ordered: Vec<&FilePatch<'_>> = files.iter().collect();
    ordered.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    let mut seen: Vec<&str> = Vec::with_capacity(ordered.len());
    for file in &ordered {
        check_path(file.path)?;
        if seen.contains(&file.path) {
            return Err(DiffError::DuplicatePath {
                path: (*file.path).to_owned(),
            });
        }
        seen.push(file.path);
        match file.kind {
            PatchKind::Create if !file.original.is_empty() => {
                return Err(DiffError::CreateWithOriginal {
                    path: file.path.to_owned(),
                });
            }
            PatchKind::Modify if file.original == file.candidate => {
                return Err(DiffError::NoopPatch {
                    path: file.path.to_owned(),
                });
            }
            PatchKind::Create | PatchKind::Modify => {}
        }
    }
    let mut out = String::new();
    for file in ordered {
        render_file(&mut out, file)?;
    }
    Ok(out)
}

fn render_file(out: &mut String, file: &FilePatch<'_>) -> Result<(), DiffError> {
    match file.kind {
        PatchKind::Modify => {
            out.push_str("--- a/");
            out.push_str(file.path);
            out.push('\n');
        }
        PatchKind::Create => {
            out.push_str("--- /dev/null\n");
        }
    }
    out.push_str("+++ b/");
    out.push_str(file.path);
    out.push('\n');
    let old = split_lines(file.original);
    let new = split_lines(file.candidate);
    for hunk in hunks(&old.lines, old.ends_nl, &new.lines, new.ends_nl)? {
        render_hunk(out, &hunk, &old, &new);
    }
    Ok(())
}

/// One side of a file diff: lines without terminators plus whether the
/// side ends with a line feed. Grouped so hunk rendering takes one
/// argument per side instead of six positional values.
struct SidedText<'a> {
    lines: Vec<&'a str>,
    ends_nl: bool,
}

/// Lines of `text` without terminators, plus whether the text ends with a
/// line feed (vacuously true for empty text, which has no lines at all).
fn split_lines(text: &str) -> SidedText<'_> {
    if text.is_empty() {
        return SidedText {
            lines: Vec::new(),
            ends_nl: true,
        };
    }
    let ends_nl = text.ends_with('\n');
    let mut lines: Vec<&str> = text.split('\n').collect();
    if ends_nl {
        lines.pop();
    }
    SidedText { lines, ends_nl }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Eq,
    Del,
    Ins,
}

/// Edit script from the first old line to the last new line, via
/// `similar`'s Myers diff (`O((N+M)D)` time, `O(N+M)` space): the retired
/// hand-rolled greedy search kept a full `O(D²)` trace. `Replace` expands to
/// deletions then insertions, matching the retired backtrack order (`-old`
/// before `+new`).
fn diff_ops(old: &[&str], new: &[&str]) -> Vec<Op> {
    use similar::{capture_diff_slices, Algorithm, DiffOp};
    let mut ops = Vec::new();
    for op in capture_diff_slices(Algorithm::Myers, old, new) {
        match op {
            DiffOp::Equal { len, .. } => ops.extend(vec![Op::Eq; len]),
            DiffOp::Delete { old_len, .. } => {
                ops.extend(vec![Op::Del; old_len]);
            }
            DiffOp::Insert { new_len, .. } => {
                ops.extend(vec![Op::Ins; new_len]);
            }
            DiffOp::Replace {
                old_len, new_len, ..
            } => {
                ops.extend(vec![Op::Del; old_len]);
                ops.extend(vec![Op::Ins; new_len]);
            }
        }
    }
    ops
}

struct Hunk {
    /// 0-based old/new line indices where the hunk starts.
    old_start: usize,
    new_start: usize,
    /// Script slice covering the hunk, context lines included.
    ops: Vec<(Op, usize, usize)>,
}

/// Groups the edit script into hunks with [`CONTEXT`] lines of context,
/// merging change groups whose context windows overlap.
///
/// An `Eq` pairing whose lines differ only in trailing-newline presence is
/// promoted to a `Del`+`Ins` pair: line diff sees identical text, but the
/// patch must still record the newline change (GNU renders `-a` + marker /
/// `+a` for it).
fn hunks(old: &[&str], old_nl: bool, new: &[&str], new_nl: bool) -> Result<Vec<Hunk>, DiffError> {
    let script = diff_ops(old, new);
    // Annotate every op with the old/new line index it consumes.
    let mut annotated: Vec<(Op, usize, usize)> = Vec::with_capacity(script.len());
    let (mut oi, mut ni) = (0usize, 0usize);
    for op in script {
        match op {
            Op::Eq => {
                let old_missing = oi + 1 == old.len() && !old_nl;
                let new_missing = ni + 1 == new.len() && !new_nl;
                if old_missing != new_missing {
                    annotated.push((Op::Del, oi, ni));
                    annotated.push((Op::Ins, oi, ni));
                } else {
                    annotated.push((op, oi, ni));
                }
                oi += 1;
                ni += 1;
            }
            Op::Del => {
                annotated.push((op, oi, ni));
                oi += 1;
            }
            Op::Ins => {
                annotated.push((op, oi, ni));
                ni += 1;
            }
        }
    }
    let changes: Vec<usize> = annotated
        .iter()
        .enumerate()
        .filter(|(_, (op, _, _))| *op != Op::Eq)
        .map(|(i, _)| i)
        .collect();
    // An empty change list happens only for an empty-file create
    // (original "" and candidate ""): byte-identical modifies are rejected
    // as NoopPatch before hunks run. No hunks means headers only.
    if changes.is_empty() {
        return Ok(Vec::new());
    }
    // Group change indices: a new hunk starts when the gap between
    // consecutive changes exceeds the two adjacent context windows.
    let mut groups: Vec<(usize, usize)> = Vec::new();
    let mut start = changes[0];
    let mut prev = changes[0];
    for &i in &changes[1..] {
        if i - prev > 2 * CONTEXT {
            groups.push((start, prev));
            start = i;
        }
        prev = i;
    }
    groups.push((start, prev));
    Ok(groups
        .into_iter()
        .map(|(first, last)| {
            let lo = first.saturating_sub(CONTEXT);
            let hi = (last + CONTEXT + 1).min(annotated.len());
            let ops = annotated[lo..hi].to_vec();
            let old_start = ops[0].1;
            let new_start = ops[0].2;
            Hunk {
                old_start,
                new_start,
                ops,
            }
        })
        .collect())
}

fn render_hunk(out: &mut String, hunk: &Hunk, old: &SidedText<'_>, new: &SidedText<'_>) {
    let ops = &hunk.ops;
    let old_start = hunk.old_start;
    let new_start = hunk.new_start;
    let old_len = ops.iter().filter(|(op, _, _)| *op != Op::Ins).count();
    let new_len = ops.iter().filter(|(op, _, _)| *op != Op::Del).count();
    let old_head = if old_len == 0 {
        old_start
    } else {
        old_start + 1
    };
    let new_head = if new_len == 0 {
        new_start
    } else {
        new_start + 1
    };
    out.push_str(&format!(
        "@@ -{old_head},{old_len} +{new_head},{new_len} @@\n"
    ));
    for (op, oi, ni) in ops {
        match op {
            Op::Eq => {
                out.push(' ');
                out.push_str(old.lines[*oi]);
                out.push('\n');
                let old_last = *oi + 1 == old.lines.len() && !old.ends_nl;
                let new_last = *ni + 1 == new.lines.len() && !new.ends_nl;
                if old_last || new_last {
                    out.push_str("\\ No newline at end of file\n");
                }
            }
            Op::Del => {
                out.push('-');
                out.push_str(old.lines[*oi]);
                out.push('\n');
                if *oi + 1 == old.lines.len() && !old.ends_nl {
                    out.push_str("\\ No newline at end of file\n");
                }
            }
            Op::Ins => {
                out.push('+');
                out.push_str(new.lines[*ni]);
                out.push('\n');
                if *ni + 1 == new.lines.len() && !new.ends_nl {
                    out.push_str("\\ No newline at end of file\n");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modify<'a>(path: &'a str, original: &'a str, candidate: &'a str) -> FilePatch<'a> {
        FilePatch {
            path,
            kind: PatchKind::Modify,
            original,
            candidate,
        }
    }

    #[test]
    fn empty_input_renders_no_bytes() {
        assert_eq!(render_patch(&[]).expect("empty"), "");
    }

    #[test]
    fn single_line_change() {
        let got = render_patch(&[modify("a.txt", "one\ntwo\n", "one\nTWO\n")]).expect("patch");
        assert_eq!(
            got,
            "--- a/a.txt\n+++ b/a.txt\n@@ -1,2 +1,2 @@\n one\n-two\n+TWO\n"
        );
    }

    #[test]
    fn create_uses_dev_null_header() {
        let patch = FilePatch {
            path: "new/f.txt",
            kind: PatchKind::Create,
            original: "",
            candidate: "hi\n",
        };
        let got = render_patch(&[patch]).expect("patch");
        assert_eq!(
            got,
            "--- /dev/null\n+++ b/new/f.txt\n@@ -0,0 +1,1 @@\n+hi\n"
        );
    }

    #[test]
    fn empty_create_renders_headers_only() {
        let patch = FilePatch {
            path: "empty/BUILD.bazel",
            kind: PatchKind::Create,
            original: "",
            candidate: "",
        };
        let got = render_patch(&[patch]).expect("patch");
        assert_eq!(got, "--- /dev/null\n+++ b/empty/BUILD.bazel\n");
    }

    #[test]
    fn files_sort_by_path_bytes() {
        let files = [modify("b.txt", "x\n", "y\n"), modify("a.txt", "x\n", "y\n")];
        let got = render_patch(&files).expect("patch");
        assert!(got.starts_with("--- a/a.txt\n"));
        assert!(got.contains("--- a/b.txt\n"));
    }

    #[test]
    fn unrepresentable_paths_fail() {
        for bad in ["a\tb", "a\rb", "a\nb"] {
            let err = render_patch(&[modify(bad, "x\n", "y\n")]).expect_err("must fail");
            assert_eq!(
                err,
                DiffError::UnrepresentablePath {
                    path: bad.to_owned()
                }
            );
        }
    }

    #[test]
    fn duplicate_paths_fail() {
        let files = [modify("a.txt", "x\n", "y\n"), modify("a.txt", "x\n", "z\n")];
        assert_eq!(
            render_patch(&files).expect_err("must fail"),
            DiffError::DuplicatePath {
                path: "a.txt".to_owned()
            }
        );
    }

    #[test]
    fn create_with_original_fails() {
        let patch = FilePatch {
            path: "n.txt",
            kind: PatchKind::Create,
            original: "x\n",
            candidate: "y\n",
        };
        assert_eq!(
            render_patch(&[patch]).expect_err("must fail"),
            DiffError::CreateWithOriginal {
                path: "n.txt".to_owned()
            }
        );
    }

    #[test]
    fn noop_modify_fails() {
        assert_eq!(
            render_patch(&[modify("a.txt", "x\n", "x\n")]).expect_err("must fail"),
            DiffError::NoopPatch {
                path: "a.txt".to_owned()
            }
        );
    }

    #[test]
    fn missing_final_newline_gets_marker() {
        let got = render_patch(&[modify("a.txt", "one\ntwo", "one\nTWO")]).expect("patch");
        assert_eq!(
            got,
            "--- a/a.txt\n+++ b/a.txt\n@@ -1,2 +1,2 @@\n one\n-two\n\\ No newline at end of file\n+TWO\n\\ No newline at end of file\n"
        );
    }

    #[test]
    fn added_final_newline_marks_old_only() {
        let got = render_patch(&[modify("a.txt", "one", "one\n")]).expect("patch");
        assert_eq!(
            got,
            "--- a/a.txt\n+++ b/a.txt\n@@ -1,1 +1,1 @@\n-one\n\\ No newline at end of file\n+one\n"
        );
    }

    #[test]
    fn distant_changes_split_hunks() {
        let old: String = (0..20).map(|i| format!("line {i}\n")).collect();
        let mut new = old.clone();
        new = new.replacen("line 1\n", "LINE 1\n", 1);
        new = new.replacen("line 18\n", "LINE 18\n", 1);
        let got = render_patch(&[modify("a.txt", &old, &new)]).expect("patch");
        assert_eq!(got.matches("@@ ").count(), 2);
        assert!(got.contains("-line 1\n+LINE 1\n"));
        assert!(got.contains("-line 18\n+LINE 18\n"));
    }

    #[test]
    fn multibyte_lines_compare_by_bytes() {
        let got = render_patch(&[modify("a.txt", "héllo\n", "héllo!\n")]).expect("patch");
        assert!(got.contains("-héllo\n+héllo!\n"));
    }

    #[test]
    fn insertion_at_start_uses_zero_head() {
        let got = render_patch(&[modify("a.txt", "b\n", "a\nb\n")]).expect("patch");
        assert_eq!(got, "--- a/a.txt\n+++ b/a.txt\n@@ -1,1 +1,2 @@\n+a\n b\n");
    }

    #[test]
    fn deletion_renders_minus_only() {
        let got = render_patch(&[modify("a.txt", "a\nb\nc\n", "a\nc\n")]).expect("patch");
        assert_eq!(
            got,
            "--- a/a.txt\n+++ b/a.txt\n@@ -1,3 +1,2 @@\n a\n-b\n c\n"
        );
    }

    #[test]
    fn full_deletion_renders_zero_new_head() {
        let got = render_patch(&[modify("a.txt", "a\nb\n", "")]).expect("patch");
        assert_eq!(got, "--- a/a.txt\n+++ b/a.txt\n@@ -1,2 +0,0 @@\n-a\n-b\n");
    }

    #[test]
    fn context_line_at_eof_gets_marker() {
        let got = render_patch(&[modify("a.txt", "a\nb\nc\nd", "a\nB\nc\nd")]).expect("patch");
        assert_eq!(
            got,
            "--- a/a.txt\n+++ b/a.txt\n@@ -1,4 +1,4 @@\n a\n-b\n+B\n c\n d\n\\ No newline at end of file\n"
        );
    }

    #[test]
    fn display_is_human_readable() {
        let unrepresentable = DiffError::UnrepresentablePath {
            path: "a\tb".to_owned(),
        };
        assert_eq!(
            unrepresentable.to_string(),
            "unrepresentable path \"a\\tb\": paths with tab, carriage return, or line feed cannot be rendered"
        );
        assert!(!unrepresentable
            .to_string()
            .contains("UnrepresentablePath {"));
        assert_eq!(
            DiffError::DuplicatePath {
                path: "a".to_owned()
            }
            .to_string(),
            "duplicate path \"a\""
        );
        assert_eq!(
            DiffError::CreateWithOriginal {
                path: "n".to_owned()
            }
            .to_string(),
            "create \"n\" carries original bytes"
        );
        assert_eq!(
            DiffError::NoopPatch {
                path: "a".to_owned()
            }
            .to_string(),
            "no change for \"a\": candidate is identical to original"
        );
    }
}
