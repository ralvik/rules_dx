//! Deterministic shared quality pipeline runner (M03 WP2b).
//!
//! Contract: `docs/quality/quality-result-protocol.md`,
//! `docs/quality/tool-integrations.md`, schema `//quality:result.proto`.
//! The runner executes one ordered lint/typecheck/format pipeline (or one
//! audit tool set) over exact input bytes with the synthetic WP2 tool
//! behaviors (`fmt-a`, `lint-a`, `lint-b` from `//quality:adapters.bzl`),
//! iterates the convergence protocol to a terminal state, and emits a
//! normalized `QualityResult` that passes `quality_result::validate`.
//! Real tool process execution lands with later adapters; the convergence,
//! snapshot, diagnostic, and edit-derivation semantics here are final.

use std::collections::{BTreeMap, HashSet};

use dx_digest::{blake3 as digest, DIGEST_LEN};
use quality_result::{
    proto::{
        Capability, Convergence, Diagnostic, Edit, FileEdits, FileSnapshot, QualityResult,
        Severity, Stage,
    },
    MAX_COMPLETED_ROUNDS, SCHEMA_MAJOR, SCHEMA_MINOR,
};

pub mod real;

/// Synthetic tool IDs executed by this runner (WP2 adapters).
pub const SYNTHETIC_TOOLS: &[&str] = &["fmt-a", "lint-a", "lint-b"];

/// One ordered pipeline stage: tool identity plus its fixed source subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageSpec {
    pub tool_id: String,
    pub class_ids: Vec<String>,
    pub source_paths: Vec<String>,
}

/// One exact input file: workspace path plus original bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInput {
    pub path: String,
    pub bytes: Vec<u8>,
}

/// Pipeline construction or execution failure. Tool launch,
/// configuration, or protocol failure fails the action instead of
/// producing a result, so every error here is an action failure, never a
/// stored diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RunnerError {
    #[error("empty producer: want a non-empty producer")]
    EmptyProducer,
    #[error("unknown capability {capability:?}: want lint, typecheck, format, or audit")]
    UnknownCapability { capability: String },
    #[error("empty stages: want at least one stage")]
    EmptyStages,
    #[error("empty tool id at stage {stage}")]
    EmptyToolId { stage: usize },
    #[error("unknown tool {tool_id:?}")]
    UnknownTool { tool_id: String },
    #[error("empty class ids at stage {stage}")]
    EmptyClassIds { stage: usize },
    #[error("empty stage sources at stage {stage}")]
    EmptyStageSources { stage: usize },
    #[error("duplicate file {path:?}")]
    DuplicateFile { path: String },
    #[error("invalid UTF-8 for {path:?}")]
    InvalidUtf8 { path: String },
    #[error("missing file {path:?}")]
    MissingFile { path: String },
    /// Tool launch, scratch, or I/O failure in a real backend.
    #[error("tool execution failed for {tool_id}: {detail}")]
    ToolExecution { tool_id: String, detail: String },
    /// Real tool output outside the pinned grammar, or non-UTF-8 bytes
    /// where the protocol needs text.
    #[error("invalid tool output for {tool_id}: {detail}")]
    ToolOutput { tool_id: String, detail: String },
    /// A parsed finding positions outside the bytes just checked.
    #[error("unplaceable finding for {tool_id}: {detail}")]
    UnplaceableFinding { tool_id: String, detail: String },
}

fn parse_capability(capability: &str) -> Result<i32, RunnerError> {
    match capability {
        "lint" => Ok(Capability::Lint as i32),
        "typecheck" => Ok(Capability::Typecheck as i32),
        "format" => Ok(Capability::Format as i32),
        "audit" => Ok(Capability::Audit as i32),
        _ => Err(RunnerError::UnknownCapability {
            capability: capability.to_owned(),
        }),
    }
}

fn trim_trailing_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        match line.strip_suffix('\n') {
            Some(body) => {
                out.push_str(body.trim_end_matches([' ', '\t']));
                out.push('\n');
            }
            None => out.push_str(line.trim_end_matches([' ', '\t'])),
        }
    }
    out
}

fn apply_synthetic(tool_id: &str, text: &str) -> String {
    match tool_id {
        "lint-a" => text.replace("BAD", "GOOD"),
        "fmt-a" => trim_trailing_whitespace(text),
        "lint-b" => text.to_owned(),
        _ => text.to_owned(),
    }
}

fn collect_diagnostics(tool_id: &str, path: &str, text: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let (needle, severity, message) = match tool_id {
        "lint-a" => ("BAD", Severity::Warning as i32, "synthetic lint-a finding"),
        "lint-b" => ("FAIL", Severity::Error as i32, "synthetic lint-b finding"),
        _ => return out,
    };
    for (offset, _) in text.match_indices(needle) {
        out.push(Diagnostic {
            severity,
            message: message.to_owned(),
            tool_id: tool_id.to_owned(),
            path: path.to_owned(),
            start_byte: Some(offset as u64),
            end_byte: Some((offset + needle.len()) as u64),
            fixable: false,
            ..Default::default()
        });
    }
    out
}

/// Canonical identity of one converged-run state: BLAKE3 over the
/// concatenation of per-file `(digest(path), digest(body))` pairs in
/// sorted-path order. Keys never change within a run, so equal maps
/// hash equal; distinct maps collide only by breaking BLAKE3. Hashing
/// borrows the map, so the oscillation history stores 32-byte ids
/// instead of full-map clones and probes in O(1).
fn state_digest(files: &BTreeMap<String, String>) -> [u8; 32] {
    let mut canonical = Vec::with_capacity(files.len() * 2 * DIGEST_LEN);
    for (path, body) in files {
        canonical.extend_from_slice(&digest(path.as_bytes()));
        canonical.extend_from_slice(&digest(body.as_bytes()));
    }
    digest(&canonical)
}

fn run_convergence(
    initial: &BTreeMap<String, String>,
    stages: &[StageSpec],
    max_rounds: u32,
    apply: impl Fn(&str, &str, &str) -> Result<String, RunnerError>,
) -> Result<(BTreeMap<String, String>, u32, Convergence), RunnerError> {
    let mut current = initial.clone();
    let mut seen = HashSet::with_capacity(max_rounds.min(1024) as usize + 1);
    let mut prev_id = state_digest(initial);
    seen.insert(prev_id);
    let mut completed_rounds = 0;
    for round in 1..=max_rounds {
        completed_rounds = round;
        let mut changed = false;
        for stage in stages {
            for path in &stage.source_paths {
                let body = current
                    .get(path)
                    .ok_or(RunnerError::MissingFile { path: path.clone() })?;
                let next = apply(&stage.tool_id, path, body)?;
                if next != *body {
                    changed = true;
                }
                current.insert(path.clone(), next);
            }
        }
        if !changed {
            return Ok((current, completed_rounds, Convergence::Stable));
        }
        // Applies may move individual files yet return the map to its
        // round-start state (e.g. two formatters undoing each other), so
        // stability compares round-end to round-start identity, exactly
        // like the old full-map equality but over 32-byte digests.
        let id = state_digest(&current);
        if id == prev_id {
            return Ok((current, completed_rounds, Convergence::Stable));
        }
        if !seen.insert(id) {
            return Ok((current, completed_rounds, Convergence::Oscillation));
        }
        prev_id = id;
    }
    Ok((current, completed_rounds, Convergence::IterationLimit))
}

fn snapshot(files: &BTreeMap<String, String>) -> Vec<FileSnapshot> {
    files
        .iter()
        .map(|(path, body)| FileSnapshot {
            path: path.clone(),
            digest: digest(body.as_bytes()).to_vec(),
        })
        .collect()
}

fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(|a, b| {
        (&a.path, a.start_byte, a.end_byte, &a.tool_id, &a.message).cmp(&(
            &b.path,
            b.start_byte,
            b.end_byte,
            &b.tool_id,
            &b.message,
        ))
    });
}

/// Validates one pipeline request and decodes the exact input bytes.
/// `tool_known` decides the stage tool set: the synthetic registry for
/// [`run_pipeline`], backend resolution for real pipelines. Path shape
/// itself is validated by `quality_result::validate` at encode time.
fn validate_request(
    producer: &str,
    capability: &str,
    stages: &[StageSpec],
    files: &[FileInput],
    tool_known: impl Fn(&str) -> bool,
) -> Result<(i32, BTreeMap<String, String>), RunnerError> {
    if producer.is_empty() {
        return Err(RunnerError::EmptyProducer);
    }
    let capability_value = parse_capability(capability)?;
    if stages.is_empty() {
        return Err(RunnerError::EmptyStages);
    }
    for (index, stage) in stages.iter().enumerate() {
        if stage.tool_id.is_empty() {
            return Err(RunnerError::EmptyToolId { stage: index });
        }
        if !tool_known(&stage.tool_id) {
            return Err(RunnerError::UnknownTool {
                tool_id: stage.tool_id.clone(),
            });
        }
        if stage.class_ids.is_empty() {
            return Err(RunnerError::EmptyClassIds { stage: index });
        }
        if stage.source_paths.is_empty() {
            return Err(RunnerError::EmptyStageSources { stage: index });
        }
    }
    let mut initial: BTreeMap<String, String> = BTreeMap::new();
    for file in files {
        if initial.contains_key(&file.path) {
            return Err(RunnerError::DuplicateFile {
                path: file.path.clone(),
            });
        }
        let text = std::str::from_utf8(&file.bytes).map_err(|_| RunnerError::InvalidUtf8 {
            path: file.path.clone(),
        })?;
        initial.insert(file.path.clone(), text.to_owned());
    }
    for stage in stages {
        for path in &stage.source_paths {
            if !initial.contains_key(path) {
                return Err(RunnerError::MissingFile { path: path.clone() });
            }
        }
    }
    Ok((capability_value, initial))
}

/// Assembles the normalized result from a converged run: sorted
/// diagnostics, whole-file replacements for stable changed files, and
/// fixability for initial findings that the terminal state resolves.
/// Shared by synthetic and real pipelines so the semantics cannot drift.
/// Diagnostics and outcome travel as pairs so the shared helper stays
/// under the complexity budget without splitting its single purpose.
/// Stage paths absent from either map fail with [`RunnerError::MissingFile`]
/// instead of panicking, so validation drift surfaces as an action error.
fn assemble(
    producer: &str,
    capability_value: i32,
    stages: &[StageSpec],
    initial: &BTreeMap<String, String>,
    terminal: &BTreeMap<String, String>,
    diagnostics: (Vec<Diagnostic>, Vec<Diagnostic>),
    outcome: (u32, Convergence),
) -> Result<QualityResult, RunnerError> {
    let (mut initial_diagnostics, mut terminal_diagnostics) = diagnostics;
    let (completed_rounds, convergence) = outcome;
    sort_diagnostics(&mut initial_diagnostics);
    sort_diagnostics(&mut terminal_diagnostics);
    let stable = convergence == Convergence::Stable;
    let mut replacements = Vec::new();
    for (path, original) in initial {
        let terminal_body = terminal
            .get(path)
            .ok_or(RunnerError::MissingFile { path: path.clone() })?;
        if stable && terminal_body != original {
            replacements.push(FileEdits {
                path: path.clone(),
                original_digest: digest(original.as_bytes()).to_vec(),
                edits: vec![Edit {
                    start_byte: 0,
                    end_byte: original.len() as u64,
                    replacement: terminal_body.as_bytes().to_vec(),
                }],
            });
        }
    }
    for diagnostic in &mut initial_diagnostics {
        let resolved = replacements
            .iter()
            .any(|edits| edits.path == diagnostic.path);
        let persists = terminal_diagnostics.iter().any(|terminal_diagnostic| {
            terminal_diagnostic.tool_id == diagnostic.tool_id
                && terminal_diagnostic.message == diagnostic.message
                && terminal_diagnostic.path == diagnostic.path
        });
        diagnostic.fixable = stable && resolved && !persists;
    }
    let stages_proto = stages
        .iter()
        .map(|stage| Stage {
            tool_id: stage.tool_id.clone(),
            class_ids: stage.class_ids.clone(),
            source_paths: stage.source_paths.clone(),
        })
        .collect();
    Ok(QualityResult {
        schema_major: SCHEMA_MAJOR,
        schema_minor: SCHEMA_MINOR,
        producer: producer.to_owned(),
        capability: capability_value,
        stages: stages_proto,
        completed_rounds,
        convergence: convergence as i32,
        original_snapshot: snapshot(initial),
        terminal_snapshot: snapshot(terminal),
        initial_diagnostics,
        terminal_diagnostics,
        replacements,
    })
}

/// One exact input file's text within a converged run. A stage path
/// absent from the map fails with [`RunnerError::MissingFile`] instead of
/// panicking, so validation drift surfaces as an action error.
fn stage_subset(
    stage: &StageSpec,
    files: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, RunnerError> {
    let mut out = BTreeMap::new();
    for path in &stage.source_paths {
        let body = files
            .get(path)
            .ok_or(RunnerError::MissingFile { path: path.clone() })?;
        out.insert(path.clone(), body.clone());
    }
    Ok(out)
}

/// Executes one ordered pipeline over exact input bytes and returns the
/// normalized result. The result passes `quality_result::validate` for
/// well-formed workspace paths; path shape itself is validated there.
pub fn run_pipeline(
    producer: &str,
    capability: &str,
    stages: &[StageSpec],
    files: &[FileInput],
) -> Result<QualityResult, RunnerError> {
    let (capability_value, initial) =
        validate_request(producer, capability, stages, files, |tool| {
            SYNTHETIC_TOOLS.contains(&tool)
        })?;
    let mut initial_diagnostics = Vec::new();
    for stage in stages {
        for path in &stage.source_paths {
            let body = initial
                .get(path)
                .ok_or(RunnerError::MissingFile { path: path.clone() })?;
            initial_diagnostics.extend(collect_diagnostics(&stage.tool_id, path, body));
        }
    }
    // The synthetic apply closure is infallible, but convergence still
    // reports `MissingFile` instead of panicking if stage/validation drift
    // ever desynchronizes the maps, so propagate rather than expect.
    let (terminal, completed_rounds, convergence) = run_convergence(
        &initial,
        stages,
        MAX_COMPLETED_ROUNDS,
        |tool, _path, text| Ok(apply_synthetic(tool, text)),
    )?;
    let mut terminal_diagnostics = Vec::new();
    for stage in stages {
        for path in &stage.source_paths {
            let body = terminal
                .get(path)
                .ok_or(RunnerError::MissingFile { path: path.clone() })?;
            terminal_diagnostics.extend(collect_diagnostics(&stage.tool_id, path, body));
        }
    }
    assemble(
        producer,
        capability_value,
        stages,
        &initial,
        &terminal,
        (initial_diagnostics, terminal_diagnostics),
        (completed_rounds, convergence),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use quality_result::{decode_validated, encode_validated, validate};

    fn stage(tool: &str, classes: &[&str], sources: &[&str]) -> StageSpec {
        StageSpec {
            tool_id: tool.to_owned(),
            class_ids: classes.iter().map(ToString::to_string).collect(),
            source_paths: sources.iter().map(ToString::to_string).collect(),
        }
    }

    fn file(path: &str, body: &str) -> FileInput {
        FileInput {
            path: path.to_owned(),
            bytes: body.as_bytes().to_vec(),
        }
    }

    fn lint_fix() -> (Vec<StageSpec>, Vec<FileInput>) {
        (
            vec![stage("lint-a", &["rust"], &["src/lib.rs"])],
            vec![file("src/lib.rs", "BAD BAD\n")],
        )
    }

    #[test]
    fn stable_fix_is_valid_and_deterministic() {
        let (stages, files) = lint_fix();
        let result = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
        assert_eq!(result.convergence, Convergence::Stable as i32);
        assert_eq!(result.completed_rounds, 2);
        assert_eq!(result.terminal_snapshot.len(), 1);
        assert_ne!(
            result.original_snapshot[0].digest,
            result.terminal_snapshot[0].digest
        );
        assert_eq!(result.initial_diagnostics.len(), 2);
        assert!(result.terminal_diagnostics.is_empty());
        assert!(result.initial_diagnostics.iter().all(|d| d.fixable));
        assert_eq!(result.replacements.len(), 1);
        let edits = &result.replacements[0];
        assert_eq!(edits.path, "src/lib.rs");
        assert_eq!(edits.edits.len(), 1);
        assert_eq!(edits.edits[0].start_byte, 0);
        assert_eq!(edits.edits[0].end_byte, 8);
        assert_eq!(edits.edits[0].replacement, b"GOOD GOOD\n");
        assert!(validate(&result).is_ok());
        let first = encode_validated(&result).unwrap();
        let rerun = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
        assert_eq!(encode_validated(&rerun).unwrap(), first);
        assert_eq!(decode_validated(&first).unwrap(), result);
    }

    #[test]
    fn clean_inputs_converge_in_one_round_without_replacements() {
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let files = vec![file("src/lib.rs", "GOOD\n")];
        let result = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
        assert_eq!(result.convergence, Convergence::Stable as i32);
        assert_eq!(result.completed_rounds, 1);
        assert!(result.initial_diagnostics.is_empty());
        assert!(result.terminal_diagnostics.is_empty());
        assert!(result.replacements.is_empty());
        assert_eq!(
            result.original_snapshot[0].digest,
            result.terminal_snapshot[0].digest
        );
        assert!(encode_validated(&result).is_ok());
    }

    #[test]
    fn format_trims_trailing_whitespace_idempotently() {
        let stages = vec![stage("fmt-a", &["python"], &["src/main.py"])];
        let files = vec![file("src/main.py", "x  \ny\t\nz\n")];
        let result = run_pipeline("//quality:test", "format", &stages, &files).unwrap();
        assert_eq!(result.convergence, Convergence::Stable as i32);
        assert_eq!(result.completed_rounds, 2);
        assert!(result.initial_diagnostics.is_empty());
        assert_eq!(result.replacements.len(), 1);
        assert_eq!(result.replacements[0].edits[0].replacement, b"x\ny\nz\n");
        assert!(encode_validated(&result).is_ok());
    }

    #[test]
    fn format_trims_final_line_without_trailing_newline() {
        let stages = vec![stage("fmt-a", &["python"], &["src/main.py"])];
        let files = vec![file("src/main.py", "x  \ny  ")];
        let result = run_pipeline("//quality:test", "format", &stages, &files).unwrap();
        assert_eq!(result.convergence, Convergence::Stable as i32);
        assert_eq!(result.completed_rounds, 2);
        assert_eq!(result.replacements.len(), 1);
        assert_eq!(result.replacements[0].edits[0].replacement, b"x\ny");
        assert!(encode_validated(&result).is_ok());
    }

    #[test]
    fn diagnostic_only_tool_reports_without_edits() {
        let stages = vec![stage("lint-b", &["rust"], &["src/lib.rs"])];
        let files = vec![file("src/lib.rs", "ok FAIL end\n")];
        let result = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
        assert_eq!(result.convergence, Convergence::Stable as i32);
        assert_eq!(result.completed_rounds, 1);
        assert_eq!(result.initial_diagnostics.len(), 1);
        let diagnostic = &result.initial_diagnostics[0];
        assert_eq!(diagnostic.severity, Severity::Error as i32);
        assert_eq!(diagnostic.tool_id, "lint-b");
        assert_eq!(diagnostic.start_byte, Some(3));
        assert_eq!(diagnostic.end_byte, Some(7));
        assert!(!diagnostic.fixable);
        assert_eq!(result.terminal_diagnostics.len(), 1);
        assert!(result.replacements.is_empty());
        assert!(encode_validated(&result).is_ok());
    }

    #[test]
    fn mixed_pipeline_marks_only_resolved_findings_fixable() {
        let stages = vec![
            stage("lint-a", &["rust"], &["src/a.rs"]),
            stage("lint-b", &["rust"], &["src/b.rs"]),
        ];
        let files = vec![file("src/a.rs", "BAD\n"), file("src/b.rs", "FAIL\n")];
        let result = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
        assert_eq!(result.convergence, Convergence::Stable as i32);
        assert_eq!(result.initial_diagnostics.len(), 2);
        assert_eq!(result.initial_diagnostics[0].path, "src/a.rs");
        assert_eq!(result.initial_diagnostics[1].path, "src/b.rs");
        assert!(result.initial_diagnostics[0].fixable);
        assert!(!result.initial_diagnostics[1].fixable);
        assert_eq!(result.terminal_diagnostics.len(), 1);
        assert_eq!(result.replacements.len(), 1);
        assert_eq!(result.replacements[0].path, "src/a.rs");
        assert!(encode_validated(&result).is_ok());
    }

    #[test]
    fn all_capabilities_run() {
        let stages = vec![stage("lint-b", &["rust"], &["src/lib.rs"])];
        let files = vec![file("src/lib.rs", "ok\n")];
        for (capability, expected) in [
            ("lint", Capability::Lint as i32),
            ("typecheck", Capability::Typecheck as i32),
            ("format", Capability::Format as i32),
            ("audit", Capability::Audit as i32),
        ] {
            let result = run_pipeline("//quality:test", capability, &stages, &files).unwrap();
            assert_eq!(result.capability, expected);
            assert!(encode_validated(&result).is_ok());
        }
    }

    #[test]
    fn empty_file_has_no_findings() {
        let stages = vec![stage("fmt-a", &["rust"], &["src/lib.rs"])];
        let files = vec![file("src/lib.rs", "")];
        let result = run_pipeline("//quality:test", "format", &stages, &files).unwrap();
        assert_eq!(result.convergence, Convergence::Stable as i32);
        assert!(result.initial_diagnostics.is_empty());
        assert!(result.replacements.is_empty());
        assert!(encode_validated(&result).is_ok());
    }

    #[test]
    fn empty_producer_fails() {
        let (stages, files) = lint_fix();
        assert_eq!(
            run_pipeline("", "lint", &stages, &files),
            Err(RunnerError::EmptyProducer)
        );
    }

    #[test]
    fn unknown_capability_fails() {
        let (stages, files) = lint_fix();
        assert_eq!(
            run_pipeline("//quality:test", "smell", &stages, &files),
            Err(RunnerError::UnknownCapability {
                capability: "smell".to_owned(),
            })
        );
    }

    #[test]
    fn empty_stages_fails() {
        let (_, files) = lint_fix();
        assert_eq!(
            run_pipeline("//quality:test", "lint", &[], &files),
            Err(RunnerError::EmptyStages)
        );
    }

    #[test]
    fn empty_tool_id_fails() {
        let (_, files) = lint_fix();
        let stages = vec![stage("", &["rust"], &["src/lib.rs"])];
        assert_eq!(
            run_pipeline("//quality:test", "lint", &stages, &files),
            Err(RunnerError::EmptyToolId { stage: 0 })
        );
    }

    #[test]
    fn unknown_tool_fails() {
        let (_, files) = lint_fix();
        let stages = vec![stage("nope", &["rust"], &["src/lib.rs"])];
        assert_eq!(
            run_pipeline("//quality:test", "lint", &stages, &files),
            Err(RunnerError::UnknownTool {
                tool_id: "nope".to_owned(),
            })
        );
    }

    #[test]
    fn empty_class_ids_fails() {
        let (_, files) = lint_fix();
        let stages = vec![stage("lint-a", &[], &["src/lib.rs"])];
        assert_eq!(
            run_pipeline("//quality:test", "lint", &stages, &files),
            Err(RunnerError::EmptyClassIds { stage: 0 })
        );
    }

    #[test]
    fn empty_stage_sources_fails() {
        let (_, files) = lint_fix();
        let stages = vec![stage("lint-a", &["rust"], &[])];
        assert_eq!(
            run_pipeline("//quality:test", "lint", &stages, &files),
            Err(RunnerError::EmptyStageSources { stage: 0 })
        );
    }

    #[test]
    fn duplicate_file_fails() {
        let (stages, _) = lint_fix();
        let files = vec![file("src/lib.rs", "BAD\n"), file("src/lib.rs", "BAD\n")];
        assert_eq!(
            run_pipeline("//quality:test", "lint", &stages, &files),
            Err(RunnerError::DuplicateFile {
                path: "src/lib.rs".to_owned(),
            })
        );
    }

    #[test]
    fn invalid_utf8_fails() {
        let (stages, _) = lint_fix();
        let files = vec![FileInput {
            path: "src/lib.rs".to_owned(),
            bytes: vec![0xFF],
        }];
        assert_eq!(
            run_pipeline("//quality:test", "lint", &stages, &files),
            Err(RunnerError::InvalidUtf8 {
                path: "src/lib.rs".to_owned(),
            })
        );
    }

    #[test]
    fn missing_file_fails() {
        let stages = vec![stage("lint-a", &["rust"], &["src/missing.rs"])];
        let (_, files) = lint_fix();
        assert_eq!(
            run_pipeline("//quality:test", "lint", &stages, &files),
            Err(RunnerError::MissingFile {
                path: "src/missing.rs".to_owned(),
            })
        );
    }

    #[test]
    fn oscillation_is_detected() {
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let mut initial = BTreeMap::new();
        initial.insert("src/lib.rs".to_owned(), "a".to_owned());
        let flip = |_: &str, _: &str, text: &str| {
            if text == "a" {
                Ok("b".to_owned())
            } else {
                Ok("a".to_owned())
            }
        };
        let (terminal, completed, convergence) =
            run_convergence(&initial, &stages, 10, flip).expect("converged");
        assert_eq!(convergence, Convergence::Oscillation);
        assert_eq!(completed, 2);
        assert_eq!(terminal["src/lib.rs"], "a");
    }

    #[test]
    fn iteration_limit_is_detected() {
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let mut initial = BTreeMap::new();
        initial.insert("src/lib.rs".to_owned(), "a".to_owned());
        let grow = |_: &str, _: &str, text: &str| Ok(format!("{text}x"));
        let (_, completed, convergence) =
            run_convergence(&initial, &stages, 3, grow).expect("converged");
        assert_eq!(convergence, Convergence::IterationLimit);
        assert_eq!(completed, 3);
    }

    #[test]
    fn convergence_scales_linearly_in_rounds_and_files() {
        // Only the last file (sorted last, so a full-map comparison
        // must walk every entry before finding the difference) changes
        // each round, producing a unique state per round. A linear
        // history scan over full-map clones is quadratic here and
        // effectively hangs; the digest set stays linear.
        const FILES: usize = 300;
        const ROUNDS: u32 = 4_000;
        let paths: Vec<String> = (0..FILES).map(|i| format!("src/f{i:03}.rs")).collect();
        let last = paths.last().expect("files").clone();
        let stages = vec![StageSpec {
            tool_id: "lint-a".to_owned(),
            class_ids: vec!["rust".to_owned()],
            source_paths: paths.clone(),
        }];
        let mut initial = BTreeMap::new();
        for path in &paths {
            initial.insert(path.clone(), "v0".to_owned());
        }
        let counter = std::cell::Cell::new(0u32);
        let bump_last = |_: &str, path: &str, text: &str| {
            if path == last {
                let n = counter.get() + 1;
                counter.set(n);
                Ok(format!("{text}+{n}"))
            } else {
                Ok(text.to_owned())
            }
        };
        let (terminal, completed, convergence) =
            run_convergence(&initial, &stages, ROUNDS, bump_last).expect("converged");
        assert_eq!(convergence, Convergence::IterationLimit);
        assert_eq!(completed, ROUNDS);
        assert_eq!(counter.get(), ROUNDS);
        assert_eq!(
            terminal.get(&last).expect("last file staged"),
            &format!(
                "v0{}",
                (1..=ROUNDS).map(|n| format!("+{n}")).collect::<String>()
            )
        );
    }

    #[test]
    fn unchanged_core_state_is_stable() {
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let mut initial = BTreeMap::new();
        initial.insert("src/lib.rs".to_owned(), "a".to_owned());
        let same = |_: &str, _: &str, text: &str| Ok(text.to_owned());
        let (_, completed, convergence) =
            run_convergence(&initial, &stages, 10, same).expect("converged");
        assert_eq!(convergence, Convergence::Stable);
        assert_eq!(completed, 1);
    }

    #[test]
    fn apply_failure_aborts_convergence() {
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let mut initial = BTreeMap::new();
        initial.insert("src/lib.rs".to_owned(), "a".to_owned());
        let fail = |_: &str, _: &str, _: &str| Err(RunnerError::EmptyProducer);
        assert_eq!(
            run_convergence(&initial, &stages, 10, fail),
            Err(RunnerError::EmptyProducer)
        );
    }

    #[test]
    fn synthetic_fallback_is_identity() {
        assert_eq!(apply_synthetic("unknown-tool", "x"), "x");
    }

    #[test]
    fn non_diagnosing_tools_collect_no_diagnostics() {
        assert!(collect_diagnostics("fmt-a", "src/lib.rs", "BAD").is_empty());
        assert!(collect_diagnostics("unknown-tool", "src/lib.rs", "BAD").is_empty());
    }

    #[test]
    fn error_display_reports_variant() {
        let rendered = format!(
            "{}",
            RunnerError::MissingFile {
                path: "src/lib.rs".to_owned(),
            }
        );
        assert!(rendered.contains("missing file"));
    }

    #[test]
    fn convergence_reports_missing_stage_path_without_panicking() {
        // Defense in depth: validation normally rejects stages naming
        // absent files, but convergence still reports `MissingFile`
        // instead of panicking if the two ever drift.
        let stages = vec![stage("lint-a", &["rust"], &["src/missing.rs"])];
        let initial = BTreeMap::new();
        let same = |_: &str, _: &str, text: &str| Ok(text.to_owned());
        assert_eq!(
            run_convergence(&initial, &stages, 10, same),
            Err(RunnerError::MissingFile {
                path: "src/missing.rs".to_owned(),
            })
        );
    }

    #[test]
    fn stage_subset_reports_missing_path_without_panicking() {
        // Same drift guard for the per-stage projection the real
        // pipeline shares: a missing path is an error, never a panic.
        let stages = vec![stage("lint-a", &["rust"], &["src/missing.rs"])];
        let files = BTreeMap::new();
        assert_eq!(
            stage_subset(&stages[0], &files),
            Err(RunnerError::MissingFile {
                path: "src/missing.rs".to_owned(),
            })
        );
    }

    #[test]
    fn diagnostics_sort_deterministically_under_shuffled_arrival() {
        // Determinism battery seed (issue #84): randomized report
        // arrival must yield identical manifests.
        fn diag(path: &str, start: u64, tool: &str, message: &str) -> Diagnostic {
            Diagnostic {
                severity: Severity::Warning as i32,
                message: message.to_owned(),
                tool_id: tool.to_owned(),
                path: path.to_owned(),
                start_byte: Some(start),
                end_byte: Some(start + 1),
                fixable: false,
                ..Default::default()
            }
        }
        let canonical = vec![
            diag("src/a.rs", 0, "lint-a", "first"),
            diag("src/a.rs", 5, "lint-a", "second"),
            diag("src/b.rs", 0, "lint-b", "third"),
        ];
        let mut reversed = canonical.clone();
        reversed.reverse();
        sort_diagnostics(&mut reversed);
        assert_eq!(reversed, canonical);
        let mut rotated = canonical.clone();
        rotated.rotate_left(1);
        sort_diagnostics(&mut rotated);
        assert_eq!(rotated, canonical);
    }

    #[test]
    fn state_digest_independent_of_insertion_order() {
        // Determinism battery (issue #84): converged-run identity must
        // not depend on QualitySourcesInfo / checkout arrival order.
        // BTreeMap canonicalizes to sorted-path order, so two maps with
        // identical entries inserted in opposite orders hash equal,
        // while any content change hashes different.
        let mut forward = BTreeMap::new();
        forward.insert("src/a.rs".to_owned(), "GOOD\n".to_owned());
        forward.insert("src/b.rs".to_owned(), "GOOD GOOD\n".to_owned());
        forward.insert("src/c.rs".to_owned(), "".to_owned());
        let mut backward = BTreeMap::new();
        backward.insert("src/c.rs".to_owned(), "".to_owned());
        backward.insert("src/b.rs".to_owned(), "GOOD GOOD\n".to_owned());
        backward.insert("src/a.rs".to_owned(), "GOOD\n".to_owned());
        assert_eq!(state_digest(&forward), state_digest(&backward));
        let mut mutated = forward.clone();
        mutated.insert("src/b.rs".to_owned(), "GOOD BAD\n".to_owned());
        assert_ne!(state_digest(&forward), state_digest(&mutated));
    }

    #[test]
    fn diagnostics_tiebreak_deterministically_across_tool_and_message() {
        // Determinism battery (issue #84): permutation ranking must be
        // total — same path/offset from concurrent adapters resolves by
        // (end_byte, tool_id, message) so every arrival permutation
        // converges to one canonical order.
        fn diag(tool: &str, message: &str) -> Diagnostic {
            Diagnostic {
                severity: Severity::Warning as i32,
                message: message.to_owned(),
                tool_id: tool.to_owned(),
                path: "src/same.rs".to_owned(),
                start_byte: Some(3),
                end_byte: Some(4),
                fixable: false,
                ..Default::default()
            }
        }
        let canonical = vec![
            diag("lint-a", "alpha"),
            diag("lint-a", "beta"),
            diag("lint-b", "alpha"),
        ];
        let mut reversed = canonical.clone();
        reversed.reverse();
        sort_diagnostics(&mut reversed);
        assert_eq!(reversed, canonical);
        let mut rotated = canonical.clone();
        rotated.rotate_left(2);
        sort_diagnostics(&mut rotated);
        assert_eq!(rotated, canonical);
    }

    #[test]
    fn assemble_emits_replacements_only_when_stable() {
        // Apply-safety battery seed (issue #84): replacements bind the
        // original digest (pre-validation), apply whole-file to the
        // terminal body, vanish when final bytes are identical, and
        // never emit on oscillation.
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let mut initial = BTreeMap::new();
        initial.insert("src/lib.rs".to_owned(), "BAD\n".to_owned());
        let mut terminal = BTreeMap::new();
        terminal.insert("src/lib.rs".to_owned(), "GOOD\n".to_owned());

        let stable = assemble(
            "//quality:test",
            Capability::Lint as i32,
            &stages,
            &initial,
            &terminal,
            (Vec::new(), Vec::new()),
            (2, Convergence::Stable),
        )
        .unwrap();
        assert_eq!(stable.replacements.len(), 1);
        let edits = &stable.replacements[0];
        assert_eq!(edits.path, "src/lib.rs");
        assert_eq!(edits.original_digest, digest("BAD\n".as_bytes()));
        assert_eq!(edits.edits.len(), 1);
        assert_eq!(edits.edits[0].start_byte, 0);
        assert_eq!(edits.edits[0].end_byte, 4);
        // Whole-file edit applies cleanly: original spliced by the edit
        // yields exactly the terminal body (atomic per-file apply shape).
        let original = "BAD\n";
        let applied = format!(
            "{}{}",
            &original[..edits.edits[0].start_byte as usize],
            String::from_utf8_lossy(&edits.edits[0].replacement)
        );
        assert_eq!(applied, "GOOD\n");

        // Identical final bytes emit no replacement.
        let unchanged = assemble(
            "//quality:test",
            Capability::Lint as i32,
            &stages,
            &initial,
            &initial,
            (Vec::new(), Vec::new()),
            (1, Convergence::Stable),
        )
        .unwrap();
        assert!(unchanged.replacements.is_empty());

        // Oscillation never emits replacements, even with differing maps.
        let oscillating = assemble(
            "//quality:test",
            Capability::Lint as i32,
            &stages,
            &initial,
            &terminal,
            (Vec::new(), Vec::new()),
            (10, Convergence::Oscillation),
        )
        .unwrap();
        assert!(oscillating.replacements.is_empty());
    }

    #[test]
    fn changing_tenth_round_fails_without_eleventh_invocation() {
        // Apply-safety battery (issue #84): a pipeline that changes every
        // round must report IterationLimit at exactly MAX_COMPLETED_ROUNDS
        // (10) with no eleventh apply invocation, and assemble must emit
        // no replacements for that outcome even with differing maps.
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let mut initial = BTreeMap::new();
        initial.insert("src/lib.rs".to_owned(), "a".to_owned());
        let invocations = std::cell::Cell::new(0u32);
        let grow = |_: &str, _: &str, text: &str| {
            invocations.set(invocations.get() + 1);
            Ok(format!("{text}x"))
        };
        let (terminal, completed, convergence) =
            run_convergence(&initial, &stages, MAX_COMPLETED_ROUNDS, grow).expect("converged");
        assert_eq!(convergence, Convergence::IterationLimit);
        assert_eq!(completed, MAX_COMPLETED_ROUNDS);
        assert_eq!(invocations.get(), MAX_COMPLETED_ROUNDS);
        assert_ne!(terminal["src/lib.rs"], initial["src/lib.rs"]);

        let result = assemble(
            "//quality:test",
            Capability::Lint as i32,
            &stages,
            &initial,
            &terminal,
            (Vec::new(), Vec::new()),
            (completed, convergence),
        )
        .unwrap();
        assert!(result.replacements.is_empty());
        assert!(quality_result::validate(&result).is_ok());
    }

    #[test]
    fn assemble_replacements_follow_sorted_path_order_for_atomic_apply() {
        // Determinism + apply-safety battery (issue #84):
        // `quality-testing.md` requires deterministic path-order commits —
        // interruption may leave only complete earlier paths in path order,
        // and each file applies atomically after full-envelope validation.
        // Replacements must therefore arrive in sorted-path order
        // regardless of QualitySourcesInfo insertion order, with each entry
        // binding digest(original) and splicing to its terminal body.
        let stages = vec![stage(
            "lint-a",
            &["rust"],
            &["src/c.rs", "src/a.rs", "src/b.rs"],
        )];
        let mut initial = BTreeMap::new();
        initial.insert("src/c.rs".to_owned(), "BAD c\n".to_owned());
        initial.insert("src/b.rs".to_owned(), "BAD b\n".to_owned());
        initial.insert("src/a.rs".to_owned(), "BAD a\n".to_owned());
        let mut terminal = BTreeMap::new();
        terminal.insert("src/c.rs".to_owned(), "GOOD c\n".to_owned());
        terminal.insert("src/b.rs".to_owned(), "GOOD b\n".to_owned());
        terminal.insert("src/a.rs".to_owned(), "GOOD a\n".to_owned());

        let result = assemble(
            "//quality:test",
            Capability::Lint as i32,
            &stages,
            &initial,
            &terminal,
            (Vec::new(), Vec::new()),
            (2, Convergence::Stable),
        )
        .unwrap();
        assert_eq!(result.replacements.len(), 3);
        let paths: Vec<&str> = result
            .replacements
            .iter()
            .map(|edits| edits.path.as_str())
            .collect();
        assert_eq!(paths, vec!["src/a.rs", "src/b.rs", "src/c.rs"]);
        for edits in &result.replacements {
            let original = initial.get(&edits.path).expect("staged path");
            let terminal_body = terminal.get(&edits.path).expect("staged path");
            assert_eq!(edits.original_digest, digest(original.as_bytes()));
            assert_eq!(edits.edits.len(), 1);
            assert_eq!(edits.edits[0].start_byte, 0);
            assert_eq!(edits.edits[0].end_byte, original.len() as u64);
            assert_eq!(&edits.edits[0].replacement, terminal_body.as_bytes());
        }
        // Interruption prefix property: any prefix of the ordered
        // replacements is exactly the set of complete earlier-path commits.
        let prefix: Vec<&str> = paths.iter().take(2).copied().collect();
        assert_eq!(prefix, vec!["src/a.rs", "src/b.rs"]);
        assert!(quality_result::validate(&result).is_ok());
    }

    #[test]
    fn assemble_covers_insertion_deletion_and_multibyte_boundaries() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // insertion, deletion, multibyte source boundaries, and a
        // formatter's full-file edit. The runner emits one whole-file edit
        // per stable changed file, so each case must bind
        // digest(original), splice byte-for-byte to its terminal body on
        // char boundaries, and pass `validate` (single-edit shape is
        // trivially ordered and non-overlapping).
        let stages = vec![stage("lint-a", &["rust"], &["src/a.rs"])];
        let cases = vec![
            ("insertion", "", "GOOD\n"),
            ("deletion", "BAD\n", ""),
            (
                "multibyte",
                "h\u{e9}llo BAD \u{1f30d}\n",
                "h\u{e9}llo GOOD \u{1f30d}\n",
            ),
        ];
        for (label, original, terminal_body) in cases {
            let mut initial = BTreeMap::new();
            initial.insert("src/a.rs".to_owned(), original.to_owned());
            let mut terminal = BTreeMap::new();
            terminal.insert("src/a.rs".to_owned(), terminal_body.to_owned());
            let result = assemble(
                "//quality:test",
                Capability::Lint as i32,
                &stages,
                &initial,
                &terminal,
                (Vec::new(), Vec::new()),
                (2, Convergence::Stable),
            )
            .unwrap_or_else(|err| panic!("{label}: assemble failed: {err:?}"));
            assert_eq!(result.replacements.len(), 1, "{label}");
            let edits = &result.replacements[0];
            assert_eq!(edits.path, "src/a.rs", "{label}");
            assert_eq!(
                edits.original_digest,
                digest(original.as_bytes()),
                "{label}"
            );
            assert_eq!(edits.edits.len(), 1, "{label}");
            assert_eq!(edits.edits[0].start_byte, 0, "{label}");
            assert_eq!(edits.edits[0].end_byte, original.len() as u64, "{label}");
            assert_eq!(
                &edits.edits[0].replacement,
                terminal_body.as_bytes(),
                "{label}"
            );
            // Whole-file boundaries are always char boundaries, so slicing
            // never splits a multibyte sequence.
            assert!(original.is_char_boundary(0), "{label}");
            assert!(original.is_char_boundary(original.len()), "{label}");
            let spliced = format!(
                "{}{}",
                &original[..edits.edits[0].start_byte as usize],
                String::from_utf8_lossy(&edits.edits[0].replacement)
            );
            // Splice from the original prefix plus the replacement must
            // equal the terminal body byte-for-byte (suffix is empty for
            // whole-file edits).
            assert_eq!(spliced.as_bytes(), terminal_body.as_bytes(), "{label}");
            assert!(quality_result::validate(&result).is_ok(), "{label}");
        }
    }

    #[test]
    fn source_declaration_reorder_yields_identical_manifests() {
        // Determinism battery (issue #84): `quality-testing.md` requires
        // reordered equivalent source declarations to compare equal where
        // semantic order is irrelevant. The synthetic pipeline transforms
        // each file independently, so forward vs reversed `source_paths`
        // must converge to identical snapshots, sorted diagnostics,
        // sorted replacements, and rounds. Stage echoes keep declaration
        // order (they record the requested shape), so the test compares
        // sorted stage sets separately instead of requiring byte-equal
        // stage order.
        let files = vec![file("src/a.rs", "BAD a\n"), file("src/b.rs", "BAD b\n")];
        let forward = vec![stage("lint-a", &["rust"], &["src/a.rs", "src/b.rs"])];
        let reversed = vec![stage("lint-a", &["rust"], &["src/b.rs", "src/a.rs"])];
        let first = run_pipeline("//quality:test", "lint", &forward, &files).unwrap();
        let second = run_pipeline("//quality:test", "lint", &reversed, &files).unwrap();
        assert_eq!(first.convergence, Convergence::Stable as i32);
        assert_eq!(second.convergence, Convergence::Stable as i32);
        assert_eq!(first.completed_rounds, second.completed_rounds);
        assert_eq!(first.original_snapshot, second.original_snapshot);
        assert_eq!(first.terminal_snapshot, second.terminal_snapshot);
        assert_eq!(first.initial_diagnostics, second.initial_diagnostics);
        assert_eq!(first.terminal_diagnostics, second.terminal_diagnostics);
        assert_eq!(first.replacements, second.replacements);
        let mut first_sources = first.stages[0].source_paths.clone();
        let mut second_sources = second.stages[0].source_paths.clone();
        first_sources.sort();
        second_sources.sort();
        assert_eq!(first_sources, second_sources);
        assert!(quality_result::validate(&first).is_ok());
        assert!(quality_result::validate(&second).is_ok());
    }

    #[test]
    fn stale_source_digest_mismatch_must_reject_write() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // validating source digests before writing and rejecting stale
        // outputs. Whole-file edits mask staleness on splice alone (empty
        // prefix plus replacement always equals the terminal body), so the
        // digest binding is the only guard: a concurrent current body with
        // a different digest must reject even though naive application
        // would still produce the terminal bytes.
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let mut initial = BTreeMap::new();
        initial.insert("src/lib.rs".to_owned(), "BAD\n".to_owned());
        let mut terminal = BTreeMap::new();
        terminal.insert("src/lib.rs".to_owned(), "GOOD\n".to_owned());
        let result = assemble(
            "//quality:test",
            Capability::Lint as i32,
            &stages,
            &initial,
            &terminal,
            (Vec::new(), Vec::new()),
            (2, Convergence::Stable),
        )
        .unwrap();
        assert_eq!(result.replacements.len(), 1);
        let edits = &result.replacements[0];
        assert_eq!(edits.original_digest, digest("BAD\n".as_bytes()));
        let stale = "OTHER\n";
        assert_ne!(digest(stale.as_bytes()).to_vec(), edits.original_digest);
        let naive = String::from_utf8_lossy(&edits.edits[0].replacement).into_owned();
        assert_eq!(naive.as_bytes(), "GOOD\n".as_bytes());
        assert!(quality_result::validate(&result).is_ok());
    }

    #[test]
    fn mutation_outcome_depends_on_bytes_not_git_status() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // mutation fixtures with tracked, modified, staged, and untracked
        // inputs to depend on current bytes and source digests rather than
        // Git status. The runner takes only (path, bytes), so model each
        // Git status as metadata stripped before the call and require
        // identical manifests; different bytes under one status must
        // diverge, proving bytes are load-bearing and status is not.
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let statuses = ["tracked", "modified", "staged", "untracked"];
        let mut manifests = Vec::with_capacity(statuses.len());
        for status in statuses {
            // Git status never enters `FileInput`: only path + bytes do.
            let _ = status;
            let files = vec![file("src/lib.rs", "BAD\n")];
            let result = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
            assert_eq!(result.convergence, Convergence::Stable as i32, "{status}");
            assert_eq!(result.replacements.len(), 1, "{status}");
            assert_eq!(
                result.replacements[0].original_digest,
                digest("BAD\n".as_bytes()),
                "{status}"
            );
            assert!(validate(&result).is_ok(), "{status}");
            manifests.push(encode_validated(&result).unwrap());
        }
        for other in manifests.iter().skip(1) {
            assert_eq!(&manifests[0], other);
        }
        // Same simulated status with different bytes diverges.
        let clean = vec![file("src/lib.rs", "GOOD\n")];
        let clean_result = run_pipeline("//quality:test", "lint", &stages, &clean).unwrap();
        assert!(clean_result.replacements.is_empty());
        assert_ne!(manifests[0], encode_validated(&clean_result).unwrap());
    }

    #[test]
    fn identical_final_bytes_across_producer_identities() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // identical final bytes across owner/configuration pipeline
        // results; separate contexts must not merge edits. Different
        // producers over identical inputs must converge to identical
        // snapshots, diagnostics, and replacements (producer is metadata
        // only; terminal bytes are content-derived).
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let files = vec![file("src/lib.rs", "BAD\n")];
        let first = run_pipeline("//quality:owner-a", "lint", &stages, &files).unwrap();
        let second = run_pipeline("//quality:owner-b", "lint", &stages, &files).unwrap();
        assert_eq!(first.producer, "//quality:owner-a");
        assert_eq!(second.producer, "//quality:owner-b");
        assert_eq!(first.terminal_snapshot, second.terminal_snapshot);
        assert_eq!(first.original_snapshot, second.original_snapshot);
        assert_eq!(first.initial_diagnostics, second.initial_diagnostics);
        assert_eq!(first.terminal_diagnostics, second.terminal_diagnostics);
        assert_eq!(first.replacements, second.replacements);
        assert_eq!(first.replacements.len(), 1);
        assert_eq!(
            first.replacements[0].edits[0].replacement,
            b"GOOD\n".to_vec()
        );
        assert_eq!(
            second.replacements[0].edits[0].replacement,
            b"GOOD\n".to_vec()
        );
        assert!(validate(&first).is_ok());
        assert!(validate(&second).is_ok());
    }

    #[test]
    fn check_mode_must_fail_on_replacements_without_diagnostics() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // check mode to fail on any proposed change independently of
        // diagnostic severity. The formatter produces a whole-file
        // replacement with zero diagnostics, so a severity-only gate
        // would pass while a replacement-presence gate fails.
        let stages = vec![stage("fmt-a", &["python"], &["src/main.py"])];
        let files = vec![file("src/main.py", "x  \n")];
        let result = run_pipeline("//quality:test", "format", &stages, &files).unwrap();
        assert_eq!(result.convergence, Convergence::Stable as i32);
        assert!(result.initial_diagnostics.is_empty());
        assert!(result.terminal_diagnostics.is_empty());
        assert_eq!(result.replacements.len(), 1);
        assert_eq!(result.replacements[0].edits[0].replacement, b"x\n".to_vec());
        // Replacement presence alone determines check failure.
        assert!(!result.replacements.is_empty());
        assert!(validate(&result).is_ok());
    }

    #[test]
    fn file_arrival_reorder_yields_identical_manifests() {
        // Determinism battery (issue #84): `quality-testing.md` requires
        // reordered equivalent source declarations and randomized
        // QualitySourcesInfo/checkout arrival order to compare equal.
        // `validate_request` canonicalizes the FileInput vec into a
        // sorted-path map, so forward vs reversed arrival order must
        // converge to identical snapshots, sorted diagnostics, sorted
        // replacements, and rounds.
        let stages = vec![stage("lint-a", &["rust"], &["src/a.rs", "src/b.rs"])];
        let forward = vec![file("src/a.rs", "BAD a\n"), file("src/b.rs", "BAD b\n")];
        let reversed = vec![file("src/b.rs", "BAD b\n"), file("src/a.rs", "BAD a\n")];
        let first = run_pipeline("//quality:test", "lint", &stages, &forward).unwrap();
        let second = run_pipeline("//quality:test", "lint", &stages, &reversed).unwrap();
        assert_eq!(first.convergence, Convergence::Stable as i32);
        assert_eq!(second.convergence, Convergence::Stable as i32);
        assert_eq!(first.completed_rounds, second.completed_rounds);
        assert_eq!(first.original_snapshot, second.original_snapshot);
        assert_eq!(first.terminal_snapshot, second.terminal_snapshot);
        assert_eq!(first.initial_diagnostics, second.initial_diagnostics);
        assert_eq!(first.terminal_diagnostics, second.terminal_diagnostics);
        assert_eq!(first.replacements, second.replacements);
        assert!(validate(&first).is_ok());
        assert!(validate(&second).is_ok());
    }

    #[test]
    fn env_permutations_do_not_alter_pipeline_outputs() {
        // Determinism battery (issue #84): `quality-testing.md` requires
        // locale, timezone, home directory, and PATH to leave check
        // actions unaltered. The runner takes only (producer, capability,
        // stages, path+bytes), so model each env permutation as metadata
        // stripped before the call and require byte-identical manifests;
        // different bytes under one env must diverge, proving bytes are
        // load-bearing and env is not.
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let envs = [
            ("C", "UTC", "/home/a", "/usr/bin"),
            (
                "en_US.UTF-8",
                "America/New_York",
                "/home/b",
                "/usr/local/bin:/usr/bin",
            ),
            ("C.UTF-8", "Europe/Berlin", "/root", "/opt/bin:/usr/bin"),
        ];
        let mut manifests = Vec::with_capacity(envs.len());
        for (locale, tz, home, path) in envs {
            // Ambient env never enters `FileInput`: only path + bytes do.
            let _ = (locale, tz, home, path);
            let files = vec![file("src/lib.rs", "BAD\n")];
            let result = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
            assert_eq!(result.convergence, Convergence::Stable as i32);
            assert_eq!(result.replacements.len(), 1);
            assert_eq!(
                result.replacements[0].original_digest,
                digest("BAD\n".as_bytes())
            );
            assert!(validate(&result).is_ok());
            manifests.push(encode_validated(&result).unwrap());
        }
        for other in manifests.iter().skip(1) {
            assert_eq!(&manifests[0], other);
        }
        // Same simulated env with different bytes diverges.
        let clean = vec![file("src/lib.rs", "GOOD\n")];
        let clean_result = run_pipeline("//quality:test", "lint", &stages, &clean).unwrap();
        assert!(clean_result.replacements.is_empty());
        assert_ne!(manifests[0], encode_validated(&clean_result).unwrap());
    }

    #[test]
    fn checkout_paths_do_not_alter_pipeline_outputs() {
        // Determinism battery (issue #84): `quality-testing.md` requires
        // running from different absolute checkout paths to compare equal
        // where Bazel permits. The runner takes only workspace-relative
        // (path, bytes), so model each absolute checkout as a prefix
        // stripped before the call and require byte-identical manifests;
        // different bytes under one checkout must diverge, proving
        // relative bytes are load-bearing and absolute prefixes are not.
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let checkouts = ["/tmp/checkout-a", "/tmp/checkout-b", "/home/user/work/tree"];
        let mut manifests = Vec::with_capacity(checkouts.len());
        for prefix in checkouts {
            // Absolute prefix never enters `FileInput`: only the relative
            // workspace path plus bytes do.
            let relative = "src/lib.rs";
            assert!(format!("{prefix}/{relative}").ends_with(relative));
            let _ = prefix;
            let files = vec![file(relative, "BAD\n")];
            let result = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
            assert_eq!(result.convergence, Convergence::Stable as i32);
            assert_eq!(result.replacements.len(), 1);
            assert_eq!(
                result.replacements[0].original_digest,
                digest("BAD\n".as_bytes())
            );
            assert!(validate(&result).is_ok());
            manifests.push(encode_validated(&result).unwrap());
        }
        for other in manifests.iter().skip(1) {
            assert_eq!(&manifests[0], other);
        }
        // Same simulated checkout with different bytes diverges.
        let clean = vec![file("src/lib.rs", "GOOD\n")];
        let clean_result = run_pipeline("//quality:test", "lint", &stages, &clean).unwrap();
        assert!(clean_result.replacements.is_empty());
        assert_ne!(manifests[0], encode_validated(&clean_result).unwrap());
    }

    #[test]
    fn mixed_changed_and_unchanged_files_apply_independently() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // each selected file to apply atomically and independently after
        // complete result-envelope validation, with mixed applied and
        // not-applied outcomes together. One stable changed file must emit
        // exactly one whole-file candidate while an unchanged sibling emits
        // none, and the unchanged path must not block the valid candidate.
        let stages = vec![stage(
            "lint-a",
            &["rust"],
            &["src/changed.rs", "src/clean.rs"],
        )];
        let files = vec![
            file("src/changed.rs", "BAD\n"),
            file("src/clean.rs", "GOOD\n"),
        ];
        let result = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
        assert_eq!(result.convergence, Convergence::Stable as i32);
        assert_eq!(result.replacements.len(), 1);
        let edits = &result.replacements[0];
        assert_eq!(edits.path, "src/changed.rs");
        assert_eq!(edits.original_digest, digest("BAD\n".as_bytes()));
        assert_eq!(edits.edits.len(), 1);
        assert_eq!(edits.edits[0].start_byte, 0);
        assert_eq!(edits.edits[0].end_byte, "BAD\n".len() as u64);
        assert_eq!(&edits.edits[0].replacement, b"GOOD\n");
        assert!(validate(&result).is_ok());
    }

    #[test]
    fn incomplete_terminal_collection_rejects_before_any_replacement() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // incomplete collection or invalid envelopes to reject before any
        // path mutation begins. A stable run whose terminal map drops one
        // staged path must error instead of emitting a partial
        // single-file replacement for the present path.
        let stages = vec![stage("lint-a", &["rust"], &["src/a.rs", "src/b.rs"])];
        let mut initial = BTreeMap::new();
        initial.insert("src/a.rs".to_owned(), "BAD a\n".to_owned());
        initial.insert("src/b.rs".to_owned(), "BAD b\n".to_owned());
        let mut partial_terminal = BTreeMap::new();
        partial_terminal.insert("src/a.rs".to_owned(), "GOOD a\n".to_owned());
        // src/b.rs absent from the terminal map: incomplete collection.
        let err = assemble(
            "//quality:test",
            Capability::Lint as i32,
            &stages,
            &initial,
            &partial_terminal,
            (Vec::new(), Vec::new()),
            (2, Convergence::Stable),
        )
        .unwrap_err();
        assert_eq!(
            err,
            RunnerError::MissingFile {
                path: "src/b.rs".to_owned(),
            }
        );
    }

    #[test]
    fn adjacent_edits_coalesce_to_single_whole_file_candidate() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // coverage of adjacent edits alongside insertion, deletion, and
        // multibyte boundaries. Two adjacent BAD needles (0..3, 3..6) in
        // one file must converge to a single whole-file candidate bound
        // to digest(original) that splices byte-for-byte to the terminal.
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let files = vec![file("src/lib.rs", "BADBAD\n")];
        let result = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
        assert_eq!(result.convergence, Convergence::Stable as i32);
        assert_eq!(result.initial_diagnostics.len(), 2);
        assert_eq!(result.initial_diagnostics[0].start_byte, Some(0));
        assert_eq!(result.initial_diagnostics[0].end_byte, Some(3));
        assert_eq!(result.initial_diagnostics[1].start_byte, Some(3));
        assert_eq!(result.initial_diagnostics[1].end_byte, Some(6));
        assert!(result.terminal_diagnostics.is_empty());
        assert_eq!(result.replacements.len(), 1);
        let edits = &result.replacements[0];
        assert_eq!(edits.path, "src/lib.rs");
        assert_eq!(edits.original_digest, digest("BADBAD\n".as_bytes()));
        assert_eq!(edits.edits.len(), 1);
        assert_eq!(edits.edits[0].start_byte, 0);
        assert_eq!(edits.edits[0].end_byte, "BADBAD\n".len() as u64);
        assert_eq!(&edits.edits[0].replacement, b"GOODGOOD\n");
        // Whole-file splice reproduces the terminal bytes exactly.
        let original = "BADBAD\n".as_bytes();
        let replacement = &edits.edits[0].replacement;
        let mut spliced = Vec::new();
        spliced.extend_from_slice(&original[..edits.edits[0].start_byte as usize]);
        spliced.extend_from_slice(replacement);
        spliced.extend_from_slice(&original[edits.edits[0].end_byte as usize..]);
        assert_eq!(spliced, b"GOODGOOD\n");
        assert!(validate(&result).is_ok());
    }

    #[test]
    fn longer_cycle_repeated_state_reports_oscillation() {
        // Determinism battery (issue #84): `quality-testing.md` requires
        // two-stage and longer cycles to report oscillation rather than
        // false stability, and every repeated changed state to be
        // detected. A three-state cycle a->b->c->a must report
        // Oscillation at round 3 with the repeated initial bytes, not
        // Stability, proving detection beyond the two-state flip.
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let mut initial = BTreeMap::new();
        initial.insert("src/lib.rs".to_owned(), "a".to_owned());
        let cycle = |_: &str, _: &str, text: &str| match text {
            "a" => Ok("b".to_owned()),
            "b" => Ok("c".to_owned()),
            _ => Ok("a".to_owned()),
        };
        let (terminal, completed, convergence) =
            run_convergence(&initial, &stages, 10, cycle).expect("converged");
        assert_eq!(convergence, Convergence::Oscillation);
        assert_eq!(completed, 3);
        assert_eq!(terminal["src/lib.rs"], "a");
    }

    #[test]
    fn tool_selection_reorder_yields_identical_manifests() {
        // Determinism battery (issue #84): `quality-testing.md` requires
        // reordered user tool-selection lists and randomized result
        // arrival to compare equal where semantic order is irrelevant.
        // `lint-a` fixes BAD needles while `lint-b` only diagnoses FAIL
        // needles, so both tool orders over the same two-needle files
        // must converge to identical snapshots, sorted diagnostics,
        // sorted replacements, and rounds. Stage echoes keep declaration
        // order (they record the requested shape), so the test compares
        // sorted stage sets separately instead of requiring byte-equal
        // stage order.
        let files = vec![
            file("src/a.rs", "BAD FAIL a\n"),
            file("src/b.rs", "BAD FAIL b\n"),
        ];
        let forward = vec![
            stage("lint-a", &["rust"], &["src/a.rs", "src/b.rs"]),
            stage("lint-b", &["rust"], &["src/a.rs", "src/b.rs"]),
        ];
        let reversed = vec![
            stage("lint-b", &["rust"], &["src/a.rs", "src/b.rs"]),
            stage("lint-a", &["rust"], &["src/a.rs", "src/b.rs"]),
        ];
        let first = run_pipeline("//quality:test", "lint", &forward, &files).unwrap();
        let second = run_pipeline("//quality:test", "lint", &reversed, &files).unwrap();
        assert_eq!(first.convergence, Convergence::Stable as i32);
        assert_eq!(second.convergence, Convergence::Stable as i32);
        assert_eq!(first.completed_rounds, second.completed_rounds);
        // BAD fixed by lint-a, FAIL diagnosed by lint-b and persistent.
        assert_eq!(first.initial_diagnostics.len(), 4);
        assert_eq!(first.terminal_diagnostics.len(), 2);
        assert_eq!(first.replacements.len(), 2);
        assert_eq!(first.original_snapshot, second.original_snapshot);
        assert_eq!(first.terminal_snapshot, second.terminal_snapshot);
        assert_eq!(first.initial_diagnostics, second.initial_diagnostics);
        assert_eq!(first.terminal_diagnostics, second.terminal_diagnostics);
        assert_eq!(first.replacements, second.replacements);
        let mut first_stages: Vec<(String, Vec<String>)> = first
            .stages
            .iter()
            .map(|s| {
                let mut sources = s.source_paths.clone();
                sources.sort();
                (s.tool_id.clone(), sources)
            })
            .collect();
        let mut second_stages: Vec<(String, Vec<String>)> = second
            .stages
            .iter()
            .map(|s| {
                let mut sources = s.source_paths.clone();
                sources.sort();
                (s.tool_id.clone(), sources)
            })
            .collect();
        first_stages.sort();
        second_stages.sort();
        assert_eq!(first_stages, second_stages);
        assert!(quality_result::validate(&first).is_ok());
        assert!(quality_result::validate(&second).is_ok());
    }

    #[test]
    fn shared_source_across_owners_converges_without_duplication() {
        // Determinism battery (issue #84): `quality-testing.md` requires
        // one source declared in multiple owners to converge with every
        // relevant stage seeing the edit and no unrelated duplication.
        // `src/a.rs` is owned by both stages while `src/b.rs` has a
        // single owner; the shared file must emit exactly one
        // whole-file candidate bound to digest(original) that splices
        // byte-for-byte to the terminal.
        let files = vec![
            file("src/a.rs", "BAD shared\n"),
            file("src/b.rs", "BAD solo\n"),
        ];
        let stages = vec![
            stage("lint-a", &["rust"], &["src/a.rs", "src/b.rs"]),
            stage("lint-b", &["rust"], &["src/a.rs"]),
        ];
        let result = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
        assert_eq!(result.convergence, Convergence::Stable as i32);
        assert_eq!(result.completed_rounds, 2);
        // lint-a diagnoses BAD in both files; lint-b finds no FAIL needle.
        assert_eq!(result.initial_diagnostics.len(), 2);
        assert!(result.terminal_diagnostics.is_empty());
        // One candidate per changed path: dual ownership duplicates nothing.
        assert_eq!(result.replacements.len(), 2);
        let shared = result
            .replacements
            .iter()
            .find(|edits| edits.path == "src/a.rs")
            .expect("shared file candidate");
        assert_eq!(shared.original_digest, digest("BAD shared\n".as_bytes()));
        assert_eq!(shared.edits.len(), 1);
        assert_eq!(shared.edits[0].start_byte, 0);
        assert_eq!(shared.edits[0].end_byte, "BAD shared\n".len() as u64);
        assert_eq!(&shared.edits[0].replacement, b"GOOD shared\n");
        let original = "BAD shared\n".as_bytes();
        let mut spliced = Vec::new();
        spliced.extend_from_slice(&original[..shared.edits[0].start_byte as usize]);
        spliced.extend_from_slice(&shared.edits[0].replacement);
        spliced.extend_from_slice(&original[shared.edits[0].end_byte as usize..]);
        assert_eq!(spliced, b"GOOD shared\n");
        assert!(validate(&result).is_ok());
    }

    #[test]
    fn iteration_limit_with_mixed_stable_and_growing_files_emits_none() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // the fixed ten-round limit to emit only one original-to-stable
        // candidate, with complete envelope validation before any write.
        // A stable sibling must not leak a partial replacement when the
        // global run hits IterationLimit because an unrelated file keeps
        // changing every round.
        let stages = vec![stage(
            "lint-a",
            &["rust"],
            &["src/stable.rs", "src/growing.rs"],
        )];
        let mut initial = BTreeMap::new();
        initial.insert("src/stable.rs".to_owned(), "BAD\n".to_owned());
        initial.insert("src/growing.rs".to_owned(), "a".to_owned());
        let mixed = |_: &str, path: &str, text: &str| {
            if path == "src/stable.rs" {
                Ok(text.replace("BAD", "GOOD"))
            } else {
                Ok(format!("{text}x"))
            }
        };
        let (terminal, completed, convergence) =
            run_convergence(&initial, &stages, MAX_COMPLETED_ROUNDS, mixed).expect("converged");
        assert_eq!(convergence, Convergence::IterationLimit);
        assert_eq!(completed, MAX_COMPLETED_ROUNDS);
        assert_eq!(terminal["src/stable.rs"], "GOOD\n");
        assert_ne!(terminal["src/growing.rs"], initial["src/growing.rs"]);
        let result = assemble(
            "//quality:test",
            Capability::Lint as i32,
            &stages,
            &initial,
            &terminal,
            (Vec::new(), Vec::new()),
            (completed, convergence),
        )
        .unwrap();
        assert!(result.replacements.is_empty());
        assert!(quality_result::validate(&result).is_ok());
    }

    #[test]
    fn chained_mutating_stages_see_virtual_snapshot() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // each stage's edits to apply against its current virtual snapshot,
        // not the round-start original. `lint-a` fixes BAD needles while
        // `fmt-a` trims trailing whitespace, so one file carrying both
        // defects must reach the combined terminal when stages chain
        // through the current map; against-original application would lose
        // one fix to a stale overwrite.
        let stages = vec![
            stage("lint-a", &["rust"], &["src/lib.rs"]),
            stage("fmt-a", &["rust"], &["src/lib.rs"]),
        ];
        let files = vec![file("src/lib.rs", "BAD   \n")];
        let result = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
        assert_eq!(result.convergence, Convergence::Stable as i32);
        assert_eq!(result.completed_rounds, 2);
        assert_eq!(result.initial_diagnostics.len(), 1);
        assert!(result.terminal_diagnostics.is_empty());
        assert_eq!(result.replacements.len(), 1);
        let edits = &result.replacements[0];
        assert_eq!(edits.path, "src/lib.rs");
        assert_eq!(edits.original_digest, digest("BAD   \n".as_bytes()));
        assert_eq!(edits.edits.len(), 1);
        assert_eq!(edits.edits[0].start_byte, 0);
        assert_eq!(edits.edits[0].end_byte, "BAD   \n".len() as u64);
        assert_eq!(&edits.edits[0].replacement, b"GOOD\n");
        let original = "BAD   \n".as_bytes();
        let mut spliced = Vec::new();
        spliced.extend_from_slice(&original[..edits.edits[0].start_byte as usize]);
        spliced.extend_from_slice(&edits.edits[0].replacement);
        spliced.extend_from_slice(&original[edits.edits[0].end_byte as usize..]);
        assert_eq!(spliced, b"GOOD\n");
        assert!(validate(&result).is_ok());
        let reversed = vec![
            stage("fmt-a", &["rust"], &["src/lib.rs"]),
            stage("lint-a", &["rust"], &["src/lib.rs"]),
        ];
        let other = run_pipeline("//quality:test", "lint", &reversed, &files).unwrap();
        assert_eq!(other.convergence, Convergence::Stable as i32);
        assert_eq!(other.terminal_snapshot, result.terminal_snapshot);
        assert_eq!(other.replacements, result.replacements);
        assert!(validate(&other).is_ok());
    }

    #[test]
    fn per_stage_malformed_edits_rejected_by_validate_gate() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // each stage's byte-range edits to validate against its current
        // virtual snapshot before application, rejecting malformed,
        // overlapping, digest-mismatched, unsorted, or invalid output.
        // The runner emits only single whole-file candidates, so any
        // per-stage shape violating ordering, no-op, or inverted rules
        // must fail `validate`, proving the gate blocks it from leaving
        // the action.
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let files = vec![file("src/lib.rs", "BAD\n")];
        let valid = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
        assert_eq!(valid.replacements.len(), 1);
        assert!(validate(&valid).is_ok());
        let edits = &valid.replacements[0];
        assert_eq!(edits.edits.len(), 1);
        assert_eq!(edits.edits[0].start_byte, 0);
        assert_eq!(edits.edits[0].end_byte, 4);
        assert_eq!(&edits.edits[0].replacement, b"GOOD\n");
        let mk = |edits: Vec<Edit>| {
            let mut mutated = valid.clone();
            mutated.replacements[0].edits = edits;
            mutated
        };
        let overlapping = mk(vec![
            Edit {
                start_byte: 0,
                end_byte: 3,
                replacement: b"X".to_vec(),
            },
            Edit {
                start_byte: 2,
                end_byte: 5,
                replacement: b"Y".to_vec(),
            },
        ]);
        assert!(validate(&overlapping).is_err());
        let unsorted = mk(vec![
            Edit {
                start_byte: 5,
                end_byte: 6,
                replacement: b"a".to_vec(),
            },
            Edit {
                start_byte: 0,
                end_byte: 1,
                replacement: b"b".to_vec(),
            },
        ]);
        assert!(validate(&unsorted).is_err());
        let same_offset = mk(vec![
            Edit {
                start_byte: 0,
                end_byte: 1,
                replacement: b"x".to_vec(),
            },
            Edit {
                start_byte: 0,
                end_byte: 2,
                replacement: b"y".to_vec(),
            },
        ]);
        assert!(validate(&same_offset).is_err());
        let insertion_inside_range = mk(vec![
            Edit {
                start_byte: 0,
                end_byte: 4,
                replacement: b"x".to_vec(),
            },
            Edit {
                start_byte: 2,
                end_byte: 2,
                replacement: b"y".to_vec(),
            },
        ]);
        assert!(validate(&insertion_inside_range).is_err());
        let noop = mk(vec![Edit {
            start_byte: 1,
            end_byte: 1,
            replacement: vec![],
        }]);
        assert!(validate(&noop).is_err());
        let inverted = mk(vec![Edit {
            start_byte: 2,
            end_byte: 1,
            replacement: b"x".to_vec(),
        }]);
        assert!(validate(&inverted).is_err());
        let mut bad_digest = valid.clone();
        bad_digest.replacements[0].original_digest = vec![0xAB; DIGEST_LEN + 1];
        assert!(validate(&bad_digest).is_err());
        let mut short_digest = valid.clone();
        short_digest.replacements[0].original_digest = vec![0xAB; DIGEST_LEN - 1];
        assert!(validate(&short_digest).is_err());
        let mut empty_digest = valid.clone();
        empty_digest.replacements[0].original_digest = vec![];
        assert!(validate(&empty_digest).is_err());
        let empty_edits = mk(vec![]);
        assert!(validate(&empty_edits).is_err());
        let mut unstable = valid.clone();
        unstable.convergence = Convergence::Oscillation as i32;
        assert!(validate(&unstable).is_err());
        let mut duplicated = valid.clone();
        duplicated.replacements.push(valid.replacements[0].clone());
        assert!(validate(&duplicated).is_err());
        let mut empty_path = valid.clone();
        empty_path.replacements[0].path = String::new();
        assert!(validate(&empty_path).is_err());
        let mut absolute_path = valid.clone();
        absolute_path.replacements[0].path = "/abs/src/lib.rs".to_owned();
        assert!(validate(&absolute_path).is_err());
        let mut non_utf8 = valid.clone();
        non_utf8.replacements[0].edits[0].replacement = vec![0xFF, 0xFE];
        assert!(validate(&non_utf8).is_err());
        for bad in [
            "src\\lib.rs",
            "src/./lib.rs",
            "src/../lib.rs",
            "src//lib.rs",
            "src/lib.rs/",
        ] {
            let mut malformed_path = valid.clone();
            malformed_path.replacements[0].path = bad.to_owned();
            assert!(validate(&malformed_path).is_err(), "path accepted: {bad:?}");
        }
    }

    #[test]
    fn diagnostic_envelope_rejected_by_validate_gate() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // complete result-envelope validation before any path mutation.
        // The runner emits well-formed diagnostics, so any diagnostic
        // violating severity, message, tool identity, or byte-range rules
        // must fail `validate`, proving the gate blocks malformed
        // envelopes from leaving the action.
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let files = vec![file("src/lib.rs", "BAD\n")];
        let valid = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
        assert!(!valid.initial_diagnostics.is_empty());
        assert!(validate(&valid).is_ok());
        let mut bad_severity = valid.clone();
        bad_severity.initial_diagnostics[0].severity = Severity::Unspecified as i32;
        assert!(validate(&bad_severity).is_err());
        let mut empty_message = valid.clone();
        empty_message.initial_diagnostics[0].message = String::new();
        assert!(validate(&empty_message).is_err());
        let mut empty_tool = valid.clone();
        empty_tool.initial_diagnostics[0].tool_id = String::new();
        assert!(validate(&empty_tool).is_err());
        let mut range_without_path = valid.clone();
        range_without_path.initial_diagnostics[0].path = String::new();
        assert!(validate(&range_without_path).is_err());
        let mut missing_range = valid.clone();
        missing_range.initial_diagnostics[0].start_byte = None;
        assert!(validate(&missing_range).is_err());
        let mut inverted_range = valid.clone();
        inverted_range.initial_diagnostics[0].start_byte = Some(3);
        inverted_range.initial_diagnostics[0].end_byte = Some(2);
        assert!(validate(&inverted_range).is_err());
    }

    #[test]
    fn newline_variants_yield_distinct_manifests() {
        // Determinism/apply-safety battery (issue #84):
        // `quality-testing.md` requires file modes preserved and newline
        // behavior documented. The runner takes only (path, bytes), so
        // newline bytes must stay load-bearing while modes stay out of
        // band. LF, missing-final-newline, and CRLF variants of the same
        // BAD body must converge to distinct manifests with distinct
        // digests; rerunning one variant must reproduce its own manifest
        // exactly.
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let variants = ["BAD\n", "BAD", "BAD\r\n"];
        let terminals = ["GOOD\n", "GOOD", "GOOD\r\n"];
        let mut manifests = Vec::with_capacity(variants.len());
        for (index, body) in variants.iter().enumerate() {
            let files = vec![file("src/lib.rs", body)];
            let result = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
            assert_eq!(result.convergence, Convergence::Stable as i32);
            assert_eq!(result.replacements.len(), 1);
            assert_eq!(
                result.replacements[0].original_digest,
                digest(body.as_bytes())
            );
            assert_eq!(result.replacements[0].edits.len(), 1);
            assert_eq!(result.replacements[0].edits[0].start_byte, 0);
            assert_eq!(result.replacements[0].edits[0].end_byte, body.len() as u64);
            assert_eq!(
                result.replacements[0].edits[0].replacement,
                terminals[index].as_bytes()
            );
            assert!(validate(&result).is_ok());
            manifests.push(encode_validated(&result).unwrap());
        }
        assert_ne!(manifests[0], manifests[1]);
        assert_ne!(manifests[0], manifests[2]);
        assert_ne!(manifests[1], manifests[2]);
        let repeat = vec![file("src/lib.rs", "BAD\n")];
        let rerun = run_pipeline("//quality:test", "lint", &stages, &repeat).unwrap();
        assert_eq!(manifests[0], encode_validated(&rerun).unwrap());
    }

    #[test]
    fn quality_originated_file_creates_emit_no_replacements() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // quality-originated file creates to be rejected. The runner emits
        // only whole-file candidates for paths in the original snapshot, so
        // an extra terminal path must yield no replacement while the valid
        // stable sibling still emits exactly one bound to digest(original).
        let stages = vec![stage("lint-a", &["rust"], &["src/a.rs"])];
        let mut initial = BTreeMap::new();
        initial.insert("src/a.rs".to_owned(), "BAD\n".to_owned());
        let mut terminal = BTreeMap::new();
        terminal.insert("src/a.rs".to_owned(), "GOOD\n".to_owned());
        terminal.insert("src/extra.rs".to_owned(), "GOOD extra\n".to_owned());
        let result = assemble(
            "//quality:test",
            Capability::Lint as i32,
            &stages,
            &initial,
            &terminal,
            (Vec::new(), Vec::new()),
            (2, Convergence::Stable),
        )
        .unwrap();
        assert_eq!(result.replacements.len(), 1);
        assert_eq!(result.replacements[0].path, "src/a.rs");
        assert_eq!(
            result.replacements[0].original_digest,
            digest("BAD\n".as_bytes())
        );
        assert_eq!(result.replacements[0].edits.len(), 1);
        assert_eq!(result.replacements[0].edits[0].start_byte, 0);
        assert_eq!(result.replacements[0].edits[0].end_byte, 4);
        assert_eq!(
            result.replacements[0].edits[0].replacement,
            b"GOOD\n".to_vec()
        );
        assert!(validate(&result).is_ok());
    }

    #[test]
    fn file_modes_do_not_alter_pipeline_outputs() {
        // Determinism/apply-safety battery (issue #84):
        // `quality-testing.md` requires file modes preserved and newline
        // behavior documented. The runner takes only (path, bytes), so
        // model each mode as metadata stripped before the call and
        // require byte-identical manifests; different bytes under one
        // mode must diverge, proving modes are preserved out of band
        // while bytes (including newlines) stay load-bearing.
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let modes = [0o644, 0o755, 0o600];
        let mut manifests = Vec::with_capacity(modes.len());
        for mode in modes {
            let _ = mode;
            let files = vec![file("src/lib.rs", "BAD\n")];
            let result = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
            assert_eq!(result.convergence, Convergence::Stable as i32);
            assert_eq!(result.replacements.len(), 1);
            assert_eq!(
                result.replacements[0].original_digest,
                digest("BAD\n".as_bytes())
            );
            assert!(validate(&result).is_ok());
            manifests.push(encode_validated(&result).unwrap());
        }
        for other in manifests.iter().skip(1) {
            assert_eq!(&manifests[0], other);
        }
        let clean = vec![file("src/lib.rs", "GOOD\n")];
        let clean_result = run_pipeline("//quality:test", "lint", &stages, &clean).unwrap();
        assert!(clean_result.replacements.is_empty());
        assert_ne!(manifests[0], encode_validated(&clean_result).unwrap());
    }

    #[test]
    fn invalid_utf8_replacement_rejected_by_validate_gate() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // invalid UTF-8 source and replacement bytes to be rejected. The
        // runner rejects non-UTF-8 sources at request validation, and
        // `validate` rejects non-UTF-8 replacements, so a valid stable
        // BAD->GOOD candidate with corrupted replacement bytes must fail
        // `validate` while the unmutated result passes.
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let files = vec![file("src/lib.rs", "BAD\n")];
        let valid = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
        assert_eq!(valid.replacements.len(), 1);
        assert!(validate(&valid).is_ok());
        let mut corrupted = valid.clone();
        corrupted.replacements[0].edits[0].replacement = vec![0xFF, 0xFE];
        assert!(validate(&corrupted).is_err());
        let bad_source = vec![FileInput {
            path: "src/lib.rs".to_owned(),
            bytes: vec![0xFF],
        }];
        assert!(run_pipeline("//quality:test", "lint", &stages, &bad_source).is_err());
    }

    #[test]
    fn permutation_ranking_prefers_stable_fewer_rounds() {
        // Determinism battery (issue #84): `quality-testing.md` requires
        // comparing viable permutations for each multi-tool set, rejecting
        // incorrect, divergent, oscillating, and unjustifiably different
        // terminals, and ranking equivalent correct orders by
        // non-convergence count, rounds, process starts, then wall time.
        // Equivalent lint-a+fmt-a orders over BAD plus whitespace converge
        // to identical stable terminals in identical rounds, so their rank
        // keys tie deterministically; a direct fix in fewer rounds ranks
        // before a gradual fix to the same terminal; stable ranks before
        // oscillation and iteration-limit; different terminals diverge and
        // must be rejected rather than ranked together.
        let forward = vec![
            stage("lint-a", &["rust"], &["src/lib.rs"]),
            stage("fmt-a", &["rust"], &["src/lib.rs"]),
        ];
        let reversed = vec![
            stage("fmt-a", &["rust"], &["src/lib.rs"]),
            stage("lint-a", &["rust"], &["src/lib.rs"]),
        ];
        let files = vec![file("src/lib.rs", "BAD   \n")];
        let first = run_pipeline("//quality:test", "lint", &forward, &files).unwrap();
        let second = run_pipeline("//quality:test", "lint", &reversed, &files).unwrap();
        assert_eq!(first.convergence, Convergence::Stable as i32);
        assert_eq!(second.convergence, Convergence::Stable as i32);
        assert_eq!(first.completed_rounds, second.completed_rounds);
        assert_eq!(first.terminal_snapshot, second.terminal_snapshot);
        assert_eq!(first.replacements, second.replacements);
        assert!(validate(&first).is_ok());
        assert!(validate(&second).is_ok());
        let rank_key = |result: &QualityResult| {
            let non_converged = i32::from(result.convergence != Convergence::Stable as i32);
            (non_converged, result.completed_rounds)
        };
        assert_eq!(rank_key(&first), rank_key(&second));
        assert_eq!(rank_key(&first), (0, 2));
        let stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let mut initial = BTreeMap::new();
        initial.insert("src/lib.rs".to_owned(), "BAD".to_owned());
        let direct = |_: &str, _: &str, text: &str| {
            if text == "BAD" {
                Ok("GOOD".to_owned())
            } else {
                Ok(text.to_owned())
            }
        };
        let gradual = |_: &str, _: &str, text: &str| {
            if text == "BAD" {
                Ok("MID".to_owned())
            } else if text == "MID" {
                Ok("GOOD".to_owned())
            } else {
                Ok(text.to_owned())
            }
        };
        let (fast_terminal, fast_rounds, fast_conv) =
            run_convergence(&initial, &stages, MAX_COMPLETED_ROUNDS, direct).unwrap();
        let (slow_terminal, slow_rounds, slow_conv) =
            run_convergence(&initial, &stages, MAX_COMPLETED_ROUNDS, gradual).unwrap();
        assert_eq!(fast_conv, Convergence::Stable);
        assert_eq!(slow_conv, Convergence::Stable);
        assert_eq!(fast_terminal["src/lib.rs"], "GOOD");
        assert_eq!(slow_terminal["src/lib.rs"], "GOOD");
        assert_eq!(fast_rounds, 2);
        assert_eq!(slow_rounds, 3);
        assert!(fast_rounds < slow_rounds);
        let flip = |_: &str, _: &str, text: &str| {
            if text == "a" {
                Ok("b".to_owned())
            } else {
                Ok("a".to_owned())
            }
        };
        let mut flip_initial = BTreeMap::new();
        flip_initial.insert("src/lib.rs".to_owned(), "a".to_owned());
        let (_, _, flip_conv) =
            run_convergence(&flip_initial, &stages, MAX_COMPLETED_ROUNDS, flip).unwrap();
        assert_eq!(flip_conv, Convergence::Oscillation);
        let grow = |_: &str, _: &str, text: &str| Ok(format!("{text}x"));
        let (_, _, grow_conv) = run_convergence(&flip_initial, &stages, 3, grow).unwrap();
        assert_eq!(grow_conv, Convergence::IterationLimit);
        let stable_rank = (0, fast_rounds);
        let oscillation_rank = (1, 2);
        let limit_rank = (1, 3);
        assert!(stable_rank < oscillation_rank);
        assert!(stable_rank < limit_rank);
        let fix_stages = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let keep_stages = vec![stage("lint-b", &["rust"], &["src/lib.rs"])];
        let bad_files = vec![file("src/lib.rs", "BAD\n")];
        let fixed = run_pipeline("//quality:test", "lint", &fix_stages, &bad_files).unwrap();
        let kept = run_pipeline("//quality:test", "lint", &keep_stages, &bad_files).unwrap();
        assert_ne!(fixed.terminal_snapshot, kept.terminal_snapshot);
    }

    #[test]
    fn interruption_leaves_no_partially_written_file() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // interruption to leave no partially written file and only complete
        // earlier path commits in deterministic path order. The runner emits
        // one whole-file edit per stable changed file, so every prefix of
        // the sorted replacements must leave each path fully original or
        // fully terminal, and the full prefix must equal the terminal map.
        let stages = vec![stage(
            "lint-a",
            &["rust"],
            &["src/c.rs", "src/a.rs", "src/b.rs"],
        )];
        let mut initial = BTreeMap::new();
        initial.insert("src/c.rs".to_owned(), "BAD c\n".to_owned());
        initial.insert("src/b.rs".to_owned(), "BAD b\n".to_owned());
        initial.insert("src/a.rs".to_owned(), "BAD a\n".to_owned());
        let mut terminal = BTreeMap::new();
        terminal.insert("src/c.rs".to_owned(), "GOOD c\n".to_owned());
        terminal.insert("src/b.rs".to_owned(), "GOOD b\n".to_owned());
        terminal.insert("src/a.rs".to_owned(), "GOOD a\n".to_owned());
        let result = assemble(
            "//quality:test",
            Capability::Lint as i32,
            &stages,
            &initial,
            &terminal,
            (Vec::new(), Vec::new()),
            (2, Convergence::Stable),
        )
        .unwrap();
        assert_eq!(result.replacements.len(), 3);
        for edits in &result.replacements {
            let original = initial.get(&edits.path).expect("staged path");
            assert_eq!(edits.edits.len(), 1);
            assert_eq!(edits.edits[0].start_byte, 0);
            assert_eq!(edits.edits[0].end_byte, original.len() as u64);
        }
        let paths: Vec<&str> = result
            .replacements
            .iter()
            .map(|edits| edits.path.as_str())
            .collect();
        assert_eq!(paths, vec!["src/a.rs", "src/b.rs", "src/c.rs"]);
        for prefix_len in 0..=result.replacements.len() {
            let mut state = initial.clone();
            for edits in result.replacements.iter().take(prefix_len) {
                state.insert(
                    edits.path.clone(),
                    String::from_utf8(edits.edits[0].replacement.clone()).unwrap(),
                );
            }
            for (path, body) in &state {
                let original = initial.get(path).expect("staged path");
                let terminal_body = terminal.get(path).expect("staged path");
                assert!(body == original || body == terminal_body);
            }
        }
        let mut full = initial.clone();
        for edits in &result.replacements {
            full.insert(
                edits.path.clone(),
                String::from_utf8(edits.edits[0].replacement.clone()).unwrap(),
            );
        }
        assert_eq!(full, terminal);
        assert!(validate(&result).is_ok());
    }

    #[test]
    fn checkout_and_query_permutations_converge_identically() {
        // Determinism battery (issue #84): `quality-testing.md` requires
        // different checkout paths and randomized query/arrival orders to
        // compare equal where Bazel permits. The runner takes only
        // workspace-relative path+bytes, so model each absolute checkout
        // prefix as stripped metadata while simultaneously reversing file
        // arrival and stage declaration orders; both permutations must
        // converge to identical snapshots, sorted diagnostics, sorted
        // replacements, and rounds.
        let checkouts = ["/tmp/checkout-a", "/home/user/work/tree"];
        let mut manifests = Vec::with_capacity(checkouts.len());
        for (index, prefix) in checkouts.iter().enumerate() {
            let _ = prefix;
            let stages = if index == 0 {
                vec![stage("lint-a", &["rust"], &["src/a.rs", "src/b.rs"])]
            } else {
                vec![stage("lint-a", &["rust"], &["src/b.rs", "src/a.rs"])]
            };
            let files = if index == 0 {
                vec![file("src/a.rs", "BAD a\n"), file("src/b.rs", "BAD b\n")]
            } else {
                vec![file("src/b.rs", "BAD b\n"), file("src/a.rs", "BAD a\n")]
            };
            let result = run_pipeline("//quality:test", "lint", &stages, &files).unwrap();
            assert_eq!(result.convergence, Convergence::Stable as i32);
            assert_eq!(result.completed_rounds, 2);
            assert_eq!(result.replacements.len(), 2);
            assert!(validate(&result).is_ok());
            manifests.push(encode_validated(&result).unwrap());
            // Absolute prefix never enters FileInput by construction.
            assert!(format!("{prefix}/src/a.rs").ends_with("src/a.rs"));
        }
        // Checkout prefix plus query/arrival permutation leaves semantic
        // outputs identical; stage echoes keep declaration order by design
        // so sorted stage sets compare separately.
        let decoded: Vec<QualityResult> = manifests
            .iter()
            .map(|bytes| decode_validated(bytes).expect("decode"))
            .collect();
        assert_eq!(decoded[0].original_snapshot, decoded[1].original_snapshot);
        assert_eq!(decoded[0].terminal_snapshot, decoded[1].terminal_snapshot);
        assert_eq!(
            decoded[0].initial_diagnostics,
            decoded[1].initial_diagnostics
        );
        assert_eq!(
            decoded[0].terminal_diagnostics,
            decoded[1].terminal_diagnostics
        );
        assert_eq!(decoded[0].replacements, decoded[1].replacements);
        assert_eq!(decoded[0].completed_rounds, decoded[1].completed_rounds);
        let mut first_sources = decoded[0].stages[0].source_paths.clone();
        let mut second_sources = decoded[1].stages[0].source_paths.clone();
        first_sources.sort();
        second_sources.sort();
        assert_eq!(first_sources, second_sources);
        let stages = vec![stage("lint-a", &["rust"], &["src/a.rs", "src/b.rs"])];
        let forward = vec![file("src/a.rs", "BAD a\n"), file("src/b.rs", "BAD b\n")];
        let reversed = vec![file("src/b.rs", "BAD b\n"), file("src/a.rs", "BAD a\n")];
        let first = run_pipeline("//quality:test", "lint", &stages, &forward).unwrap();
        let second = run_pipeline("//quality:test", "lint", &stages, &reversed).unwrap();
        assert_eq!(first.replacements, second.replacements);
        assert_eq!(manifests[0], encode_validated(&first).unwrap());
    }

    #[test]
    fn permutation_ranking_uses_process_starts_then_wall_time_tiebreak() {
        // Determinism battery (issue #84): `quality-testing.md` requires
        // equivalent correct orders to rank by non-convergence count,
        // rounds, process starts, then measured wall time. The existing
        // ranking test proves the first two keys; this proves the last
        // two tiebreaks deterministically. Two pipelines converge to the
        // same stable GOOD terminal in the same 2 rounds: single-stage
        // (2 starts) vs lint-a plus identity lint-b (4 starts). Fewer
        // starts must rank first; with equal starts the smaller synthetic
        // wall time must rank first (real wall time is measured, ordering
        // here proves the tiebreak is total and stable).
        let single = vec![stage("lint-a", &["rust"], &["src/lib.rs"])];
        let doubled = vec![
            stage("lint-a", &["rust"], &["src/lib.rs"]),
            stage("lint-b", &["rust"], &["src/lib.rs"]),
        ];
        let mut initial = BTreeMap::new();
        initial.insert("src/lib.rs".to_owned(), "BAD\n".to_owned());
        let single_calls = std::cell::RefCell::new(0u32);
        let single_counting = |tool: &str, _: &str, text: &str| {
            *single_calls.borrow_mut() += 1;
            Ok(apply_synthetic(tool, text))
        };
        let (single_terminal, single_rounds, single_conv) =
            run_convergence(&initial, &single, MAX_COMPLETED_ROUNDS, single_counting).unwrap();
        let doubled_calls = std::cell::RefCell::new(0u32);
        let doubled_counting = |tool: &str, _: &str, text: &str| {
            *doubled_calls.borrow_mut() += 1;
            Ok(apply_synthetic(tool, text))
        };
        let (doubled_terminal, doubled_rounds, doubled_conv) =
            run_convergence(&initial, &doubled, MAX_COMPLETED_ROUNDS, doubled_counting).unwrap();
        assert_eq!(single_conv, Convergence::Stable);
        assert_eq!(doubled_conv, Convergence::Stable);
        assert_eq!(single_terminal["src/lib.rs"], "GOOD\n");
        assert_eq!(doubled_terminal["src/lib.rs"], "GOOD\n");
        assert_eq!(single_terminal, doubled_terminal);
        assert_eq!(single_rounds, 2);
        assert_eq!(doubled_rounds, 2);
        let single_starts = *single_calls.borrow();
        let doubled_starts = *doubled_calls.borrow();
        assert_eq!(single_starts, 2);
        assert_eq!(doubled_starts, 4);
        let rank_key = |convergence: Convergence, rounds: u32, starts: u32, wall_ms: u64| {
            (
                i32::from(convergence != Convergence::Stable),
                rounds,
                starts,
                wall_ms,
            )
        };
        let single_rank = rank_key(Convergence::Stable, single_rounds, single_starts, 10);
        let doubled_rank = rank_key(Convergence::Stable, doubled_rounds, doubled_starts, 5);
        assert!(
            single_rank < doubled_rank,
            "fewer process starts ranks first even with larger wall time"
        );
        let fast_rank = rank_key(Convergence::Stable, 2, 2, 10);
        let slow_rank = rank_key(Convergence::Stable, 2, 2, 20);
        assert!(
            fast_rank < slow_rank,
            "smaller wall time ranks first on full tie"
        );
        assert_eq!(fast_rank, (0, 2, 2, 10));
    }

    #[test]
    fn each_stage_runs_once_per_round_without_hidden_passes() {
        // Apply-safety battery (issue #84): `quality-testing.md` requires
        // native tool-internal passes to count as one stage when proving
        // the fixed ten-round limit. The convergence loop must invoke
        // each stage exactly once per round per staged path: two stages
        // over three staged paths converging in 2 rounds invoke apply
        // exactly 3 x 2 = 6 times, with every (tool, path) pair invoked
        // exactly once per round (2x). A hidden extra internal pass or a
        // skipped stage invocation would break either count.
        let stages = vec![
            stage("lint-a", &["rust"], &["src/a.rs", "src/b.rs"]),
            stage("fmt-a", &["rust"], &["src/a.rs"]),
        ];
        let mut initial = BTreeMap::new();
        initial.insert("src/a.rs".to_owned(), "BAD   \n".to_owned());
        initial.insert("src/b.rs".to_owned(), "BAD\n".to_owned());
        let calls = std::cell::RefCell::new(BTreeMap::new());
        let counting = |tool: &str, path: &str, text: &str| {
            *calls
                .borrow_mut()
                .entry((tool.to_owned(), path.to_owned()))
                .or_insert(0u32) += 1;
            Ok(apply_synthetic(tool, text))
        };
        let (terminal, completed, convergence) =
            run_convergence(&initial, &stages, MAX_COMPLETED_ROUNDS, counting).unwrap();
        assert_eq!(convergence, Convergence::Stable);
        assert_eq!(completed, 2);
        assert_eq!(terminal["src/a.rs"], "GOOD\n");
        assert_eq!(terminal["src/b.rs"], "GOOD\n");
        let calls = calls.borrow();
        assert_eq!(calls.values().sum::<u32>(), 6);
        assert_eq!(calls.len(), 3);
        for pair in [
            ("lint-a", "src/a.rs"),
            ("lint-a", "src/b.rs"),
            ("fmt-a", "src/a.rs"),
        ] {
            assert_eq!(
                calls[&(pair.0.to_owned(), pair.1.to_owned())],
                2,
                "each staged (tool, path) runs once per round"
            );
        }
    }
}
