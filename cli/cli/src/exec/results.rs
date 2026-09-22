//! BEP results collection and proto mapping.
//!
//! Split from [`super::common`]: owns the `dx_results` collection
//! domain — [`Collected`], [`map_severity`], [`map_diagnostic`],
//! [`map_change`], [`collect_results`], and [`collect_results_in`] —
//! BEP shard collection over the quality output group plus validated
//! proto-to-event mapping. [`super::common`] keeps the execution
//! environment, stable codes, source verification, and status helpers;
//! the sole production caller is [`super::quality::execute_quality`].

use std::collections::{BTreeMap, BTreeSet};
use std::io::BufReader;
use std::path::Path;

use crate::plan::OUTPUT_GROUP;
use dx_bep::{collect, CollectorConfig};
use dx_output::{DiagnosticEvent, Severity, Snapshot};
use quality_result::{decode_validated, proto};

use super::common::{FileChange, FsArtifacts, CODE_INVALID_BEP, CODE_UNREADABLE_BEP};

/// Normalized collection: current findings, validated changes, and the
/// executed tool set. `complete` is false when Bazel failed, a target
/// failed, or any result was undecodable; default mode never mutates
/// while incomplete.
pub(crate) struct Collected {
    pub(crate) tools: Vec<String>,
    pub(crate) initial: Vec<DiagnosticEvent>,
    pub(crate) terminal: Vec<DiagnosticEvent>,
    pub(crate) changes: Vec<FileChange>,
    pub(crate) terminal_digests: BTreeMap<String, [u8; 32]>,
    pub(crate) complete: bool,
}

pub(crate) fn map_severity(value: i32) -> Option<Severity> {
    match proto::Severity::try_from(value).ok()? {
        proto::Severity::Unspecified => None,
        proto::Severity::Info => Some(Severity::Info),
        proto::Severity::Warning => Some(Severity::Warning),
        proto::Severity::Error => Some(Severity::Error),
    }
}

pub(crate) fn map_diagnostic(
    diagnostic: &proto::Diagnostic,
    snapshot: Snapshot,
) -> Option<DiagnosticEvent> {
    let range = match (diagnostic.start_byte, diagnostic.end_byte) {
        (Some(start), Some(end)) => Some((start, end)),
        (None, None) => None,
        _ => return None,
    };
    let path = (!diagnostic.path.is_empty()).then(|| diagnostic.path.clone());
    if path.is_none() && range.is_some() {
        return None;
    }
    Some(DiagnosticEvent {
        severity: map_severity(diagnostic.severity)?,
        tool: diagnostic.tool_id.clone(),
        message: diagnostic.message.clone(),
        rule: (!diagnostic.rule_id.is_empty()).then(|| diagnostic.rule_id.clone()),
        path,
        range,
        snapshot,
        fixable: diagnostic.fixable,
        resolution: None,
    })
}

pub(crate) fn map_change(change: &proto::FileEdits) -> Option<FileChange> {
    let original_digest: [u8; 32] = change.original_digest.as_slice().try_into().ok()?;
    let mut edits = Vec::with_capacity(change.edits.len());
    for edit in &change.edits {
        edits.push((edit.start_byte, edit.end_byte, edit.replacement.clone()));
    }
    Some(FileChange {
        path: change.path.clone(),
        original_digest,
        edits,
    })
}

/// Collects, decodes, and maps every `dx_results` artifact in the BEP
/// stream at `bep`. Undecodable results and failed targets mark the
/// collection incomplete while retaining validated findings.
pub(crate) fn collect_results(bep: &Path) -> Result<Collected, (String, String)> {
    collect_results_in(bep, OUTPUT_GROUP)
}

/// Collects artifacts for one output group. Split from
/// [`collect_results`] so unit tests can prove the invalid-group arm
/// without touching the pinned production group.
pub(crate) fn collect_results_in(bep: &Path, group: &str) -> Result<Collected, (String, String)> {
    let file = std::fs::File::open(bep).map_err(|err| {
        (
            CODE_UNREADABLE_BEP.to_owned(),
            format!("failed to read build events: {err}"),
        )
    })?;
    let config = CollectorConfig::new(group).map_err(|err| {
        (
            CODE_INVALID_BEP.to_owned(),
            format!("invalid BEP config: {err}"),
        )
    })?;
    let targets = collect(BufReader::new(file), &config, &FsArtifacts).map_err(|err| {
        (
            CODE_INVALID_BEP.to_owned(),
            format!("invalid build events: {err}"),
        )
    })?;
    let mut tools = BTreeSet::new();
    let mut initial = Vec::new();
    let mut terminal = Vec::new();
    let mut changes = Vec::new();
    let mut terminal_digests = BTreeMap::new();
    let mut complete = true;
    for target in &targets {
        if !target.success {
            complete = false;
            continue;
        }
        let mut staged_initial = Vec::new();
        let mut staged_terminal = Vec::new();
        let mut staged_changes = Vec::new();
        let mut staged_digests = Vec::new();
        let mut staged_tools = Vec::new();
        let mut target_ok = true;
        for artifact in &target.artifacts {
            let result = match decode_validated(&artifact.bytes) {
                Ok(result) => result,
                Err(_) => {
                    target_ok = false;
                    break;
                }
            };
            for stage in &result.stages {
                staged_tools.push(stage.tool_id.clone());
            }
            for snapshot in &result.terminal_snapshot {
                match snapshot.digest.as_slice().try_into() {
                    Ok(digest) => staged_digests.push((snapshot.path.clone(), digest)),
                    Err(_) => {
                        target_ok = false; // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
                        break; // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
                    }
                }
            }
            if !target_ok {
                break; // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
            }
            for diagnostic in &result.initial_diagnostics {
                match map_diagnostic(diagnostic, Snapshot::Initial) {
                    Some(mapped) => staged_initial.push(mapped),
                    None => {
                        target_ok = false; // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
                        break; // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
                    }
                }
            }
            if !target_ok {
                break; // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
            }
            for diagnostic in &result.terminal_diagnostics {
                match map_diagnostic(diagnostic, Snapshot::Terminal) {
                    Some(mapped) => staged_terminal.push(mapped),
                    None => {
                        target_ok = false; // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
                        break; // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
                    }
                }
            }
            if !target_ok {
                break; // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
            }
            for file in &result.replacements {
                match map_change(file) {
                    Some(mapped) => staged_changes.push(mapped),
                    None => {
                        target_ok = false; // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
                        break; // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
                    }
                }
            }
            if !target_ok {
                break; // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
            }
        }
        if !target_ok {
            complete = false;
            continue;
        }
        tools.extend(staged_tools);
        initial.extend(staged_initial);
        terminal.extend(staged_terminal);
        changes.extend(staged_changes);
        terminal_digests.extend(staged_digests);
    }
    Ok(Collected {
        tools: tools.into_iter().collect(),
        initial,
        terminal,
        changes,
        terminal_digests,
        complete,
    })
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;
    use quality_result::proto;

    #[test]
    fn severity_mapping_covers_all_arms() {
        assert!(map_severity(proto::Severity::Unspecified as i32).is_none());
        assert!(matches!(
            map_severity(proto::Severity::Info as i32),
            Some(Severity::Info)
        ));
        assert!(matches!(
            map_severity(proto::Severity::Warning as i32),
            Some(Severity::Warning)
        ));
        assert!(matches!(
            map_severity(proto::Severity::Error as i32),
            Some(Severity::Error)
        ));
        assert!(map_severity(99).is_none());
    }

    #[test]
    fn diagnostic_mapping_rejects_bad_shapes() {
        let base = Harness::diagnostic("m", false);
        let mut partial = base.clone();
        partial.end_byte = None;
        assert!(map_diagnostic(&partial, Snapshot::Initial).is_none());
        let mut pathless = base.clone();
        pathless.path = String::new();
        assert!(map_diagnostic(&pathless, Snapshot::Initial).is_none());
        assert!(map_diagnostic(&pathless, Snapshot::Terminal).is_none());
        let mut bare = pathless.clone();
        bare.start_byte = None;
        bare.end_byte = None;
        assert!(map_diagnostic(&bare, Snapshot::Terminal).is_some());
    }

    #[test]
    fn invalid_output_group_rejected() {
        let dir = temp_dir("group-tmp");
        let bep = dir.path().join("empty.json");
        std::fs::write(&bep, "").expect("bep");
        let Err((code, message)) = collect_results_in(&bep, "") else {
            panic!("empty output group must fail"); // LCOV_EXCL_LINE - policy: docs/testing/README.md#coverage
        };
        assert_eq!(code, CODE_INVALID_BEP);
        assert!(message.contains("invalid BEP config"));
    }
}
