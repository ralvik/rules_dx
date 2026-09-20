//! Deterministic shared quality pipeline runner (WP2b).
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

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

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
#[path = "lib_tests_a.rs"]
mod lib_tests_a;
#[cfg(test)]
#[path = "lib_tests_b.rs"]
mod lib_tests_b;
#[cfg(test)]
#[path = "lib_tests_c.rs"]
mod lib_tests_c;
