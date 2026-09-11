//! Intended-manifest finalizer (M10 WP1).
//!
//! The Gazelle extension ([`gazelle/rust/manifest.go`](../../gazelle/rust/manifest.go))
//! witnesses the *intended* file contents for a `//dx:generate` run as JSON on
//! `DX_GENERATE_INTENDED` (candidate bytes, original bytes, and byte-offset
//! edits, all base64) plus `DX_GENERATE_SCOPE` and `DX_GENERATE_MODE`.
//! [`finalize`] turns that witness into a [`GenerationManifest`]: it decodes
//! the payload, reconstructs the candidate bytes and stamps the BLAKE3
//! original digest through `generation_result::candidate`, and reads the
//! workspace once per file to determine the write outcome in default mode:
//!
//! * workspace bytes equal the intended bytes → [`WriteOutcome::Applied`],
//! * anything else (mismatch, missing, unreadable) →
//!   [`WriteOutcome::NotApplied`] with a machine-readable `failure_code`
//!   (`"write_mismatch"`, `"missing_file"`, `"unreadable_file"`).
//!
//! In check mode no workspace file is read at all: the manifest carries
//! [`WriteOutcome::Unspecified`] outcomes, and every scope must witness
//! `results_complete` (a check run that did not finish every scope fails
//! validation rather than producing a manifest that claims otherwise).
//! A complete check witness finalizes even when the Gazelle run exits
//! nonzero: upstream writes the witness in `AfterResolvingDeps` before
//! the emit loop, and without `-patch` the only post-witness failure is
//! `ErrDiff` ("changes found") — the expected check signal, reported as
//! exit 1 with the changes listed. Malformed payloads fail closed with [`FinalizeError`] before any filesystem
//! access beyond the (guarded) outcome reads; paths that are not safe to join
//! under the workspace root are rejected without being touched.
//!
//! Wire shape (base64 `[]byte` fields mirror the Go structs field-for-field):
//! ```json
//! {
//!   "schema_major": 1, "schema_minor": 0, "mode": "check",
//!   "scopes": [{"value": "//...", "results_complete": true}],
//!   "files": [{
//!     "path": "rust/hello/BUILD.bazel", "scope_index": 0,
//!     "original_content": "<base64>", "create_content": "<base64>",
//!     "edits": [{"start_byte": 0, "end_byte": 5, "replacement": "<base64>"}]
//!   }],
//!   "ignored_imports": [{
//!     "path": "rust/hello/BUILD.bazel", "language": "rust",
//!     "import": "serde", "scope_index": 0
//!   }]
//! }
//! ```
//!
//! A file carries `create_content` (create) or `original_content` plus `edits`
//! (modify); carrying both is a contradiction and rejected.

use generation_result::proto::{
    file_result, Edit, FileResult, GenerationManifest, IgnoredImport, Mode, Modification, Scope,
    WriteOutcome,
};
use serde::Deserialize;
use std::fmt;
use std::path::Path;

/// Failure codes recorded on [`WriteOutcome::NotApplied`] file results.
pub const FAILURE_WRITE_MISMATCH: &str = "write_mismatch";
pub const FAILURE_MISSING_FILE: &str = "missing_file";
pub const FAILURE_UNREADABLE_FILE: &str = "unreadable_file";

/// Input to [`finalize`].
pub struct FinalizeInput<'a> {
    /// Raw bytes of the `DX_GENERATE_INTENDED` payload.
    pub intended_json: &'a [u8],
    /// Workspace root the gazelle run operated on; outcome reads join here.
    pub workspace: &'a Path,
    /// Whether the wrapper ran gazelle with `--mode=check`.
    pub check: bool,
    /// Whether the gazelle subprocess exited successfully.
    pub gazelle_ok: bool,
}

/// Ways finalization can fail. IO problems reading workspace files are *not*
/// errors: they become [`WriteOutcome::NotApplied`] results with a
/// `failure_code`, because a concurrent workspace mutation must degrade to a
/// failing manifest rather than a CLI crash.
#[derive(Debug, PartialEq, Eq)]
pub enum FinalizeError {
    /// Payload is not well-formed JSON of the expected shape (includes bad
    /// base64, schema or mode mismatch, contradictory create+modify, and
    /// paths unsafe to join under the workspace root).
    Malformed(String),
    /// A check run whose gazelle subprocess failed carries no trustworthy
    /// witness, so there is nothing to finalize.
    IncompleteCheck,
    /// A structurally complete manifest failed `generation_result` validation.
    Invalid(generation_result::Error),
}

impl fmt::Display for FinalizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FinalizeError::Malformed(detail) => write!(f, "malformed intended manifest: {detail}"),
            FinalizeError::IncompleteCheck => write!(
                f,
                "check run cannot be finalized: gazelle did not complete successfully"
            ),
            FinalizeError::Invalid(err) => write!(f, "invalid generation manifest: {err}"),
        }
    }
}

impl std::error::Error for FinalizeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FinalizeError::Invalid(err) => Some(err),
            _ => None,
        }
    }
}

/// Decode strict standard base64 (RFC 4648 alphabet with `=` padding).
/// Whitespace and non-alphabet bytes are rejected: the payload is
/// machine-generated, so leniency would only mask corruption.
fn decode_b64(value: &str) -> Result<Vec<u8>, ()> {
    fn sextet(c: u8) -> Result<u8, ()> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(()),
        }
    }
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if !bytes.len().is_multiple_of(4) {
        return Err(());
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let (quanta, _) = bytes.as_chunks::<4>();
    for (qi, chunk) in quanta.iter().enumerate() {
        let is_last = qi + 1 == quanta.len();
        let mut n: u32 = 0;
        let mut pad = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            if c == b'=' {
                // Padding is only legal at the tail of the final quantum.
                if !is_last || i < 2 {
                    return Err(());
                }
                pad += 1;
                n <<= 6;
            } else {
                if pad > 0 {
                    return Err(());
                }
                n = (n << 6) | u32::from(sextet(c)?);
            }
        }
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad == 0 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

fn de_b64<'de, D>(d: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    decode_b64(&s).map_err(|()| serde::de::Error::custom("invalid base64"))
}

fn de_opt_b64<'de, D>(d: D) -> Result<Option<Vec<u8>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(d)?;
    opt.map(|s| decode_b64(&s).map_err(|()| serde::de::Error::custom("invalid base64")))
        .transpose()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IntendedPayload {
    schema_major: u32,
    schema_minor: u32,
    mode: String,
    #[serde(default)]
    scopes: Vec<IntendedScope>,
    #[serde(default)]
    files: Vec<IntendedFile>,
    #[serde(default)]
    ignored_imports: Vec<IntendedIgnored>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IntendedScope {
    value: String,
    #[serde(default)]
    results_complete: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IntendedFile {
    path: String,
    #[serde(default)]
    scope_index: u32,
    #[serde(default, deserialize_with = "de_opt_b64")]
    original_content: Option<Vec<u8>>,
    #[serde(default, deserialize_with = "de_opt_b64")]
    create_content: Option<Vec<u8>>,
    #[serde(default)]
    edits: Vec<IntendedEdit>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IntendedEdit {
    start_byte: u64,
    end_byte: u64,
    #[serde(deserialize_with = "de_b64")]
    replacement: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IntendedIgnored {
    path: String,
    language: String,
    #[serde(rename = "import")]
    import: String,
    #[serde(default)]
    scope_index: u32,
}

/// Reject paths that are unsafe to join under the workspace root before any
/// filesystem access: absolute paths, `..` escapes, and empty components all
/// fail closed. (Full naming rules such as the no-`./`-prefix convention are
/// enforced by `generation_result::validate` on the finished manifest.)
fn check_joinable(path: &str) -> Result<(), FinalizeError> {
    if path.is_empty() || path.starts_with('/') {
        return Err(FinalizeError::Malformed(format!(
            "unsafe path {path:?}: must be workspace-relative"
        )));
    }
    if path
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(FinalizeError::Malformed(format!(
            "unsafe path {path:?}: empty or dot component"
        )));
    }
    Ok(())
}

/// Determine the default-mode outcome by comparing workspace bytes against the
/// intended bytes: equal means Gazelle already wrote them ([`Applied`]),
/// anything else degrades to [`NotApplied`] with a machine-readable
/// `failure_code`.
fn default_outcome(workspace: &Path, path: &str, intended: &[u8]) -> (i32, String) {
    match std::fs::read(workspace.join(path)) {
        Ok(actual) if actual == intended => (WriteOutcome::Applied as i32, String::new()),
        Ok(_) => (
            WriteOutcome::NotApplied as i32,
            FAILURE_WRITE_MISMATCH.to_owned(),
        ),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => (
            WriteOutcome::NotApplied as i32,
            FAILURE_MISSING_FILE.to_owned(),
        ),
        Err(_) => (
            WriteOutcome::NotApplied as i32,
            FAILURE_UNREADABLE_FILE.to_owned(),
        ),
    }
}

/// Build a validated [`GenerationManifest`] from a Gazelle intended-manifest
/// witness. See the [module-level documentation](self) for the protocol.
pub fn finalize(input: &FinalizeInput<'_>) -> Result<GenerationManifest, FinalizeError> {
    let payload: IntendedPayload = serde_json::from_slice(input.intended_json)
        .map_err(|err| FinalizeError::Malformed(format!("invalid intended JSON: {err}")))?;
    if payload.schema_major != generation_result::SCHEMA_MAJOR
        || payload.schema_minor != generation_result::SCHEMA_MINOR
    {
        return Err(FinalizeError::Malformed(format!(
            "unsupported schema {}.{}; want {}.{}",
            payload.schema_major,
            payload.schema_minor,
            generation_result::SCHEMA_MAJOR,
            generation_result::SCHEMA_MINOR,
        )));
    }
    let want_mode = if input.check { "check" } else { "default" };
    if payload.mode != want_mode {
        return Err(FinalizeError::Malformed(format!(
            "mode mismatch: payload says {:?}, wrapper ran {want_mode:?}",
            payload.mode,
        )));
    }
    // A failed check run with an incomplete witness carries nothing
    // trustworthy; a *complete* witness finalizes despite the nonzero
    // exit (see the module docs for the `ErrDiff` ordering argument).
    let witnessed_complete = payload.scopes.iter().all(|s| s.results_complete);
    if input.check && !witnessed_complete && !input.gazelle_ok {
        return Err(FinalizeError::IncompleteCheck);
    }
    // Guard every path before touching the filesystem.
    for file in &payload.files {
        check_joinable(&file.path)?;
    }
    for ignored in &payload.ignored_imports {
        check_joinable(&ignored.path)?;
    }

    let scopes = payload
        .scopes
        .into_iter()
        .map(|scope| Scope {
            value: scope.value,
            // Check mode reports what the witness records: a complete
            // witness stays complete past the expected `ErrDiff` exit.
            // Default mode ANDs in run success so a late failure still
            // reports its validated attempted prefix as incomplete.
            results_complete: Some(if input.check {
                scope.results_complete
            } else {
                input.gazelle_ok && scope.results_complete
            }),
        })
        .collect();

    let mut files = Vec::with_capacity(payload.files.len());
    for file in &payload.files {
        let change = match (&file.create_content, &file.original_content) {
            (Some(_), Some(_)) => {
                return Err(FinalizeError::Malformed(format!(
                    "contradictory file {:?}: both create and modify content",
                    file.path,
                )));
            }
            (Some(content), None) => file_result::Change::CreateContent(content.clone()),
            (None, original) => {
                let original = original.clone().unwrap_or_default();
                let edits = file
                    .edits
                    .iter()
                    .map(|edit| Edit {
                        start_byte: edit.start_byte,
                        end_byte: edit.end_byte,
                        replacement: edit.replacement.clone(),
                    })
                    .collect();
                // The digest is finalizer-owned (O13): the extension streams
                // raw original bytes, and stamping the digest here binds them.
                file_result::Change::Modification(Modification {
                    original_digest: generation_result::digest(&original).to_vec(),
                    original_content: original,
                    edits,
                })
            }
        };
        // Reconstruct the candidate through the crate so file-local rules
        // (UTF-8, edit order/bounds, non-empty change) are enforced by the
        // same code that will validate the finished manifest.
        let probe = FileResult {
            path: file.path.clone(),
            scope_index: file.scope_index,
            change: Some(change),
            outcome: WriteOutcome::Unspecified as i32,
            failure_code: String::new(),
        };
        let intended = generation_result::candidate(&probe).map_err(|err| {
            FinalizeError::Malformed(format!(
                "file {:?} change does not apply: {err:?}",
                file.path
            ))
        })?;
        let (outcome, failure_code) = if input.check {
            (WriteOutcome::Unspecified as i32, String::new())
        } else {
            default_outcome(input.workspace, &file.path, &intended)
        };
        files.push(FileResult {
            outcome,
            failure_code,
            ..probe
        });
    }

    let ignored_imports = payload
        .ignored_imports
        .into_iter()
        .map(|ignored| IgnoredImport {
            path: ignored.path,
            language: ignored.language,
            import: ignored.import,
            scope_index: ignored.scope_index,
        })
        .collect();

    let manifest = GenerationManifest {
        schema_major: generation_result::SCHEMA_MAJOR,
        schema_minor: generation_result::SCHEMA_MINOR,
        mode: if input.check {
            Mode::Check as i32
        } else {
            Mode::Default as i32
        },
        scopes,
        files,
        ignored_imports,
    };
    generation_result::validate(&manifest).map_err(FinalizeError::Invalid)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    /// Base64 of the witness Go emits for `original "abc\n"` replaced by
    /// `"xyz\n"` inside one scope.
    fn payload(mode: &str, results_complete: bool) -> Vec<u8> {
        format!(
            concat!(
                r#"{{"schema_major":1,"schema_minor":0,"mode":"{mode}","#,
                r#""scopes":[{{"value":"//...","results_complete":{complete}}}],"#,
                r#""files":[{{"path":"rust/hello/BUILD.bazel","scope_index":0,"#,
                r#""original_content":"YWJjCg==","#,
                r#""edits":[{{"start_byte":0,"end_byte":4,"replacement":"eHl6Cg=="}}]}}],"#,
                r#""ignored_imports":[{{"path":"rust/hello/BUILD.bazel","language":"rust","#,
                r#""import":"serde","scope_index":0}}]}}"#
            ),
            mode = mode,
            complete = results_complete,
        )
        .into_bytes()
    }

    fn workspace_with(contents: &[u8]) -> tempfile_like::TempDir {
        let dir = tempfile_like::TempDir::new();
        let target = dir.path().join("rust/hello/BUILD.bazel");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, contents).unwrap();
        dir
    }

    /// Minimal tempdir helper: std-only so the finalizer itself gains no
    /// dev-dependency surface.
    mod tempfile_like {
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT: AtomicU64 = AtomicU64::new(0);

        pub struct TempDir {
            path: std::path::PathBuf,
        }

        impl TempDir {
            pub fn new() -> Self {
                let id = NEXT.fetch_add(1, Ordering::SeqCst);
                let path = std::env::temp_dir().join(format!(
                    "dx-finalize-test-{}-{}",
                    std::process::id(),
                    id
                ));
                std::fs::create_dir_all(&path).unwrap();
                TempDir { path }
            }

            pub fn path(&self) -> &std::path::Path {
                &self.path
            }
        }

        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }
    }

    #[test]
    fn default_mode_applied_when_workspace_matches() {
        let dir = workspace_with(b"xyz\n");
        let manifest = finalize(&FinalizeInput {
            intended_json: &payload("default", true),
            workspace: dir.path(),
            check: false,
            gazelle_ok: true,
        })
        .unwrap();
        assert_eq!(manifest.mode, Mode::Default as i32);
        let file = manifest.files.first().unwrap();
        assert_eq!(file.outcome, WriteOutcome::Applied as i32);
        assert!(file.failure_code.is_empty());
        assert_eq!(generation_result::candidate(file).unwrap(), b"xyz\n");
        assert!(matches!(
            file.change,
            Some(file_result::Change::Modification(_))
        ));
        let Some(file_result::Change::Modification(modification)) = &file.change else {
            panic!("expected modification change"); // LCOV_EXCL_LINE - reason: defense-in-depth; the matches! assert above pins the variant, so this else is unreachable.
        };
        assert_eq!(modification.original_content, b"abc\n");
        assert_eq!(
            modification.original_digest,
            generation_result::digest(b"abc\n").to_vec()
        );
        assert_eq!(manifest.scopes[0].results_complete, Some(true));
        assert_eq!(manifest.ignored_imports.len(), 1);
    }

    #[test]
    fn default_mode_mismatch_missing_and_unreadable_degrade() {
        let dir = workspace_with(b"stale\n");
        let run = |workspace: &Path| {
            finalize(&FinalizeInput {
                intended_json: &payload("default", true),
                workspace,
                check: false,
                gazelle_ok: true,
            })
            .unwrap()
            .files
            .pop()
            .unwrap()
        };
        let mismatched = run(dir.path());
        assert_eq!(mismatched.outcome, WriteOutcome::NotApplied as i32);
        assert_eq!(mismatched.failure_code, FAILURE_WRITE_MISMATCH);

        let missing_dir = tempfile_like::TempDir::new();
        let missing = run(missing_dir.path());
        assert_eq!(missing.outcome, WriteOutcome::NotApplied as i32);
        assert_eq!(missing.failure_code, FAILURE_MISSING_FILE);

        // A directory where the file should be is not a readable file.
        std::fs::create_dir_all(dir.path().join("rust/hello/BUILD.bazel.dir")).unwrap();
        std::fs::rename(
            dir.path().join("rust/hello/BUILD.bazel"),
            dir.path().join("rust/hello/BUILD.bazel.dir/BUILD.bazel"),
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("rust/hello/BUILD.bazel")).unwrap();
        let unreadable = run(dir.path());
        assert_eq!(unreadable.outcome, WriteOutcome::NotApplied as i32);
        assert_eq!(unreadable.failure_code, FAILURE_UNREADABLE_FILE);
    }

    #[test]
    fn check_mode_reads_no_files_and_marks_unspecified() {
        let dir = tempfile_like::TempDir::new();
        // No workspace files exist at all: check mode must still succeed.
        let manifest = finalize(&FinalizeInput {
            intended_json: &payload("check", true),
            workspace: dir.path(),
            check: true,
            gazelle_ok: true,
        })
        .unwrap();
        assert_eq!(manifest.mode, Mode::Check as i32);
        let file = manifest.files.first().unwrap();
        assert_eq!(file.outcome, WriteOutcome::Unspecified as i32);
        assert!(file.failure_code.is_empty());
        assert_eq!(manifest.scopes[0].results_complete, Some(true));
    }

    #[test]
    fn check_mode_complete_witness_finalizes_after_failed_run() {
        // Upstream `-mode diff` exits 1 (`ErrDiff`) exactly when the
        // witness carries changes, after `AfterResolvingDeps` wrote it:
        // a complete check witness is trustworthy despite the failure,
        // and check mode still reads no workspace files.
        let dir = tempfile_like::TempDir::new();
        // No workspace files exist at all: the manifest still succeeds.
        let manifest = finalize(&FinalizeInput {
            intended_json: &payload("check", true),
            workspace: dir.path(),
            check: true,
            gazelle_ok: false,
        })
        .expect("complete check witness finalizes");
        assert_eq!(manifest.mode, Mode::Check as i32);
        assert_eq!(manifest.scopes[0].results_complete, Some(true));
        let file = manifest.files.first().expect("witness file");
        assert_eq!(file.outcome, WriteOutcome::Unspecified as i32);
        assert!(file.failure_code.is_empty());
        assert_eq!(manifest.ignored_imports.len(), 1);
    }

    #[test]
    fn check_mode_rejects_incomplete_scope() {
        // A check run that did not finish a scope must not produce a manifest
        // claiming otherwise: crate validation fails the whole artifact.
        let dir = tempfile_like::TempDir::new();
        let err = finalize(&FinalizeInput {
            intended_json: &payload("check", false),
            workspace: dir.path(),
            check: true,
            gazelle_ok: true,
        })
        .unwrap_err();
        assert!(matches!(err, FinalizeError::Invalid(_)), "{err}");
    }

    #[test]
    fn create_file_finalizes() {
        let json = concat!(
            r#"{"schema_major":1,"schema_minor":0,"mode":"default","#,
            r#""scopes":[{"value":"//...","results_complete":true}],"#,
            r#""files":[{"path":"new/BUILD.bazel","scope_index":0,"#,
            r#""create_content":"eAo="}],"ignored_imports":[]}"#,
        );
        let dir = workspace_with(b"");
        std::fs::create_dir_all(dir.path().join("new")).unwrap();
        std::fs::write(dir.path().join("new/BUILD.bazel"), b"x\n").unwrap();
        let manifest = finalize(&FinalizeInput {
            intended_json: json.as_bytes(),
            workspace: dir.path(),
            check: false,
            gazelle_ok: true,
        })
        .unwrap();
        let file = manifest.files.first().unwrap();
        assert!(matches!(
            file.change,
            Some(file_result::Change::CreateContent(_))
        ));
        assert_eq!(file.outcome, WriteOutcome::Applied as i32);
    }

    #[test]
    fn rejects_garbage_schema_mode_and_shape() {
        let dir = tempfile_like::TempDir::new();
        let run = |json: &[u8], check: bool, gazelle_ok: bool| {
            finalize(&FinalizeInput {
                intended_json: json,
                workspace: dir.path(),
                check,
                gazelle_ok,
            })
            .unwrap_err()
        };
        assert!(matches!(
            run(b"not json", false, true),
            FinalizeError::Malformed(_)
        ));
        assert!(matches!(
            run(br#"{"schema_major":1}"#, false, true),
            FinalizeError::Malformed(_)
        ));
        let bad_schema = br#"{"schema_major":2,"schema_minor":0,"mode":"default"}"#;
        assert!(matches!(
            run(bad_schema, false, true),
            FinalizeError::Malformed(_)
        ));
        assert!(matches!(
            run(&payload("check", true), false, true),
            FinalizeError::Malformed(_)
        ));
        assert!(matches!(
            run(&payload("default", true), true, true),
            FinalizeError::Malformed(_)
        ));
        assert!(matches!(
            run(b"not json", true, false),
            FinalizeError::Malformed(_)
        ));
        assert!(matches!(
            run(&payload("default", true), true, false),
            FinalizeError::Malformed(_)
        ));
        assert!(matches!(
            run(&payload("check", false), true, false),
            FinalizeError::IncompleteCheck
        ));
        assert!(matches!(
            run(
                br#"{"schema_major":1,"schema_minor":0,"mode":"default","bogus":[]}"#,
                false,
                true
            ),
            FinalizeError::Malformed(_)
        ));
        let both = concat!(
            r#"{"schema_major":1,"schema_minor":0,"mode":"default","scopes":[],"#,
            r#""files":[{"path":"a","create_content":"eA==","original_content":"eQ=="}],"#,
            r#""ignored_imports":[]}"#,
        );
        let both_err = run(both.as_bytes(), false, true);
        assert!(matches!(both_err, FinalizeError::Malformed(_)));
        assert!(
            format!("{both_err}").contains("contradictory file"),
            "unexpected message: {both_err}"
        );
        let bad_b64 = concat!(
            r#"{"schema_major":1,"schema_minor":0,"mode":"default","scopes":[],"#,
            r#""files":[{"path":"a","create_content":"!!!"}],"ignored_imports":[]}"#,
        );
        assert!(matches!(
            run(bad_b64.as_bytes(), false, true),
            FinalizeError::Malformed(_)
        ));
        let bad_edits = concat!(
            r#"{"schema_major":1,"schema_minor":0,"mode":"default","scopes":[],"#,
            r#""files":[{"path":"a","original_content":"YWJjCg==","#,
            r#""edits":[{"start_byte":9,"end_byte":2,"replacement":"eA=="}]}],"#,
            r#""ignored_imports":[]}"#,
        );
        let edits_err = run(bad_edits.as_bytes(), false, true);
        assert!(matches!(edits_err, FinalizeError::Malformed(_)));
        assert!(
            format!("{edits_err}").contains("does not apply"),
            "unexpected message: {edits_err}"
        );
    }

    #[test]
    fn rejects_unsafe_paths_before_filesystem_access() {
        let dir = tempfile_like::TempDir::new();
        // Even a workspace containing a matching file must not satisfy an
        // escaping path: rejection happens before any join.
        std::fs::create_dir_all(dir.path().join("etc")).unwrap();
        std::fs::write(dir.path().join("etc/passwd"), b"xyz\n").unwrap();
        for path in ["../etc/passwd", "/etc/passwd", "", "a//b", "a/./b"] {
            let json = format!(
                concat!(
                    r#"{{"schema_major":1,"schema_minor":0,"mode":"default","scopes":[],"#,
                    r#""files":[{{"path":{path:?},"create_content":"eHl6Cg=="}}],"#,
                    r#""ignored_imports":[]}}"#
                ),
                path = path,
            );
            let err = finalize(&FinalizeInput {
                intended_json: json.as_bytes(),
                workspace: dir.path(),
                check: false,
                gazelle_ok: true,
            })
            .unwrap_err();
            assert!(matches!(err, FinalizeError::Malformed(_)), "{path}: {err}");
        }
    }

    #[test]
    fn crate_validation_still_guards_structural_rules() {
        // Scope index out of range passes JSON shape checks but must fail
        // `generation_result::validate`.
        let json = concat!(
            r#"{"schema_major":1,"schema_minor":0,"mode":"default","#,
            r#""scopes":[{"value":"//...","results_complete":true}],"#,
            r#""files":[{"path":"a","scope_index":7,"create_content":"eA=="}],"#,
            r#""ignored_imports":[]}"#,
        );
        let dir = tempfile_like::TempDir::new();
        let err = finalize(&FinalizeInput {
            intended_json: json.as_bytes(),
            workspace: dir.path(),
            check: false,
            gazelle_ok: true,
        })
        .unwrap_err();
        assert!(matches!(err, FinalizeError::Invalid(_)), "{err}");
        assert!(format!("{err}").contains("invalid generation manifest"));
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn error_display_and_incomplete_results_complete() {
        let dir = tempfile_like::TempDir::new();
        let err = finalize(&FinalizeInput {
            intended_json: &payload("check", false),
            workspace: dir.path(),
            check: true,
            gazelle_ok: false,
        })
        .unwrap_err();
        assert_eq!(
            format!("{err}"),
            "check run cannot be finalized: gazelle did not complete successfully"
        );
        assert!(std::error::Error::source(&err).is_none());

        // A scope Gazelle did not finish keeps results_complete false even
        // when the process exited zero: the witness is authoritative.
        let dir = workspace_with(b"xyz\n");
        let manifest = finalize(&FinalizeInput {
            intended_json: &payload("default", false),
            workspace: dir.path(),
            check: false,
            gazelle_ok: true,
        })
        .unwrap();
        assert_eq!(manifest.scopes[0].results_complete, Some(false));
    }

    #[test]
    fn base64_vectors() {
        assert_eq!(decode_b64("").unwrap(), b"");
        assert_eq!(decode_b64("Zg==").unwrap(), b"f");
        assert_eq!(decode_b64("Zm8=").unwrap(), b"fo");
        assert_eq!(decode_b64("Zm9v").unwrap(), b"foo");
        assert_eq!(decode_b64("Zm9vYmFy").unwrap(), b"foobar");
        assert_eq!(decode_b64("+/8=").unwrap(), vec![0xFB, 0xFF]);
        assert_eq!(decode_b64("YWJjCg==").unwrap(), b"abc\n");
        for bad in [
            "Zg=", "Z", "Zm9!", "Zm9v\n", " Zm9v", "====", "Zm9v====", "Zg==Ym8=", "Zm==Ym8=",
            "Zm=8", "=m9v", "Z=9v",
        ] {
            assert!(decode_b64(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn scope_shape_round_trip() {
        // A scope entry survives with its value and completion flag intact.
        let json = concat!(
            r#"{"schema_major":1,"schema_minor":0,"mode":"default","#,
            r#""scopes":[{"value":"//rust/...","results_complete":true}],"#,
            r#""files":[],"ignored_imports":[]}"#,
        );
        let dir = tempfile_like::TempDir::new();
        let manifest = finalize(&FinalizeInput {
            intended_json: json.as_bytes(),
            workspace: dir.path(),
            check: false,
            gazelle_ok: true,
        })
        .unwrap();
        assert_eq!(
            manifest.scopes,
            vec![Scope {
                value: "//rust/...".to_owned(),
                results_complete: Some(true),
            }]
        );
    }

    #[test]
    fn display_and_debug_formatting() {
        let err = FinalizeError::Malformed("x".to_owned());
        assert!(format!("{err:?}").contains("Malformed"));
        let mut buf = String::new();
        write!(buf, "{err}").unwrap();
        assert!(buf.contains("malformed intended manifest"));
    }
}
