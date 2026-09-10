//! Real-tool pipeline backend (M04 WP2).
//!
//! Contract: `docs/quality/tool-integrations.md`,
//! `docs/tools/tool-acquisition.md`, `docs/quality/native-configuration.md`
//! (sole behavioral policy; without a hint every tool runs its pinned
//! upstream defaults, except Vale, which has no defaults and requires a
//! config). The backend executes the pinned check/fix invocations from
//! `quality_adapter::commands` over exact input bytes materialized into a
//! fresh hermetic scratch tree (`quality_adapter::exec`), parses the
//! pinned grammars (`quality_adapter::parsers`), places findings onto the
//! checked bytes, and folds everything into the frozen convergence
//! protocol and result assembly shared with the synthetic runner. Every
//! tool launch, grammar, or placement failure is an action failure
//! (`RunnerError`), never a skipped finding.
//!
//! Capability mapping: each M04 tool owns its capability slice outright
//! except Buildifier, whose single check reports both format findings
//! (empty rule: unformatted or syntax) and lint warnings (the rule
//! carries the category). The backend keeps only the findings matching
//! the running capability, so lint results never carry format findings
//! while format fixes converge them away. Taplo selects its mode by
//! command instead: `lint` for lint pipelines, `format --check` for
//! format pipelines.
//!
//! Fix application is best-effort per file: a nonzero fix exit leaves
//! the bytes unchanged and the check diagnostics report the cause, so
//! syntax-broken files surface findings instead of failing the action.
//! Spawn, materialization, and re-read failures still fail the action.
//! Clippy has no fix command: the backend applies `MachineApplicable`
//! suggestions in memory and re-checks the patched bytes on the next
//! round, so only suggestions that truly resolve their finding mark it
//! fixable. Vale is check-only and never rewrites.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};

use quality_adapter::commands::{self, Invocation};
use quality_adapter::exec::{self, ChildOutput, MirrorContents, MirrorFile, Scratch};
use quality_adapter::parsers::{self, FileFinding, ParseError};
use quality_adapter::{place_finding, suggest};

use crate::{
    assemble, run_convergence, stage_subset, validate_request, FileInput, QualityResult,
    RunnerError, StageSpec, MAX_COMPLETED_ROUNDS,
};
use quality_result::proto::Diagnostic;

/// Real tool IDs for the M04 initial adapters. Mirrors `REAL_ADAPTERS`
/// in `//quality:adapters.bzl`; the Starlark registry stays authoritative
/// for pipeline construction, this list pins the dispatch the backend
/// implements.
pub const REAL_TOOLS: &[&str] = &["buildifier", "clippy", "rustfmt", "taplo", "vale"];

/// Scratch-relative home for the materialized rustfmt defaults: without
/// a hinted config the tool still gets an explicit `--config-path`, so
/// no upward discovery can observe ambient state.
const RUSTFMT_DEFAULTS_REL: &str = "dx-rustfmt-default.toml";

/// One resolved real tool: absolute binary, extra hermetic environment
/// entries, optional mirror-relative config, and extra mirrored files
/// (hinted configs, Vale styles). The action maps its inputs to the
/// mirror paths through the CLI.
pub struct RealTool {
    pub binary: PathBuf,
    pub extra_env: Vec<(String, String)>,
    pub config_rel: Option<String>,
    pub tool_files: Vec<(String, Vec<u8>)>,
}

/// Injected tool spawner: absolute argv, scratch working directory,
/// and hermetic environment. Production uses [`real_spawn`].
pub type SpawnFn = fn(&[OsString], &Path, &[(String, String)]) -> io::Result<ChildOutput>;

/// Executable backend over resolved real tools. `spawn` is injected so
/// unit tests prove the materialize/parse/place chain against canned
/// tool outputs; production uses [`real_spawn`].
pub struct RealBackend {
    tools: BTreeMap<String, RealTool>,
    scratch_parent: PathBuf,
    spawn: SpawnFn,
}

/// Production spawner: one absolute tool binary with a cleared
/// environment. `PATH` is never set; see `exec::hermetic_env`.
pub fn real_spawn(
    argv: &[OsString],
    cwd: &Path,
    env: &[(String, String)],
) -> io::Result<ChildOutput> {
    exec::spawn(argv, cwd, env)
}

fn execution(tool_id: &str, detail: String) -> RunnerError {
    RunnerError::ToolExecution {
        tool_id: tool_id.to_owned(),
        detail,
    }
}

fn parsed<T>(tool_id: &str, result: Result<T, ParseError>) -> Result<T, RunnerError> {
    result.map_err(|err| RunnerError::ToolOutput {
        tool_id: tool_id.to_owned(),
        detail: err.to_string(),
    })
}

fn fresh_scratch(parent: &Path, tool_id: &str) -> Result<Scratch, RunnerError> {
    Scratch::create(parent).map_err(|err| execution(tool_id, format!("scratch: {err}")))
}

fn write_all(scratch: &Scratch, tool_id: &str, mirrors: &[MirrorFile]) -> Result<(), RunnerError> {
    scratch
        .materialize(mirrors)
        .map_err(|err| execution(tool_id, format!("materialize: {err}")))
}

/// Selects the Buildifier config directory: the mirrored config's
/// parent when hinted, so upward discovery finds exactly the hint.
fn buildifier_dir<'a>(config_rel: Option<&'a str>, cwd_rel: &'a str) -> Option<&'a str> {
    config_rel.map(|_| cwd_rel)
}

fn parent_rel(rel: &str) -> String {
    Path::new(rel)
        .parent()
        .and_then(|parent| parent.to_str())
        .unwrap_or_default()
        .to_owned()
}

impl RealBackend {
    /// Resolves production execution over `tools` with scratch trees
    /// under `scratch_parent` (normally `TMPDIR`, already action-scoped
    /// under Bazel).
    pub fn new(tools: BTreeMap<String, RealTool>, scratch_parent: PathBuf) -> Self {
        Self {
            tools,
            scratch_parent,
            spawn: real_spawn,
        }
    }

    /// Reports whether `tool_id` has a resolution. Unknown stage tools
    /// fail validation as `UnknownTool`, never as silent omissions.
    pub fn supports(&self, tool_id: &str) -> bool {
        self.tools.contains_key(tool_id)
    }

    fn tool(&self, tool_id: &str) -> Result<&RealTool, RunnerError> {
        self.tools
            .get(tool_id)
            .ok_or_else(|| RunnerError::UnknownTool {
                tool_id: tool_id.to_owned(),
            })
    }

    fn run(
        &self,
        tool_id: &str,
        tool: &RealTool,
        invocation: &Invocation,
        scratch: &Scratch,
    ) -> Result<ChildOutput, RunnerError> {
        // `cwd_rel` derives from an already-resolved config (or is
        // empty), so joining cannot escape the scratch tree.
        let cwd = scratch.root().join(&invocation.cwd_rel);
        let extra: Vec<(&str, &str)> = tool
            .extra_env
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        let env = exec::hermetic_env(scratch.root(), &extra);
        (self.spawn)(&invocation.argv, &cwd, &env)
            .map_err(|err| execution(tool_id, format!("spawn: {err}")))
    }

    fn mirror_tool_files(tool_id: &str, tool: &RealTool) -> Vec<MirrorFile> {
        let mut mirrors = Vec::with_capacity(tool.tool_files.len() + 1);
        for (rel, bytes) in &tool.tool_files {
            mirrors.push(MirrorFile {
                mirror_rel: PathBuf::from(rel),
                contents: MirrorContents::Bytes(bytes.clone()),
            });
        }
        if tool_id == "rustfmt" && tool.config_rel.is_none() {
            mirrors.push(MirrorFile {
                mirror_rel: PathBuf::from(RUSTFMT_DEFAULTS_REL),
                contents: MirrorContents::Bytes(Vec::new()),
            });
        }
        mirrors
    }

    /// Materializes one scratch tree with the exact source bytes plus
    /// the tool files, returning the scratch and the absolute path per
    /// workspace path in sorted order.
    fn stage_scratch(
        &self,
        tool_id: &str,
        tool: &RealTool,
        files: &BTreeMap<String, String>,
    ) -> Result<(Scratch, Vec<(String, PathBuf)>), RunnerError> {
        let scratch = fresh_scratch(&self.scratch_parent, tool_id)?;
        let mut mirrors = Vec::with_capacity(files.len() + tool.tool_files.len() + 1);
        let mut pairs = Vec::with_capacity(files.len());
        for (path, text) in files {
            let absolute = scratch
                .resolve(Path::new(path))
                .map_err(|err| execution(tool_id, format!("scratch file: {err}")))?;
            mirrors.push(MirrorFile {
                mirror_rel: PathBuf::from(path),
                contents: MirrorContents::Bytes(text.as_bytes().to_vec()),
            });
            pairs.push((path.clone(), absolute));
        }
        mirrors.extend(Self::mirror_tool_files(tool_id, tool));
        write_all(&scratch, tool_id, &mirrors)?;
        Ok((scratch, pairs))
    }

    /// Resolves the tool config to an absolute scratch path. rustfmt
    /// always resolves: the hinted config, else the materialized
    /// defaults. Every other tool resolves only its hint.
    fn config_abs(
        &self,
        tool_id: &str,
        tool: &RealTool,
        scratch: &Scratch,
    ) -> Result<Option<PathBuf>, RunnerError> {
        if tool_id == "rustfmt" {
            let rel = tool.config_rel.as_deref().unwrap_or(RUSTFMT_DEFAULTS_REL);
            return scratch
                .resolve(Path::new(rel))
                .map(Some)
                .map_err(|err| execution(tool_id, format!("scratch config: {err}")));
        }
        tool.config_rel
            .as_deref()
            .map(|rel| {
                scratch
                    .resolve(Path::new(rel))
                    .map_err(|err| execution(tool_id, format!("scratch config: {err}")))
            })
            .transpose()
    }

    /// Scratch working directory for check commands: Buildifier and
    /// Vale discover native config upward from the working directory,
    /// so with a hint the command runs from the mirrored config
    /// directory; every other tool runs from the scratch root.
    fn cwd_rel(tool_id: &str, config_rel: Option<&str>) -> String {
        match tool_id {
            "buildifier" | "vale" => config_rel.map(parent_rel).unwrap_or_default(),
            _ => String::new(),
        }
    }

    /// Runs one check over the staged files and returns the parsed
    /// findings still addressed by absolute scratch path.
    fn run_check(
        &self,
        tool_id: &str,
        tool: &RealTool,
        capability: &str,
        pairs: &[(String, PathBuf)],
        scratch: &Scratch,
    ) -> Result<Vec<FileFinding>, RunnerError> {
        let refs: Vec<&Path> = pairs
            .iter()
            .map(|(_, absolute)| absolute.as_path())
            .collect();
        let names: Vec<String> = pairs
            .iter()
            .map(|(_, absolute)| absolute.to_string_lossy().into_owned())
            .collect();
        let strs: Vec<&str> = names.iter().map(String::as_str).collect();
        let config = self.config_abs(tool_id, tool, scratch)?;
        let cwd_rel = Self::cwd_rel(tool_id, tool.config_rel.as_deref());
        match tool_id {
            "buildifier" => {
                let invocation = commands::buildifier_check(
                    &tool.binary,
                    &refs,
                    buildifier_dir(tool.config_rel.as_deref(), &cwd_rel),
                );
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                let reported = parsed(
                    tool_id,
                    parsers::parse_buildifier(&out.stdout, &out.stderr, &strs),
                )?;
                let mut kept = Vec::with_capacity(reported.len());
                for found in reported {
                    if (capability == "format") == found.finding.rule_id.is_empty() {
                        kept.push(found);
                    }
                }
                Ok(kept)
            }
            "clippy" => {
                let out_dir = scratch.root().join("dx-clippy-out");
                std::fs::create_dir_all(&out_dir)
                    .map_err(|err| execution(tool_id, format!("out dir: {err}")))?;
                let mut findings = Vec::new();
                for (workspace, absolute) in pairs {
                    let stem = Path::new(workspace)
                        .file_stem()
                        .map(|stem| stem.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let invocation = commands::clippy_check(
                        &tool.binary,
                        absolute,
                        &commands::crate_name_for(&stem),
                        &out_dir,
                    );
                    let out = self.run(tool_id, tool, &invocation, scratch)?;
                    let name = absolute.to_string_lossy().into_owned();
                    findings.extend(parsed(
                        tool_id,
                        parsers::parse_clippy(&out.stderr, out.code, &[&name]),
                    )?);
                }
                Ok(findings)
            }
            "rustfmt" => {
                let invocation = commands::rustfmt(
                    &tool.binary,
                    &refs,
                    config.as_ref().expect("rustfmt always resolves a config"),
                    true,
                );
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                parsed(
                    tool_id,
                    parsers::parse_rustfmt(&out.stdout, &out.stderr, out.code, &strs),
                )
            }
            "taplo" => {
                let invocation = if capability == "format" {
                    commands::taplo_format(&tool.binary, &refs, config.as_deref(), true)
                } else {
                    commands::taplo_lint(&tool.binary, &refs, config.as_deref())
                };
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                if capability == "format" {
                    parsed(
                        tool_id,
                        parsers::parse_taplo_format_check(&out.stderr, out.code, &strs),
                    )
                } else {
                    parsed(
                        tool_id,
                        parsers::parse_taplo_lint(&out.stderr, out.code, &strs),
                    )
                }
            }
            "vale" => {
                let invocation = match config.as_ref() {
                    Some(ini) => commands::vale_check(&tool.binary, ini, &refs, &cwd_rel),
                    None => return Err(execution(tool_id, "vale requires a config".to_owned())),
                };
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                parsed(tool_id, parsers::parse_vale(&out.stdout, out.code, &strs))
            }
            _ => Err(execution(
                tool_id,
                format!("unsupported real tool: {tool_id}"),
            )),
        }
    }

    /// Runs one check over the exact file bytes and returns normalized
    /// diagnostics with `fixable` cleared; the convergence pass marks
    /// fixability, never the backend.
    pub fn diagnose(
        &self,
        tool_id: &str,
        capability: &str,
        files: &BTreeMap<String, String>,
    ) -> Result<Vec<Diagnostic>, RunnerError> {
        let tool = self.tool(tool_id)?;
        let (scratch, pairs) = self.stage_scratch(tool_id, tool, files)?;
        let collected = self.run_check(tool_id, tool, capability, &pairs, &scratch)?;
        let mut diagnostics = Vec::with_capacity(collected.len());
        for found in &collected {
            let workspace = pairs
                .iter()
                .find(|(_, absolute)| absolute.as_os_str() == OsStr::new(&found.file))
                .map(|pair| pair.0.clone())
                .expect("parsed file was checked");
            let text = files.get(&workspace).expect("placed path checked");
            diagnostics.push(
                place_finding(&found.finding, &workspace, text).map_err(|err| {
                    RunnerError::UnplaceableFinding {
                        tool_id: tool_id.to_owned(),
                        detail: err.to_string(),
                    }
                })?,
            );
        }
        Ok(diagnostics)
    }

    /// Applies one fix round to a single file's bytes and returns the
    /// result. Format tools run their in-place fix and the bytes are
    /// re-read; Clippy applies `MachineApplicable` suggestions from a
    /// fresh check in memory; Vale returns its input.
    pub fn apply_fix(&self, tool_id: &str, path: &str, text: &str) -> Result<String, RunnerError> {
        let tool = self.tool(tool_id)?;
        match tool_id {
            "rustfmt" | "buildifier" | "taplo" => self.run_fix(tool_id, tool, path, text),
            "clippy" => self.apply_clippy(tool_id, path, text),
            "vale" => Ok(text.to_owned()),
            _ => Err(execution(
                tool_id,
                format!("unsupported real tool: {tool_id}"),
            )),
        }
    }

    fn run_fix(
        &self,
        tool_id: &str,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<String, RunnerError> {
        let scratch = fresh_scratch(&self.scratch_parent, tool_id)?;
        let mut mirrors = vec![MirrorFile {
            mirror_rel: PathBuf::from(path),
            contents: MirrorContents::Bytes(text.as_bytes().to_vec()),
        }];
        mirrors.extend(Self::mirror_tool_files(tool_id, tool));
        write_all(&scratch, tool_id, &mirrors)?;
        let absolute = scratch.root().join(path);
        let refs = [absolute.as_path()];
        let config = self.config_abs(tool_id, tool, &scratch)?;
        let cwd_rel = Self::cwd_rel(tool_id, tool.config_rel.as_deref());
        let invocation = match tool_id {
            "rustfmt" => commands::rustfmt(
                &tool.binary,
                &refs,
                config.as_ref().expect("rustfmt always resolves a config"),
                false,
            ),
            "buildifier" => commands::buildifier_fix(
                &tool.binary,
                &refs,
                buildifier_dir(tool.config_rel.as_deref(), &cwd_rel),
            ),
            _ => commands::taplo_format(&tool.binary, &refs, config.as_deref(), false),
        };
        let out = self.run(tool_id, tool, &invocation, &scratch)?;
        if out.code != Some(0) {
            return Ok(text.to_owned());
        }
        let fixed = std::fs::read(&absolute)
            .map_err(|err| execution(tool_id, format!("re-read fixed file: {err}")))?;
        String::from_utf8(fixed).map_err(|err| RunnerError::ToolOutput {
            tool_id: tool_id.to_owned(),
            detail: format!("fixed file is not UTF-8: {err}"),
        })
    }

    fn apply_clippy(&self, tool_id: &str, path: &str, text: &str) -> Result<String, RunnerError> {
        let tool = self.tool(tool_id)?;
        let mut single = BTreeMap::new();
        single.insert(path.to_owned(), text.to_owned());
        let (scratch, pairs) = self.stage_scratch(tool_id, tool, &single)?;
        let collected = self.run_check(tool_id, tool, "lint", &pairs, &scratch)?;
        let mut patched = text.as_bytes().to_vec();
        for found in &collected {
            if let Some(next) = suggest::apply_suggestions(&patched, &found.finding.suggestions) {
                patched = next;
            }
        }
        // `patched` starts as the valid input text and suggestions only
        // ever produce valid UTF-8, so this never fails on real runs.
        Ok(String::from_utf8(patched).expect("suggestions preserve UTF-8"))
    }
}

/// Executes one ordered pipeline over exact input bytes through real
/// tool binaries and returns the normalized result, sharing validation,
/// convergence, and assembly with [`crate::run_pipeline`] so the
/// semantics cannot drift between synthetic and real modes.
pub fn run_real_pipeline(
    producer: &str,
    capability: &str,
    stages: &[StageSpec],
    files: &[FileInput],
    backend: &RealBackend,
) -> Result<QualityResult, RunnerError> {
    let (capability_value, initial) =
        validate_request(producer, capability, stages, files, |tool| {
            backend.supports(tool)
        })?;
    let mut initial_diagnostics = Vec::new();
    for stage in stages {
        let subset = stage_subset(stage, &initial);
        initial_diagnostics.extend(backend.diagnose(&stage.tool_id, capability, &subset)?);
    }
    let (terminal, completed_rounds, convergence) = run_convergence(
        &initial,
        stages,
        MAX_COMPLETED_ROUNDS,
        |tool, path, text| backend.apply_fix(tool, path, text),
    )?;
    let mut terminal_diagnostics = Vec::new();
    for stage in stages {
        let subset = stage_subset(stage, &terminal);
        terminal_diagnostics.extend(backend.diagnose(&stage.tool_id, capability, &subset)?);
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
    use quality_result::encode_validated;
    use quality_result::proto::Convergence;

    type Spawn = SpawnFn;

    const BUILDIFIER_MIXED: &str = r#"{"success":false,"files":[{"filename":"FILE","formatted":false,"valid":true,"warnings":[{"start":{"line":1,"column":1},"end":{"line":1,"column":2},"category":"module-docstring","message":"The file has no module docstring."}]}]}"#;
    const BUILDIFIER_FAR: &str = r#"{"success":false,"files":[{"filename":"FILE","formatted":true,"valid":true,"warnings":[{"start":{"line":99,"column":1},"end":{"line":99,"column":2},"category":"module-docstring","message":"Far away."}]}]}"#;
    const TAPLO_BLOCK: &str = "error: invalid TOML\n  \u{250c}\u{2500} FILE:2:5\n  \u{2502}  \n2 \u{2502}   b = \n  \u{2502} \u{256d}\u{2500}\u{2500}\u{2500}\u{2500}^\n  \u{2502} \u{2570}^ expected value\n";
    const VALE_ALERT: &str = r#"{"FILE": [{"Action": {"Name": "", "Params": null}, "Span": [5, 10], "Check": "Test.Cotton", "Description": "", "Link": "", "Message": "Avoid cotton.", "Severity": "error", "Match": "cotton", "Line": 1}]}"#;
    const CLIPPY_WARN: &str = r#"{"$message_type":"diagnostic","message":"length comparison to zero","code":{"code":"clippy::len_zero","explanation":null},"level":"warning","spans":[{"file_name":"FILE","byte_start":8,"byte_end":20,"line_start":1,"line_end":1,"column_start":9,"column_end":21,"is_primary":true,"text":[],"label":null,"suggested_replacement":null,"suggestion_applicability":null,"expansion":null}],"children":[{"message":"use is_empty","code":null,"level":"help","spans":[{"file_name":"FILE","byte_start":8,"byte_end":20,"line_start":1,"line_end":1,"column_start":9,"column_end":21,"is_primary":true,"text":[],"label":null,"suggested_replacement":"!v.is_empty()","suggestion_applicability":"MachineApplicable","expansion":null}],"children":[],"rendered":null}],"rendered":null}
{"$message_type":"diagnostic","message":"1 warning emitted","code":null,"level":"warning","spans":[],"children":[],"rendered":null}"#;
    const CLIPPY_FAR: &str = r#"{"$message_type":"diagnostic","message":"length comparison to zero","code":{"code":"clippy::len_zero","explanation":null},"level":"warning","spans":[{"file_name":"FILE","byte_start":8,"byte_end":19,"line_start":1,"line_end":1,"column_start":9,"column_end":20,"is_primary":true,"text":[],"label":null,"suggested_replacement":null,"suggestion_applicability":null,"expansion":null}],"children":[{"message":"use is_empty","code":null,"level":"help","spans":[{"file_name":"FILE","byte_start":0,"byte_end":999,"line_start":1,"line_end":1,"column_start":9,"column_end":20,"is_primary":true,"text":[],"label":null,"suggested_replacement":"!v.is_empty()","suggestion_applicability":"MachineApplicable","expansion":null}],"children":[],"rendered":null}],"rendered":null}
{"$message_type":"diagnostic","message":"1 warning emitted","code":null,"level":"warning","spans":[],"children":[],"rendered":null}"#;

    fn plain_tool() -> RealTool {
        RealTool {
            binary: PathBuf::from("/fake/bin/tool"),
            extra_env: Vec::new(),
            config_rel: None,
            tool_files: Vec::new(),
        }
    }

    fn backend_for(tool_id: &str, tool: RealTool, spawn: Spawn) -> RealBackend {
        let mut tools = BTreeMap::new();
        tools.insert(tool_id.to_owned(), tool);
        RealBackend {
            tools,
            scratch_parent: std::env::temp_dir(),
            spawn,
        }
    }

    fn single(path: &str, text: &str) -> BTreeMap<String, String> {
        let mut files = BTreeMap::new();
        files.insert(path.to_owned(), text.to_owned());
        files
    }

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

    fn assert_hermetic(env: &[(String, String)]) {
        assert!(
            env.iter().any(|(key, _)| key == "TMPDIR"),
            "TMPDIR is always set"
        );
        assert!(
            !env.iter().any(|(key, _)| key == "PATH"),
            "PATH is never set"
        );
    }

    fn last_file(argv: &[OsString]) -> String {
        argv.last()
            .map(|arg| arg.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn trim_end(line: &[u8]) -> &[u8] {
        let mut end = line.len();
        while end > 0 && (line[end - 1] == b' ' || line[end - 1] == b'\t') {
            end -= 1;
        }
        &line[..end]
    }

    /// Content-aware rustfmt double: check reports a diff exactly when
    /// the materialized file has trailing whitespace, fix trims it.
    fn roundtrip_rustfmt(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let file = last_file(argv);
        let bytes = std::fs::read(&file).expect("checked file is materialized");
        if argv.iter().any(|arg| arg == "--check") {
            let dirty = bytes
                .split(|byte| *byte == b'\n')
                .any(|line| line.ends_with(b" ") || line.ends_with(b"\t"));
            if dirty {
                let stdout = format!("Diff in {file}:1:\n-x  \n+x\n");
                return Ok(ChildOutput {
                    code: Some(1),
                    stdout: stdout.into_bytes(),
                    stderr: Vec::new(),
                });
            }
            return Ok(ChildOutput {
                code: Some(0),
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        }
        let mut fixed = Vec::with_capacity(bytes.len());
        for line in bytes.split_inclusive(|byte| *byte == b'\n') {
            let trailing = line.ends_with(b"\n");
            let body = if trailing {
                &line[..line.len() - 1]
            } else {
                line
            };
            fixed.extend_from_slice(trim_end(body));
            if trailing {
                fixed.push(b'\n');
            }
        }
        std::fs::write(&file, fixed).expect("fix writes back");
        Ok(ChildOutput {
            code: Some(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    fn buildifier_plain(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let stdout = BUILDIFIER_MIXED.replace("FILE", &last_file(argv));
        Ok(ChildOutput {
            code: Some(0),
            stdout: stdout.into_bytes(),
            stderr: Vec::new(),
        })
    }

    fn buildifier_hinted(
        argv: &[OsString],
        cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        assert!(
            cwd.ends_with("cfg"),
            "hinted buildifier runs from the config dir"
        );
        buildifier_plain(argv, cwd, env)
    }

    fn buildifier_far(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let stdout = BUILDIFIER_FAR.replace("FILE", &last_file(argv));
        Ok(ChildOutput {
            code: Some(0),
            stdout: stdout.into_bytes(),
            stderr: Vec::new(),
        })
    }

    fn garbage_stdout(
        _argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        Ok(ChildOutput {
            code: Some(0),
            stdout: b"not json".to_vec(),
            stderr: Vec::new(),
        })
    }

    fn missing_spawn(
        _argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        Err(io::Error::new(io::ErrorKind::NotFound, "no such binary"))
    }

    fn taplo_either(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let file = last_file(argv);
        if argv.iter().any(|arg| arg == "--check") {
            let stderr = format!(
                "ERROR taplo:format_files: the file is not properly formatted path=\"{file}\"\n"
            );
            return Ok(ChildOutput {
                code: Some(1),
                stdout: Vec::new(),
                stderr: stderr.into_bytes(),
            });
        }
        let stderr = TAPLO_BLOCK.replace("FILE", &file);
        Ok(ChildOutput {
            code: Some(1),
            stdout: Vec::new(),
            stderr: stderr.into_bytes(),
        })
    }

    fn taplo_garbage(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let _ = last_file(argv);
        Ok(ChildOutput {
            code: Some(1),
            stdout: Vec::new(),
            stderr: b"unexpected taplo output".to_vec(),
        })
    }

    fn vale_hinted(
        argv: &[OsString],
        cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        assert!(
            cwd.ends_with("vdir"),
            "hinted vale runs from the config dir"
        );
        let stdout = VALE_ALERT.replace("FILE", &last_file(argv));
        Ok(ChildOutput {
            code: Some(1),
            stdout: stdout.into_bytes(),
            stderr: Vec::new(),
        })
    }

    fn clippy_len_zero(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let out_dir = argv
            .windows(2)
            .find(|pair| pair[0] == "--out-dir")
            .map(|pair| pair[1].clone())
            .expect("clippy passes --out-dir");
        assert!(
            Path::new(&out_dir).is_dir(),
            "clippy out dir is materialized"
        );
        let stderr = CLIPPY_WARN.replace("FILE", &last_file(argv));
        Ok(ChildOutput {
            code: Some(0),
            stdout: Vec::new(),
            stderr: stderr.into_bytes(),
        })
    }

    fn clippy_far(
        argv: &[OsString],
        cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let _ = cwd;
        let stderr = CLIPPY_FAR.replace("FILE", &last_file(argv));
        Ok(ChildOutput {
            code: Some(0),
            stdout: Vec::new(),
            stderr: stderr.into_bytes(),
        })
    }

    fn clippy_garbage(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let _ = last_file(argv);
        Ok(ChildOutput {
            code: Some(1),
            stdout: Vec::new(),
            stderr: b"not json lines".to_vec(),
        })
    }

    fn rustfmt_hinted(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let config = argv
            .windows(2)
            .find(|pair| pair[0] == "--config-path")
            .map(|pair| pair[1].clone())
            .expect("rustfmt passes --config-path");
        assert!(
            config.to_string_lossy().ends_with("hint.toml"),
            "hinted config reaches the tool"
        );
        assert!(
            Path::new(&config).is_file(),
            "hinted config is materialized"
        );
        Ok(ChildOutput {
            code: Some(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    fn rustfmt_defaults(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let config = argv
            .windows(2)
            .find(|pair| pair[0] == "--config-path")
            .map(|pair| pair[1].clone())
            .expect("rustfmt passes --config-path");
        assert!(
            config.to_string_lossy().ends_with(RUSTFMT_DEFAULTS_REL),
            "unhinted rustfmt gets the materialized defaults"
        );
        assert!(
            Path::new(&config).is_file(),
            "defaults file is materialized"
        );
        Ok(ChildOutput {
            code: Some(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    fn failing_fix(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let _ = last_file(argv);
        Ok(ChildOutput {
            code: Some(1),
            stdout: Vec::new(),
            stderr: b"syntax error".to_vec(),
        })
    }

    fn deleting_fix(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        std::fs::remove_file(last_file(argv)).expect("fix removes the file");
        Ok(ChildOutput {
            code: Some(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    fn binary_fix(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        std::fs::write(last_file(argv), b"\xff").expect("fix writes bytes");
        Ok(ChildOutput {
            code: Some(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    #[test]
    fn real_tools_pin_the_m04_set() {
        assert_eq!(
            REAL_TOOLS,
            &["buildifier", "clippy", "rustfmt", "taplo", "vale"]
        );
    }

    #[test]
    fn real_spawn_reports_missing_binaries() {
        assert!(real_spawn(
            &[OsString::from("/nonexistent-dx-tool-binary")],
            Path::new("/tmp"),
            &[]
        )
        .is_err());
    }

    #[test]
    fn supports_follows_resolution() {
        let backend = backend_for("rustfmt", plain_tool(), rustfmt_defaults);
        assert!(backend.supports("rustfmt"));
        assert!(!backend.supports("lint-a"));
    }

    #[test]
    fn rustfmt_pipeline_fixes_dirty_files_to_stable() {
        let backend = backend_for("rustfmt", plain_tool(), roundtrip_rustfmt);
        let stages = vec![stage("rustfmt", &["rust"], &["src/main.rs"])];
        let files = vec![file("src/main.rs", "x  \ny\t")];
        let result = run_real_pipeline("//quality:test", "format", &stages, &files, &backend)
            .expect("real pipeline");
        assert_eq!(result.convergence, Convergence::Stable as i32);
        assert_eq!(result.completed_rounds, 2);
        assert_eq!(result.initial_diagnostics.len(), 1);
        assert_eq!(result.initial_diagnostics[0].tool_id, "rustfmt");
        assert_eq!(
            result.initial_diagnostics[0].message,
            "file is not formatted"
        );
        assert!(result.terminal_diagnostics.is_empty());
        assert!(result.initial_diagnostics.iter().all(|d| d.fixable));
        assert_eq!(result.replacements.len(), 1);
        assert_eq!(result.replacements[0].edits[0].replacement, b"x\ny");
        assert!(encode_validated(&result).is_ok());
    }

    #[test]
    fn vale_pipeline_reports_without_rewriting() {
        let tool = RealTool {
            config_rel: Some("vdir/.vale.ini".to_owned()),
            tool_files: vec![(
                "vdir/.vale.ini".to_owned(),
                b"StylesPath = styles\n".to_vec(),
            )],
            ..plain_tool()
        };
        let backend = backend_for("vale", tool, vale_hinted);
        let stages = vec![stage("vale", &["markdown"], &["doc.md"])];
        let files = vec![file("doc.md", "aaa cotton bbb\n")];
        let result = run_real_pipeline("//quality:test", "lint", &stages, &files, &backend)
            .expect("real pipeline");
        assert_eq!(result.convergence, Convergence::Stable as i32);
        assert_eq!(result.completed_rounds, 1);
        assert_eq!(result.initial_diagnostics.len(), 1);
        assert_eq!(result.initial_diagnostics[0].rule_id, "Test.Cotton");
        assert_eq!(
            (
                result.initial_diagnostics[0].start_byte,
                result.initial_diagnostics[0].end_byte
            ),
            (Some(4), Some(10))
        );
        assert!(!result.initial_diagnostics[0].fixable);
        assert_eq!(result.terminal_diagnostics.len(), 1);
        assert!(result.replacements.is_empty());
        assert!(encode_validated(&result).is_ok());
    }

    #[test]
    fn unknown_stage_tool_fails_real_validation() {
        let backend = backend_for("rustfmt", plain_tool(), rustfmt_defaults);
        let stages = vec![stage("nope", &["rust"], &["src/main.rs"])];
        let files = vec![file("src/main.rs", "x\n")];
        assert_eq!(
            run_real_pipeline("//quality:test", "format", &stages, &files, &backend),
            Err(RunnerError::UnknownTool {
                tool_id: "nope".to_owned(),
            })
        );
    }

    #[test]
    fn buildifier_lint_keeps_only_categorized_warnings() {
        let backend = backend_for("buildifier", plain_tool(), buildifier_plain);
        let findings = backend
            .diagnose("buildifier", "lint", &single("a.bzl", "x = 1\n"))
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "module-docstring");
        assert_eq!(findings[0].start_byte, Some(0));
    }

    #[test]
    fn buildifier_format_keeps_only_format_findings() {
        let backend = backend_for("buildifier", plain_tool(), buildifier_plain);
        let findings = backend
            .diagnose("buildifier", "format", &single("a.bzl", "x = 1\n"))
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
        assert!(findings[0].rule_id.is_empty());
        assert_eq!(findings[0].message, "file is not formatted");
    }

    #[test]
    fn buildifier_hint_runs_from_the_config_dir() {
        let tool = RealTool {
            config_rel: Some("cfg/.buildifier.json".to_owned()),
            tool_files: vec![("cfg/.buildifier.json".to_owned(), b"{}".to_vec())],
            ..plain_tool()
        };
        let backend = backend_for("buildifier", tool, buildifier_hinted);
        let findings = backend
            .diagnose("buildifier", "lint", &single("a.bzl", "x = 1\n"))
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn taplo_lint_reports_syntax_blocks() {
        let backend = backend_for("taplo", plain_tool(), taplo_either);
        let findings = backend
            .diagnose("taplo", "lint", &single("a.toml", "a = 1\nb = \n"))
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tool_id, "taplo");
    }

    #[test]
    fn taplo_format_reports_unformatted_files() {
        let tool = RealTool {
            config_rel: Some("taplo.toml".to_owned()),
            tool_files: vec![("taplo.toml".to_owned(), b"".to_vec())],
            ..plain_tool()
        };
        let backend = backend_for("taplo", tool, taplo_either);
        let findings = backend
            .diagnose("taplo", "format", &single("a.toml", "a=1\n"))
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].message, "file is not formatted");
    }

    #[test]
    fn rustfmt_hinted_config_reaches_the_tool() {
        let tool = RealTool {
            config_rel: Some("hint.toml".to_owned()),
            tool_files: vec![("hint.toml".to_owned(), b"".to_vec())],
            ..plain_tool()
        };
        let backend = backend_for("rustfmt", tool, rustfmt_hinted);
        let findings = backend
            .diagnose(
                "rustfmt",
                "format",
                &single("src/main.rs", "fn main() {}\n"),
            )
            .expect("diagnosed");
        assert!(findings.is_empty());
    }

    #[test]
    fn rustfmt_without_hint_gets_materialized_defaults() {
        let backend = backend_for("rustfmt", plain_tool(), rustfmt_defaults);
        let findings = backend
            .diagnose(
                "rustfmt",
                "format",
                &single("src/main.rs", "fn main() {}\n"),
            )
            .expect("diagnosed");
        assert!(findings.is_empty());
    }

    #[test]
    fn clippy_reports_warnings_with_suggestions() {
        let backend = backend_for("clippy", plain_tool(), clippy_len_zero);
        let findings = backend
            .diagnose(
                "clippy",
                "lint",
                &single("src/main.rs", "let y = v.len() == 0;\n"),
            )
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "clippy::len_zero");
        assert_eq!(
            (findings[0].start_byte, findings[0].end_byte),
            (Some(8), Some(20))
        );
    }

    #[test]
    fn clippy_apply_uses_machine_applicable_suggestions() {
        let backend = backend_for("clippy", plain_tool(), clippy_len_zero);
        let patched = backend
            .apply_fix("clippy", "src/main.rs", "let y = v.len() == 0;\n")
            .expect("patched");
        assert_eq!(patched, "let y = !v.is_empty();\n");
    }

    #[test]
    fn clippy_apply_keeps_bytes_without_applicable_suggestions() {
        let backend = backend_for("clippy", plain_tool(), clippy_far);
        let patched = backend
            .apply_fix("clippy", "src/main.rs", "let y = v.len() == 0;\n")
            .expect("unchanged");
        assert_eq!(patched, "let y = v.len() == 0;\n");
    }

    #[test]
    fn diagnose_unknown_tool_fails() {
        let backend = backend_for("rustfmt", plain_tool(), rustfmt_defaults);
        assert_eq!(
            backend.diagnose("nope", "format", &single("src/main.rs", "x\n")),
            Err(RunnerError::UnknownTool {
                tool_id: "nope".to_owned(),
            })
        );
        assert_eq!(
            backend.apply_fix("nope", "src/main.rs", "x\n"),
            Err(RunnerError::UnknownTool {
                tool_id: "nope".to_owned(),
            })
        );
    }

    #[test]
    fn custom_tool_ids_have_no_dispatch() {
        let backend = backend_for("custom", plain_tool(), rustfmt_defaults);
        let err = backend
            .diagnose("custom", "format", &single("src/main.rs", "x\n"))
            .expect_err("no dispatch");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
        assert!(err.to_string().contains("unsupported real tool"));
        let err = backend
            .apply_fix("custom", "src/main.rs", "x\n")
            .expect_err("no dispatch");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
    }

    #[test]
    fn spawn_failure_fails_the_action() {
        let backend = backend_for("rustfmt", plain_tool(), missing_spawn);
        let err = backend
            .diagnose("rustfmt", "format", &single("src/main.rs", "x\n"))
            .expect_err("spawn fails");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
        assert!(err.to_string().contains("spawn"));
    }

    #[test]
    fn grammar_failure_fails_the_action() {
        let backend = backend_for("buildifier", plain_tool(), garbage_stdout);
        let err = backend
            .diagnose("buildifier", "lint", &single("a.bzl", "x = 1\n"))
            .expect_err("grammar fails");
        assert!(matches!(err, RunnerError::ToolOutput { .. }));
    }

    #[test]
    fn taplo_grammar_failure_fails_the_action() {
        let backend = backend_for("taplo", plain_tool(), taplo_garbage);
        let err = backend
            .diagnose("taplo", "lint", &single("a.toml", "a = 1\n"))
            .expect_err("grammar fails");
        assert!(matches!(err, RunnerError::ToolOutput { .. }));
        let err = backend
            .apply_fix("clippy", "src/main.rs", "x\n")
            .expect_err("clippy without resolution fails");
        assert!(matches!(err, RunnerError::UnknownTool { .. }));
    }

    #[test]
    fn unplaceable_findings_fail_the_action() {
        let backend = backend_for("buildifier", plain_tool(), buildifier_far);
        let err = backend
            .diagnose("buildifier", "lint", &single("a.bzl", "x = 1\n"))
            .expect_err("placement fails");
        assert!(matches!(err, RunnerError::UnplaceableFinding { .. }));
    }

    #[test]
    fn vale_without_config_fails_fast() {
        let backend = backend_for("vale", plain_tool(), vale_hinted);
        let err = backend
            .diagnose("vale", "lint", &single("doc.md", "cotton\n"))
            .expect_err("vale needs a config");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
        assert!(err.to_string().contains("requires a config"));
    }

    #[test]
    fn escaping_paths_fail_the_action() {
        let backend = backend_for("rustfmt", plain_tool(), roundtrip_rustfmt);
        let err = backend
            .diagnose("rustfmt", "format", &single("../evil.rs", "x\n"))
            .expect_err("escape fails");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
        let err = backend
            .apply_fix("rustfmt", "../evil.rs", "x\n")
            .expect_err("escape fails");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
    }

    #[test]
    fn escaping_tool_files_fail_the_action() {
        let tool = RealTool {
            tool_files: vec![("../evil".to_owned(), b"".to_vec())],
            ..plain_tool()
        };
        let backend = backend_for("rustfmt", tool, roundtrip_rustfmt);
        let err = backend
            .diagnose("rustfmt", "format", &single("src/main.rs", "x\n"))
            .expect_err("escape fails");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
        assert!(err.to_string().contains("materialize"));
    }

    #[test]
    fn unusable_scratch_parent_fails_the_action() {
        let backend = RealBackend {
            tools: BTreeMap::from([("rustfmt".to_owned(), plain_tool())]),
            scratch_parent: PathBuf::from("/nonexistent-dx-scratch-parent"),
            spawn: roundtrip_rustfmt,
        };
        let err = backend
            .diagnose("rustfmt", "format", &single("src/main.rs", "x\n"))
            .expect_err("scratch fails");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
        assert!(err.to_string().contains("scratch"));
        let err = backend
            .apply_fix("rustfmt", "src/main.rs", "x\n")
            .expect_err("scratch fails");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
    }

    #[test]
    fn escaping_configs_fail_the_action() {
        let tool = RealTool {
            config_rel: Some("../evil.toml".to_owned()),
            ..plain_tool()
        };
        let backend = backend_for("taplo", tool, taplo_either);
        let err = backend
            .diagnose("taplo", "lint", &single("a.toml", "a = 1\n"))
            .expect_err("escape fails");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
        assert!(err.to_string().contains("scratch config"));
        let tool = RealTool {
            config_rel: Some("../evil.toml".to_owned()),
            ..plain_tool()
        };
        let backend = backend_for("rustfmt", tool, rustfmt_hinted);
        let err = backend
            .diagnose("rustfmt", "format", &single("src/main.rs", "x\n"))
            .expect_err("escape fails");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
    }

    #[test]
    fn nonzero_fix_keeps_the_input_bytes() {
        let backend = backend_for("rustfmt", plain_tool(), failing_fix);
        let kept = backend
            .apply_fix("rustfmt", "src/main.rs", "x  \n")
            .expect("kept");
        assert_eq!(kept, "x  \n");
    }

    #[test]
    fn fix_without_output_file_fails_the_action() {
        let backend = backend_for("rustfmt", plain_tool(), deleting_fix);
        let err = backend
            .apply_fix("rustfmt", "src/main.rs", "x\n")
            .expect_err("re-read fails");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
        assert!(err.to_string().contains("re-read"));
    }

    #[test]
    fn non_utf8_fix_fails_the_action() {
        let backend = backend_for("rustfmt", plain_tool(), binary_fix);
        let err = backend
            .apply_fix("rustfmt", "src/main.rs", "x\n")
            .expect_err("encoding fails");
        assert!(matches!(err, RunnerError::ToolOutput { .. }));
    }

    #[test]
    fn clippy_diagnose_failure_aborts_apply() {
        let backend = backend_for("clippy", plain_tool(), clippy_garbage);
        let err = backend
            .apply_fix("clippy", "src/main.rs", "x\n")
            .expect_err("diagnose fails");
        assert!(matches!(err, RunnerError::ToolOutput { .. }));
    }

    #[test]
    fn error_display_reports_variants() {
        let rendered = format!(
            "{}",
            RunnerError::ToolOutput {
                tool_id: "taplo".to_owned(),
                detail: "bad grammar".to_owned(),
            }
        );
        assert!(rendered.contains("ToolOutput"));
    }

    fn buildifier_fix_ok(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        std::fs::write(last_file(argv), "fixed\n").expect("fix writes back");
        Ok(ChildOutput {
            code: Some(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    fn taplo_fix_ok(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        std::fs::write(last_file(argv), "a = 1\n").expect("fix writes back");
        Ok(ChildOutput {
            code: Some(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    fn check_ok_fix_missing(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        if argv.iter().any(|arg| arg == "--check") {
            return Ok(ChildOutput {
                code: Some(0),
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        }
        Err(io::Error::new(io::ErrorKind::NotFound, "fix binary gone"))
    }

    #[test]
    fn production_constructor_resolves_tools() {
        let tools = BTreeMap::from([("rustfmt".to_owned(), plain_tool())]);
        let backend = RealBackend::new(tools, std::env::temp_dir());
        assert!(backend.supports("rustfmt"));
        assert!(!backend.supports("nope"));
    }

    #[test]
    fn buildifier_fix_rewrites_the_file() {
        let backend = backend_for("buildifier", plain_tool(), buildifier_fix_ok);
        let fixed = backend
            .apply_fix("buildifier", "a.bzl", "x = 1\n")
            .expect("fixed");
        assert_eq!(fixed, "fixed\n");
    }

    #[test]
    fn taplo_fix_rewrites_the_file() {
        let backend = backend_for("taplo", plain_tool(), taplo_fix_ok);
        let fixed = backend
            .apply_fix("taplo", "a.toml", "a=1\n")
            .expect("fixed");
        assert_eq!(fixed, "a = 1\n");
    }

    #[test]
    fn fix_failure_aborts_real_convergence() {
        let backend = backend_for("rustfmt", plain_tool(), check_ok_fix_missing);
        let stages = vec![stage("rustfmt", &["rust"], &["src/main.rs"])];
        let files = vec![file("src/main.rs", "x  \n")];
        let err = run_real_pipeline("//quality:test", "format", &stages, &files, &backend)
            .expect_err("fix fails");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
        assert!(err.to_string().contains("spawn"));
    }
}
