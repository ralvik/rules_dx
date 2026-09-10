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

use std::collections::BTreeMap;

use quality_result::{
    digest,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerError {
    EmptyProducer,
    UnknownCapability {
        capability: String,
    },
    EmptyStages,
    EmptyToolId {
        stage: usize,
    },
    UnknownTool {
        tool_id: String,
    },
    EmptyClassIds {
        stage: usize,
    },
    EmptyStageSources {
        stage: usize,
    },
    DuplicateFile {
        path: String,
    },
    InvalidUtf8 {
        path: String,
    },
    MissingFile {
        path: String,
    },
    /// Tool launch, scratch, or I/O failure in a real backend.
    ToolExecution {
        tool_id: String,
        detail: String,
    },
    /// Real tool output outside the pinned grammar, or non-UTF-8 bytes
    /// where the protocol needs text.
    ToolOutput {
        tool_id: String,
        detail: String,
    },
    /// A parsed finding positions outside the bytes just checked.
    UnplaceableFinding {
        tool_id: String,
        detail: String,
    },
}

impl std::fmt::Display for RunnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for RunnerError {}

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

fn run_convergence(
    initial: &BTreeMap<String, String>,
    stages: &[StageSpec],
    max_rounds: u32,
    apply: impl Fn(&str, &str, &str) -> Result<String, RunnerError>,
) -> Result<(BTreeMap<String, String>, u32, Convergence), RunnerError> {
    let mut current = initial.clone();
    let mut seen = vec![initial.clone()];
    let mut completed_rounds = 0;
    for round in 1..=max_rounds {
        completed_rounds = round;
        let before = current.clone();
        for stage in stages {
            for path in &stage.source_paths {
                let body = current.get(path).expect("staged path validated present");
                let next = apply(&stage.tool_id, path, body)?;
                current.insert(path.clone(), next);
            }
        }
        if current == before {
            return Ok((current, completed_rounds, Convergence::Stable));
        }
        if seen.contains(&current) {
            return Ok((current, completed_rounds, Convergence::Oscillation));
        }
        seen.push(current.clone());
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
fn assemble(
    producer: &str,
    capability_value: i32,
    stages: &[StageSpec],
    initial: &BTreeMap<String, String>,
    terminal: &BTreeMap<String, String>,
    diagnostics: (Vec<Diagnostic>, Vec<Diagnostic>),
    outcome: (u32, Convergence),
) -> QualityResult {
    let (mut initial_diagnostics, mut terminal_diagnostics) = diagnostics;
    let (completed_rounds, convergence) = outcome;
    sort_diagnostics(&mut initial_diagnostics);
    sort_diagnostics(&mut terminal_diagnostics);
    let stable = convergence == Convergence::Stable;
    let mut replacements = Vec::new();
    for (path, original) in initial {
        let terminal_body = terminal.get(path).expect("staged path validated present");
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
    QualityResult {
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
    }
}

/// One exact input file's text within a converged run.
fn stage_subset(stage: &StageSpec, files: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    stage
        .source_paths
        .iter()
        .map(|path| {
            (
                path.clone(),
                files
                    .get(path)
                    .expect("staged path validated present")
                    .clone(),
            )
        })
        .collect()
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
            let body = initial.get(path).expect("staged path validated present");
            initial_diagnostics.extend(collect_diagnostics(&stage.tool_id, path, body));
        }
    }
    // Synthetic apply never fails, so a failure here is a runner bug,
    // not a pipeline error to propagate.
    let (terminal, completed_rounds, convergence) = run_convergence(
        &initial,
        stages,
        MAX_COMPLETED_ROUNDS,
        |tool, _path, text| Ok(apply_synthetic(tool, text)),
    )
    .expect("synthetic apply never fails");
    let mut terminal_diagnostics = Vec::new();
    for stage in stages {
        for path in &stage.source_paths {
            let body = terminal.get(path).expect("staged path validated present");
            terminal_diagnostics.extend(collect_diagnostics(&stage.tool_id, path, body));
        }
    }
    Ok(assemble(
        producer,
        capability_value,
        stages,
        &initial,
        &terminal,
        (initial_diagnostics, terminal_diagnostics),
        (completed_rounds, convergence),
    ))
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
        assert!(rendered.contains("MissingFile"));
    }
}
