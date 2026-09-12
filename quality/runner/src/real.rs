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
//! format pipelines. Ruff likewise: `check` for lint pipelines,
//! `format --check` for format pipelines, and its fix mode follows the
//! same capability split (`check --fix` versus `format`).
//!
//! Fix application is best-effort per file: a nonzero fix exit leaves
//! the bytes unchanged and the check diagnostics report the cause, so
//! syntax-broken files surface findings instead of failing the action.
//! The one exception is Ruff lint fix: `check --fix` exits 1 when
//! unfixable findings remain *after* applying the fixable ones, so the
//! backend re-reads the bytes on exit 0 or 1 and keeps its input only on
//! any other exit. Spawn, materialization, and re-read failures still
//! fail the action.
//! Clippy has no fix command: the backend applies `MachineApplicable`
//! suggestions in memory and re-checks the patched bytes on the next
//! round, so only suggestions that truly resolve their finding mark it
//! fixable. Vale, the Markdown checker, rustc typecheck, Ty, pydoclint,
//! flake8, and pylint are check-only and never rewrite.

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

/// Real tool IDs for the M04 initial adapters plus the M12 rustc
/// typecheck adapter and the M15 Python adapters (Ruff, Ty, pydoclint,
/// flake8, pylint).
/// Mirrors `REAL_ADAPTERS`
/// in `//quality:adapters.bzl`; the Starlark registry stays authoritative
/// for pipeline construction, this list pins the dispatch the backend
/// implements.
pub const REAL_TOOLS: &[&str] = &[
    "buildifier",
    "clippy",
    "flake8",
    "markdown_check",
    "pydoclint",
    "pylint",
    "ruff",
    "rustc",
    "rustfmt",
    "taplo",
    "ty",
    "vale",
];

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

/// One staged file: workspace path plus its scratch-absolute path.
type StagedPair = (String, PathBuf);

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

/// Selects the working directory for tools with no config flag that
/// discover native config upward from the working directory (Buildifier,
/// Clippy): the mirrored config's parent when hinted, so discovery finds
/// exactly the hint.
fn hint_dir<'a>(config_rel: Option<&'a str>, cwd_rel: &'a str) -> Option<&'a str> {
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
    /// workspace path in sorted order. Siblings mirror alongside the
    /// sources so link-resolution siblings exist on disk, but they stay
    /// out of `pairs`: they are never linted and findings can never
    /// address them.
    fn stage_scratch(
        &self,
        tool_id: &str,
        tool: &RealTool,
        files: &BTreeMap<String, String>,
        siblings: &BTreeMap<String, String>,
    ) -> Result<(Scratch, Vec<StagedPair>, Vec<StagedPair>), RunnerError> {
        let scratch = fresh_scratch(&self.scratch_parent, tool_id)?;
        let mut mirrors =
            Vec::with_capacity(files.len() + siblings.len() + tool.tool_files.len() + 1);
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
        let mut sibling_pairs = Vec::with_capacity(siblings.len());
        for (path, text) in siblings {
            let absolute = scratch
                .resolve(Path::new(path))
                .map_err(|err| execution(tool_id, format!("scratch sibling: {err}")))?;
            mirrors.push(MirrorFile {
                mirror_rel: PathBuf::from(path),
                contents: MirrorContents::Bytes(text.as_bytes().to_vec()),
            });
            sibling_pairs.push((path.clone(), absolute));
        }
        mirrors.extend(Self::mirror_tool_files(tool_id, tool));
        write_all(&scratch, tool_id, &mirrors)?;
        Ok((scratch, pairs, sibling_pairs))
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

    /// Scratch working directory for check commands: Buildifier, Clippy,
    /// and Vale discover native config upward from the working directory,
    /// so with a hint the command runs from the mirrored config
    /// directory; every other tool runs from the scratch root.
    fn cwd_rel(tool_id: &str, config_rel: Option<&str>) -> String {
        match tool_id {
            "buildifier" | "clippy" | "vale" => config_rel.map(parent_rel).unwrap_or_default(),
            _ => String::new(),
        }
    }

    /// Runs one check over the staged files and returns the parsed
    /// findings still addressed by absolute scratch path. Sibling pairs
    /// reach only the Markdown checker as `--sibling` mappings; every
    /// other tool ignores them.
    fn run_check(
        &self,
        tool_id: &str,
        tool: &RealTool,
        capability: &str,
        pairs: &[(String, PathBuf)],
        sibling_pairs: &[(String, PathBuf)],
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
                    hint_dir(tool.config_rel.as_deref(), &cwd_rel),
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
                        hint_dir(tool.config_rel.as_deref(), &cwd_rel),
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
            "rustc" => {
                let out_dir = scratch.root().join("dx-rustc-out");
                std::fs::create_dir_all(&out_dir)
                    .map_err(|err| execution(tool_id, format!("out dir: {err}")))?;
                let mut findings = Vec::new();
                for (workspace, absolute) in pairs {
                    let stem = Path::new(workspace)
                        .file_stem()
                        .map(|stem| stem.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let invocation = commands::rustc_check(
                        &tool.binary,
                        absolute,
                        &commands::crate_name_for(&stem),
                        &out_dir,
                    );
                    let out = self.run(tool_id, tool, &invocation, scratch)?;
                    let name = absolute.to_string_lossy().into_owned();
                    findings.extend(parsed(
                        tool_id,
                        parsers::parse_rustc(&out.stderr, out.code, &[&name]),
                    )?);
                }
                Ok(findings)
            }
            "markdown_check" => {
                let specs: Vec<(&str, &Path)> = pairs
                    .iter()
                    .map(|(workspace, absolute)| (workspace.as_str(), absolute.as_path()))
                    .collect();
                let sibling_specs: Vec<(&str, &Path)> = sibling_pairs
                    .iter()
                    .map(|(workspace, absolute)| (workspace.as_str(), absolute.as_path()))
                    .collect();
                let invocation = commands::markdown_check(&tool.binary, &specs, &sibling_specs);
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                let workspaces: Vec<&str> = pairs
                    .iter()
                    .map(|(workspace, _)| workspace.as_str())
                    .collect();
                let reported = parsed(
                    tool_id,
                    parsers::parse_markdown_findings(&out.stdout, out.code, &workspaces),
                )?;
                // The checker keys findings off the `--source` workspace
                // paths; re-root each validated path onto its
                // scratch-absolute path for placement. The lookup is
                // infallible: the parser already rejected unknown paths.
                let mut rerooted = Vec::with_capacity(reported.len());
                for found in reported {
                    let absolute = pairs
                        .iter()
                        .find(|(workspace, _)| *workspace == found.file)
                        .map(|(_, absolute)| absolute.to_string_lossy().into_owned())
                        .expect("parsed path was checked");
                    rerooted.push(FileFinding {
                        file: absolute,
                        finding: found.finding,
                    });
                }
                Ok(rerooted)
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
            "ruff" => {
                let config_ref = config.as_deref();
                let invocation = if capability == "format" {
                    commands::ruff_format_check(&tool.binary, &refs, config_ref)
                } else {
                    commands::ruff_check(&tool.binary, &refs, config_ref)
                };
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                if capability == "format" {
                    parsed(
                        tool_id,
                        parsers::parse_ruff_format(&out.stdout, out.code, &strs),
                    )
                } else {
                    parsed(tool_id, parsers::parse_ruff(&out.stdout, out.code, &strs))
                }
            }
            "ty" => {
                let invocation = commands::ty_check(&tool.binary, &refs);
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                // Ty prints concise paths relative to its working
                // directory even for absolute arguments, so attribute
                // against the workspace-relative mirror paths, then
                // re-anchor each finding to its absolute scratch path:
                // the caller contract stays absolute-addressed.
                let workspaces: Vec<&str> = pairs
                    .iter()
                    .map(|(workspace, _)| workspace.as_str())
                    .collect();
                let mut findings = parsed(
                    tool_id,
                    parsers::parse_ty(&out.stdout, out.code, &workspaces),
                )?;
                for found in &mut findings {
                    let absolute = pairs
                        .iter()
                        .find(|(workspace, _)| *workspace == found.file)
                        .map(|(_, absolute)| absolute.clone())
                        .expect("parsed file was checked");
                    found.file = absolute.to_string_lossy().into_owned();
                }
                Ok(findings)
            }
            "pydoclint" => {
                let invocation = commands::pydoclint_check(&tool.binary, &refs);
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                parsed(
                    tool_id,
                    parsers::parse_pydoclint(&out.stderr, out.code, &strs),
                )
            }
            "flake8" => {
                let invocation = commands::flake8_check(&tool.binary, &refs);
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                parsed(tool_id, parsers::parse_flake8(&out.stdout, out.code, &strs))
            }
            "pylint" => {
                let invocation = commands::pylint_check(&tool.binary, &refs);
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                parsed(tool_id, parsers::parse_pylint(&out.stdout, out.code, &strs))
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
        self.diagnose_with_siblings(tool_id, capability, files, &BTreeMap::new())
    }

    /// Sibling-aware [`Self::diagnose`]: siblings mirror into the scratch
    /// tree for Markdown link resolution but stay out of findings,
    /// snapshots, and fixes.
    pub fn diagnose_with_siblings(
        &self,
        tool_id: &str,
        capability: &str,
        files: &BTreeMap<String, String>,
        siblings: &BTreeMap<String, String>,
    ) -> Result<Vec<Diagnostic>, RunnerError> {
        let tool = self.tool(tool_id)?;
        let (scratch, pairs, sibling_pairs) = self.stage_scratch(tool_id, tool, files, siblings)?;
        let collected =
            self.run_check(tool_id, tool, capability, &pairs, &sibling_pairs, &scratch)?;
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
    /// re-read; Ruff follows the running capability (`check --fix` for
    /// lint, `format` for format) and its lint fix re-reads on exit 0
    /// or 1 (exit 1 signals remaining unfixable findings after the
    /// fixable ones were applied); Clippy applies `MachineApplicable`
    /// suggestions from a fresh check in memory; Vale, the Markdown
    /// checker, rustc typecheck, Ty, pydoclint, flake8, and pylint return
    /// their input.
    pub fn apply_fix(
        &self,
        tool_id: &str,
        path: &str,
        text: &str,
        capability: &str,
    ) -> Result<String, RunnerError> {
        let tool = self.tool(tool_id)?;
        match tool_id {
            "rustfmt" | "buildifier" | "taplo" => self.run_fix(tool_id, tool, path, text),
            "ruff" => self.run_ruff_fix(tool, path, text, capability == "format"),
            "clippy" => self.apply_clippy(tool_id, path, text),
            "vale" | "markdown_check" | "rustc" | "ty" | "pydoclint" | "flake8" | "pylint" => {
                Ok(text.to_owned())
            }
            _ => Err(execution(
                tool_id,
                format!("unsupported real tool: {tool_id}"),
            )),
        }
    }

    /// Stages one fix scratch tree with the exact file bytes plus the
    /// tool files, returning the scratch and the file's absolute path.
    fn fix_scratch(
        &self,
        tool_id: &str,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<(Scratch, PathBuf), RunnerError> {
        let scratch = fresh_scratch(&self.scratch_parent, tool_id)?;
        let mut mirrors = vec![MirrorFile {
            mirror_rel: PathBuf::from(path),
            contents: MirrorContents::Bytes(text.as_bytes().to_vec()),
        }];
        mirrors.extend(Self::mirror_tool_files(tool_id, tool));
        write_all(&scratch, tool_id, &mirrors)?;
        let absolute = scratch.root().join(path);
        Ok((scratch, absolute))
    }

    /// Re-reads a fixed file as UTF-8. Re-read failures fail the action;
    /// non-UTF-8 fix output is a tool-output failure, never silent bytes.
    fn reread_fixed(tool_id: &str, absolute: &Path) -> Result<String, RunnerError> {
        let fixed = std::fs::read(absolute)
            .map_err(|err| execution(tool_id, format!("re-read fixed file: {err}")))?;
        String::from_utf8(fixed).map_err(|err| RunnerError::ToolOutput {
            tool_id: tool_id.to_owned(),
            detail: format!("fixed file is not UTF-8: {err}"),
        })
    }

    fn run_fix(
        &self,
        tool_id: &str,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<String, RunnerError> {
        let (scratch, absolute) = self.fix_scratch(tool_id, tool, path, text)?;
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
                hint_dir(tool.config_rel.as_deref(), &cwd_rel),
            ),
            _ => commands::taplo_format(&tool.binary, &refs, config.as_deref(), false),
        };
        let out = self.run(tool_id, tool, &invocation, &scratch)?;
        if out.code != Some(0) {
            return Ok(text.to_owned());
        }
        Self::reread_fixed(tool_id, &absolute)
    }

    /// Runs one Ruff fix round: `format` for format pipelines,
    /// `check --fix` for everything else. The format fix re-reads only
    /// on exit 0 like every other format tool; the lint fix re-reads on
    /// exit 0 or 1 because exit 1 signals remaining unfixable findings
    /// after the fixable ones were applied. Any other exit keeps the
    /// input: the check diagnostics report the cause.
    fn run_ruff_fix(
        &self,
        tool: &RealTool,
        path: &str,
        text: &str,
        format: bool,
    ) -> Result<String, RunnerError> {
        const TOOL_ID: &str = "ruff";
        let (scratch, absolute) = self.fix_scratch(TOOL_ID, tool, path, text)?;
        let refs = [absolute.as_path()];
        let config = self.config_abs(TOOL_ID, tool, &scratch)?;
        let invocation = if format {
            commands::ruff_format_fix(&tool.binary, &refs, config.as_deref())
        } else {
            commands::ruff_fix(&tool.binary, &refs, config.as_deref())
        };
        let out = self.run(TOOL_ID, tool, &invocation, &scratch)?;
        let keep_input = if format {
            out.code != Some(0)
        } else {
            out.code != Some(0) && out.code != Some(1)
        };
        if keep_input {
            return Ok(text.to_owned());
        }
        Self::reread_fixed(TOOL_ID, &absolute)
    }

    fn apply_clippy(&self, tool_id: &str, path: &str, text: &str) -> Result<String, RunnerError> {
        let tool = self.tool(tool_id)?;
        let mut single = BTreeMap::new();
        single.insert(path.to_owned(), text.to_owned());
        let (scratch, pairs, _) = self.stage_scratch(tool_id, tool, &single, &BTreeMap::new())?;
        let collected = self.run_check(tool_id, tool, "lint", &pairs, &[], &scratch)?;
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
    run_real_pipeline_with_siblings(producer, capability, stages, files, &[], backend)
}

/// Sibling-aware [`run_real_pipeline`]: siblings are unclassified
/// link-resolution bytes for the Markdown checker. They must be UTF-8,
/// must not collide with a checked path or each other, and never enter
/// snapshots, stages, or fixes, so a stage naming a sibling still fails
/// `MissingFile`.
pub fn run_real_pipeline_with_siblings(
    producer: &str,
    capability: &str,
    stages: &[StageSpec],
    files: &[FileInput],
    siblings: &[FileInput],
    backend: &RealBackend,
) -> Result<QualityResult, RunnerError> {
    let (capability_value, initial) =
        validate_request(producer, capability, stages, files, |tool| {
            backend.supports(tool)
        })?;
    let mut sibling_texts = BTreeMap::new();
    for sibling in siblings {
        if initial.contains_key(&sibling.path) || sibling_texts.contains_key(&sibling.path) {
            return Err(RunnerError::DuplicateFile {
                path: sibling.path.clone(),
            });
        }
        let text = std::str::from_utf8(&sibling.bytes).map_err(|_| RunnerError::InvalidUtf8 {
            path: sibling.path.clone(),
        })?;
        sibling_texts.insert(sibling.path.clone(), text.to_owned());
    }
    let mut initial_diagnostics = Vec::new();
    for stage in stages {
        let subset = stage_subset(stage, &initial);
        initial_diagnostics.extend(backend.diagnose_with_siblings(
            &stage.tool_id,
            capability,
            &subset,
            &sibling_texts,
        )?);
    }
    let (terminal, completed_rounds, convergence) = run_convergence(
        &initial,
        stages,
        MAX_COMPLETED_ROUNDS,
        |tool, path, text| backend.apply_fix(tool, path, text, capability),
    )?;
    let mut terminal_diagnostics = Vec::new();
    for stage in stages {
        let subset = stage_subset(stage, &terminal);
        terminal_diagnostics.extend(backend.diagnose_with_siblings(
            &stage.tool_id,
            capability,
            &subset,
            &sibling_texts,
        )?);
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
    const RUFF_F401: &str = r#"[{"cell":null,"code":"F401","end_location":{"column":10,"row":1},"filename":"FILE","fix":{"applicability":"safe","edits":[],"message":"Remove unused import"},"location":{"column":8,"row":1},"message":"`os` imported but unused","name":"unused-import","noqa_row":1,"severity":"error","url":"https://docs.astral.sh/ruff/rules/unused-import"}]"#;
    const RUFF_UNFORMATTED: &str = r#"[{"cell":null,"code":"unformatted","end_location":{"column":3,"row":1},"filename":"FILE","fix":null,"location":{"column":3,"row":1},"message":"File would be reformatted","name":"unformatted","noqa_row":null,"severity":"error","url":null}]"#;

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

    /// Asserts the Ruff hermetic flags shared by every shape, so a
    /// dropped flag fails here instead of silently observing ambient
    /// state.
    fn assert_ruff_hermetic(argv: &[OsString], env: &[(String, String)]) {
        assert_hermetic(env);
        assert!(
            argv.iter().any(|arg| arg == "--no-cache"),
            "ruff never caches"
        );
        assert!(
            argv.iter().any(|arg| arg == "--no-respect-gitignore"),
            "ruff never observes VCS state"
        );
    }

    /// Content-aware Ruff double: lint check reports F401 exactly when
    /// the materialized file imports `os`; `check --fix` strips that
    /// import and exits 1 when `UNFIXABLE` remains (the pinned
    /// partial-fix semantic), else 0; `format --check` reports
    /// unformatted exactly on trailing whitespace; `format` trims it.
    /// Unhinted runs assert `--isolated` (pinned upstream defaults).
    fn roundtrip_ruff(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_ruff_hermetic(argv, env);
        assert!(
            argv.iter().any(|arg| arg == "--isolated"),
            "unhinted ruff pins upstream defaults"
        );
        ruff_behavior(argv)
    }

    /// Hinted Ruff double: asserts the `--config` selection (never
    /// `--isolated`) before delegating to [`ruff_behavior`].
    fn roundtrip_ruff_hinted(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_ruff_hermetic(argv, env);
        assert!(
            argv.iter().any(|arg| arg == "--config"),
            "hinted ruff takes the hint"
        );
        assert!(
            !argv.iter().any(|arg| arg == "--isolated"),
            "hinted ruff never isolates"
        );
        ruff_behavior(argv)
    }

    fn ruff_behavior(argv: &[OsString]) -> io::Result<ChildOutput> {
        let file = last_file(argv);
        let bytes = std::fs::read(&file).expect("checked file is materialized");
        let text = String::from_utf8(bytes).expect("fix bytes stay UTF-8");
        if argv.iter().any(|arg| arg == "format") {
            if argv.iter().any(|arg| arg == "--check") {
                let dirty = text
                    .lines()
                    .any(|line| line.ends_with(' ') || line.ends_with('\t'));
                if dirty {
                    let stdout = RUFF_UNFORMATTED.replace("FILE", &file);
                    return Ok(ChildOutput {
                        code: Some(1),
                        stdout: stdout.into_bytes(),
                        stderr: Vec::new(),
                    });
                }
                return Ok(ChildOutput {
                    code: Some(0),
                    stdout: b"[]".to_vec(),
                    stderr: Vec::new(),
                });
            }
            let mut fixed = Vec::with_capacity(text.len());
            for line in text.split_inclusive('\n') {
                let trailing = line.ends_with('\n');
                let body = if trailing {
                    &line[..line.len() - 1]
                } else {
                    line
                };
                fixed.extend_from_slice(trim_end(body.as_bytes()));
                if trailing {
                    fixed.push(b'\n');
                }
            }
            std::fs::write(&file, fixed).expect("fix writes back");
            return Ok(ChildOutput {
                code: Some(0),
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        }
        if argv.iter().any(|arg| arg == "--fix") {
            let kept: Vec<&str> = text
                .lines()
                .filter(|line| !line.contains("import os"))
                .collect();
            let mut fixed = kept.join("\n");
            if text.ends_with('\n') {
                fixed.push('\n');
            }
            std::fs::write(&file, fixed.clone()).expect("fix writes back");
            return Ok(ChildOutput {
                code: Some(i32::from(fixed.contains("UNFIXABLE"))),
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        }
        if text.contains("import os") {
            let stdout = RUFF_F401.replace("FILE", &file);
            return Ok(ChildOutput {
                code: Some(1),
                stdout: stdout.into_bytes(),
                stderr: Vec::new(),
            });
        }
        Ok(ChildOutput {
            code: Some(0),
            stdout: b"[]".to_vec(),
            stderr: Vec::new(),
        })
    }

    /// Content-aware Ty double: reports invalid-assignment exactly when
    /// the materialized file contains BADTYPE, else `All checks passed!`.
    /// The reported path is working-directory-relative like the real Ty,
    /// which relativizes concise paths against its working directory even
    /// for absolute arguments.
    fn roundtrip_ty(
        argv: &[OsString],
        cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        assert!(
            argv.iter().any(|arg| arg == "--no-respect-ignore-files"),
            "ty never observes VCS state"
        );
        let file = last_file(argv);
        let reported = Path::new(&file)
            .strip_prefix(cwd)
            .map(|relative| relative.to_string_lossy().into_owned())
            .unwrap_or(file.clone());
        let bytes = std::fs::read(&file).expect("checked file is materialized");
        let text = String::from_utf8(bytes).expect("checked bytes stay UTF-8");
        if text.contains("BADTYPE") {
            let stdout = format!(
                "{reported}:1:10: error[invalid-assignment] Object of type `Literal[\"hello\"]` is not assignable to `int`\nFound 1 diagnostic\n"
            );
            return Ok(ChildOutput {
                code: Some(1),
                stdout: stdout.into_bytes(),
                stderr: Vec::new(),
            });
        }
        Ok(ChildOutput {
            code: Some(0),
            stdout: b"All checks passed!\n".to_vec(),
            stderr: Vec::new(),
        })
    }

    /// Content-aware pydoclint double: reports DOC201 exactly when the
    /// materialized file contains NODOC, on stderr under a path header.
    fn roundtrip_pydoclint(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        assert!(
            argv.iter().any(|arg| arg == "--quiet"),
            "pydoclint stays quiet"
        );
        let file = last_file(argv);
        let bytes = std::fs::read(&file).expect("checked file is materialized");
        let text = String::from_utf8(bytes).expect("checked bytes stay UTF-8");
        if text.contains("NODOC") {
            let stderr =
                format!("{file}\n    2: DOC201: Function `foo` does not have a return section in docstring\n");
            return Ok(ChildOutput {
                code: Some(1),
                stdout: Vec::new(),
                stderr: stderr.into_bytes(),
            });
        }
        Ok(ChildOutput {
            code: Some(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    /// Content-aware flake8 double: reports F401 exactly when the
    /// materialized file imports `os`, on stdout as
    /// `path:row:col:code:message`. Asserts the hermetic flags
    /// (`--isolated` blocks config discovery, `--jobs=1` keeps output
    /// order deterministic, `--color=never` blocks ANSI).
    fn roundtrip_flake8(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        assert!(
            argv.iter().any(|arg| arg == "--isolated"),
            "flake8 pins upstream defaults"
        );
        assert!(
            argv.iter().any(|arg| arg == "--jobs=1"),
            "flake8 keeps output order deterministic"
        );
        assert!(
            argv.iter().any(|arg| arg == "--color=never"),
            "flake8 never emits color"
        );
        let file = last_file(argv);
        let bytes = std::fs::read(&file).expect("checked file is materialized");
        let text = String::from_utf8(bytes).expect("checked bytes stay UTF-8");
        if text.contains("import os") {
            let stdout = format!("{file}:1:1:F401:'os' imported but unused\n");
            return Ok(ChildOutput {
                code: Some(1),
                stdout: stdout.into_bytes(),
                stderr: Vec::new(),
            });
        }
        Ok(ChildOutput {
            code: Some(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    /// Content-aware pylint double: reports W0611 exactly when the
    /// materialized file imports `os`, on stdout as the pinned JSON
    /// array with 0-based columns. Asserts the hermetic flags
    /// (`--persistent=n` disables the cache, `--reports=n`/`--score=n`
    /// suppress the human report).
    fn roundtrip_pylint(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        assert!(
            argv.iter().any(|arg| arg == "--persistent=n"),
            "pylint never caches"
        );
        assert!(
            argv.iter().any(|arg| arg == "--reports=n"),
            "pylint suppresses the report"
        );
        assert!(
            argv.iter().any(|arg| arg == "--score=n"),
            "pylint suppresses the score"
        );
        assert!(
            argv.iter().any(|arg| arg == "--output-format=json"),
            "pylint reports JSON"
        );
        let file = last_file(argv);
        let bytes = std::fs::read(&file).expect("checked file is materialized");
        let text = String::from_utf8(bytes).expect("checked bytes stay UTF-8");
        if text.contains("import os") {
            let stdout = format!(
                "[{{\"type\": \"warning\", \"module\": \"a\", \"obj\": \"\", \"line\": 1, \"column\": 0, \"endLine\": 1, \"endColumn\": 9, \"path\": \"{file}\", \"symbol\": \"unused-import\", \"message\": \"Unused import os\", \"message-id\": \"W0611\"}}]"
            );
            return Ok(ChildOutput {
                code: Some(4),
                stdout: stdout.into_bytes(),
                stderr: Vec::new(),
            });
        }
        Ok(ChildOutput {
            code: Some(0),
            stdout: b"[]".to_vec(),
            stderr: Vec::new(),
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

    /// Repo-owned Markdown checker double: reports one
    /// `missing-file-target` finding per `--source` workspace path whose
    /// materialized bytes contain the `BROKEN` marker, keyed by workspace
    /// path exactly like the real binary.
    fn markdown_links(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let mut out = String::new();
        // From argv[0]: the binary path takes the non-`--source` branch,
        // like any non-mapping argument would.
        let mut index = 0;
        while index < argv.len() {
            if argv[index] == "--source" {
                let mapping = argv[index + 1].to_string_lossy().into_owned();
                let (workspace, absolute) = mapping.split_once('=').expect("--source maps WS=ABS");
                let bytes = std::fs::read(absolute).expect("checked file is materialized");
                let text = String::from_utf8(bytes).expect("checked bytes are UTF-8");
                if text.contains("BROKEN") {
                    out.push_str(&format!(
                        "{{\"path\":\"{workspace}\",\"line\":2,\"kind\":\"missing-file-target\",\"message\":\"link target nope.md does not match a checked source\"}}\n"
                    ));
                }
                index += 1;
            }
            index += 1;
        }
        Ok(ChildOutput {
            code: Some(0),
            stdout: out.into_bytes(),
            stderr: Vec::new(),
        })
    }

    fn markdown_broken(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let _ = last_file(argv);
        Ok(ChildOutput {
            code: Some(2),
            stdout: Vec::new(),
            stderr: b"markdown_check: bad usage".to_vec(),
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

    fn clippy_hinted(
        argv: &[OsString],
        cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        assert!(
            cwd.ends_with("cfg"),
            "hinted clippy runs from the config dir"
        );
        clippy_len_zero(argv, cwd, env)
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

    const RUSTC_TYPE_ERROR: &str = r#"{"$message_type":"diagnostic","message":"mismatched types","code":{"code":"E0308","explanation":null},"level":"error","spans":[{"file_name":"FILE","byte_start":27,"byte_end":32,"line_start":2,"line_end":2,"column_start":9,"column_end":14,"is_primary":true,"text":[],"label":"expected `i32`, found `&str`","suggested_replacement":null,"suggestion_applicability":null,"expansion":null}],"children":[],"rendered":null}
{"$message_type":"diagnostic","message":"aborting due to 1 previous error","code":null,"level":"error","spans":[],"children":[],"rendered":null}"#;

    fn rustc_type_error(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let out_dir = argv
            .windows(2)
            .find(|pair| pair[0] == "--out-dir")
            .map(|pair| pair[1].clone())
            .expect("rustc passes --out-dir");
        assert!(
            Path::new(&out_dir).is_dir(),
            "rustc out dir is materialized"
        );
        assert!(
            argv.iter().any(|arg| arg == "--crate-type=lib"),
            "rustc typechecks as a lib root"
        );
        let stderr = RUSTC_TYPE_ERROR.replace("FILE", &last_file(argv));
        Ok(ChildOutput {
            code: Some(1),
            stdout: Vec::new(),
            stderr: stderr.into_bytes(),
        })
    }

    fn rustc_garbage(
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
    fn real_tools_pin_the_dispatched_set() {
        assert_eq!(
            REAL_TOOLS,
            &[
                "buildifier",
                "clippy",
                "flake8",
                "markdown_check",
                "pydoclint",
                "pylint",
                "ruff",
                "rustc",
                "rustfmt",
                "taplo",
                "ty",
                "vale"
            ]
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
    fn markdown_check_reports_workspace_keyed_findings() {
        let backend = backend_for("markdown_check", plain_tool(), markdown_links);
        let mut inputs = BTreeMap::new();
        inputs.insert("doc/guide.md".to_owned(), "# Guide\n\nBROKEN\n".to_owned());
        inputs.insert("README.md".to_owned(), "# Readme\n".to_owned());
        let findings = backend
            .diagnose("markdown_check", "lint", &inputs)
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tool_id, "markdown_check");
        assert_eq!(findings[0].rule_id, "missing-file-target");
        assert_eq!(findings[0].path, "doc/guide.md");
        // Line 2, column 1 places at the second line's first byte.
        assert_eq!(findings[0].start_byte, Some(8));
        assert_eq!(findings[0].end_byte, Some(8));
        assert!(!findings[0].fixable);
    }

    #[test]
    fn markdown_check_pipeline_is_stable_without_rewriting() {
        let backend = backend_for("markdown_check", plain_tool(), markdown_links);
        let stages = vec![stage("markdown_check", &["markdown"], &["doc/guide.md"])];
        let files = vec![file("doc/guide.md", "# Guide\n\nBROKEN\n")];
        let result = run_real_pipeline("//quality:test", "lint", &stages, &files, &backend)
            .expect("real pipeline");
        assert_eq!(result.convergence, Convergence::Stable as i32);
        assert_eq!(result.completed_rounds, 1);
        assert_eq!(result.initial_diagnostics.len(), 1);
        assert_eq!(result.initial_diagnostics[0].tool_id, "markdown_check");
        assert_eq!(result.terminal_diagnostics.len(), 1);
        assert!(result.replacements.is_empty());
        assert!(encode_validated(&result).is_ok());
    }

    /// Sibling-aware markdown double: a BROKEN link resolves exactly when
    /// at least one `--sibling` mapping reaches the invocation, proving
    /// the backend threads siblings through to the checker.
    fn markdown_sibling_links(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let mut out = String::new();
        // Sibling mappings trail the source mappings in argv, so
        // pre-scan for their presence before judging any source.
        let seen_sibling = argv.iter().any(|arg| arg == "--sibling");
        let mut index = 0;
        while index < argv.len() {
            if argv[index] == "--source" {
                let mapping = argv[index + 1].to_string_lossy().into_owned();
                let (workspace, absolute) = mapping.split_once('=').expect("--source maps WS=ABS");
                let bytes = std::fs::read(absolute).expect("checked file is materialized");
                let text = String::from_utf8(bytes).expect("checked bytes are UTF-8");
                if text.contains("BROKEN") && !seen_sibling {
                    out.push_str(&format!(
                        "{{\"path\":\"{workspace}\",\"line\":2,\"kind\":\"missing-file-target\",\"message\":\"link target nope.md does not match a checked source\"}}\n"
                    ));
                }
                index += 1;
            }
            index += 1;
        }
        Ok(ChildOutput {
            code: Some(0),
            stdout: out.into_bytes(),
            stderr: Vec::new(),
        })
    }

    #[test]
    fn markdown_siblings_reach_the_checker_invocation() {
        let backend = backend_for("markdown_check", plain_tool(), markdown_sibling_links);
        let mut siblings = BTreeMap::new();
        siblings.insert("LICENSE".to_owned(), "license text\n".to_owned());
        let without = backend
            .diagnose_with_siblings(
                "markdown_check",
                "lint",
                &single("doc/guide.md", "# Guide\n\nBROKEN\n"),
                &BTreeMap::new(),
            )
            .expect("diagnosed");
        assert_eq!(without.len(), 1);
        let with = backend
            .diagnose_with_siblings(
                "markdown_check",
                "lint",
                &single("doc/guide.md", "# Guide\n\nBROKEN\n"),
                &siblings,
            )
            .expect("diagnosed");
        assert!(with.is_empty());
    }

    #[test]
    fn markdown_siblings_stay_out_of_snapshots_and_stages() {
        let backend = backend_for("markdown_check", plain_tool(), markdown_links);
        let stages = vec![stage("markdown_check", &["markdown"], &["doc/guide.md"])];
        let files = vec![file("doc/guide.md", "# Guide\n")];
        let siblings = vec![file("LICENSE", "license text\n")];
        let result = run_real_pipeline_with_siblings(
            "//quality:test",
            "lint",
            &stages,
            &files,
            &siblings,
            &backend,
        )
        .expect("real pipeline");
        assert!(result.initial_diagnostics.is_empty());
        assert!(result.terminal_diagnostics.is_empty());
        assert_eq!(result.original_snapshot.len(), 1);
        assert_eq!(result.original_snapshot[0].path, "doc/guide.md");
        assert_eq!(result.terminal_snapshot.len(), 1);
        assert!(result.replacements.is_empty());
        assert!(encode_validated(&result).is_ok());
    }

    #[test]
    fn sibling_colliding_with_a_source_fails() {
        let backend = backend_for("markdown_check", plain_tool(), markdown_links);
        let stages = vec![stage("markdown_check", &["markdown"], &["doc/guide.md"])];
        let files = vec![file("doc/guide.md", "# Guide\n")];
        let siblings = vec![file("doc/guide.md", "other\n")];
        assert_eq!(
            run_real_pipeline_with_siblings(
                "//quality:test",
                "lint",
                &stages,
                &files,
                &siblings,
                &backend
            ),
            Err(RunnerError::DuplicateFile {
                path: "doc/guide.md".to_owned(),
            })
        );
    }

    #[test]
    fn sibling_non_utf8_fails() {
        let backend = backend_for("markdown_check", plain_tool(), markdown_links);
        let stages = vec![stage("markdown_check", &["markdown"], &["doc/guide.md"])];
        let files = vec![file("doc/guide.md", "# Guide\n")];
        let siblings = vec![FileInput {
            path: "LICENSE".to_owned(),
            bytes: vec![0xff],
        }];
        assert_eq!(
            run_real_pipeline_with_siblings(
                "//quality:test",
                "lint",
                &stages,
                &files,
                &siblings,
                &backend
            ),
            Err(RunnerError::InvalidUtf8 {
                path: "LICENSE".to_owned(),
            })
        );
    }

    #[test]
    fn stage_naming_a_sibling_fails_missing_file() {
        let backend = backend_for("markdown_check", plain_tool(), markdown_links);
        let stages = vec![stage(
            "markdown_check",
            &["markdown"],
            &["doc/guide.md", "LICENSE"],
        )];
        let files = vec![file("doc/guide.md", "# Guide\n")];
        let siblings = vec![file("LICENSE", "license text\n")];
        assert_eq!(
            run_real_pipeline_with_siblings(
                "//quality:test",
                "lint",
                &stages,
                &files,
                &siblings,
                &backend
            ),
            Err(RunnerError::MissingFile {
                path: "LICENSE".to_owned(),
            })
        );
    }

    #[test]
    fn markdown_check_failure_fails_the_action() {
        let backend = backend_for("markdown_check", plain_tool(), markdown_broken);
        let err = backend
            .diagnose(
                "markdown_check",
                "lint",
                &single("doc/guide.md", "# Guide\n"),
            )
            .expect_err("exit 2 fails");
        assert!(matches!(err, RunnerError::ToolOutput { .. }));
        assert!(err.to_string().contains("findings exist only on exit 0"));
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
    fn clippy_hint_runs_from_the_config_dir() {
        let tool = RealTool {
            config_rel: Some("cfg/clippy.toml".to_owned()),
            tool_files: vec![("cfg/clippy.toml".to_owned(), b"".to_vec())],
            ..plain_tool()
        };
        let backend = backend_for("clippy", tool, clippy_hinted);
        let findings = backend
            .diagnose(
                "clippy",
                "lint",
                &single("src/main.rs", "let y = v.len() == 0;\n"),
            )
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "clippy::len_zero");
    }

    #[test]
    fn clippy_apply_uses_machine_applicable_suggestions() {
        let backend = backend_for("clippy", plain_tool(), clippy_len_zero);
        let patched = backend
            .apply_fix("clippy", "src/main.rs", "let y = v.len() == 0;\n", "lint")
            .expect("patched");
        assert_eq!(patched, "let y = !v.is_empty();\n");
    }

    #[test]
    fn clippy_apply_keeps_bytes_without_applicable_suggestions() {
        let backend = backend_for("clippy", plain_tool(), clippy_far);
        let patched = backend
            .apply_fix("clippy", "src/main.rs", "let y = v.len() == 0;\n", "lint")
            .expect("unchanged");
        assert_eq!(patched, "let y = v.len() == 0;\n");
    }

    #[test]
    fn rustc_reports_type_errors_as_lib_root() {
        let backend = backend_for("rustc", plain_tool(), rustc_type_error);
        let text = "fn f(x: i32) {}\nfn g() { f(\"oops\"); }\n";
        let findings = backend
            .diagnose("rustc", "typecheck", &single("src/main.rs", text))
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tool_id, "rustc");
        assert_eq!(findings[0].rule_id, "E0308");
        assert_eq!(
            (findings[0].start_byte, findings[0].end_byte),
            (Some(24), Some(29))
        );
    }

    #[test]
    fn rustc_apply_is_check_only() {
        let backend = backend_for("rustc", plain_tool(), rustc_type_error);
        let text = "fn f(x: i32) {}\nfn g() { f(\"oops\"); }\n";
        let patched = backend
            .apply_fix("rustc", "src/main.rs", text, "typecheck")
            .expect("unchanged");
        assert_eq!(patched, text);
    }

    #[test]
    fn rustc_diagnose_failure_aborts_apply() {
        let backend = backend_for("rustc", plain_tool(), rustc_garbage);
        backend
            .diagnose("rustc", "typecheck", &single("src/main.rs", "x\n"))
            .expect_err("rustc garbage fails");
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
            backend.apply_fix("nope", "src/main.rs", "x\n", "lint"),
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
            .apply_fix("custom", "src/main.rs", "x\n", "lint")
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
            .apply_fix("clippy", "src/main.rs", "x\n", "lint")
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
            .apply_fix("rustfmt", "../evil.rs", "x\n", "format")
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
            .apply_fix("rustfmt", "src/main.rs", "x\n", "format")
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
            .apply_fix("rustfmt", "src/main.rs", "x  \n", "format")
            .expect("kept");
        assert_eq!(kept, "x  \n");
    }

    #[test]
    fn fix_without_output_file_fails_the_action() {
        let backend = backend_for("rustfmt", plain_tool(), deleting_fix);
        let err = backend
            .apply_fix("rustfmt", "src/main.rs", "x\n", "format")
            .expect_err("re-read fails");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
        assert!(err.to_string().contains("re-read"));
    }

    #[test]
    fn non_utf8_fix_fails_the_action() {
        let backend = backend_for("rustfmt", plain_tool(), binary_fix);
        let err = backend
            .apply_fix("rustfmt", "src/main.rs", "x\n", "format")
            .expect_err("encoding fails");
        assert!(matches!(err, RunnerError::ToolOutput { .. }));
    }

    #[test]
    fn clippy_diagnose_failure_aborts_apply() {
        let backend = backend_for("clippy", plain_tool(), clippy_garbage);
        let err = backend
            .apply_fix("clippy", "src/main.rs", "x\n", "lint")
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

    /// Check double that poisons the file on fix: the initial check
    /// succeeds, convergence applies the poison, and the terminal check
    /// fails on the poisoned bytes.
    fn check_ok_fix_poisons(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        let file = last_file(argv);
        if argv.iter().any(|arg| arg == "--check") {
            let bytes = std::fs::read(&file).expect("checked file is materialized");
            if bytes == b"POISON\n" {
                return Err(io::Error::new(io::ErrorKind::Other, "poisoned terminal"));
            }
            return Ok(ChildOutput {
                code: Some(0),
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        }
        std::fs::write(&file, b"POISON\n").expect("fix writes back");
        Ok(ChildOutput {
            code: Some(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
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
            .apply_fix("buildifier", "a.bzl", "x = 1\n", "format")
            .expect("fixed");
        assert_eq!(fixed, "fixed\n");
    }

    #[test]
    fn taplo_fix_rewrites_the_file() {
        let backend = backend_for("taplo", plain_tool(), taplo_fix_ok);
        let fixed = backend
            .apply_fix("taplo", "a.toml", "a=1\n", "lint")
            .expect("fixed");
        assert_eq!(fixed, "a = 1\n");
    }

    #[test]
    fn ruff_lint_reports_and_fix_rereads_on_exit_one() {
        let backend = backend_for("ruff", plain_tool(), roundtrip_ruff);
        let findings = backend
            .diagnose("ruff", "lint", &single("a.py", "import os\n# UNFIXABLE\n"))
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tool_id, "ruff");
        assert_eq!(findings[0].rule_id, "F401");
        assert_eq!(
            findings[0].severity,
            quality_result::proto::Severity::Error as i32
        );
        // `check --fix` exits 1 with the unfixable marker remaining, but
        // the applied import strip is kept, not discarded.
        let fixed = backend
            .apply_fix("ruff", "a.py", "import os\n# UNFIXABLE\n", "lint")
            .expect("fixed");
        assert_eq!(fixed, "# UNFIXABLE\n");
        assert!(backend
            .diagnose("ruff", "lint", &single("a.py", "x = 1\n"))
            .expect("diagnosed")
            .is_empty());
    }

    #[test]
    fn ruff_format_reports_and_rewrites() {
        let backend = backend_for("ruff", plain_tool(), roundtrip_ruff);
        let findings = backend
            .diagnose("ruff", "format", &single("a.py", "x = 1  \n"))
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "unformatted");
        let fixed = backend
            .apply_fix("ruff", "a.py", "x = 1  \n", "format")
            .expect("fixed");
        assert_eq!(fixed, "x = 1\n");
    }

    #[test]
    fn ruff_fix_follows_the_running_capability() {
        let backend = backend_for("ruff", plain_tool(), roundtrip_ruff);
        // A lint fix strips the unused import but never reformats:
        // trailing whitespace outside the stripped line survives.
        let fixed = backend
            .apply_fix("ruff", "a.py", "import os\nx = 1  \n", "lint")
            .expect("fixed");
        assert_eq!(fixed, "x = 1  \n");
        // A format fix trims trailing whitespace but never strips imports.
        let fixed = backend
            .apply_fix("ruff", "a.py", "import os\nx = 1  \n", "format")
            .expect("fixed");
        assert_eq!(fixed, "import os\nx = 1\n");
    }

    #[test]
    fn ruff_hinted_config_reaches_the_tool() {
        let tool = RealTool {
            config_rel: Some("ruff.toml".to_owned()),
            tool_files: vec![("ruff.toml".to_owned(), b"".to_vec())],
            ..plain_tool()
        };
        let backend = backend_for("ruff", tool, roundtrip_ruff_hinted);
        let findings = backend
            .diagnose("ruff", "lint", &single("a.py", "import os\n"))
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "F401");
    }

    #[test]
    fn ty_reports_and_is_check_only() {
        let backend = backend_for("ty", plain_tool(), roundtrip_ty);
        let findings = backend
            .diagnose("ty", "typecheck", &single("a.py", "x: int = BADTYPE\n"))
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tool_id, "ty");
        assert_eq!(findings[0].rule_id, "invalid-assignment");
        assert_eq!(
            findings[0].severity,
            quality_result::proto::Severity::Error as i32
        );
        assert!(backend
            .diagnose("ty", "typecheck", &single("a.py", "x: int = 1\n"))
            .expect("diagnosed")
            .is_empty());
        let text = "x: int = BADTYPE\n";
        assert_eq!(
            backend
                .apply_fix("ty", "a.py", text, "typecheck")
                .expect("check-only"),
            text
        );
    }

    #[test]
    fn pydoclint_reports_and_is_check_only() {
        let backend = backend_for("pydoclint", plain_tool(), roundtrip_pydoclint);
        let findings = backend
            .diagnose(
                "pydoclint",
                "lint",
                &single("a.py", "def foo():\n    NODOC\n"),
            )
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tool_id, "pydoclint");
        assert_eq!(findings[0].rule_id, "DOC201");
        assert_eq!(findings[0].path, "a.py");
        assert!(backend
            .diagnose(
                "pydoclint",
                "lint",
                &single("a.py", "\"\"\"Module.\"\"\"\n")
            )
            .expect("diagnosed")
            .is_empty());
        let text = "def foo():\n    NODOC\n";
        assert_eq!(
            backend
                .apply_fix("pydoclint", "a.py", text, "lint")
                .expect("check-only"),
            text
        );
    }

    #[test]
    fn flake8_reports_and_is_check_only() {
        let backend = backend_for("flake8", plain_tool(), roundtrip_flake8);
        let findings = backend
            .diagnose("flake8", "lint", &single("a.py", "import os\n"))
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tool_id, "flake8");
        assert_eq!(findings[0].rule_id, "F401");
        assert_eq!(findings[0].path, "a.py");
        assert_eq!(
            findings[0].severity,
            quality_result::proto::Severity::Error as i32
        );
        assert!(backend
            .diagnose("flake8", "lint", &single("a.py", "\"\"\"Module.\"\"\"\n"))
            .expect("diagnosed")
            .is_empty());
        let text = "import os\n";
        assert_eq!(
            backend
                .apply_fix("flake8", "a.py", text, "lint")
                .expect("check-only"),
            text
        );
    }

    #[test]
    fn pylint_reports_and_is_check_only() {
        let backend = backend_for("pylint", plain_tool(), roundtrip_pylint);
        let findings = backend
            .diagnose("pylint", "lint", &single("a.py", "import os\n"))
            .expect("diagnosed");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tool_id, "pylint");
        assert_eq!(findings[0].rule_id, "W0611");
        assert_eq!(findings[0].path, "a.py");
        assert_eq!(
            findings[0].severity,
            quality_result::proto::Severity::Warning as i32
        );
        // 0-based columns 0..9 place as the 1-based range 1..10.
        assert_eq!(
            (findings[0].start_byte, findings[0].end_byte),
            (Some(0), Some(9))
        );
        assert!(backend
            .diagnose("pylint", "lint", &single("a.py", "\"\"\"Module.\"\"\"\n"))
            .expect("diagnosed")
            .is_empty());
        let text = "import os\n";
        assert_eq!(
            backend
                .apply_fix("pylint", "a.py", text, "lint")
                .expect("check-only"),
            text
        );
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

    #[test]
    fn check_spawn_failure_aborts_initial_diagnose() {
        let backend = backend_for("rustfmt", plain_tool(), missing_spawn);
        let stages = vec![stage("rustfmt", &["rust"], &["src/main.rs"])];
        let files = vec![file("src/main.rs", "x\n")];
        let err = run_real_pipeline("//quality:test", "format", &stages, &files, &backend)
            .expect_err("check spawn fails");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
        assert!(err.to_string().contains("spawn"));
    }

    #[test]
    fn terminal_check_failure_aborts_the_pipeline() {
        let backend = backend_for("rustfmt", plain_tool(), check_ok_fix_poisons);
        let stages = vec![stage("rustfmt", &["rust"], &["src/main.rs"])];
        let files = vec![file("src/main.rs", "x\n")];
        let err = run_real_pipeline("//quality:test", "format", &stages, &files, &backend)
            .expect_err("terminal check fails");
        assert!(matches!(err, RunnerError::ToolExecution { .. }));
        assert!(err.to_string().contains("spawn"));
    }

    fn ruff_fix_crashes(
        argv: &[OsString],
        cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        if argv.iter().any(|arg| arg == "--fix") {
            assert_ruff_hermetic(argv, env);
            return Ok(ChildOutput {
                code: Some(2),
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        }
        roundtrip_ruff(argv, cwd, env)
    }

    fn ty_garbage(
        argv: &[OsString],
        _cwd: &Path,
        env: &[(String, String)],
    ) -> io::Result<ChildOutput> {
        assert_hermetic(env);
        assert!(
            argv.iter().any(|arg| arg == "--no-respect-ignore-files"),
            "ty never observes VCS state"
        );
        Ok(ChildOutput {
            code: Some(1),
            stdout: b"garbage\n".to_vec(),
            stderr: Vec::new(),
        })
    }

    #[test]
    fn ruff_format_clean_and_unterminated_fix() {
        let backend = backend_for("ruff", plain_tool(), roundtrip_ruff);
        // Clean format check reports no findings (the format-check
        // clean branch).
        assert!(backend
            .diagnose("ruff", "format", &single("a.py", "x = 1\n"))
            .expect("diagnosed")
            .is_empty());
        // A format fix without a trailing newline trims the last line
        // without appending one.
        let fixed = backend
            .apply_fix("ruff", "a.py", "x = 1  ", "format")
            .expect("fixed");
        assert_eq!(fixed, "x = 1");
    }

    #[test]
    fn ruff_fix_failure_keeps_input() {
        let backend = backend_for("ruff", plain_tool(), ruff_fix_crashes);
        let text = "import os\n";
        assert_eq!(
            backend
                .apply_fix("ruff", "a.py", text, "lint")
                .expect("kept"),
            text
        );
        assert_eq!(
            backend
                .apply_fix("ruff", "a.py", text, "format")
                .expect("kept"),
            text
        );
    }

    #[test]
    fn ty_output_failure_aborts_diagnose() {
        let backend = backend_for("ty", plain_tool(), ty_garbage);
        let err = backend
            .diagnose("ty", "typecheck", &single("a.py", "x: int = 1\n"))
            .expect_err("parse fails");
        assert!(matches!(err, RunnerError::ToolOutput { .. }));
    }
}
