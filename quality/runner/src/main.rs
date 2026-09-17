//! M03 WP2b pipeline runner binary: thin CLI shim over the runner library.
//! All pipeline semantics live in the library and are unit-tested there.
//!
//! Usage:
//! ```text
//! quality_runner --producer LABEL --capability CAP --output OUT.pb \
//!   --stage TOOL;class,class;path,path [--stage ...] \
//!   --source WORKSPACE_PATH=EXEC_PATH [--source ...] \
//!   [--sibling WORKSPACE_PATH=EXEC_PATH [--sibling ...]] \
//!   [--real --tool-binary TOOL=ABS_PATH [--tool-binary ...] \
//!    [--tool-config TOOL=MIRROR_REL] [--tool-edition TOOL=EDITION] \
//!    [--tool-file TOOL=MIRROR_REL=EXEC_PATH] \
//!    [--upstream-diagnostics TOOL=EXEC_PATH] \
//!    [--tool-env TOOL=KEY=VALUE] [--scratch-parent PATH]]
//! ```
//! Stages run in argument order. Each `--source` maps one workspace path
//! to the action-local file holding its bytes. Each `--sibling` maps one
//! unclassified link-resolution file (Markdown `--sibling` inputs): sibling
//! bytes are never linted and never enter snapshots. Without `--real` the
//! synthetic M03 pipeline runs. With `--real` the M04 real backend runs
//! `run_real_pipeline` over the resolved tools: each stage tool needs one
//! `--tool-binary`, configs are mirror-relative `--tool-config` paths whose
//! bytes arrive via `--tool-file`, crate editions arrive via
//! `--tool-edition` (rustfmt only: the aspect passes the `CrateInfo`
//! edition, `RUST_EDITION` for provider-less targets; the runner itself
//! never guesses), and extra hermetic env entries arrive
//! via `--tool-env`. Delegated tools (Clippy, #47) take no binary:
//! each `--upstream-diagnostics` maps one authoritative upstream
//! diagnostics file the backend parses without spawning. Scratch trees
//! default under `TMPDIR`. Failures exit
//! nonzero with a message on stderr and write no output.

// LCOV_EXCL_START - reason: thin binary shim; CLI parsing and file I/O failures are operational action failures verified by build and WP2c aspect execution, not unit coverage.
use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::{
    error::{ContextKind, ContextValue, ErrorKind},
    Parser,
};
use quality_result::encode_validated;
use quality_runner::{
    real::{RealBackend, RealTool},
    run_pipeline, FileInput, StageSpec,
};

/// Pipeline runner failure (issue #230).
///
/// Every variant renders the legacy operational message verbatim so
/// action diagnostics stay byte-identical while callers gain a matchable
/// type.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RunnerError {
    /// `clap` tokenizing failure mapped onto the legacy surface.
    #[error("{message}")]
    Args { message: String },
    /// `--stage` value without `TOOL;classes;paths` shape.
    #[error("malformed --stage {spec:?}, want TOOL;classes;paths")]
    BadStage { spec: String },
    /// `--tool-binary` value without `TOOL=ABS_PATH` shape.
    #[error("malformed --tool-binary {spec:?}, want TOOL=ABS_PATH")]
    BadToolBinary { spec: String },
    /// `--tool-config` value without `TOOL=MIRROR_REL` shape.
    #[error("malformed --tool-config {spec:?}, want TOOL=MIRROR_REL")]
    BadToolConfig { spec: String },
    /// `--tool-edition` value without `TOOL=EDITION` shape.
    #[error("malformed --tool-edition {spec:?}, want TOOL=EDITION")]
    BadToolEdition { spec: String },
    /// `--tool-file` value without `TOOL=MIRROR_REL=EXEC` shape.
    #[error("malformed --tool-file {spec:?}, want TOOL=MIRROR_REL=EXEC")]
    BadToolFile { spec: String },
    /// `--tool-env` value without `TOOL=KEY=VALUE` shape.
    #[error("malformed --tool-env {spec:?}, want TOOL=KEY=VALUE")]
    BadToolEnv { spec: String },
    /// `--upstream-diagnostics` value without `TOOL=EXEC_PATH` shape.
    #[error("malformed --upstream-diagnostics {spec:?}, want TOOL=EXEC_PATH")]
    BadUpstreamDiagnostics { spec: String },
    /// Required scalar flag missing.
    #[error("--{flag} is required")]
    MissingRequired { flag: &'static str },
    /// `--source` mapping without `WS_PATH=EXEC` shape.
    #[error("malformed --source {mapping:?}, want WS_PATH=EXEC")]
    BadSource { mapping: String },
    /// `--sibling` mapping without `WS_PATH=EXEC` shape.
    #[error("malformed --sibling {mapping:?}, want WS_PATH=EXEC")]
    BadSibling { mapping: String },
    /// Source bytes unreadable.
    #[error("cannot read {workspace:?}: {detail}")]
    UnreadableSource { workspace: String, detail: String },
    /// Sibling bytes unreadable.
    #[error("cannot read sibling {workspace:?}: {detail}")]
    UnreadableSibling { workspace: String, detail: String },
    /// Pipeline execution failed.
    #[error("pipeline failed: {detail}")]
    PipelineFailed { detail: String },
    /// Result encoding failed validation.
    #[error("invalid result: {detail}")]
    InvalidResult { detail: String },
    /// Output write failed.
    #[error("cannot write {output:?}: {detail}")]
    UnwritableOutput { output: String, detail: String },
    /// Duplicate `--tool-binary` for one tool.
    #[error("duplicate --tool-binary for {tool:?}")]
    DuplicateToolBinary { tool: String },
    /// `--tool-config` without a preceding `--tool-binary`.
    #[error("--tool-config for unknown tool {tool:?}: pass --tool-binary first")]
    UnknownToolConfig { tool: String },
    /// Duplicate `--tool-config` for one tool.
    #[error("duplicate --tool-config for {tool:?}")]
    DuplicateToolConfig { tool: String },
    /// `--tool-edition` without a preceding `--tool-binary`.
    #[error("--tool-edition for unknown tool {tool:?}: pass --tool-binary first")]
    UnknownToolEdition { tool: String },
    /// Duplicate `--tool-edition` for one tool.
    #[error("duplicate --tool-edition for {tool:?}")]
    DuplicateToolEdition { tool: String },
    /// Tool file bytes unreadable.
    #[error("cannot read tool file {rel:?} for {tool:?}: {detail}")]
    UnreadableToolFile {
        rel: String,
        tool: String,
        detail: String,
    },
    /// `--tool-file` without a preceding `--tool-binary`.
    #[error("--tool-file for unknown tool {tool:?}: pass --tool-binary first")]
    UnknownToolFile { tool: String },
    /// `--tool-env` without a preceding `--tool-binary`.
    #[error("--tool-env for unknown tool {tool:?}: pass --tool-binary first")]
    UnknownToolEnv { tool: String },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("quality_runner: {error}");
        std::process::exit(1);
    }
}

/// `argv` tokenizer. Repeatable options append in argument order (stages
/// run in that order); scalars keep last-wins repeats; every value option
/// consumes the next token unconditionally (even a `--`-led token), matching
/// the legacy hand loop. Only tokenizing moves to `clap`; all value-shape
/// validation (`parse_stage`, `parse_tool_*`, mapping splits) is untouched.
#[derive(Parser)]
#[command(disable_help_flag = true)]
struct Cli {
    #[arg(long, allow_hyphen_values = true, overrides_with = "producer")]
    producer: Option<String>,
    #[arg(long, allow_hyphen_values = true, overrides_with = "capability")]
    capability: Option<String>,
    #[arg(long, allow_hyphen_values = true, overrides_with = "output")]
    output: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    stage: Vec<String>,
    #[arg(long, allow_hyphen_values = true)]
    source: Vec<String>,
    #[arg(long, allow_hyphen_values = true)]
    sibling: Vec<String>,
    #[arg(long)]
    real: bool,
    #[arg(long, allow_hyphen_values = true, overrides_with = "scratch_parent")]
    scratch_parent: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    tool_binary: Vec<String>,
    #[arg(long, allow_hyphen_values = true)]
    tool_config: Vec<String>,
    #[arg(long, allow_hyphen_values = true)]
    tool_file: Vec<String>,
    #[arg(long, allow_hyphen_values = true)]
    tool_edition: Vec<String>,
    #[arg(long, allow_hyphen_values = true)]
    tool_env: Vec<String>,
    #[arg(long, allow_hyphen_values = true)]
    upstream_diagnostics: Vec<String>,
}

/// Raw `argv` token behind a [`clap::Error`], e.g. `--bogus` or `oops`.
fn invalid_token(error: &clap::Error) -> String {
    match error.get(ContextKind::InvalidArg) {
        Some(ContextValue::String(token)) => token.clone(),
        Some(ContextValue::Strings(tokens)) => tokens.first().cloned().unwrap_or_default(),
        _ => String::new(),
    }
}

/// Map `clap` tokenizing failures onto the legacy `run()` error surface.
/// Only [`ErrorKind::UnknownArgument`] and [`ErrorKind::InvalidValue`] (a
/// present flag with no consumable value) are reachable: every option takes
/// plain strings, so no value parser, conflict, or count error can fire.
fn parse_error(error: clap::Error, args: &[String]) -> String {
    let token = invalid_token(&error);
    match error.kind() {
        // `clap` strips an attached `=value` from the reported token; the
        // legacy loop echoed the whole `argv` element, so recover it.
        ErrorKind::UnknownArgument => {
            let echoed = args
                .iter()
                .find(|arg| *arg == &token)
                .or_else(|| {
                    args.iter()
                        .find(|arg| arg.starts_with(&format!("{token}=")))
                })
                .map_or(token.clone(), Clone::clone);
            format!("unknown flag {echoed:?}")
        }
        ErrorKind::InvalidValue => {
            // `clap` renders the pending option as `--flag <VALUE>`; the
            // legacy message names the bare `--flag`.
            let flag = token.split_whitespace().next().unwrap_or(&token);
            format!("missing value for {flag}")
        }
        _ => error
            .to_string()
            .lines()
            .next()
            .unwrap_or("invalid arguments")
            .to_owned(),
    }
}

fn parse_args(args: &[String]) -> Result<Cli, RunnerError> {
    Cli::try_parse_from(std::iter::once("quality_runner").chain(args.iter().map(|arg| arg as &str)))
        .map_err(|error| RunnerError::Args {
            message: parse_error(error, args),
        })
}

fn parse_stage(spec: &str) -> Result<StageSpec, RunnerError> {
    let (tool_id, rest) = spec.split_once(';').ok_or_else(|| RunnerError::BadStage {
        spec: spec.to_owned(),
    })?;
    let (classes, paths) = rest.split_once(';').ok_or_else(|| RunnerError::BadStage {
        spec: spec.to_owned(),
    })?;
    Ok(StageSpec {
        tool_id: tool_id.to_owned(),
        class_ids: classes.split(',').map(str::to_owned).collect(),
        source_paths: paths.split(',').map(str::to_owned).collect(),
    })
}

fn parse_tool_binary(spec: &str) -> Result<(String, PathBuf), RunnerError> {
    let (tool, path) = spec
        .split_once('=')
        .ok_or_else(|| RunnerError::BadToolBinary {
            spec: spec.to_owned(),
        })?;
    if tool.is_empty() || path.is_empty() {
        return Err(RunnerError::BadToolBinary {
            spec: spec.to_owned(),
        });
    }
    Ok((tool.to_owned(), PathBuf::from(path)))
}

fn parse_tool_config(spec: &str) -> Result<(String, String), RunnerError> {
    let (tool, rel) = spec
        .split_once('=')
        .ok_or_else(|| RunnerError::BadToolConfig {
            spec: spec.to_owned(),
        })?;
    if tool.is_empty() || rel.is_empty() {
        return Err(RunnerError::BadToolConfig {
            spec: spec.to_owned(),
        });
    }
    Ok((tool.to_owned(), rel.to_owned()))
}

fn parse_tool_edition(spec: &str) -> Result<(String, String), RunnerError> {
    let (tool, edition) = spec
        .split_once('=')
        .ok_or_else(|| RunnerError::BadToolEdition {
            spec: spec.to_owned(),
        })?;
    if tool.is_empty() || edition.is_empty() {
        return Err(RunnerError::BadToolEdition {
            spec: spec.to_owned(),
        });
    }
    Ok((tool.to_owned(), edition.to_owned()))
}

fn parse_tool_file(spec: &str) -> Result<(String, String, String), RunnerError> {
    let (tool, rest) = spec
        .split_once('=')
        .ok_or_else(|| RunnerError::BadToolFile {
            spec: spec.to_owned(),
        })?;
    let (rel, exec) = rest
        .split_once('=')
        .ok_or_else(|| RunnerError::BadToolFile {
            spec: spec.to_owned(),
        })?;
    if tool.is_empty() || rel.is_empty() || exec.is_empty() {
        return Err(RunnerError::BadToolFile {
            spec: spec.to_owned(),
        });
    }
    Ok((tool.to_owned(), rel.to_owned(), exec.to_owned()))
}

fn parse_tool_env(spec: &str) -> Result<(String, String, String), RunnerError> {
    let (tool, rest) = spec
        .split_once('=')
        .ok_or_else(|| RunnerError::BadToolEnv {
            spec: spec.to_owned(),
        })?;
    let (key, value) = rest
        .split_once('=')
        .ok_or_else(|| RunnerError::BadToolEnv {
            spec: spec.to_owned(),
        })?;
    if tool.is_empty() || key.is_empty() {
        return Err(RunnerError::BadToolEnv {
            spec: spec.to_owned(),
        });
    }
    Ok((tool.to_owned(), key.to_owned(), value.to_owned()))
}

fn parse_upstream_diagnostics(spec: &str) -> Result<(String, PathBuf), RunnerError> {
    let (tool, path) = spec
        .split_once('=')
        .ok_or_else(|| RunnerError::BadUpstreamDiagnostics {
            spec: spec.to_owned(),
        })?;
    if tool.is_empty() || path.is_empty() {
        return Err(RunnerError::BadUpstreamDiagnostics {
            spec: spec.to_owned(),
        });
    }
    Ok((tool.to_owned(), PathBuf::from(path)))
}

fn run() -> Result<(), RunnerError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = parse_args(&args)?;
    let producer = cli
        .producer
        .ok_or(RunnerError::MissingRequired { flag: "producer" })?;
    let capability = cli
        .capability
        .ok_or(RunnerError::MissingRequired { flag: "capability" })?;
    let output = cli
        .output
        .ok_or(RunnerError::MissingRequired { flag: "output" })?;
    let mut stages: Vec<StageSpec> = Vec::with_capacity(cli.stage.len());
    for spec in &cli.stage {
        stages.push(parse_stage(spec)?);
    }
    let mut sources: Vec<(String, String)> = Vec::with_capacity(cli.source.len());
    for mapping in &cli.source {
        let (workspace, exec) = mapping
            .split_once('=')
            .ok_or_else(|| RunnerError::BadSource {
                mapping: mapping.clone(),
            })?;
        sources.push((workspace.to_owned(), exec.to_owned()));
    }
    let mut siblings: Vec<(String, String)> = Vec::with_capacity(cli.sibling.len());
    for mapping in &cli.sibling {
        let (workspace, exec) = mapping
            .split_once('=')
            .ok_or_else(|| RunnerError::BadSibling {
                mapping: mapping.clone(),
            })?;
        siblings.push((workspace.to_owned(), exec.to_owned()));
    }
    let real = cli.real;
    let scratch_parent = cli.scratch_parent;
    let mut binaries: Vec<(String, PathBuf)> = Vec::with_capacity(cli.tool_binary.len());
    for spec in &cli.tool_binary {
        binaries.push(parse_tool_binary(spec)?);
    }
    let mut configs: Vec<(String, String)> = Vec::with_capacity(cli.tool_config.len());
    for spec in &cli.tool_config {
        configs.push(parse_tool_config(spec)?);
    }
    let mut editions: Vec<(String, String)> = Vec::with_capacity(cli.tool_edition.len());
    for spec in &cli.tool_edition {
        editions.push(parse_tool_edition(spec)?);
    }
    let mut tool_files: Vec<(String, String, String)> = Vec::with_capacity(cli.tool_file.len());
    for spec in &cli.tool_file {
        tool_files.push(parse_tool_file(spec)?);
    }
    let mut tool_env: Vec<(String, String, String)> = Vec::with_capacity(cli.tool_env.len());
    for spec in &cli.tool_env {
        tool_env.push(parse_tool_env(spec)?);
    }
    let mut upstream: Vec<(String, PathBuf)> = Vec::with_capacity(cli.upstream_diagnostics.len());
    for spec in &cli.upstream_diagnostics {
        upstream.push(parse_upstream_diagnostics(spec)?);
    }
    let mut files = Vec::with_capacity(sources.len());
    for (workspace, exec) in &sources {
        let bytes = std::fs::read(exec).map_err(|e| RunnerError::UnreadableSource {
            workspace: workspace.clone(),
            detail: e.to_string(),
        })?;
        files.push(FileInput {
            path: workspace.clone(),
            bytes,
        });
    }
    let mut sibling_files = Vec::with_capacity(siblings.len());
    for (workspace, exec) in &siblings {
        let bytes = std::fs::read(exec).map_err(|e| RunnerError::UnreadableSibling {
            workspace: workspace.clone(),
            detail: e.to_string(),
        })?;
        sibling_files.push(FileInput {
            path: workspace.clone(),
            bytes,
        });
    }
    if !real {
        let result = run_pipeline(&producer, &capability, &stages, &files).map_err(|e| {
            RunnerError::PipelineFailed {
                detail: e.to_string(),
            }
        })?;
        let bytes = encode_validated(&result).map_err(|e| RunnerError::InvalidResult {
            detail: e.to_string(),
        })?;
        dx_atomic_fs::write_atomic(std::path::Path::new(&output), &bytes).map_err(|e| {
            RunnerError::UnwritableOutput {
                output: output.clone(),
                detail: e.to_string(),
            }
        })?;
        return Ok(());
    }
    let mut tools: BTreeMap<String, RealTool> = BTreeMap::new();
    for (tool_id, binary) in binaries {
        if tools.contains_key(&tool_id) {
            return Err(RunnerError::DuplicateToolBinary { tool: tool_id });
        }
        // Bazel actions pass exec-root-relative tool paths while the backend
        // spawns from scratch trees under TMPDIR, so resolve relatives against
        // the startup working directory (the action exec root) now. Absolute
        // paths pass through unchanged.
        let absolute = if binary.is_absolute() {
            binary
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(&binary))
                .unwrap_or(binary)
        };
        tools.insert(
            tool_id,
            RealTool {
                binary: absolute,
                extra_env: Vec::new(),
                config_rel: None,
                edition: None,
                tool_files: Vec::new(),
                upstream_diagnostics: Vec::new(),
            },
        );
    }
    for (tool_id, exec) in upstream {
        // Like binaries, Bazel actions pass exec-root-relative paths
        // while the backend reads from its startup working directory
        // (the action exec root). Delegated tools carry no binary: the
        // entry exists so stage validation resolves, with an empty
        // binary no delegated path ever spawns.
        let absolute = if exec.is_absolute() {
            exec
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(&exec))
                .unwrap_or(exec)
        };
        tools
            .entry(tool_id.clone())
            .or_insert(RealTool {
                binary: PathBuf::new(),
                extra_env: Vec::new(),
                config_rel: None,
                edition: None,
                tool_files: Vec::new(),
                upstream_diagnostics: Vec::new(),
            })
            .upstream_diagnostics
            .push(absolute);
    }
    for (tool_id, rel) in configs {
        let tool = tools
            .get_mut(&tool_id)
            .ok_or_else(|| RunnerError::UnknownToolConfig {
                tool: tool_id.clone(),
            })?;
        if tool.config_rel.is_some() {
            return Err(RunnerError::DuplicateToolConfig { tool: tool_id });
        }
        tool.config_rel = Some(rel);
    }
    for (tool_id, edition) in editions {
        let tool = tools
            .get_mut(&tool_id)
            .ok_or_else(|| RunnerError::UnknownToolEdition {
                tool: tool_id.clone(),
            })?;
        if tool.edition.is_some() {
            return Err(RunnerError::DuplicateToolEdition { tool: tool_id });
        }
        tool.edition = Some(edition);
    }
    for (tool_id, rel, exec) in tool_files {
        let bytes = std::fs::read(&exec).map_err(|e| RunnerError::UnreadableToolFile {
            rel: rel.clone(),
            tool: tool_id.clone(),
            detail: e.to_string(),
        })?;
        let tool = tools
            .get_mut(&tool_id)
            .ok_or_else(|| RunnerError::UnknownToolFile {
                tool: tool_id.clone(),
            })?;
        tool.tool_files.push((rel, bytes));
    }
    for (tool_id, key, value) in tool_env {
        let tool = tools
            .get_mut(&tool_id)
            .ok_or_else(|| RunnerError::UnknownToolEnv {
                tool: tool_id.clone(),
            })?;
        tool.extra_env.push((key, value));
    }
    let scratch_parent = scratch_parent
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let backend = RealBackend::new(tools, scratch_parent);
    let result = quality_runner::real::run_real_pipeline_with_siblings(
        &producer,
        &capability,
        &stages,
        &files,
        &sibling_files,
        &backend,
    )
    .map_err(|e| RunnerError::PipelineFailed {
        detail: e.to_string(),
    })?;
    let bytes = encode_validated(&result).map_err(|e| RunnerError::InvalidResult {
        detail: e.to_string(),
    })?;
    dx_atomic_fs::write_atomic(std::path::Path::new(&output), &bytes).map_err(|e| {
        RunnerError::UnwritableOutput {
            output: output.clone(),
            detail: e.to_string(),
        }
    })?;
    Ok(())
}
// LCOV_EXCL_STOP - reason: end of thin binary shim exclusion.
