//! Quality diff-patch rendering: unified patch over
//! verified sources for `--output=diff`.
//!
//! Extracted from [`super::quality`] without behavior change: the
//! execution root still owns Bazel launch, result collection, event
//! emission, and report writing; this module owns only the verified-
//! source to unified-patch projection.

use super::common::{apply_to_bytes, FileChange, SourceRead};
use dx_diff::{render_patch, FilePatch, PatchKind};
use std::collections::BTreeMap;

/// Renders the unified patch for `collected.changes` against verified
/// `sources`. Returns the rendered patch, or the operational detail
/// for `diff_failed` when a source is missing, non-UTF-8, unappliable,
/// or unrenderable.
pub(crate) fn render_diff_patch(
    sources: &BTreeMap<String, SourceRead>,
    changes: &[FileChange],
) -> Result<String, String> {
    let mut owned: Vec<(String, String, String)> = Vec::with_capacity(changes.len());
    for change in changes {
        let original = match sources.get(&change.path) {
            Some(SourceRead::Bytes(bytes)) => bytes,
            _ => {
                return Err(format!(
                    "cannot render patch without verified source for {}",
                    change.path
                ));
            }
        };
        let original_text = match std::str::from_utf8(original) {
            Ok(text) => text,
            Err(_) => {
                return Err(format!("source for {} is not UTF-8 text", change.path));
            }
        };
        let Some(candidate) = apply_to_bytes(original, &change.edits) else {
            return Err(format!("cannot apply recorded edits for {}", change.path));
        };
        let candidate_text = match String::from_utf8(candidate) {
            Ok(text) => text,
            // LCOV_EXCL_START - policy: docs/testing/README.md#coverage
            Err(_) => {
                return Err(format!("candidate for {} is not UTF-8 text", change.path));
            } // LCOV_EXCL_STOP - policy: docs/testing/README.md#coverage
        };
        owned.push((
            change.path.clone(),
            original_text.to_owned(),
            candidate_text,
        ));
    }
    let patches: Vec<FilePatch<'_>> = owned
        .iter()
        .map(|(path, original, candidate)| FilePatch {
            path,
            kind: PatchKind::Modify,
            original,
            candidate,
        })
        .collect();
    match render_patch(&patches) {
        Ok(rendered) => Ok(rendered),
        Err(error) => Err(format!("failed to render patch: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::super::common::{FileChange, SourceRead};
    use super::render_diff_patch;
    use std::collections::BTreeMap;

    #[test]
    fn diff_patch_is_deterministic_and_untruncated() {
        // Apply-safety battery: `quality-testing.md` requires
        // complete deterministic diff-mode patches from the same edit set
        // without rerunning tools or truncating replacement content.
        let original = b"line one\nline two\n";
        let long_body = format!("{}\n", "x".repeat(10_000));
        let mut sources = BTreeMap::new();
        sources.insert("src/a.py".to_owned(), SourceRead::Bytes(original.to_vec()));
        sources.insert(
            "src/long.py".to_owned(),
            SourceRead::Bytes(b"old\n".to_vec()),
        );
        let changes = vec![
            FileChange {
                path: "src/a.py".to_owned(),
                original_digest: dx_digest::blake3(original),
                edits: vec![(0, original.len() as u64, b"line one\nline TWO\n".to_vec())],
            },
            FileChange {
                path: "src/long.py".to_owned(),
                original_digest: dx_digest::blake3(b"old\n"),
                edits: vec![(0, 4, long_body.as_bytes().to_vec())],
            },
        ];
        let first = render_diff_patch(&sources, &changes).expect("render patch");
        let second = render_diff_patch(&sources, &changes).expect("render patch");
        assert_eq!(first, second);
        assert!(first.contains("--- a/src/a.py"));
        assert!(first.contains("+++ b/src/a.py"));
        assert!(first.contains("line TWO"));
        assert!(first.contains(&"x".repeat(10_000)));
    }
}
