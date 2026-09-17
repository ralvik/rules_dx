//! Public output modes and versioned NDJSON protocol for the `dx` CLI
//! (M06 WP2).
//!
//! Contract: `docs/cli/output-protocol.md`. This crate owns live-mode
//! selection (`text`/`diff`/`json`), stdout-ownership policy, the v1 NDJSON
//! event shapes with exact field presence and omission rules, deterministic
//! diagnostic ordering, and protocol-shape validation (workspace paths,
//! digest spellings, edit ordering). It never renders subprocess argv,
//! option values, credentials, or original source bytes: replacement text
//! appears only inside validated `change` events built by the caller.
//!
//! JSON objects serialize with `serde_json`'s default key ordering; NDJSON
//! validity never depends on key order. Consumers parse values, not bytes.
//!
//! Domain split (issue #236): structured diagnostics and human status
//! emission live in the `diagnostics` module, live output modes and stdout
//! ownership live in the `modes` module, normalized severity and fail-on
//! threshold live in the `severity` module, protocol-shape validation
//! (paths, digests, edits, `OutputError`) lives in the `validation`
//! module, command lifecycle NDJSON events (schema, `command_started` /
//! `operation` / `report` / `selection` / `error` / `command_finished`,
//! `FinishedCounts`, `write_event`) live in the `lifecycle` module. This
//! facade keeps diagnostic/notice/change/mutation events; the public path
//! stays `dx_output::{...}` via the re-exports below.

// Issue #238: infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

pub mod diagnostics;
pub mod lifecycle;
pub mod modes;
pub mod severity;
pub mod validation;

pub use diagnostics::{
    color_enabled, colors_allowed, emit_status, format_status, init_diagnostics, styled_status,
    styled_status_for, DEFAULT_LOG_FILTER, VERBOSE_LOG_FILTER,
};
pub use lifecycle::{
    command_finished, command_started, error_event, operation_event, report_event, schema,
    selection_event, write_event, FinishedCounts, SCHEMA_MAJOR, SCHEMA_MINOR,
};
pub use modes::{
    check_output_conflict, dx_text_visible, stdout_owner, OutputMode, OutputModeName, StdoutOwner,
};
pub use severity::{meets_threshold, Severity, Threshold};
pub use validation::{check_edits, check_path, parse_digest, Edit, OutputError};

use lifecycle::{base, nonempty};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Diagnostic / notice / change / mutation events (still on the facade;
// lifecycle constructors live in `lifecycle`)
// ---------------------------------------------------------------------------

/// Which pipeline snapshot a diagnostic addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Snapshot {
    Initial,
    Terminal,
}

/// Resolution of a default-mode initial diagnostic after terminal apply
/// outcomes are known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Fixed,
    Remaining,
    NotApplied,
}

impl Resolution {
    fn name(self) -> &'static str {
        match self {
            Resolution::Fixed => "fixed",
            Resolution::Remaining => "remaining",
            Resolution::NotApplied => "not_applied",
        }
    }
}

/// One normalized source finding. Terminal diagnostics always use
/// `fixable=false`; `resolution` is required exactly for initial
/// diagnostics in default mutating mode (`mutating=true`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticEvent {
    pub severity: Severity,
    pub tool: String,
    pub message: String,
    pub rule: Option<String>,
    pub path: Option<String>,
    pub range: Option<(u64, u64)>,
    pub snapshot: Snapshot,
    pub fixable: bool,
    pub resolution: Option<Resolution>,
}

pub fn diagnostic_event(
    diagnostic: &DiagnosticEvent,
    mutating: bool,
) -> Result<Value, OutputError> {
    nonempty("tool", &diagnostic.tool)?;
    nonempty("message", &diagnostic.message)?;
    if let Some(path) = &diagnostic.path {
        check_path(path)?;
    } else if diagnostic.range.is_some() {
        return Err(OutputError::RangeWithoutPath);
    }
    if let Some((start, end)) = diagnostic.range {
        if start > end {
            return Err(OutputError::InvertedRange);
        }
    }
    match (diagnostic.snapshot, diagnostic.resolution, mutating) {
        (Snapshot::Terminal, Some(_), _) | (Snapshot::Initial, Some(_), false) => {
            return Err(OutputError::UnexpectedResolution);
        }
        (Snapshot::Initial, None, true) => return Err(OutputError::MissingResolution),
        (Snapshot::Initial, _, _) | (Snapshot::Terminal, None, _) => {}
    }
    let mut map = base("diagnostic");
    map.insert(
        "severity".to_owned(),
        Value::String(diagnostic.severity.name().to_owned()),
    );
    map.insert("tool".to_owned(), Value::String(diagnostic.tool.clone()));
    map.insert(
        "message".to_owned(),
        Value::String(diagnostic.message.clone()),
    );
    if let Some(rule) = &diagnostic.rule {
        map.insert("rule".to_owned(), Value::String(rule.clone()));
    }
    if let Some(path) = &diagnostic.path {
        map.insert("path".to_owned(), Value::String(path.clone()));
    }
    if let Some((start, end)) = diagnostic.range {
        map.insert(
            "range".to_owned(),
            json!({"start_byte": start, "end_byte": end}),
        );
    }
    map.insert(
        "snapshot".to_owned(),
        Value::String(
            match diagnostic.snapshot {
                Snapshot::Initial => "initial",
                Snapshot::Terminal => "terminal",
            }
            .to_owned(),
        ),
    );
    map.insert("fixable".to_owned(), Value::Bool(diagnostic.fixable));
    if let Some(resolution) = diagnostic.resolution {
        map.insert(
            "resolution".to_owned(),
            Value::String(resolution.name().to_owned()),
        );
    }
    Ok(Value::Object(map))
}

/// Sorts diagnostics into the deterministic protocol order: initial before
/// terminal, present paths before pathless findings, path by UTF-8 bytes,
/// absent range before present range, start byte, end byte, severity rank,
/// tool, absent rule before present rule, rule, then message. Fixability
/// and resolution do not alter ordering.
pub fn sort_diagnostics(diagnostics: &mut [DiagnosticEvent]) {
    diagnostics.sort_by(|a, b| {
        a.snapshot
            .cmp(&b.snapshot)
            .then_with(|| a.path.is_none().cmp(&b.path.is_none()))
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.range.is_none().cmp(&b.range.is_none()))
            .then_with(|| a.range.map(|r| r.0).cmp(&b.range.map(|r| r.0)))
            .then_with(|| a.range.map(|r| r.1).cmp(&b.range.map(|r| r.1)))
            .then_with(|| a.severity.rank().cmp(&b.severity.rank()))
            .then_with(|| a.tool.cmp(&b.tool))
            .then_with(|| a.rule.is_none().cmp(&b.rule.is_none()))
            .then_with(|| a.rule.cmp(&b.rule))
            .then_with(|| a.message.cmp(&b.message))
    });
}

/// Non-fatal user-facing information that is neither a source finding nor
/// an operational failure. `ignored_import` notices require
/// path/language/import and omit scope and related command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeEvent {
    pub level: String,
    pub code: String,
    pub message: String,
    pub related_command: Option<String>,
    pub scope: Option<Vec<String>>,
    pub path: Option<String>,
    pub language: Option<String>,
    pub import: Option<String>,
}

pub fn notice_event(notice: &NoticeEvent) -> Result<Value, OutputError> {
    if notice.level != "info" && notice.level != "warning" {
        return Err(OutputError::EmptyField { field: "level" });
    }
    nonempty("code", &notice.code)?;
    nonempty("message", &notice.message)?;
    if notice.code == "ignored_import" {
        let path = notice
            .path
            .as_ref()
            .ok_or(OutputError::EmptyField { field: "path" })?;
        check_path(path)?;
        nonempty("language", notice.language.as_deref().unwrap_or(""))?;
        nonempty("import", notice.import.as_deref().unwrap_or(""))?;
        if notice.scope.is_some() || notice.related_command.is_some() {
            return Err(OutputError::UnexpectedResolution);
        }
    }
    let mut map = base("notice");
    map.insert("level".to_owned(), Value::String(notice.level.clone()));
    map.insert("code".to_owned(), Value::String(notice.code.clone()));
    map.insert("message".to_owned(), Value::String(notice.message.clone()));
    if let Some(related) = &notice.related_command {
        map.insert("related_command".to_owned(), Value::String(related.clone()));
    }
    if let Some(scope) = &notice.scope {
        map.insert(
            "scope".to_owned(),
            Value::Array(scope.iter().map(|s| Value::String(s.clone())).collect()),
        );
    }
    if let Some(path) = &notice.path {
        map.insert("path".to_owned(), Value::String(path.clone()));
    }
    if let Some(language) = &notice.language {
        map.insert("language".to_owned(), Value::String(language.clone()));
    }
    if let Some(import) = &notice.import {
        map.insert("import".to_owned(), Value::String(import.clone()));
    }
    Ok(Value::Object(map))
}

/// Whether a change modifies an existing file or creates a new one.
/// Generate-only creates are the sole v1 source of new files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Modify,
    Create,
}

impl ChangeKind {
    fn name(self) -> &'static str {
        match self {
            ChangeKind::Modify => "modify",
            ChangeKind::Create => "create",
        }
    }
}

/// One exact workspace file change: intended bytes, not whether they were
/// written. A `create` omits `source_digest` and holds exactly one `0..0`
/// insertion with the complete new file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeEvent {
    pub path: String,
    pub kind: ChangeKind,
    pub source_digest: Option<String>,
    pub edits: Vec<Edit>,
}

pub fn change_event(change: &ChangeEvent) -> Result<Value, OutputError> {
    check_path(&change.path)?;
    check_edits(&change.edits)?;
    match change.kind {
        ChangeKind::Modify => {
            let digest = change
                .source_digest
                .as_deref()
                .ok_or(OutputError::BadDigest {
                    field: "source_digest",
                    value: String::new(),
                })?;
            parse_digest("source_digest", digest)?;
        }
        ChangeKind::Create => {
            if change.source_digest.is_some() {
                return Err(OutputError::BadDigest {
                    field: "source_digest",
                    value: change.source_digest.clone().unwrap_or_default(),
                });
            }
            if change.edits.len() != 1 || change.edits[0].start != 0 || change.edits[0].end != 0 {
                return Err(OutputError::BadEdit {
                    index: 0,
                    reason: "create holds exactly one 0..0 insertion",
                });
            }
        }
    }
    let mut map = base("change");
    map.insert("path".to_owned(), Value::String(change.path.clone()));
    map.insert(
        "kind".to_owned(),
        Value::String(change.kind.name().to_owned()),
    );
    if let Some(digest) = &change.source_digest {
        map.insert("source_digest".to_owned(), Value::String(digest.clone()));
    }
    map.insert(
        "edits".to_owned(),
        Value::Array(
            change
                .edits
                .iter()
                .map(|e| {
                    json!({
                        "start_byte": e.start,
                        "end_byte": e.end,
                        "replacement": e.replacement,
                    })
                })
                .collect(),
        ),
    );
    Ok(Value::Object(map))
}

/// Terminal outcome of one handled workspace file. Post-commit records
/// omit `reason`; `not_applied` records carry the stable per-file or
/// terminal operational error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationOutcome {
    Applied,
    NotApplied,
}

pub fn mutation_event(
    path: &str,
    kind: ChangeKind,
    outcome: MutationOutcome,
    reason: Option<&str>,
) -> Result<Value, OutputError> {
    check_path(path)?;
    match (outcome, reason) {
        (MutationOutcome::NotApplied, None) | (MutationOutcome::NotApplied, Some("")) => {
            return Err(OutputError::MissingReason {
                path: path.to_owned(),
            });
        }
        (MutationOutcome::Applied, Some(_)) => {
            return Err(OutputError::UnexpectedReason {
                path: path.to_owned(),
            });
        }
        (MutationOutcome::Applied, None) | (MutationOutcome::NotApplied, Some(_)) => {}
    }
    let mut map = base("mutation");
    map.insert("path".to_owned(), Value::String(path.to_owned()));
    map.insert("kind".to_owned(), Value::String(kind.name().to_owned()));
    map.insert(
        "outcome".to_owned(),
        Value::String(
            match outcome {
                MutationOutcome::Applied => "applied",
                MutationOutcome::NotApplied => "not_applied",
            }
            .to_owned(),
        ),
    );
    if let Some(reason) = reason {
        map.insert("reason".to_owned(), Value::String(reason.to_owned()));
    }
    Ok(Value::Object(map))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn finding() -> DiagnosticEvent {
        DiagnosticEvent {
            severity: Severity::Warning,
            tool: "ruff".to_owned(),
            message: "Imported but unused".to_owned(),
            rule: Some("F401".to_owned()),
            path: Some("src/app.py".to_owned()),
            range: Some((18, 24)),
            snapshot: Snapshot::Initial,
            fixable: true,
            resolution: None,
        }
    }

    #[test]
    fn diagnostic_resolution_rules() {
        // Check mode initial omits resolution.
        diagnostic_event(&finding(), false).expect("check initial");
        // Default mode initial requires it.
        assert_eq!(
            diagnostic_event(&finding(), true).expect_err("missing resolution"),
            OutputError::MissingResolution
        );
        let mut terminal = finding();
        terminal.snapshot = Snapshot::Terminal;
        terminal.fixable = false;
        terminal.resolution = None;
        diagnostic_event(&terminal, true).expect("terminal");
        terminal.resolution = Some(Resolution::Fixed);
        assert_eq!(
            diagnostic_event(&terminal, true).expect_err("terminal resolution"),
            OutputError::UnexpectedResolution
        );
        let mut ranged = finding();
        ranged.path = None;
        assert_eq!(
            diagnostic_event(&ranged, false).expect_err("range without path"),
            OutputError::RangeWithoutPath
        );
    }

    #[test]
    fn diagnostics_sort_deterministically() {
        let mut events = vec![
            DiagnosticEvent {
                severity: Severity::Error,
                tool: "b".to_owned(),
                message: "m".to_owned(),
                rule: None,
                path: None,
                range: None,
                snapshot: Snapshot::Terminal,
                fixable: false,
                resolution: None,
            },
            DiagnosticEvent {
                severity: Severity::Info,
                tool: "a".to_owned(),
                message: "m".to_owned(),
                rule: None,
                path: Some("z.py".to_owned()),
                range: None,
                snapshot: Snapshot::Initial,
                fixable: false,
                resolution: None,
            },
            DiagnosticEvent {
                severity: Severity::Warning,
                tool: "a".to_owned(),
                message: "m".to_owned(),
                rule: None,
                path: Some("a.py".to_owned()),
                range: Some((5, 9)),
                snapshot: Snapshot::Initial,
                fixable: false,
                resolution: None,
            },
        ];
        sort_diagnostics(&mut events);
        let paths: Vec<Option<&str>> = events.iter().map(|e| e.path.as_deref()).collect();
        assert_eq!(paths, vec![Some("a.py"), Some("z.py"), None]);
        assert_eq!(events[2].snapshot, Snapshot::Terminal);
    }

    #[test]
    fn ignored_import_notice_shape() {
        let notice = NoticeEvent {
            level: "warning".to_owned(),
            code: "ignored_import".to_owned(),
            message: "ignored".to_owned(),
            related_command: None,
            scope: None,
            path: Some("src/plugin.py".to_owned()),
            language: Some("python".to_owned()),
            import: Some("optional_runtime_module".to_owned()),
        };
        let event = notice_event(&notice).expect("notice");
        assert!(event.get("scope").is_none());
        assert_eq!(
            event["import"],
            Value::String("optional_runtime_module".to_owned())
        );
        let mut bad = notice.clone();
        bad.language = None;
        assert!(notice_event(&bad).is_err());
    }

    #[test]
    fn change_shapes_validated() {
        let change = ChangeEvent {
            path: "src/app.py".to_owned(),
            kind: ChangeKind::Modify,
            source_digest: Some(DIGEST.to_owned()),
            edits: vec![Edit {
                start: 18,
                end: 24,
                replacement: String::new(),
            }],
        };
        let event = change_event(&change).expect("change");
        assert_eq!(event["edits"][0]["start_byte"], Value::from(18));
        let mut no_digest = change.clone();
        no_digest.source_digest = None;
        assert!(change_event(&no_digest).is_err());
        let create = ChangeEvent {
            path: "new/package/BUILD.bazel".to_owned(),
            kind: ChangeKind::Create,
            source_digest: None,
            edits: vec![Edit {
                start: 0,
                end: 0,
                replacement: "py_library(\n)\n".to_owned(),
            }],
        };
        change_event(&create).expect("create");
        let mut create_digest = create.clone();
        create_digest.source_digest = Some(DIGEST.to_owned());
        assert!(change_event(&create_digest).is_err());
    }

    #[test]
    fn mutation_reasons_required() {
        mutation_event("a.py", ChangeKind::Modify, MutationOutcome::Applied, None)
            .expect("applied");
        assert_eq!(
            mutation_event(
                "a.py",
                ChangeKind::Modify,
                MutationOutcome::NotApplied,
                None
            )
            .expect_err("reason required"),
            OutputError::MissingReason {
                path: "a.py".to_owned()
            }
        );
        assert!(mutation_event(
            "a.py",
            ChangeKind::Modify,
            MutationOutcome::Applied,
            Some("stale_source"),
        )
        .is_err());
    }

    #[test]
    fn inverted_range_fails() {
        let mut bad = finding();
        bad.range = Some((24, 18));
        assert_eq!(
            diagnostic_event(&bad, false).expect_err("inverted range"),
            OutputError::InvertedRange
        );
    }

    #[test]
    fn pathless_rangeless_diagnostic_renders() {
        let mut tool_level = finding();
        tool_level.path = None;
        tool_level.range = None;
        tool_level.snapshot = Snapshot::Terminal;
        let event = diagnostic_event(&tool_level, false).expect("tool-level");
        assert_eq!(event["snapshot"], Value::String("terminal".to_owned()));
        assert!(event.get("path").is_none());
        assert!(event.get("range").is_none());
    }

    #[test]
    fn resolutions_render_by_name() {
        for (resolution, name) in [
            (Resolution::Fixed, "fixed"),
            (Resolution::Remaining, "remaining"),
            (Resolution::NotApplied, "not_applied"),
        ] {
            let mut resolved = finding();
            resolved.resolution = Some(resolution);
            let event = diagnostic_event(&resolved, true).expect("resolved");
            assert_eq!(event["resolution"], Value::String(name.to_owned()));
        }
    }

    #[test]
    fn notice_level_and_scope_rules() {
        let mut bad_level = NoticeEvent {
            level: "debug".to_owned(),
            code: "other".to_owned(),
            message: "m".to_owned(),
            related_command: None,
            scope: None,
            path: None,
            language: None,
            import: None,
        };
        assert!(notice_event(&bad_level).is_err());
        bad_level.level = "info".to_owned();
        let event = notice_event(&bad_level).expect("plain notice");
        assert_eq!(event["level"], Value::String("info".to_owned()));
        // Non-import notices may carry scope and a related command.
        let mut scoped = bad_level.clone();
        scoped.related_command = Some("lint".to_owned());
        scoped.scope = Some(vec!["//src/...".to_owned()]);
        let event = notice_event(&scoped).expect("scoped notice");
        assert_eq!(event["related_command"], Value::String("lint".to_owned()));
        assert_eq!(event["scope"][0], Value::String("//src/...".to_owned()));
        // Import notices must not carry them.
        let mut import = NoticeEvent {
            level: "warning".to_owned(),
            code: "ignored_import".to_owned(),
            message: "ignored".to_owned(),
            related_command: Some("lint".to_owned()),
            scope: None,
            path: Some("src/plugin.py".to_owned()),
            language: Some("python".to_owned()),
            import: Some("mod".to_owned()),
        };
        assert_eq!(
            notice_event(&import).expect_err("scoped import"),
            OutputError::UnexpectedResolution
        );
        import.related_command = None;
        import.scope = Some(vec!["//src/...".to_owned()]);
        assert_eq!(
            notice_event(&import).expect_err("scoped import"),
            OutputError::UnexpectedResolution
        );
    }

    #[test]
    fn create_shape_violations_fail() {
        let two_edits = ChangeEvent {
            path: "new.txt".to_owned(),
            kind: ChangeKind::Create,
            source_digest: None,
            edits: vec![
                Edit {
                    start: 0,
                    end: 0,
                    replacement: "a".to_owned(),
                },
                Edit {
                    start: 0,
                    end: 0,
                    replacement: "b".to_owned(),
                },
            ],
        };
        assert!(change_event(&two_edits).is_err());
        let shifted = ChangeEvent {
            path: "new.txt".to_owned(),
            kind: ChangeKind::Create,
            source_digest: None,
            edits: vec![Edit {
                start: 3,
                end: 3,
                replacement: "a".to_owned(),
            }],
        };
        assert!(change_event(&shifted).is_err());
    }

    #[test]
    fn mutation_empty_reason_and_reason_field() {
        assert_eq!(
            mutation_event(
                "a.py",
                ChangeKind::Modify,
                MutationOutcome::NotApplied,
                Some("")
            )
            .expect_err("empty reason"),
            OutputError::MissingReason {
                path: "a.py".to_owned()
            }
        );
        let event = mutation_event(
            "a.py",
            ChangeKind::Modify,
            MutationOutcome::NotApplied,
            Some("disagreeing_agents"),
        )
        .expect("reasoned");
        assert_eq!(event["outcome"], Value::String("not_applied".to_owned()));
        assert_eq!(
            event["reason"],
            Value::String("disagreeing_agents".to_owned())
        );
    }
}
