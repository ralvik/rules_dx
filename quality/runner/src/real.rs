//! Real-tool pipeline backend (WP2).
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
//! Capability mapping: each tool owns its capability slice outright
//! except Buildifier, whose single check reports both format findings
//! (empty rule: unformatted or syntax) and lint warnings (the rule
//! carries the category). The backend keeps only the findings matching
//! the running capability, so lint results never carry format findings
//! while format fixes converge them away. Taplo selects its mode by
//! command instead: `lint` for lint pipelines, `format --check` for
//! format pipelines. Ruff likewise: `check` for lint pipelines,
//! `format --check` for format pipelines, and its fix mode follows the
//! same capability split (`check --fix` versus `format`). Biome
//! likewise: `lint` for lint pipelines, `format` check for format
//! pipelines, with format fix via `format --write`; Biome lint is
//! check-only and converges on format. ESLint is lint-only with
//! `--fix` re-read on exit 0 or 1; Prettier is format-only with
//! `--write` re-read on exit 0.
//!
//! Fix application is best-effort per file: a nonzero fix exit leaves
//! the bytes unchanged and the check diagnostics report the cause, so
//! syntax-broken files surface findings instead of failing the action.
//! The exceptions are Ruff lint fix and ESLint fix: they exit 1 when
//! unfixable findings remain *after* applying the fixable ones, so the
//! backend re-reads the bytes on exit 0 or 1 and keeps its input only on
//! any other exit. Spawn, materialization, and re-read failures still
//! fail the action.
//! Clippy has no fix command and is check-only: its suggestions
//! ride the frozen authoritative upstream diagnostics and never
//! rewrite. Vale, the Markdown checker, rustc typecheck, Ty,
//! pydoclint, flake8, pylint, and Biome lint are check-only and never
//! rewrite.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};

use quality_adapter::commands::{self, Invocation};
use quality_adapter::exec::{self, ChildOutput, MirrorContents, MirrorFile, Scratch};
use quality_adapter::parsers::{self, FileFinding, ParseError};
use quality_adapter::place_finding;

use crate::{
    assemble, run_convergence, stage_subset, validate_request, FileInput, QualityResult,
    RunnerError, StageSpec, MAX_COMPLETED_ROUNDS,
};
use quality_result::proto::Diagnostic;

/// Real tool IDs for the initial adapters plus the rustc
/// typecheck adapter, the Python adapters (Ruff, Ty, pydoclint,
/// flake8, pylint), the JavaScript/TypeScript/JSON adapters
/// (Biome, ESLint, Prettier; target-coupled tsc stays pipeline-only and
/// never runs as a bare backend invocation), and the Scala/.NET cohort
/// (Scalafmt format, Scalafix lint via callback, CSharpier format,
/// Fantomas format, Roslyn lint via delegated SARIF, FSharpLint lint
/// via library API).
/// Mirrors `REAL_ADAPTERS`
/// in `//quality:adapters.bzl`; the Starlark registry stays authoritative
/// for pipeline construction, this list pins the dispatch the backend
/// implements.
///
/// See: `docs/quality/tool-integrations.md#initial-adapter-qualification`
pub const REAL_TOOLS: &[&str] = &[
    "biome",
    "buildifier",
    "clippy",
    "csharpier",
    "eslint",
    "fantomas",
    "flake8",
    "fsharplint",
    "markdown_check",
    "prettier",
    "pydoclint",
    "pylint",
    "roslyn",
    "ruff",
    "rustc",
    "rustfmt",
    "scalafix",
    "scalafmt",
    "taplo",
    "ty",
    "vale",
];

/// Scratch-relative home for the materialized rustfmt defaults: without
/// a hinted config the tool still gets an explicit `--config-path`, so
/// no upward discovery can observe ambient state.
const RUSTFMT_DEFAULTS_REL: &str = "dx-rustfmt-default.toml";

/// Scratch-relative home for the materialized Biome defaults: without a
/// hinted config the tool still gets an explicit `--config-path` dir
/// holding exactly one `biome.json` (`{}`), so no upward discovery can
/// observe ambient state. The directory must never contain linted
/// sources; workspace sources live at their mirror paths while this dir
/// holds only the defaults file.
const BIOME_DEFAULTS_REL: &str = "dx-biome-default/biome.json";
/// Pinned Biome defaults bytes: empty object selects pinned upstream
/// defaults.
const BIOME_DEFAULTS_BYTES: &[u8] = b"{}";

/// One resolved real tool: absolute binary, extra hermetic environment
/// entries, optional mirror-relative config, optional crate edition
/// (rustfmt only: read from `CrateInfo` by the quality aspect, so the
/// CLI `--edition` flag matches what the crate compiles as), and extra
/// mirrored files (hinted configs, Vale styles). The action maps its
/// inputs to the mirror paths through the CLI. Delegated tools (Clippy,
///) carry authoritative upstream diagnostics files instead of a
/// spawned binary: the aspect declares the files as action inputs and
/// maps them here, and the backend parses them without spawning.
pub struct RealTool {
    pub binary: PathBuf,
    pub extra_env: Vec<(String, String)>,
    pub config_rel: Option<String>,
    pub edition: Option<String>,
    pub tool_files: Vec<(String, Vec<u8>)>,
    pub upstream_diagnostics: Vec<PathBuf>,
}

/// Injected tool spawner: absolute argv, scratch working directory,
/// and hermetic environment. Production uses [`real_spawn`].
pub type SpawnFn = fn(&[OsString], &Path, &[(String, String)]) -> io::Result<ChildOutput>;

/// One staged file: workspace path plus its scratch-absolute path.
type StagedPair = (String, PathBuf);

/// One staged scratch tree: the scratch plus checked, sibling, and
/// resolve pairs. Resolve pairs (ty dep context,) are staged for
/// import resolution but never checked.
struct StagedScratch {
    scratch: Scratch,
    pairs: Vec<StagedPair>,
    sibling_pairs: Vec<StagedPair>,
    resolve_pairs: Vec<StagedPair>,
}

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

/// Re-anchors a parsed finding's workspace path onto its staged
/// scratch-absolute path. Parsers already reject unknown paths, so a
/// miss is a tool-output failure surfaced as
/// [`RunnerError::UnplaceableFinding`], never a panic.
fn reanchor(
    tool_id: &str,
    pairs: &[(String, PathBuf)],
    workspace: &str,
) -> Result<PathBuf, RunnerError> {
    pairs
        .iter()
        .find(|(known, _)| known == workspace)
        .map(|(_, absolute)| absolute.clone())
        .ok_or_else(|| RunnerError::UnplaceableFinding {
            tool_id: tool_id.to_owned(),
            detail: format!("finding names unstaged file: {workspace}"),
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

/// Closes `scratch`, surfacing cleanup failures as action errors, and
/// returns `value`. Owners use this for every success return so a
/// failed cleanup fails the action instead of vanishing in `Drop`;
/// early-error paths propagate their primary error and rely on the
/// best-effort `Drop` fallback.
fn cleaned<T>(tool_id: &str, scratch: Scratch, value: T) -> Result<T, RunnerError> {
    scratch
        .close()
        .map_err(|err| execution(tool_id, format!("scratch cleanup: {err}")))?;
    Ok(value)
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
        if tool_id == "biome" && tool.config_rel.is_none() {
            mirrors.push(MirrorFile {
                mirror_rel: PathBuf::from(BIOME_DEFAULTS_REL),
                contents: MirrorContents::Bytes(BIOME_DEFAULTS_BYTES.to_vec()),
            });
        }
        mirrors
    }

    /// Materializes one scratch tree with the exact source bytes plus
    /// the tool files, returning the scratch and the absolute path per
    /// workspace path in sorted order. Siblings mirror alongside the
    /// sources so link-resolution siblings exist on disk, but they stay
    /// out of `pairs`: they are never linted and findings can never
    /// address them. Resolve files (ty dep context,) mirror alongside
    /// for import resolution but stay out of `pairs`: they are never
    /// checked and their findings are filtered by the ty branch, never
    /// reported (their own targets' actions own them).
    fn stage_scratch(
        &self,
        tool_id: &str,
        tool: &RealTool,
        files: &BTreeMap<String, String>,
        siblings: &BTreeMap<String, String>,
        resolve: &BTreeMap<String, String>,
    ) -> Result<StagedScratch, RunnerError> {
        let scratch = fresh_scratch(&self.scratch_parent, tool_id)?;
        let mut mirrors = Vec::with_capacity(
            files.len() + siblings.len() + resolve.len() + tool.tool_files.len() + 1,
        );
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
        let mut resolve_pairs = Vec::with_capacity(resolve.len());
        for (path, text) in resolve {
            let absolute = scratch
                .resolve(Path::new(path))
                .map_err(|err| execution(tool_id, format!("scratch resolve: {err}")))?;
            mirrors.push(MirrorFile {
                mirror_rel: PathBuf::from(path),
                contents: MirrorContents::Bytes(text.as_bytes().to_vec()),
            });
            resolve_pairs.push((path.clone(), absolute));
        }
        mirrors.extend(Self::mirror_tool_files(tool_id, tool));
        write_all(&scratch, tool_id, &mirrors)?;
        Ok(StagedScratch {
            scratch,
            pairs,
            sibling_pairs,
            resolve_pairs,
        })
    }

    /// Resolves the tool config to an absolute scratch path. rustfmt
    /// always resolves: the hinted config, else the materialized
    /// defaults. Every other file-config tool resolves only its hint;
    /// Biome resolves its config directory separately (see
    /// [`Self::biome_config_dir`]).
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

    /// Resolves the rustfmt crate edition: authoritative context from
    /// the aspect (`CrateInfo.edition`), never guessed here. Defaulting
    /// would silently reformat e.g. Edition 2015/2024 crates with the
    /// wrong rules, and the adapter must not reconstruct rustc/edition
    /// state. A missing edition fails the action so the wiring
    /// gap surfaces instead of producing wrong diffs.
    fn rustfmt_edition(tool: &RealTool) -> Result<&str, RunnerError> {
        const TOOL_ID: &str = "rustfmt";
        tool.edition.as_deref().ok_or_else(|| {
            execution(
                TOOL_ID,
                "missing tool edition: the quality aspect must pass the CrateInfo edition via --tool-edition".to_owned(),
            )
        })
    }

    /// Resolves the Biome `--config-path` directory to an absolute
    /// scratch path: the hinted config's parent directory, else the
    /// materialized defaults directory. The directory holds exactly one
    /// `biome.json` and never the linted sources (sources mirror at
    /// their workspace paths).
    fn biome_config_dir(tool: &RealTool, scratch: &Scratch) -> Result<PathBuf, RunnerError> {
        const TOOL_ID: &str = "biome";
        let rel = tool.config_rel.as_deref().unwrap_or(BIOME_DEFAULTS_REL);
        let dir_rel = Path::new(rel)
            .parent()
            .and_then(|parent| parent.to_str())
            .unwrap_or_default();
        let dir = if dir_rel.is_empty() {
            scratch.root().to_path_buf()
        } else {
            scratch
                .resolve(Path::new(dir_rel))
                .map_err(|err| execution(TOOL_ID, format!("scratch config: {err}")))?
        };
        Ok(dir)
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

    /// Clippy check: parses the authoritative upstream
    /// diagnostics files the aspect declared as action inputs. Upstream
    /// spans already address workspace paths, so findings are
    /// re-addressed to the staged scratch-absolute paths the
    /// diagnose caller remaps back to workspace paths. Nothing spawns.
    fn check_clippy_delegated(
        &self,
        tool_id: &str,
        tool: &RealTool,
        pairs: &[(String, PathBuf)],
    ) -> Result<Vec<FileFinding>, RunnerError> {
        let workspaces: Vec<&str> = pairs
            .iter()
            .map(|(workspace, _)| workspace.as_str())
            .collect();
        let mut findings = Vec::new();
        for path in &tool.upstream_diagnostics {
            let bytes = std::fs::read(path)
                .map_err(|err| execution(tool_id, format!("upstream diagnostics: {err}")))?;
            findings.extend(parsed(
                tool_id,
                parsers::parse_clippy(&bytes, Some(0), &workspaces),
            )?);
        }
        for found in &mut findings {
            let absolute = reanchor(tool_id, pairs, &found.file)?;
            found.file = absolute.to_string_lossy().into_owned();
        }
        Ok(findings)
    }

    /// rustc check: parses the authoritative upstream
    /// diagnostics files the aspect declared as action inputs. Upstream
    /// spans already address workspace paths, so findings are
    /// re-addressed to the staged scratch-absolute paths the
    /// diagnose caller remaps back to workspace paths. Nothing spawns.
    fn check_rustc_delegated(
        &self,
        tool_id: &str,
        tool: &RealTool,
        pairs: &[(String, PathBuf)],
    ) -> Result<Vec<FileFinding>, RunnerError> {
        let workspaces: Vec<&str> = pairs
            .iter()
            .map(|(workspace, _)| workspace.as_str())
            .collect();
        let mut findings = Vec::new();
        for path in &tool.upstream_diagnostics {
            let bytes = std::fs::read(path)
                .map_err(|err| execution(tool_id, format!("upstream diagnostics: {err}")))?;
            findings.extend(parsed(
                tool_id,
                parsers::parse_rustc(&bytes, Some(0), &workspaces),
            )?);
        }
        for found in &mut findings {
            let absolute = reanchor(tool_id, pairs, &found.file)?;
            found.file = absolute.to_string_lossy().into_owned();
        }
        Ok(findings)
    }

    /// Roslyn check: parses the authoritative per-pivot SARIF files the
    /// aspect declared as action inputs (one `/errorlog` SARIF per
    /// TFM/RID pivot, concatenated as a union with per-pivot provenance).
    /// Artifact URIs already address workspace paths, so findings are
    /// re-addressed to staged scratch-absolute paths. Nothing spawns.
    fn check_roslyn_delegated(
        &self,
        tool_id: &str,
        tool: &RealTool,
        pairs: &[(String, PathBuf)],
    ) -> Result<Vec<FileFinding>, RunnerError> {
        let workspaces: Vec<&str> = pairs
            .iter()
            .map(|(workspace, _)| workspace.as_str())
            .collect();
        let mut findings = Vec::new();
        for path in &tool.upstream_diagnostics {
            let bytes = std::fs::read(path)
                .map_err(|err| execution(tool_id, format!("upstream diagnostics: {err}")))?;
            findings.extend(parsed(tool_id, parsers::parse_roslyn(&bytes, &workspaces))?);
        }
        for found in &mut findings {
            let absolute = reanchor(tool_id, pairs, &found.file)?;
            found.file = absolute.to_string_lossy().into_owned();
        }
        Ok(findings)
    }

    /// Scalafix check via recorded callback NDJSON: parses the
    /// authoritative upstream diagnostics files the aspect declared as
    /// action inputs (one JSON record per line from the
    /// `ScalafixMainCallback` entrypoint). Records address workspace
    /// paths, so findings are re-addressed to staged scratch-absolute
    /// paths like Clippy/Roslyn. Nothing spawns.
    fn check_scalafix_delegated(
        &self,
        tool_id: &str,
        tool: &RealTool,
        pairs: &[(String, PathBuf)],
    ) -> Result<Vec<FileFinding>, RunnerError> {
        let workspaces: Vec<&str> = pairs
            .iter()
            .map(|(workspace, _)| workspace.as_str())
            .collect();
        let mut findings = Vec::new();
        for path in &tool.upstream_diagnostics {
            let bytes = std::fs::read(path)
                .map_err(|err| execution(tool_id, format!("upstream diagnostics: {err}")))?;
            findings.extend(parsed(
                tool_id,
                parsers::parse_scalafix(&bytes, Some(0), &workspaces),
            )?);
        }
        for found in &mut findings {
            let absolute = reanchor(tool_id, pairs, &found.file)?;
            found.file = absolute.to_string_lossy().into_owned();
        }
        Ok(findings)
    }

    /// FSharpLint check via recorded library NDJSON: parses the
    /// authoritative upstream diagnostics files the aspect declared as
    /// action inputs (one JSON record per line from the
    /// `FSharpLint.Application.Lint` entrypoint). Records address
    /// workspace paths, so findings are re-addressed to staged
    /// scratch-absolute paths like Clippy/Roslyn. Nothing spawns.
    fn check_fsharplint_delegated(
        &self,
        tool_id: &str,
        tool: &RealTool,
        pairs: &[(String, PathBuf)],
    ) -> Result<Vec<FileFinding>, RunnerError> {
        let workspaces: Vec<&str> = pairs
            .iter()
            .map(|(workspace, _)| workspace.as_str())
            .collect();
        let mut findings = Vec::new();
        for path in &tool.upstream_diagnostics {
            let bytes = std::fs::read(path)
                .map_err(|err| execution(tool_id, format!("upstream diagnostics: {err}")))?;
            findings.extend(parsed(
                tool_id,
                parsers::parse_fsharplint(&bytes, Some(0), &workspaces),
            )?);
        }
        for found in &mut findings {
            let absolute = reanchor(tool_id, pairs, &found.file)?;
            found.file = absolute.to_string_lossy().into_owned();
        }
        Ok(findings)
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
        staged: &StagedScratch,
    ) -> Result<Vec<FileFinding>, RunnerError> {
        let pairs = &staged.pairs;
        let sibling_pairs = &staged.sibling_pairs;
        let resolve_pairs = &staged.resolve_pairs;
        let scratch = &staged.scratch;
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
            "clippy" => self.check_clippy_delegated(tool_id, tool, pairs),
            "rustc" => self.check_rustc_delegated(tool_id, tool, pairs),
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
                // scratch-absolute path for placement. A miss is a
                // tool-output failure, never a panic.
                let mut rerooted = Vec::with_capacity(reported.len());
                for found in reported {
                    rerooted.push(FileFinding {
                        file: reanchor(tool_id, pairs, &found.file)?
                            .to_string_lossy()
                            .into_owned(),
                        finding: found.finding,
                    });
                }
                Ok(rerooted)
            }
            "rustfmt" => {
                // `config_abs` always resolves rustfmt (hinted config,
                // else materialized defaults); a miss fails the action,
                // never panics.
                let Some(cfg) = config.as_ref() else {
                    return Err(execution(tool_id, "rustfmt requires a config".to_owned()));
                };
                let invocation =
                    commands::rustfmt(&tool.binary, &refs, cfg, Self::rustfmt_edition(tool)?, true);
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
                // Ty import context: resolve files are staged for
                // import resolution but never checked. Derive search dirs
                // from staged Python files (checked plus resolve) so
                // same-package top-level imports resolve. Findings in
                // resolve files are filtered below (their own targets'
                // actions own them); only checked-file findings report.
                let mut dirs: Vec<PathBuf> = Vec::new();
                for (_, absolute) in pairs.iter().chain(resolve_pairs.iter()) {
                    if let Some(parent) = absolute.parent() {
                        if !dirs.contains(&parent.to_path_buf()) {
                            dirs.push(parent.to_path_buf());
                        }
                    }
                }
                let dir_refs: Vec<&Path> = dirs.iter().map(|dir| dir.as_path()).collect();
                let invocation = commands::ty_check(&tool.binary, &refs, &dir_refs);
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                // Ty prints concise paths relative to its working
                // directory even for absolute arguments, so attribute
                // against the workspace-relative mirror paths, then
                // re-anchor each finding to its absolute scratch path:
                // the caller contract stays absolute-addressed.
                let workspaces: Vec<&str> = pairs
                    .iter()
                    .chain(resolve_pairs.iter())
                    .map(|(workspace, _)| workspace.as_str())
                    .collect();
                let mut findings = parsed(
                    tool_id,
                    parsers::parse_ty(&out.stdout, out.code, &workspaces),
                )?;
                let checked: std::collections::BTreeSet<&str> = pairs
                    .iter()
                    .map(|(workspace, _)| workspace.as_str())
                    .collect();
                findings.retain(|found| checked.contains(found.file.as_str()));
                for found in &mut findings {
                    let absolute = reanchor(tool_id, pairs, &found.file)?;
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
                // Pylint relativizes reported paths against its working
                // directory (the scratch root) even for absolute
                // arguments, so attribute against the workspace-relative
                // mirror paths, then re-anchor each finding to its
                // absolute scratch path: the caller contract stays
                // absolute-addressed. Mirrors the ty branch.
                let workspaces: Vec<&str> = pairs
                    .iter()
                    .map(|(workspace, _)| workspace.as_str())
                    .collect();
                let mut findings = parsed(
                    tool_id,
                    parsers::parse_pylint(&out.stdout, out.code, &workspaces),
                )?;
                for found in &mut findings {
                    let absolute = reanchor(tool_id, pairs, &found.file)?;
                    found.file = absolute.to_string_lossy().into_owned();
                }
                Ok(findings)
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
            "biome" => {
                let config_dir = Self::biome_config_dir(tool, scratch)?;
                let invocation = if capability == "format" {
                    commands::biome_format_check(&tool.binary, &refs, &config_dir)
                } else {
                    commands::biome_lint_check(&tool.binary, &refs, &config_dir)
                };
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                // Biome reports paths relative to its working directory
                // (the scratch root), so attribute against the
                // workspace-relative mirror paths, then re-anchor each
                // finding to its absolute scratch path: the caller
                // contract stays absolute-addressed. Mirrors the prettier
                // branch below.
                let workspaces: Vec<&str> = pairs
                    .iter()
                    .map(|(workspace, _)| workspace.as_str())
                    .collect();
                let report = if capability == "format" {
                    parsers::parse_biome_format(&out.stdout, out.code, &workspaces)
                } else {
                    parsers::parse_biome_lint(&out.stdout, out.code, &workspaces)
                };
                let mut findings = parsed(tool_id, report)?;
                for found in &mut findings {
                    let absolute = reanchor(tool_id, pairs, &found.file)?;
                    found.file = absolute.to_string_lossy().into_owned();
                }
                Ok(findings)
            }
            "eslint" => {
                let invocation = match config.as_ref() {
                    Some(cfg) => commands::eslint_check(&tool.binary, &refs, cfg),
                    None => {
                        return Err(execution(tool_id, "eslint requires a config".to_owned()));
                    }
                };
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                parsed(tool_id, parsers::parse_eslint(&out.stdout, out.code, &strs))
            }
            "prettier" => {
                let invocation = commands::prettier_check(&tool.binary, &refs);
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                // Prettier reports working-directory-relative paths even
                // for absolute arguments, so attribute against the
                // workspace-relative mirror paths, then re-anchor each
                // finding to its absolute scratch path: the caller
                // contract stays absolute-addressed.
                let workspaces: Vec<&str> = pairs
                    .iter()
                    .map(|(workspace, _)| workspace.as_str())
                    .collect();
                let report = parsers::parse_prettier_check(&out.stderr, out.code, &workspaces);
                let mut findings = parsed(tool_id, report)?;
                for found in &mut findings {
                    let absolute = reanchor(tool_id, pairs, &found.file)?;
                    found.file = absolute.to_string_lossy().into_owned();
                }
                Ok(findings)
            }
            "scalafmt" => {
                let invocation = commands::scalafmt_check(&tool.binary, &refs, config.as_deref());
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                parsed(
                    tool_id,
                    parsers::parse_scalafmt(&out.stdout, out.code, &strs),
                )
            }
            "scalafix" => {
                if !tool.upstream_diagnostics.is_empty() {
                    self.check_scalafix_delegated(tool_id, tool, pairs)
                } else {
                    let invocation =
                        commands::scalafix_check(&tool.binary, &refs, None, None, None);
                    let out = self.run(tool_id, tool, &invocation, scratch)?;
                    parsed(
                        tool_id,
                        parsers::parse_scalafix(&out.stdout, out.code, &strs),
                    )
                }
            }
            "csharpier" => {
                let invocation = commands::csharpier_check(&tool.binary, &refs, config.as_deref());
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                let workspaces: Vec<&str> = pairs
                    .iter()
                    .map(|(workspace, _)| workspace.as_str())
                    .collect();
                let mut findings = parsed(
                    tool_id,
                    parsers::parse_csharpier(&out.stdout, out.code, &workspaces),
                )?;
                for found in &mut findings {
                    let absolute = reanchor(tool_id, pairs, &found.file)?;
                    found.file = absolute.to_string_lossy().into_owned();
                }
                Ok(findings)
            }
            "fantomas" => {
                let invocation = commands::fantomas_check(&tool.binary, &refs);
                let out = self.run(tool_id, tool, &invocation, scratch)?;
                let workspaces: Vec<&str> = pairs
                    .iter()
                    .map(|(workspace, _)| workspace.as_str())
                    .collect();
                let mut findings = parsed(
                    tool_id,
                    parsers::parse_fantomas(&out.stdout, out.code, &workspaces),
                )?;
                for found in &mut findings {
                    let absolute = reanchor(tool_id, pairs, &found.file)?;
                    found.file = absolute.to_string_lossy().into_owned();
                }
                Ok(findings)
            }
            "roslyn" => self.check_roslyn_delegated(tool_id, tool, pairs),
            "fsharplint" => {
                if !tool.upstream_diagnostics.is_empty() {
                    self.check_fsharplint_delegated(tool_id, tool, pairs)
                } else {
                    let invocation =
                        commands::fsharplint_check(&tool.binary, &refs, None, config.as_deref());
                    let out = self.run(tool_id, tool, &invocation, scratch)?;
                    parsed(
                        tool_id,
                        parsers::parse_fsharplint(&out.stdout, out.code, &strs),
                    )
                }
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
        self.diagnose_with_resolve(tool_id, capability, files, siblings, &BTreeMap::new())
    }

    /// Resolve-aware [`Self::diagnose_with_siblings`]: resolve files (ty
    /// dep context,) mirror into the scratch tree for import
    /// resolution but stay out of findings, snapshots, and fixes. Findings
    /// addressing resolve files are filtered by the ty branch (their own
    /// targets' actions own them); findings addressing truly unstaged files
    /// still fail as tool-output errors.
    pub fn diagnose_with_resolve(
        &self,
        tool_id: &str,
        capability: &str,
        files: &BTreeMap<String, String>,
        siblings: &BTreeMap<String, String>,
        resolve: &BTreeMap<String, String>,
    ) -> Result<Vec<Diagnostic>, RunnerError> {
        let tool = self.tool(tool_id)?;
        let staged = self.stage_scratch(tool_id, tool, files, siblings, resolve)?;
        let collected = self.run_check(tool_id, tool, capability, &staged)?;
        let StagedScratch {
            scratch,
            pairs,
            sibling_pairs: _,
            resolve_pairs: _,
        } = staged;
        let mut diagnostics = Vec::with_capacity(collected.len());
        for found in &collected {
            let workspace = pairs
                .iter()
                .find(|(_, absolute)| absolute.as_os_str() == OsStr::new(&found.file))
                .map(|pair| pair.0.clone())
                .ok_or_else(|| RunnerError::UnplaceableFinding {
                    tool_id: tool_id.to_owned(),
                    detail: format!("finding names unstaged file: {}", found.file),
                })?;
            let text = files.get(&workspace).ok_or(RunnerError::MissingFile {
                path: workspace.clone(),
            })?;
            diagnostics.push(
                place_finding(&found.finding, &workspace, text).map_err(|err| {
                    RunnerError::UnplaceableFinding {
                        tool_id: tool_id.to_owned(),
                        detail: err.to_string(),
                    }
                })?,
            );
        }
        cleaned(tool_id, scratch, diagnostics)
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
    run_real_pipeline_with_resolve(producer, capability, stages, files, siblings, &[], backend)
}

/// Resolve-aware [`run_real_pipeline_with_siblings`]: resolve files are ty
/// dep-context bytes. They must be UTF-8, must not collide with a
/// checked, sibling, or fellow resolve path, and never enter snapshots,
/// stages, or fixes, so a stage naming a resolve path still fails
/// `MissingFile`. Findings addressing resolve files are filtered (their
/// own targets' actions own them).
pub fn run_real_pipeline_with_resolve(
    producer: &str,
    capability: &str,
    stages: &[StageSpec],
    files: &[FileInput],
    siblings: &[FileInput],
    resolve: &[FileInput],
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
    let mut resolve_texts = BTreeMap::new();
    for item in resolve {
        if initial.contains_key(&item.path)
            || sibling_texts.contains_key(&item.path)
            || resolve_texts.contains_key(&item.path)
        {
            return Err(RunnerError::DuplicateFile {
                path: item.path.clone(),
            });
        }
        let text = std::str::from_utf8(&item.bytes).map_err(|_| RunnerError::InvalidUtf8 {
            path: item.path.clone(),
        })?;
        resolve_texts.insert(item.path.clone(), text.to_owned());
    }
    let mut initial_diagnostics = Vec::new();
    for stage in stages {
        let subset = stage_subset(stage, &initial)?;
        initial_diagnostics.extend(backend.diagnose_with_resolve(
            &stage.tool_id,
            capability,
            &subset,
            &sibling_texts,
            &resolve_texts,
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
        let subset = stage_subset(stage, &terminal)?;
        terminal_diagnostics.extend(backend.diagnose_with_resolve(
            &stage.tool_id,
            capability,
            &subset,
            &sibling_texts,
            &resolve_texts,
        )?);
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

#[path = "real_fix.rs"]
mod real_fix;

#[cfg(test)]
#[path = "real_tests_a.rs"]
mod real_tests_a;
#[cfg(test)]
#[path = "real_tests_b.rs"]
mod real_tests_b;
#[cfg(test)]
#[path = "real_tests_c.rs"]
mod real_tests_c;
