//! Shared fakes and helpers for the exec family unit tests.

use super::{execute, Env};
use crate::args::parse;
use crate::args::Invocation;
use crate::plan::GENERATE_ENV_INTENDED;
use crate::resolve::{QueryResult, QueryRunner};
use dx_digest::blake3 as digest;
use dx_process::{ChildStatus, Runner};
use dx_setup::{
    commit_pair, setup_hex, GenerationId, SetupPair, ENVIRONMENTS_DIR_NAME, GENERATED_DIR_NAME,
};
use quality_result::proto;
use quality_result::proto::{Capability, Convergence, FileSnapshot, QualityResult, Stage};
use quality_result::{encode_validated, SCHEMA_MAJOR, SCHEMA_MINOR};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self};
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

pub(crate) fn invocation(words: &[&str]) -> Invocation {
    parse(&words.iter().map(ToString::to_string).collect::<Vec<_>>()).expect("parse")
}

pub(crate) fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dx-exec-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

pub(crate) struct Harness {
    pub(crate) workspace: PathBuf,
    pub(crate) temp: PathBuf,
    pub(crate) results: HashMap<String, Vec<u8>>,
    pub(crate) bazel_code: i32,
    pub(crate) fail_target: bool,
    pub(crate) io_error: bool,
    pub(crate) signalled: bool,
    pub(crate) skip_bep: bool,
    pub(crate) raw_bep: Option<Vec<String>>,
    pub(crate) query: ScriptQuery,
    /// Canned `DX_GENERATE_INTENDED` witness the fake runner writes
    /// for generate runs; `None` exercises the missing-witness
    /// paths.
    pub(crate) intended: Option<Vec<u8>>,
    /// Dispatch environments observed by the fake runner, one entry
    /// per launch in call order.
    pub(crate) seen_env: Rc<RefCell<Vec<Vec<(String, String)>>>>,
}

/// Scripted ownership-query runner: replays canned outputs in call
/// order and records argv. Empty outputs panic, so tests that never
/// resolve file scopes prove they issue no queries.
pub(crate) struct ScriptQuery {
    pub(crate) calls: RefCell<Vec<Vec<String>>>,
    pub(crate) outputs: RefCell<Vec<QueryResult>>,
}

impl ScriptQuery {
    pub(crate) fn script_owners(&self, owners: &str) {
        self.outputs.borrow_mut().push(QueryResult {
            code: Some(0),
            stdout: owners.as_bytes().to_vec(),
            stderr: Vec::new(),
        });
    }
}

impl QueryRunner for ScriptQuery {
    fn run_query(&self, argv: &[String], _cwd: &Path) -> io::Result<QueryResult> {
        self.calls.borrow_mut().push(argv.to_vec());
        Ok(self.outputs.borrow_mut().remove(0))
    }
}

impl Harness {
    pub(crate) fn new(name: &str) -> Self {
        Harness {
            workspace: temp_dir(&format!("{name}-ws")),
            temp: temp_dir(&format!("{name}-tmp")),
            results: HashMap::new(),
            bazel_code: 0,
            fail_target: false,
            io_error: false,
            signalled: false,
            skip_bep: false,
            raw_bep: None,
            query: ScriptQuery {
                calls: RefCell::new(Vec::new()),
                outputs: RefCell::new(Vec::new()),
            },
            intended: None,
            seen_env: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub(crate) fn write_source(&self, path: &str, text: &str) {
        let full = self.workspace.join(path);
        std::fs::create_dir_all(full.parent().expect("parent")).expect("parent dir");
        std::fs::write(full, text).expect("write source");
    }

    pub(crate) fn valid_result(
        &self,
        initial: Vec<proto::Diagnostic>,
        replacements: Vec<proto::FileEdits>,
    ) -> Vec<u8> {
        let original = std::fs::read(self.workspace.join("src/a.py")).unwrap_or_default();
        self.result_full(
            initial,
            vec![],
            replacements,
            vec![FileSnapshot {
                path: "src/a.py".to_owned(),
                digest: digest(&original).to_vec(),
            }],
        )
    }

    pub(crate) fn result_full(
        &self,
        initial: Vec<proto::Diagnostic>,
        terminal: Vec<proto::Diagnostic>,
        replacements: Vec<proto::FileEdits>,
        snapshots: Vec<FileSnapshot>,
    ) -> Vec<u8> {
        let result = QualityResult {
            schema_major: SCHEMA_MAJOR,
            schema_minor: SCHEMA_MINOR,
            producer: "//test:corpus".to_owned(),
            capability: Capability::Lint as i32,
            stages: vec![Stage {
                tool_id: "lint-tool".to_owned(),
                class_ids: vec!["python".to_owned()],
                source_paths: vec!["src/a.py".to_owned()],
            }],
            completed_rounds: 1,
            convergence: Convergence::Stable as i32,
            original_snapshot: snapshots.clone(),
            terminal_snapshot: snapshots,
            initial_diagnostics: initial,
            terminal_diagnostics: terminal,
            replacements,
        };
        encode_validated(&result).expect("encode")
    }

    pub(crate) fn diagnostic(message: &str, fixable: bool) -> proto::Diagnostic {
        Self::diagnostic_with(
            proto::Severity::Warning as i32,
            "lint-tool",
            "src/a.py",
            message,
            fixable,
        )
    }

    pub(crate) fn diagnostic_with(
        severity: i32,
        tool: &str,
        path: &str,
        message: &str,
        fixable: bool,
    ) -> proto::Diagnostic {
        proto::Diagnostic {
            severity,
            message: message.to_owned(),
            tool_id: tool.to_owned(),
            rule_id: format!("{tool}/rule"),
            path: path.to_owned(),
            start_byte: Some(0),
            end_byte: Some(1),
            fixable,
        }
    }

    pub(crate) fn replacement(&self, replacement: &[u8]) -> proto::FileEdits {
        let original = std::fs::read(self.workspace.join("src/a.py")).expect("source");
        self.replacement_at(
            "src/a.py",
            digest(&original).to_vec(),
            vec![proto::Edit {
                start_byte: 0,
                end_byte: 1,
                replacement: replacement.to_vec(),
            }],
        )
    }

    pub(crate) fn replacement_at(
        &self,
        path: &str,
        original_digest: Vec<u8>,
        edits: Vec<proto::Edit>,
    ) -> proto::FileEdits {
        proto::FileEdits {
            path: path.to_owned(),
            original_digest,
            edits,
        }
    }

    pub(crate) fn runner(&self) -> FakeRunner {
        if let Some(lines) = &self.raw_bep {
            return FakeRunner {
                code: Some(self.bazel_code),
                bep_lines: lines.clone(),
                io_error: self.io_error,
                skip_bep: self.skip_bep,
                intended: self.intended.clone(),
                seen_env: Rc::clone(&self.seen_env),
            };
        }
        let mut lines = Vec::new();
        let mut files = Vec::new();
        for (label, bytes) in &self.results {
            let safe = label.replace(['/', ':'], "_");
            let artifact = self.temp.join(format!("{safe}.pb"));
            std::fs::write(&artifact, bytes).expect("artifact");
            files.push(format!(
                "{{\"uri\": \"file://{}\"}}",
                artifact.to_string_lossy()
            ));
        }
        lines.push(format!(
            "{{\"id\": {{\"namedSet\": {{\"id\": \"0\"}}}}, \"namedSetOfFiles\": {{\"files\": [{}]}}}}",
            files.join(",")
        ));
        let success = if self.fail_target { "false" } else { "true" };
        lines.push(format!(
            "{{\"id\": {{\"targetCompleted\": {{\"label\": \"//test:corpus\"}}}}, \"completed\": {{\"success\": {success}, \"outputGroup\": [{{\"name\": \"dx_results\", \"fileSets\": [{{\"id\": \"0\"}}]}}]}}}}"
        ));
        FakeRunner {
            code: if self.signalled {
                None
            } else {
                Some(self.bazel_code)
            },
            bep_lines: lines,
            io_error: self.io_error,
            skip_bep: self.skip_bep,
            intended: self.intended.clone(),
            seen_env: Rc::clone(&self.seen_env),
        }
    }

    pub(crate) fn run(&self, words: &[&str]) -> (i32, String, String) {
        self.run_with_ci(words, false)
    }

    /// `dx run` CI-gate probe: drives `execute` with the startup
    /// refusal bit set, without touching process-global
    /// environment (parallel tests share one process).
    pub(crate) fn run_with_ci(&self, words: &[&str], ci: bool) -> (i32, String, String) {
        let inv = invocation(words);
        let runner = self.runner();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute(
            &inv,
            Env {
                workspace: &self.workspace,
                runner: &runner,
                query_runner: &self.query,
                temp_dir: &self.temp,
                pid: std::process::id(),
                nonce: 0,
                out: &mut out,
                err: &mut err,
                ci,
            },
        );
        (
            code,
            String::from_utf8(out).expect("stdout"),
            String::from_utf8(err).expect("stderr"),
        )
    }
}

pub(crate) struct FakeRunner {
    pub(crate) code: Option<i32>,
    pub(crate) bep_lines: Vec<String>,
    pub(crate) io_error: bool,
    pub(crate) skip_bep: bool,
    pub(crate) intended: Option<Vec<u8>>,
    pub(crate) seen_env: Rc<RefCell<Vec<Vec<(String, String)>>>>,
}

impl Runner for FakeRunner {
    fn run(&self, argv: &[String], _cwd: &Path, env: &[(&str, &str)]) -> io::Result<ChildStatus> {
        if self.io_error {
            return Err(io::Error::other("fake launch failure"));
        }
        self.seen_env.borrow_mut().push(
            env.iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect(),
        );
        if !self.skip_bep {
            if let Some(bep) = argv
                .iter()
                .find_map(|arg| arg.strip_prefix("--build_event_json_file="))
            {
                std::fs::write(bep, self.bep_lines.join("\n")).expect("BEP file");
            }
        }
        if let Some(path) = env
            .iter()
            .find_map(|(key, value)| (*key == GENERATE_ENV_INTENDED).then_some(*value))
        {
            if let Some(witness) = &self.intended {
                std::fs::write(path, witness).expect("intended witness");
            }
        }
        Ok(ChildStatus { code: self.code })
    }
}

/// Standard base64 for canned intended-manifest witnesses.
pub(crate) fn b64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = u32::from(*chunk.get(1).unwrap_or(&0));
        let b2 = u32::from(*chunk.get(2).unwrap_or(&0));
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Canned `DX_GENERATE_INTENDED` witness: one scope, the given file
/// entries, and the given ignored-import entries.
pub(crate) fn intended_witness(mode: &str, complete: bool, files: &str, ignored: &str) -> Vec<u8> {
    format!(
        concat!(
            r#"{{"schema_major":1,"schema_minor":0,"mode":"{mode}","#,
            r#""scopes":[{{"value":"//...","results_complete":{complete}}}],"#,
            r#""files":[{files}],"ignored_imports":[{ignored}]}}"#
        ),
        mode = mode,
        complete = complete,
        files = files,
        ignored = ignored,
    )
    .into_bytes()
}

/// One modify entry replacing `original` with `candidate` through a
/// single full-span edit.
pub(crate) fn intended_modify(path: &str, original: &[u8], candidate: &[u8]) -> String {
    format!(
        concat!(
            r#"{{"path":{path},"scope_index":0,"original_content":"{original}","#,
            r#""edits":[{{"start_byte":0,"end_byte":{end},"replacement":"{candidate}"}}]}}"#
        ),
        path = serde_json::to_string(path).expect("path JSON"),
        original = b64(original),
        end = original.len(),
        candidate = b64(candidate),
    )
}

/// One ignored-import audit entry.
pub(crate) fn intended_ignored(path: &str, language: &str, import: &str) -> String {
    format!(
        r#"{{"path":{path},"language":{language},"import":{import},"scope_index":0}}"#,
        path = serde_json::to_string(path).expect("path JSON"),
        language = serde_json::to_string(language).expect("language JSON"),
        import = serde_json::to_string(import).expect("import JSON"),
    )
}

/// Argv-recording launcher probe for passthrough tests: the
/// shared `FakeRunner` never observes argv, which is the whole
/// contract under test here.
pub(crate) struct ArgvProbe {
    pub(crate) code: Option<i32>,
    pub(crate) seen: Rc<RefCell<Vec<Vec<String>>>>,
}

impl Runner for ArgvProbe {
    fn run(&self, argv: &[String], _cwd: &Path, _env: &[(&str, &str)]) -> io::Result<ChildStatus> {
        self.seen.borrow_mut().push(argv.to_vec());
        Ok(ChildStatus { code: self.code })
    }
}

pub(crate) fn json_events(out: &str) -> Vec<serde_json::Value> {
    out.lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .expect("NDJSON")
}

pub(crate) fn event<'a>(events: &'a [serde_json::Value], kind: &str) -> &'a serde_json::Value {
    events
        .iter()
        .find(|event| event["event"] == serde_json::json!(kind))
        .unwrap_or_else(|| panic!("missing {kind} event"))
}

pub(crate) fn write_bep_artifact(harness: &Harness, name: &str, bytes: &[u8]) -> String {
    let path = harness.temp.join(name);
    std::fs::write(&path, bytes).expect("artifact");
    format!("file://{}", path.display())
}

pub(crate) fn test_result_line(label: &str, entries: &[(String, String)]) -> String {
    let outputs = entries
        .iter()
        .map(|(name, uri)| format!("{{\"name\": \"{name}\", \"uri\": \"{uri}\"}}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"id\": {{\"testResult\": {{\"label\": \"{label}\"}} }}, \"testResult\": {{\"status\": \"PASSED\", \"testActionOutput\": [{outputs}]}}}}"
    )
}

pub(crate) fn coverage_harness(name: &str, tracefile: &[u8]) -> Harness {
    let harness = Harness::new(name);
    let uri = write_bep_artifact(&harness, "coverage.dat", tracefile);
    Harness {
        raw_bep: Some(vec![test_result_line(
            "//a:t",
            &[(String::from("test.lcov"), uri)],
        )]),
        ..harness
    }
}

/// Default-mode witness for one `rust/hello/BUILD.bazel` modify
/// (`abc` to `xyz`); the workspace holds `workspace_text` so the
/// caller selects the write outcome.
pub(crate) fn generate_witness(workspace_text: &str, name: &str) -> Harness {
    let mut harness = Harness::new(name);
    harness.write_source("rust/hello/BUILD.bazel", workspace_text);
    harness.intended = Some(intended_witness(
        "default",
        true,
        &intended_modify("rust/hello/BUILD.bazel", b"abc\n", b"xyz\n"),
        "",
    ));
    harness
}

/// Managed-state fixture for the `dx clean` exec tests (issue
/// #20): commits `pair` through the real `dx_setup` commit path and
/// materializes both generation directories, returning the setup
/// record hex. Digest tags mirror the `dx_clean` fixtures (one
/// lowercase-hex character repeated to 64).
pub(crate) fn commit_clean_pair(harness: &Harness, env: char, gen: char) -> String {
    let pair = SetupPair {
        environment: GenerationId::new(&env.to_string().repeat(64)).expect("environment digest"),
        generated: GenerationId::new(&gen.to_string().repeat(64)).expect("generated digest"),
    };
    commit_pair(&harness.workspace, &pair).expect("commit pair");
    for (dir, tag) in [(ENVIRONMENTS_DIR_NAME, env), (GENERATED_DIR_NAME, gen)] {
        std::fs::create_dir_all(
            harness
                .workspace
                .join(".dx")
                .join(dir)
                .join(tag.to_string().repeat(64)),
        )
        .expect("generation dir");
    }
    setup_hex(&pair)
}

/// Canned BEP stream referencing one shard file for one managed
/// output group: the fake runner writes these lines verbatim.
pub(crate) fn managed_shard_bep(shard: &Path, group: &str) -> Vec<String> {
    vec![
        format!(
            "{{\"id\": {{\"namedSet\": {{\"id\": \"0\"}}}}, \"namedSetOfFiles\": {{\"files\": [{{\"uri\": \"file://{}\"}}]}}}}",
            shard.to_string_lossy()
        ),
        format!(
            "{{\"id\": {{\"targetCompleted\": {{\"label\": \"//a:one\"}}}}, \"completed\": {{\"success\": true, \"outputGroup\": [{{\"name\": \"{group}\", \"fileSets\": [{{\"id\": \"0\"}}]}}]}}}}"
        ),
    ]
}

/// Fresh managed workspace plus two real artifact files the tests
/// stage mirror leaves against.
pub(crate) fn managed_stage_fixture(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let workspace = temp_dir(&format!("{name}-ws"));
    let artifacts = temp_dir(&format!("{name}-artifacts"));
    let first = artifacts.join("first.txt");
    let second = artifacts.join("second.txt");
    std::fs::write(&first, "first").expect("artifact");
    std::fs::write(&second, "second").expect("artifact");
    (workspace, first, second)
}

pub(crate) fn codegen_entry(logical: &str, artifact: &Path) -> dx_codegen::ProjectionEntry {
    dx_codegen::ProjectionEntry {
        logical_path: logical.to_owned(),
        artifact: artifact.to_string_lossy().into_owned(),
        import_root: String::new(),
        namespace: String::new(),
    }
}

pub(crate) fn env_entry(key: &str, value: &str, artifact: &Path) -> dx_env_plan::ProjectionEntry {
    dx_env_plan::ProjectionEntry {
        key: key.to_owned(),
        value: value.to_owned(),
        artifact: artifact.to_string_lossy().into_owned(),
    }
}

/// Sets Unix permission bits; the managed suites run on Linux-only
/// CI, so filesystem-failure injection through read-only
/// directories is deterministic.
pub(crate) fn set_mode(path: &Path, mode: u32) {
    let mut permissions = std::fs::metadata(path)
        .expect("mode metadata")
        .permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, mode);
    std::fs::set_permissions(path, permissions).expect("set mode");
}

/// Umbrella fixture: clean quality results plus a clean
/// check-mode generation witness, so every phase succeeds.
pub(crate) fn umbrella_clean(name: &str) -> Harness {
    let mut harness = Harness::new(name);
    harness.write_source("src/a.py", "x = 1\n");
    harness.results.insert(
        "//test:corpus".to_owned(),
        harness.valid_result(vec![], vec![]),
    );
    harness.intended = Some(intended_witness("check", true, "", ""));
    harness
}

/// Umbrella fixture: one fixable lint finding, so the format
/// phase reports changes in check mode and applies them in
/// default mode (staling the replayed fixture downstream).
pub(crate) fn umbrella_findings(name: &str) -> Harness {
    let mut harness = Harness::new(name);
    harness.write_source("src/a.py", "x = 1\n");
    harness.results.insert(
        "//test:corpus".to_owned(),
        harness.valid_result(
            vec![Harness::diagnostic("unused", true)],
            vec![harness.replacement(b"y")],
        ),
    );
    harness
}
