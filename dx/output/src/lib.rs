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

use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Schema version
// ---------------------------------------------------------------------------

/// Breaking semantic version of the NDJSON stream. One invocation never
/// mixes schema versions; removing a field, making an optional field
/// required, or changing semantics requires a new major version.
pub const SCHEMA_MAJOR: u32 = 1;
/// Additive feature version: producers may add fields and event kinds and
/// consumers must ignore unknown ones within one major version.
pub const SCHEMA_MINOR: u32 = 0;

/// The `schema` object shared by every event in one invocation.
pub fn schema() -> Value {
    json!({"major": SCHEMA_MAJOR, "minor": SCHEMA_MINOR})
}

// ---------------------------------------------------------------------------
// Live output modes and stream ownership
// ---------------------------------------------------------------------------

/// Live output mode selecting stdout ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Concise human summaries; preserves subprocess stdout and stderr.
    Text { quiet: bool },
    /// Stdout carries only the complete unified patch.
    Diff,
    /// Stdout carries only NDJSON; subprocess output moves to stderr.
    Json,
}

impl OutputMode {
    /// Parses a `--output` value. `quiet` comes from `--quiet` and only
    /// affects text mode.
    pub fn parse(text: &str, quiet: bool) -> Result<Self, OutputError> {
        match text {
            "text" => Ok(OutputMode::Text { quiet }),
            "diff" => Ok(OutputMode::Diff),
            "json" => Ok(OutputMode::Json),
            _ => Err(OutputError::UnknownOutputMode {
                value: text.to_owned(),
            }),
        }
    }

    /// Canonical mode name for summaries and errors.
    pub fn name(&self) -> &'static str {
        match self {
            OutputMode::Text { .. } => "text",
            OutputMode::Diff => "diff",
            OutputMode::Json => "json",
        }
    }
}

/// Who owns stdout under a mode and report selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdoutOwner {
    /// `dx` text summaries share stdout with subprocess passthrough.
    DxText,
    /// Complete unified patch only; summaries and normalized diagnostics
    /// are suppressed rather than moved to stderr.
    Patch,
    /// One JSON object per line; no prose or subprocess bytes.
    Ndjson,
    /// Exactly one standard-report document; summaries and subprocess
    /// output move to stderr.
    Report,
}

/// Selects the stdout owner. `stdout_report` reports whether exactly one
/// standard report targets `-`; multiple stdout reports are rejected by
/// [`check_output_conflict`] before this is consulted.
pub fn stdout_owner(mode: &OutputMode, stdout_report: bool) -> StdoutOwner {
    if stdout_report {
        return StdoutOwner::Report;
    }
    match mode {
        OutputMode::Text { .. } => StdoutOwner::DxText,
        OutputMode::Diff => StdoutOwner::Patch,
        OutputMode::Json => StdoutOwner::Ndjson,
    }
}

/// Reports whether `dx` operation summaries are visible. `--quiet`
/// suppresses them in text mode; diff mode never shows them.
pub fn dx_text_visible(mode: &OutputMode) -> bool {
    matches!(mode, OutputMode::Text { quiet: false })
}

/// Rejects incompatible live-mode and stdout-report combinations: a stdout
/// report conflicts with `--output diff`, `--output json`, and any second
/// stdout report. File reports may accompany any live mode.
pub fn check_output_conflict(mode: &OutputMode, stdout_reports: usize) -> Result<(), OutputError> {
    if stdout_reports > 1 {
        return Err(OutputError::SecondStdoutReport);
    }
    if stdout_reports == 1 && !matches!(mode, OutputMode::Text { .. }) {
        return Err(OutputError::ConflictingStdoutReport { mode: mode.name() });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Severity and fail-on threshold
// ---------------------------------------------------------------------------

/// Normalized diagnostic severity. The rank order mirrors the direct-Bazel
/// evaluator scale so the CLI maps `--fail-on` to the same comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn rank(self) -> u32 {
        match self {
            Severity::Info => 0,
            Severity::Warning => 1,
            Severity::Error => 2,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }

    pub fn parse(text: &str) -> Result<Self, OutputError> {
        match text {
            "info" => Ok(Severity::Info),
            "warning" => Ok(Severity::Warning),
            "error" => Ok(Severity::Error),
            _ => Err(OutputError::BadSeverity {
                value: text.to_owned(),
            }),
        }
    }
}

/// Lowest diagnostic severity that fails a quality command. The default is
/// `warning`; there is no `never` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Threshold {
    Info,
    Warning,
    Error,
}

impl Threshold {
    pub fn rank(self) -> u32 {
        match self {
            Threshold::Info => 0,
            Threshold::Warning => 1,
            Threshold::Error => 2,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Threshold::Info => "info",
            Threshold::Warning => "warning",
            Threshold::Error => "error",
        }
    }

    pub fn parse(text: &str) -> Result<Self, OutputError> {
        match text {
            "info" => Ok(Threshold::Info),
            "warning" => Ok(Threshold::Warning),
            "error" => Ok(Threshold::Error),
            _ => Err(OutputError::BadThreshold {
                value: text.to_owned(),
            }),
        }
    }
}

/// Reports whether a finding at `severity` fails under `threshold`, using
/// the same rank comparison as direct-Bazel evaluation.
pub fn meets_threshold(severity: Severity, threshold: Threshold) -> bool {
    severity.rank() >= threshold.rank()
}

// ---------------------------------------------------------------------------
// Protocol-shape validation
// ---------------------------------------------------------------------------

/// Output or protocol failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputError {
    UnknownOutputMode {
        value: String,
    },
    /// A stdout report combined with `--output diff` or `--output json`.
    ConflictingStdoutReport {
        mode: &'static str,
    },
    /// More than one standard report targets stdout.
    SecondStdoutReport,
    EmptyField {
        field: &'static str,
    },
    /// `command_started` mode outside `default|check`.
    BadCommandMode {
        value: String,
    },
    BadSeverity {
        value: String,
    },
    BadThreshold {
        value: String,
    },
    BadPath {
        path: String,
        reason: &'static str,
    },
    BadDigest {
        field: &'static str,
        value: String,
    },
    BadEdit {
        index: usize,
        reason: &'static str,
    },
    RangeWithoutPath,
    InvertedRange,
    /// A default-mode initial diagnostic without its required resolution.
    MissingResolution,
    /// A check-mode or terminal diagnostic carrying a resolution.
    UnexpectedResolution,
    /// A mutation without its required failure reason, or an applied
    /// mutation carrying one.
    MissingReason {
        path: String,
    },
    UnexpectedReason {
        path: String,
    },
    /// A value that is not an NDJSON event object.
    NotAnEvent,
    Io(String),
}

impl std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for OutputError {}

fn nonempty(field: &'static str, value: &str) -> Result<(), OutputError> {
    if value.is_empty() {
        return Err(OutputError::EmptyField { field });
    }
    Ok(())
}

/// Validates a normalized workspace-relative source path: valid UTF-8,
/// slash-separated, non-empty, lexical, no `.` or `..` component, beneath
/// the main workspace. Mirrors the result-protocol path rules so JSON-shape
/// failures surface before diff output or mutation planning.
pub fn check_path(path: &str) -> Result<(), OutputError> {
    let reason = if path.is_empty() {
        Some("path must be non-empty")
    } else if path.starts_with('/') {
        Some("path must be workspace-relative, not absolute")
    } else if path.contains('\\') {
        Some("path must use forward slashes")
    } else if path.split('/').any(str::is_empty) {
        Some("path must have no empty component")
    } else if path.split('/').any(|c| c == ".") {
        Some("path must have no '.' component")
    } else if path.split('/').any(|c| c == "..") {
        Some("path must have no '..' component")
    } else {
        None
    };
    match reason {
        Some(reason) => Err(OutputError::BadPath {
            path: path.to_owned(),
            reason,
        }),
        None => Ok(()),
    }
}

/// Parses exactly 64 lowercase hexadecimal characters encoding a
/// BLAKE3-256 digest, used for change source digests and selection IDs.
pub fn parse_digest(field: &'static str, text: &str) -> Result<[u8; 32], OutputError> {
    let bad = || OutputError::BadDigest {
        field,
        value: text.to_owned(),
    };
    if text.len() != 64
        || !text
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(bad());
    }
    let mut out = [0u8; 32];
    for (i, chunk) in text.as_bytes().chunks(2).enumerate() {
        out[i] = u8::from_str_radix(std::str::from_utf8(chunk).map_err(|_| bad())?, 16)
            .map_err(|_| bad())?;
    }
    Ok(out)
}

/// One exact replacement edit: a half-open UTF-8 byte range in the
/// original file plus its exact new UTF-8 content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub start: u64,
    pub end: u64,
    pub replacement: String,
}

/// Validates edit shape without source bytes: the set is non-empty, in
/// strictly increasing `start` order with distinct starts, every
/// `start <= end`, and no insertion sits inside a replaced range
/// (`prev.end <= next.start`). No-op edits, UTF-8 boundaries, and digest
/// agreement need source bytes and are validated during apply.
pub fn check_edits(edits: &[Edit]) -> Result<(), OutputError> {
    if edits.is_empty() {
        return Err(OutputError::BadEdit {
            index: 0,
            reason: "edit set must be non-empty",
        });
    }
    for (index, edit) in edits.iter().enumerate() {
        if edit.start > edit.end {
            return Err(OutputError::BadEdit {
                index,
                reason: "start must not exceed end",
            });
        }
        if index > 0 {
            let prev = &edits[index - 1];
            if edit.start <= prev.start {
                return Err(OutputError::BadEdit {
                    index,
                    reason: "edits must be in strictly increasing start order",
                });
            }
            if prev.end > edit.start {
                return Err(OutputError::BadEdit {
                    index,
                    reason: "an insertion cannot sit inside a replaced range",
                });
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// NDJSON events
// ---------------------------------------------------------------------------

fn base(event: &str) -> serde_json::Map<String, Value> {
    let mut map = serde_json::Map::new();
    map.insert("schema".to_owned(), schema());
    map.insert("event".to_owned(), Value::String(event.to_owned()));
    map
}

/// First event of every successfully initialized JSON stream.
pub fn command_started(command: &str, dry_run: bool, mode: &str) -> Result<Value, OutputError> {
    nonempty("command", command)?;
    if mode != "default" && mode != "check" {
        return Err(OutputError::BadCommandMode {
            value: mode.to_owned(),
        });
    }
    let mut map = base("command_started");
    map.insert("command".to_owned(), Value::String(command.to_owned()));
    map.insert("dry_run".to_owned(), Value::Bool(dry_run));
    map.insert("mode".to_owned(), Value::String(mode.to_owned()));
    Ok(Value::Object(map))
}

/// Announces a durable workflow phase before it begins (or would begin
/// under dry-run). `scope` is the compact effective main-workspace scope,
/// omitted before graph resolution.
pub fn operation_event(
    command: &str,
    phase: &str,
    scope: Option<&[String]>,
) -> Result<Value, OutputError> {
    nonempty("command", command)?;
    nonempty("phase", phase)?;
    let mut map = base("operation");
    map.insert("command".to_owned(), Value::String(command.to_owned()));
    map.insert("phase".to_owned(), Value::String(phase.to_owned()));
    if let Some(scope) = scope {
        map.insert(
            "scope".to_owned(),
            Value::Array(scope.iter().map(|s| Value::String(s.clone())).collect()),
        );
    }
    Ok(Value::Object(map))
}

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

/// Confirms a successfully emitted file report after atomic replacement.
/// A stdout report cannot coexist with NDJSON, so this event is file-only.
pub fn report_event(
    format: &str,
    path: &str,
    results_complete: bool,
) -> Result<Value, OutputError> {
    nonempty("format", format)?;
    nonempty("path", path)?;
    let mut map = base("report");
    map.insert("format".to_owned(), Value::String(format.to_owned()));
    map.insert("path".to_owned(), Value::String(path.to_owned()));
    map.insert("results_complete".to_owned(), Value::Bool(results_complete));
    Ok(Value::Object(map))
}

/// Confirms the successfully validated state selected by `env`, `codegen`,
/// or `setup`. All identities are 64 lowercase hexadecimal characters.
pub fn selection_event(
    setup_id: &str,
    environment_id: &str,
    codegen_id: &str,
) -> Result<Value, OutputError> {
    parse_digest("setup_id", setup_id)?;
    parse_digest("environment_id", environment_id)?;
    parse_digest("codegen_id", codegen_id)?;
    let mut map = base("selection");
    map.insert("setup_id".to_owned(), Value::String(setup_id.to_owned()));
    map.insert(
        "environment_id".to_owned(),
        Value::String(environment_id.to_owned()),
    );
    map.insert(
        "codegen_id".to_owned(),
        Value::String(codegen_id.to_owned()),
    );
    Ok(Value::Object(map))
}

/// CLI, orchestration, protocol, or infrastructure failure. Never carries
/// argv, option values, environment values, external labels, or raw tool
/// output; `code` is stable machine data while `message` is for people.
pub fn error_event(
    code: &str,
    message: &str,
    path: Option<&str>,
    flag: Option<&str>,
    phase: Option<&str>,
) -> Result<Value, OutputError> {
    nonempty("code", code)?;
    nonempty("message", message)?;
    if let Some(path) = path {
        check_path(path)?;
    }
    if let Some(flag) = flag {
        nonempty("flag", flag)?;
    }
    if let Some(phase) = phase {
        nonempty("phase", phase)?;
    }
    let mut map = base("error");
    map.insert("code".to_owned(), Value::String(code.to_owned()));
    map.insert("message".to_owned(), Value::String(message.to_owned()));
    if let Some(path) = path {
        map.insert("path".to_owned(), Value::String(path.to_owned()));
    }
    if let Some(flag) = flag {
        map.insert("flag".to_owned(), Value::String(flag.to_owned()));
    }
    if let Some(phase) = phase {
        map.insert("phase".to_owned(), Value::String(phase.to_owned()));
    }
    Ok(Value::Object(map))
}

/// Optional aggregate counts for the final event. Presence follows the
/// protocol: each object appears exactly for the commands and modes that
/// produce it, including zero values. Callers select presence; this crate
/// only renders.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FinishedCounts {
    pub results_complete: Option<bool>,
    pub diagnostics: Option<[u64; 3]>,
    pub changes: Option<[u64; 2]>,
    pub mutations: Option<[u64; 2]>,
}

/// Last event of every normally terminating JSON invocation. There is no
/// textual status: `exit_code == 0` means success.
pub fn command_finished(exit_code: i32, counts: &FinishedCounts) -> Value {
    let mut map = base("command_finished");
    map.insert("exit_code".to_owned(), Value::from(exit_code));
    if let Some(results_complete) = counts.results_complete {
        map.insert("results_complete".to_owned(), Value::Bool(results_complete));
    }
    if let Some([info, warning, error]) = counts.diagnostics {
        map.insert(
            "diagnostics".to_owned(),
            json!({"info": info, "warning": warning, "error": error}),
        );
    }
    if let Some([create, modify]) = counts.changes {
        map.insert(
            "changes".to_owned(),
            json!({"create": create, "modify": modify}),
        );
    }
    if let Some([applied, not_applied]) = counts.mutations {
        map.insert(
            "mutations".to_owned(),
            json!({"applied": applied, "not_applied": not_applied}),
        );
    }
    Value::Object(map)
}

/// Writes one NDJSON line: exactly one complete UTF-8 JSON object followed
/// by `\n`. Rejects values that are not event objects so prose can never
/// leak into a machine-only stream.
pub fn write_event(writer: &mut dyn std::io::Write, event: &Value) -> Result<(), OutputError> {
    match event {
        Value::Object(map) => {
            let is_event =
                matches!(map.get("event"), Some(Value::String(_))) && map.contains_key("schema");
            if !is_event {
                return Err(OutputError::NotAnEvent);
            }
        }
        _ => return Err(OutputError::NotAnEvent),
    }
    serde_json::to_writer(&mut *writer, event).map_err(|e| OutputError::Io(e.to_string()))?;
    writer
        .write_all(b"\n")
        .map_err(|e| OutputError::Io(e.to_string()))
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
    fn modes_parse_and_name() {
        assert_eq!(
            OutputMode::parse("text", false).expect("text"),
            OutputMode::Text { quiet: false }
        );
        assert_eq!(
            OutputMode::parse("text", true).expect("quiet"),
            OutputMode::Text { quiet: true }
        );
        assert_eq!(
            OutputMode::parse("diff", false).expect("diff"),
            OutputMode::Diff
        );
        assert_eq!(
            OutputMode::parse("json", true).expect("json"),
            OutputMode::Json
        );
        assert!(OutputMode::parse("xml", false).is_err());
    }

    #[test]
    fn stdout_ownership_per_mode() {
        let text = OutputMode::Text { quiet: false };
        assert_eq!(stdout_owner(&text, false), StdoutOwner::DxText);
        assert_eq!(stdout_owner(&text, true), StdoutOwner::Report);
        assert_eq!(stdout_owner(&OutputMode::Diff, false), StdoutOwner::Patch);
        assert_eq!(stdout_owner(&OutputMode::Json, false), StdoutOwner::Ndjson);
        assert_eq!(stdout_owner(&OutputMode::Json, true), StdoutOwner::Report);
    }

    #[test]
    fn quiet_suppresses_text_only() {
        assert!(dx_text_visible(&OutputMode::Text { quiet: false }));
        assert!(!dx_text_visible(&OutputMode::Text { quiet: true }));
        assert!(!dx_text_visible(&OutputMode::Diff));
        assert!(!dx_text_visible(&OutputMode::Json));
    }

    #[test]
    fn stdout_report_conflicts() {
        let text = OutputMode::Text { quiet: false };
        check_output_conflict(&text, 0).expect("none ok");
        check_output_conflict(&text, 1).expect("one text report ok");
        assert_eq!(
            check_output_conflict(&text, 2).expect_err("second stdout report"),
            OutputError::SecondStdoutReport
        );
        assert_eq!(
            check_output_conflict(&OutputMode::Diff, 1).expect_err("diff conflict"),
            OutputError::ConflictingStdoutReport { mode: "diff" }
        );
        assert_eq!(
            check_output_conflict(&OutputMode::Json, 1).expect_err("json conflict"),
            OutputError::ConflictingStdoutReport { mode: "json" }
        );
    }

    #[test]
    fn threshold_comparison_matches_evaluator_order() {
        assert!(meets_threshold(Severity::Warning, Threshold::Warning));
        assert!(meets_threshold(Severity::Error, Threshold::Warning));
        assert!(!meets_threshold(Severity::Info, Threshold::Warning));
        assert!(meets_threshold(Severity::Info, Threshold::Info));
        assert!(!meets_threshold(Severity::Warning, Threshold::Error));
        assert_eq!(
            Threshold::parse("warning").expect("warning"),
            Threshold::Warning
        );
        assert!(Threshold::parse("never").is_err());
        assert!(Severity::parse("error").expect("error") == Severity::Error);
    }

    #[test]
    fn paths_mirror_result_protocol_rules() {
        check_path("src/app.py").expect("valid");
        assert!(check_path("").is_err());
        assert!(check_path("/abs").is_err());
        assert!(check_path("a\\b").is_err());
        assert!(check_path("a//b").is_err());
        assert!(check_path("./a").is_err());
        assert!(check_path("a/../b").is_err());
    }

    #[test]
    fn digests_require_lowercase_hex() {
        assert_eq!(parse_digest("d", DIGEST).expect("valid").len(), 32);
        assert!(parse_digest("d", &DIGEST.to_uppercase()).is_err());
        assert!(parse_digest("d", "0123").is_err());
        assert!(parse_digest("d", &format!("{DIGEST}00")).is_err());
        assert!(parse_digest("d", &"zz".repeat(32)).is_err());
    }

    #[test]
    fn edit_shapes_validated() {
        let edits = vec![
            Edit {
                start: 0,
                end: 5,
                replacement: String::new(),
            },
            Edit {
                start: 5,
                end: 5,
                replacement: "x".to_owned(),
            },
        ];
        check_edits(&edits).expect("adjacent ok");
        assert!(check_edits(&[]).is_err());
        assert!(check_edits(&[Edit {
            start: 9,
            end: 3,
            replacement: String::new(),
        }])
        .is_err());
        // Shared start offsets are invalid even for pure insertions.
        assert!(check_edits(&[
            Edit {
                start: 4,
                end: 4,
                replacement: "a".to_owned(),
            },
            Edit {
                start: 4,
                end: 4,
                replacement: "b".to_owned(),
            },
        ])
        .is_err());
        // An insertion inside a replaced range is invalid.
        assert!(check_edits(&[
            Edit {
                start: 2,
                end: 8,
                replacement: "a".to_owned(),
            },
            Edit {
                start: 5,
                end: 5,
                replacement: "b".to_owned(),
            },
        ])
        .is_err());
    }

    #[test]
    fn command_started_shape() {
        let event = command_started("lint", false, "default").expect("started");
        assert_eq!(event["event"], Value::String("command_started".to_owned()));
        assert_eq!(event["schema"], schema());
        assert_eq!(event["mode"], Value::String("default".to_owned()));
        assert!(command_started("lint", false, "fancy").is_err());
    }

    #[test]
    fn operation_scope_omitted_before_resolution() {
        let event = operation_event("lint", "resolve", None).expect("op");
        assert!(event.get("scope").is_none());
        let scope = vec!["//src/auth/...".to_owned()];
        let event = operation_event("lint", "execute", Some(&scope)).expect("op");
        assert_eq!(
            event["scope"][0],
            Value::String("//src/auth/...".to_owned())
        );
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
    fn error_event_never_carries_values() {
        let event = error_event(
            "conflicting_option",
            "keeps going",
            None,
            Some("--keep_going"),
            None,
        )
        .expect("error");
        assert_eq!(event["flag"], Value::String("--keep_going".to_owned()));
        assert!(event.get("path").is_none());
    }

    #[test]
    fn finished_counts_render_exactly() {
        let counts = FinishedCounts {
            results_complete: Some(true),
            diagnostics: Some([0, 3, 1]),
            changes: None,
            mutations: Some([0, 2]),
        };
        let event = command_finished(1, &counts);
        assert_eq!(event["exit_code"], Value::from(1));
        assert_eq!(event["diagnostics"]["warning"], Value::from(3));
        assert!(event.get("changes").is_none());
    }

    #[test]
    fn writer_rejects_prose() {
        let mut buf = Vec::new();
        let event = command_started("lint", false, "default").expect("started");
        write_event(&mut buf, &event).expect("write");
        assert!(buf.ends_with(b"\n"));
        assert_eq!(buf.iter().filter(|b| **b == b'\n').count(), 1);
        let prose = Value::String("Running lint".to_owned());
        assert_eq!(
            write_event(&mut Vec::new(), &prose).expect_err("prose rejected"),
            OutputError::NotAnEvent
        );
    }

    #[test]
    fn mode_names_cover_all_modes() {
        assert_eq!(OutputMode::Text { quiet: false }.name(), "text");
        assert_eq!(OutputMode::Diff.name(), "diff");
        assert_eq!(OutputMode::Json.name(), "json");
    }

    #[test]
    fn severity_names_and_bad_parse() {
        assert_eq!(Severity::Info.name(), "info");
        assert_eq!(Severity::Warning.name(), "warning");
        assert_eq!(Severity::Error.name(), "error");
        assert_eq!(
            Severity::parse("bogus").expect_err("bad severity"),
            OutputError::BadSeverity {
                value: "bogus".to_owned()
            }
        );
    }

    #[test]
    fn threshold_names_cover_all_levels() {
        assert_eq!(Threshold::Info.name(), "info");
        assert_eq!(Threshold::Warning.name(), "warning");
        assert_eq!(Threshold::Error.name(), "error");
    }

    #[test]
    fn display_renders_debug_shape() {
        assert_eq!(
            OutputError::SecondStdoutReport.to_string(),
            "SecondStdoutReport"
        );
        assert_eq!(
            OutputError::MissingResolution.to_string(),
            "MissingResolution"
        );
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

    #[test]
    fn report_event_shape() {
        let event = report_event("sarif", "out.sarif", true).expect("report");
        assert_eq!(event["event"], Value::String("report".to_owned()));
        assert_eq!(event["format"], Value::String("sarif".to_owned()));
        assert_eq!(event["results_complete"], Value::Bool(true));
        assert!(report_event("", "out.sarif", true).is_err());
        assert!(report_event("sarif", "", true).is_err());
    }

    #[test]
    fn selection_event_shape() {
        let event = selection_event(DIGEST, DIGEST, DIGEST).expect("selection");
        assert_eq!(event["event"], Value::String("selection".to_owned()));
        assert_eq!(event["setup_id"], Value::String(DIGEST.to_owned()));
        assert!(selection_event("nope", DIGEST, DIGEST).is_err());
        assert!(selection_event(DIGEST, DIGEST, "nope").is_err());
    }

    #[test]
    fn error_event_carries_optional_fields() {
        let event = error_event(
            "bazel_failed",
            "build broke",
            Some("src/app.py"),
            None,
            Some("execute"),
        )
        .expect("error");
        assert_eq!(event["path"], Value::String("src/app.py".to_owned()));
        assert_eq!(event["phase"], Value::String("execute".to_owned()));
        assert!(event.get("flag").is_none());
        assert!(error_event("", "m", None, None, None).is_err());
        assert!(error_event("c", "", None, None, None).is_err());
        assert!(error_event("c", "m", Some("/abs"), None, None).is_err());
        assert!(error_event("c", "m", None, Some(""), None).is_err());
        assert!(error_event("c", "m", None, None, Some("")).is_err());
    }

    #[test]
    fn finished_changes_render() {
        let counts = FinishedCounts {
            results_complete: None,
            diagnostics: None,
            changes: Some([1, 2]),
            mutations: None,
        };
        let event = command_finished(0, &counts);
        assert_eq!(event["changes"]["create"], Value::from(1));
        assert_eq!(event["changes"]["modify"], Value::from(2));
        assert!(event.get("diagnostics").is_none());
    }

    #[test]
    fn writer_rejects_schemaless_object() {
        let fake = serde_json::json!({"event": "command_started"});
        assert_eq!(
            write_event(&mut Vec::new(), &fake).expect_err("no schema"),
            OutputError::NotAnEvent
        );
    }
}
