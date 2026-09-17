//! Quality diff-patch rendering (issue #236): unified patch over
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
            // LCOV_EXCL_START - reason: apply_to_bytes only returns valid UTF-8.
            Err(_) => {
                return Err(format!("candidate for {} is not UTF-8 text", change.path));
            } // LCOV_EXCL_STOP - reason: end of unreachable candidate arm.
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
