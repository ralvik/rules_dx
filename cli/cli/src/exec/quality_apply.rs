//! Quality mutation and status projection (issue #236): verified
//! source reads plus check/complete-gated atomic apply, and
//! check-mode vs default-mode status with the fail-closed flag.
//!
//! Extracted from [`super::quality`] without behavior change: the
//! execution root still owns Bazel launch, result collection, patch
//! rendering, event emission, and report writing; this module owns
//! only mutation plus status projection.

use super::common::{
    apply_to_bytes, read_verified, FileChange, SourceRead, REASON_INCOMPLETE_COLLECTION,
    REASON_INVALID_EDITS, REASON_STALE_SOURCE, REASON_UNREADABLE_SOURCE,
};
use dx_apply::{FileSystem, RealFileSystem};
use dx_output::{meets_threshold, DiagnosticEvent, Threshold};
use std::collections::BTreeMap;
use std::path::Path;

/// Verified sources plus per-file apply outcome for one quality run.
pub(crate) struct ApplyOutcome {
    pub(crate) sources: BTreeMap<String, SourceRead>,
    pub(crate) applied: BTreeMap<String, bool>,
    pub(crate) not_applied: Vec<(String, &'static str)>,
}

/// Reads verified source bytes for every changed file, then applies
/// stable candidates per the check/complete gate:
///
/// * check mode never mutates; any proposed change fails the run
///   downstream via the non-empty change set.
/// * incomplete collection marks every change not-applied with
///   `incomplete_collection`.
/// * otherwise each change applies against its verified bytes; stale,
///   unreadable, or invalid edits mark that file not-applied with the
///   stable reason while other files still apply.
pub(crate) fn apply_collected_changes(
    workspace: &Path,
    check: bool,
    complete: bool,
    changes: &[FileChange],
) -> ApplyOutcome {
    // Verified source bytes for changed files, read before any
    // mutation while the workspace still matches the analysis.
    let mut sources: BTreeMap<String, SourceRead> = BTreeMap::new();
    for change in changes {
        sources
            .entry(change.path.clone())
            .or_insert_with(|| read_verified(workspace, &change.path, &change.original_digest));
    }

    let fs = RealFileSystem;
    let mut applied: BTreeMap<String, bool> = BTreeMap::new();
    let mut not_applied: Vec<(String, &'static str)> = Vec::new();
    if check {
        // Check mode never mutates; any proposed change fails the run.
    } else if !complete {
        for change in changes {
            applied.insert(change.path.clone(), false);
            not_applied.push((change.path.clone(), REASON_INCOMPLETE_COLLECTION));
        }
    } else {
        for change in changes {
            let reason = match sources.get(&change.path) {
                Some(SourceRead::Bytes(original)) => {
                    match apply_to_bytes(original, &change.edits) {
                        Some(candidate) => {
                            match fs.write_atomic(&workspace.join(&change.path), &candidate) {
                                Ok(()) => {
                                    applied.insert(change.path.clone(), true);
                                    None
                                }
                                Err(_) => Some(REASON_UNREADABLE_SOURCE),
                            }
                        }
                        None => Some(REASON_INVALID_EDITS),
                    }
                }
                Some(SourceRead::Unreadable) => Some(REASON_UNREADABLE_SOURCE),
                Some(SourceRead::Stale) | None => Some(REASON_STALE_SOURCE),
            };
            if let Some(reason) = reason {
                applied.insert(change.path.clone(), false);
                not_applied.push((change.path.clone(), reason));
            }
        }
    }
    ApplyOutcome {
        sources,
        applied,
        not_applied,
    }
}

/// Projects status findings under the command policy: every initial
/// diagnostic in check mode; terminal diagnostics for applied files
/// plus initial diagnostics for all other files in default mode.
/// Fixed initials (applied guaranteed fixes) leave all projections.
///
/// Returns the projected status plus the fail-closed `failed` flag:
/// any status finding at or above `fail_on` fails, and check mode with
/// a non-empty change set always fails.
pub(crate) fn project_status(
    check: bool,
    initial: &[DiagnosticEvent],
    terminal: &[DiagnosticEvent],
    applied: &BTreeMap<String, bool>,
    fail_on: Threshold,
    has_changes: bool,
) -> (Vec<DiagnosticEvent>, bool) {
    let mut status: Vec<DiagnosticEvent> = Vec::new();
    if check {
        status.extend(initial.iter().cloned());
    } else {
        status.extend(terminal.iter().cloned());
        for diagnostic in initial {
            let is_applied = diagnostic
                .path
                .as_ref()
                .is_some_and(|path| applied.get(path).copied().unwrap_or(false));
            if is_applied && diagnostic.fixable {
                continue;
            }
            status.push(diagnostic.clone());
        }
        dx_output::sort_diagnostics(&mut status);
    }
    let failing = status
        .iter()
        .any(|diagnostic| meets_threshold(diagnostic.severity, fail_on));
    let mut failed = failing;
    if check && has_changes {
        failed = true;
    }
    (status, failed)
}
