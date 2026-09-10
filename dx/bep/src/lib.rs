//! Streaming Bazel Build Event Protocol collector for the `dx` CLI
//! (M06 WP2).
//!
//! Contract: `docs/quality/quality-result-protocol.md` (transport and
//! collection) and `docs/testing/environments.md` (BEP and projection
//! tests). The collector parses one newline-delimited JSON build-event
//! stream incrementally, resolves the requested output group (such as
//! `dx_results`) through BEP named sets, and reads exactly the reported
//! artifact bytes through an injected reader.
//!
//! The collector never walks `bazel-out`, never constructs artifact paths
//! from output-tree layout, and never fetches over the network: Bazel
//! materializes requested remote outputs before local collection, so a
//! non-`file://` URI (such as `bytestream://`) fails instead of triggering
//! a CLI download. Unknown event kinds are ignored for forward
//! compatibility within one stream; malformed lines, references to
//! undefined named sets, and unreadable reported files fail the whole
//! collection.
//!
//! Memory is bounded by the stream index, not by artifact contents: only
//! named-set ids with their reported URIs and one record per matching
//! completed label are retained while streaming. Artifact bytes are read
//! after the stream ends, one file at a time, so peak memory is the index
//! plus the collected result bytes. Returned records sort by label bytes
//! and artifacts sort by path bytes, so consensus never depends on BEP
//! arrival order.

use std::collections::{BTreeMap, BTreeSet};
use std::io::BufRead;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Reads reported artifact bytes from the local filesystem. Bazel owns
/// remote materialization; this seam performs no network fetch.
pub trait ArtifactReader {
    fn read_artifact(&self, path: &Path) -> std::io::Result<Vec<u8>>;
}

/// Which output group to collect, such as `dx_results`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectorConfig {
    output_group: String,
}

impl CollectorConfig {
    pub fn new(output_group: &str) -> Result<Self, BepError> {
        if output_group.is_empty() {
            return Err(BepError::EmptyOutputGroup);
        }
        Ok(CollectorConfig {
            output_group: output_group.to_owned(),
        })
    }

    pub fn output_group(&self) -> &str {
        &self.output_group
    }
}

/// One collected artifact: the local path parsed from its reported
/// `file://` URI plus its exact bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedArtifact {
    pub exec_path: PathBuf,
    pub bytes: Vec<u8>,
}

/// One completed label in the requested output group. `success=false`
/// records a failed action whose results are unavailable; valid results
/// from other keep-going actions stay available for partial reports, but
/// no mutation is allowed until complete collection is validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetOutput {
    pub label: String,
    pub success: bool,
    pub artifacts: Vec<CollectedArtifact>,
}

/// BEP collection failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BepError {
    EmptyOutputGroup,
    /// A stream line is not a JSON build-event object with the required
    /// shape. `reason` carries the parse or shape detail without secrets.
    MalformedEvent {
        line: u64,
        reason: String,
    },
    /// A referenced artifact URI that is not a local `file://` URI, such
    /// as a remote `bytestream://` that Bazel never materialized. The CLI
    /// performs no network fetch.
    UnsupportedUri {
        uri: String,
    },
    /// A completed target references a named set the stream never defined.
    /// `line` is the completion line holding the dangling reference.
    MissingNamedSet {
        id: String,
        line: u64,
    },
    /// A reported local file cannot be read.
    UnreadableArtifact {
        path: String,
        message: String,
    },
}

impl std::fmt::Display for BepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for BepError {}

struct RawFile {
    line: u64,
    uri: Option<String>,
}

struct PendingTarget {
    line: u64,
    label: String,
    success: bool,
    set_ids: Vec<String>,
}

/// Collects the requested output group from one BEP JSON stream.
///
/// `reader` yields one JSON build event per line. Returns one record per
/// completed label that requested the output group, sorted by label bytes
/// with artifacts sorted by path bytes and deduplicated. Labels that never
/// requested the group are skipped; failed labels report `success=false`
/// with no artifacts.
pub fn collect(
    reader: impl BufRead,
    config: &CollectorConfig,
    artifacts: &dyn ArtifactReader,
) -> Result<Vec<TargetOutput>, BepError> {
    let mut sets: BTreeMap<String, Vec<RawFile>> = BTreeMap::new();
    let mut pending: Vec<PendingTarget> = Vec::new();
    for (index, line) in reader.lines().enumerate() {
        let line_no = (index + 1) as u64;
        let text = line.map_err(|e| BepError::MalformedEvent {
            line: line_no,
            reason: format!("unreadable stream line: {e}"),
        })?;
        let event: Value = serde_json::from_str(&text).map_err(|e| BepError::MalformedEvent {
            line: line_no,
            reason: format!("invalid JSON: {e}"),
        })?;
        let object = event.as_object().ok_or_else(|| BepError::MalformedEvent {
            line: line_no,
            reason: "event must be a JSON object".to_owned(),
        })?;
        let id = object.get("id").and_then(Value::as_object).ok_or_else(|| {
            BepError::MalformedEvent {
                line: line_no,
                reason: "event without id".to_owned(),
            }
        })?;
        if let Some(set_id) = id
            .get("namedSetOfFiles")
            .and_then(Value::as_object)
            .and_then(|named| named.get("id"))
            .and_then(Value::as_str)
        {
            let files = object
                .get("namedSetOfFiles")
                .and_then(Value::as_object)
                .and_then(|named| named.get("files"));
            let mut raw = Vec::new();
            if let Some(files) = files {
                let files = files.as_array().ok_or_else(|| BepError::MalformedEvent {
                    line: line_no,
                    reason: "named set files must be an array".to_owned(),
                })?;
                for file in files {
                    let uri = file
                        .as_object()
                        .and_then(|entry| entry.get("uri"))
                        .and_then(Value::as_str)
                        .map(str::to_owned);
                    raw.push(RawFile { line: line_no, uri });
                }
            }
            // Named sets are immutable: the first definition wins and a
            // repeated id keeps stream order irrelevant.
            sets.entry(set_id.to_owned()).or_insert(raw);
        }
        if let Some(completed) = object.get("completed") {
            if completed.is_null() {
                continue;
            }
            let completed = completed
                .as_object()
                .ok_or_else(|| BepError::MalformedEvent {
                    line: line_no,
                    reason: "completed must be an object".to_owned(),
                })?;
            let label = id
                .get("targetCompleted")
                .and_then(Value::as_object)
                .and_then(|target| target.get("label"))
                .and_then(Value::as_str)
                .ok_or_else(|| BepError::MalformedEvent {
                    line: line_no,
                    reason: "completed event without target label".to_owned(),
                })?;
            let success = completed
                .get("success")
                .and_then(Value::as_bool)
                .ok_or_else(|| BepError::MalformedEvent {
                    line: line_no,
                    reason: "completed event without success".to_owned(),
                })?;
            let mut set_ids = Vec::new();
            if success {
                if let Some(groups) = completed.get("outputGroup") {
                    let groups = groups.as_array().ok_or_else(|| BepError::MalformedEvent {
                        line: line_no,
                        reason: "outputGroup must be an array".to_owned(),
                    })?;
                    for group in groups {
                        let group = group.as_object().ok_or_else(|| BepError::MalformedEvent {
                            line: line_no,
                            reason: "output group must be an object".to_owned(),
                        })?;
                        if group.get("name").and_then(Value::as_str) != Some(&config.output_group) {
                            continue;
                        }
                        if let Some(file_sets) = group.get("fileSets") {
                            let file_sets =
                                file_sets
                                    .as_array()
                                    .ok_or_else(|| BepError::MalformedEvent {
                                        line: line_no,
                                        reason: "fileSets must be an array".to_owned(),
                                    })?;
                            for set in file_sets {
                                let set_id = set
                                    .as_object()
                                    .and_then(|s| s.get("id"))
                                    .and_then(Value::as_str)
                                    .ok_or_else(|| BepError::MalformedEvent {
                                        line: line_no,
                                        reason: "file set without id".to_owned(),
                                    })?;
                                set_ids.push(set_id.to_owned());
                            }
                        }
                    }
                }
            }
            if success && set_ids.is_empty() && !mentions_group(completed, config.output_group()) {
                // A successful target that never requested the group is not
                // part of this collection.
                continue;
            }
            pending.push(PendingTarget {
                line: line_no,
                label: label.to_owned(),
                success,
                set_ids,
            });
        }
    }
    let mut outputs = Vec::with_capacity(pending.len());
    for target in &pending {
        let mut uris: BTreeSet<String> = BTreeSet::new();
        for set_id in &target.set_ids {
            let files = sets.get(set_id).ok_or_else(|| BepError::MissingNamedSet {
                id: set_id.clone(),
                line: target.line,
            })?;
            for file in files {
                let uri = file.uri.clone().ok_or_else(|| BepError::MalformedEvent {
                    line: file.line,
                    reason: "named set file without uri".to_owned(),
                })?;
                uris.insert(uri);
            }
        }
        let mut collected = Vec::with_capacity(uris.len());
        for uri in &uris {
            let path = file_uri_to_path(uri)?;
            let bytes =
                artifacts
                    .read_artifact(&path)
                    .map_err(|e| BepError::UnreadableArtifact {
                        path: path.display().to_string(),
                        message: e.to_string(),
                    })?;
            collected.push(CollectedArtifact {
                exec_path: path,
                bytes,
            });
        }
        collected.sort_by(|a, b| {
            a.exec_path
                .as_os_str()
                .as_encoded_bytes()
                .cmp(b.exec_path.as_os_str().as_encoded_bytes())
        });
        outputs.push(TargetOutput {
            label: target.label.clone(),
            success: target.success,
            artifacts: collected,
        });
    }
    outputs.sort_by(|a, b| a.label.as_bytes().cmp(b.label.as_bytes()));
    Ok(outputs)
}

/// Reports whether a completed object mentions the requested output group
/// by name, distinguishing "no artifacts requested" from "requested but
/// the file set list is empty".
fn mentions_group(completed: &serde_json::Map<String, Value>, group: &str) -> bool {
    completed
        .get("outputGroup")
        .and_then(Value::as_array)
        .map(|groups| {
            groups.iter().any(|g| {
                g.as_object()
                    .and_then(|g| g.get("name"))
                    .and_then(Value::as_str)
                    == Some(group)
            })
        })
        .unwrap_or(false)
}

/// Parses a reported `file://` URI into a local path without touching the
/// filesystem. Any other scheme (notably remote `bytestream://`) fails so
/// the CLI never performs a network fetch for unmaterialized outputs.
fn file_uri_to_path(uri: &str) -> Result<PathBuf, BepError> {
    let unsupported = || BepError::UnsupportedUri {
        uri: uri.to_owned(),
    };
    let rest = uri.strip_prefix("file://").ok_or_else(unsupported)?;
    let path = match rest.strip_prefix("localhost/") {
        Some(trailing) => format!("/{trailing}"),
        None => rest.to_owned(),
    };
    if !path.starts_with('/') {
        return Err(unsupported());
    }
    Ok(PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::Cursor;

    struct FakeArtifacts {
        files: HashMap<PathBuf, Vec<u8>>,
    }

    impl ArtifactReader for FakeArtifacts {
        fn read_artifact(&self, path: &Path) -> std::io::Result<Vec<u8>> {
            self.files.get(path).cloned().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "missing artifact")
            })
        }
    }

    fn config() -> CollectorConfig {
        CollectorConfig::new("dx_results").expect("group")
    }

    fn named_set(id: &str, uris: &[&str]) -> String {
        let files: Vec<String> = uris
            .iter()
            .map(|u| format!(r#"{{"name": "result.pb", "uri": "{u}"}}"#))
            .collect();
        format!(
            r#"{{"id": {{"namedSetOfFiles": {{"id": "{id}"}}}}, "namedSetOfFiles": {{"files": [{}]}}}}"#,
            files.join(",")
        )
    }

    fn completed(label: &str, success: bool, groups: &str) -> String {
        format!(
            r#"{{"id": {{"targetCompleted": {{"label": "{label}"}}}}, "completed": {{"success": {success}, "outputGroup": [{groups}]}}}}"#
        )
    }

    fn group_ref(name: &str, sets: &[&str]) -> String {
        let refs: Vec<String> = sets.iter().map(|s| format!(r#"{{"id": "{s}"}}"#)).collect();
        format!(r#"{{"name": "{name}", "fileSets": [{}]}}"#, refs.join(","))
    }

    #[test]
    fn empty_group_rejected() {
        assert_eq!(
            CollectorConfig::new("").expect_err("empty"),
            BepError::EmptyOutputGroup
        );
    }

    #[test]
    fn display_renders_debug_shape() {
        assert_eq!(BepError::EmptyOutputGroup.to_string(), "EmptyOutputGroup");
    }

    #[test]
    fn unreadable_stream_lines_fail() {
        struct FailRead;
        impl std::io::Read for FailRead {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "boom"))
            }
        }
        let artifacts = FakeArtifacts {
            files: HashMap::new(),
        };
        let err = collect(std::io::BufReader::new(FailRead), &config(), &artifacts)
            .expect_err("unreadable line");
        assert!(matches!(err, BepError::MalformedEvent { .. }));
    }

    #[test]
    fn named_set_files_must_be_array() {
        let stream =
            r#"{"id": {"namedSetOfFiles": {"id": "1"}}, "namedSetOfFiles": {"files": {}}}"#;
        let artifacts = FakeArtifacts {
            files: HashMap::new(),
        };
        let err = collect(Cursor::new(stream), &config(), &artifacts).expect_err("object files");
        assert!(matches!(err, BepError::MalformedEvent { .. }));
    }

    #[test]
    fn named_set_without_files_resolves_empty() {
        let stream = [
            r#"{"id": {"namedSetOfFiles": {"id": "1"}}}"#.to_owned(),
            completed("//q:a", true, &group_ref("dx_results", &["1"])),
        ]
        .join("\n");
        let artifacts = FakeArtifacts {
            files: HashMap::new(),
        };
        let got = collect(Cursor::new(stream), &config(), &artifacts).expect("collect");
        assert_eq!(got.len(), 1);
        assert!(got[0].artifacts.is_empty());
    }

    #[test]
    fn null_completed_events_are_ignored() {
        let stream = [
            r#"{"id": {"targetCompleted": {"label": "//q:a"}}, "completed": null}"#.to_owned(),
            named_set("1", &["file:///out/a.pb"]),
            completed("//q:a", true, &group_ref("dx_results", &["1"])),
        ]
        .join("\n");
        let artifacts = FakeArtifacts {
            files: HashMap::from([(PathBuf::from("/out/a.pb"), vec![9])]),
        };
        let got = collect(Cursor::new(stream), &config(), &artifacts).expect("collect");
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn malformed_completed_events_fail() {
        let artifacts = FakeArtifacts {
            files: HashMap::new(),
        };
        let stream = r#"{"id": {"targetCompleted": {"label": "//q:a"}}, "completed": []}"#;
        assert!(matches!(
            collect(Cursor::new(stream), &config(), &artifacts),
            Err(BepError::MalformedEvent { .. })
        ));
        let stream = r#"{"id": {}, "completed": {"success": true}}"#;
        assert!(matches!(
            collect(Cursor::new(stream), &config(), &artifacts),
            Err(BepError::MalformedEvent { .. })
        ));
        let stream = r#"{"id": {"targetCompleted": {"label": "//q:a"}}, "completed": {}}"#;
        assert!(matches!(
            collect(Cursor::new(stream), &config(), &artifacts),
            Err(BepError::MalformedEvent { .. })
        ));
    }

    #[test]
    fn malformed_output_groups_fail() {
        let artifacts = FakeArtifacts {
            files: HashMap::new(),
        };
        let stream = r#"{"id": {"targetCompleted": {"label": "//q:a"}}, "completed": {"success": true, "outputGroup": {}}}"#;
        assert!(matches!(
            collect(Cursor::new(stream), &config(), &artifacts),
            Err(BepError::MalformedEvent { .. })
        ));
        let stream = r#"{"id": {"targetCompleted": {"label": "//q:a"}}, "completed": {"success": true, "outputGroup": ["dx_results"]}}"#;
        assert!(matches!(
            collect(Cursor::new(stream), &config(), &artifacts),
            Err(BepError::MalformedEvent { .. })
        ));
        let stream = r#"{"id": {"targetCompleted": {"label": "//q:a"}}, "completed": {"success": true, "outputGroup": [{"name": "dx_results", "fileSets": {}}]}}"#;
        assert!(matches!(
            collect(Cursor::new(stream), &config(), &artifacts),
            Err(BepError::MalformedEvent { .. })
        ));
        let stream = r#"{"id": {"targetCompleted": {"label": "//q:a"}}, "completed": {"success": true, "outputGroup": [{"name": "dx_results", "fileSets": [{}]}]}}"#;
        assert!(matches!(
            collect(Cursor::new(stream), &config(), &artifacts),
            Err(BepError::MalformedEvent { .. })
        ));
    }

    #[test]
    fn successful_target_without_group_is_skipped() {
        let stream = [
            r#"{"id": {"targetCompleted": {"label": "//q:a"}}, "completed": {"success": true}}"#
                .to_owned(),
        ]
        .join("\n");
        let artifacts = FakeArtifacts {
            files: HashMap::new(),
        };
        let got = collect(Cursor::new(stream), &config(), &artifacts).expect("collect");
        assert!(got.is_empty());
    }

    #[test]
    fn failed_targets_report_without_artifacts() {
        let stream = [completed("//q:a", false, &group_ref("dx_results", &["1"]))].join("\n");
        let artifacts = FakeArtifacts {
            files: HashMap::new(),
        };
        let got = collect(Cursor::new(stream), &config(), &artifacts).expect("collect");
        assert_eq!(got.len(), 1);
        assert!(!got[0].success);
        assert!(got[0].artifacts.is_empty());
    }

    #[test]
    fn group_without_filesets_reports_no_artifacts() {
        let stream = [completed("//q:a", true, r#"{"name": "dx_results"}"#)].join("\n");
        let artifacts = FakeArtifacts {
            files: HashMap::new(),
        };
        let got = collect(Cursor::new(stream), &config(), &artifacts).expect("collect");
        assert_eq!(got.len(), 1);
        assert!(got[0].success);
        assert!(got[0].artifacts.is_empty());
    }

    #[test]
    fn named_set_file_without_uri_fails() {
        let stream = [
            r#"{"id": {"namedSetOfFiles": {"id": "1"}}, "namedSetOfFiles": {"files": [{}]}}"#
                .to_owned(),
            completed("//q:a", true, &group_ref("dx_results", &["1"])),
        ]
        .join("\n");
        let artifacts = FakeArtifacts {
            files: HashMap::new(),
        };
        let err = collect(Cursor::new(stream), &config(), &artifacts).expect_err("uri missing");
        assert!(matches!(err, BepError::MalformedEvent { .. }));
    }

    #[test]
    fn artifacts_sort_by_path_bytes() {
        let stream = [
            named_set("1", &["file:///out/b.pb", "file:///out/a.pb"]),
            completed("//q:a", true, &group_ref("dx_results", &["1"])),
        ]
        .join("\n");
        let artifacts = FakeArtifacts {
            files: HashMap::from([
                (PathBuf::from("/out/a.pb"), vec![1]),
                (PathBuf::from("/out/b.pb"), vec![2]),
            ]),
        };
        let got = collect(Cursor::new(stream), &config(), &artifacts).expect("collect");
        assert_eq!(got[0].artifacts.len(), 2);
        assert_eq!(got[0].artifacts[0].exec_path, PathBuf::from("/out/a.pb"));
        assert_eq!(got[0].artifacts[1].exec_path, PathBuf::from("/out/b.pb"));
    }

    #[test]
    fn collects_reported_artifacts_only() {
        let stream = [
            named_set("1", &["file:///out/a.pb"]),
            named_set("2", &["file:///out/unrelated.pb"]),
            completed("//q:a", true, &group_ref("dx_results", &["1"])),
            completed("//q:b", true, &group_ref("other_group", &["2"])),
        ]
        .join("\n");
        let artifacts = FakeArtifacts {
            files: HashMap::from([(PathBuf::from("/out/a.pb"), vec![1, 2, 3])]),
        };
        let got = collect(Cursor::new(stream), &config(), &artifacts).expect("collect");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].label, "//q:a");
        assert!(got[0].success);
        assert_eq!(got[0].artifacts.len(), 1);
        assert_eq!(got[0].artifacts[0].exec_path, PathBuf::from("/out/a.pb"));
        assert_eq!(got[0].artifacts[0].bytes, vec![1, 2, 3]);
    }

    #[test]
    fn records_sort_by_label_bytes() {
        let stream = [
            named_set("1", &["file:///out/b.pb"]),
            named_set("2", &["file:///out/a.pb"]),
            completed("//q:b", true, &group_ref("dx_results", &["1"])),
            completed("//q:a", true, &group_ref("dx_results", &["2"])),
        ]
        .join("\n");
        let artifacts = FakeArtifacts {
            files: HashMap::from([
                (PathBuf::from("/out/a.pb"), vec![1]),
                (PathBuf::from("/out/b.pb"), vec![2]),
            ]),
        };
        let got = collect(Cursor::new(stream), &config(), &artifacts).expect("collect");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].label, "//q:a");
        assert_eq!(got[1].label, "//q:b");
    }

    #[test]
    fn failed_targets_keep_partial_records_without_reads() {
        let stream = completed("//q:a", false, "");
        let artifacts = FakeArtifacts {
            files: HashMap::new(),
        };
        let got = collect(Cursor::new(stream), &config(), &artifacts).expect("collect");
        assert_eq!(got.len(), 1);
        assert!(!got[0].success);
        assert!(got[0].artifacts.is_empty());
    }

    #[test]
    fn successful_targets_without_group_are_skipped() {
        let stream = completed("//q:a", true, &group_ref("other_group", &["9"]));
        let artifacts = FakeArtifacts {
            files: HashMap::new(),
        };
        let got = collect(Cursor::new(stream), &config(), &artifacts).expect("collect");
        assert!(got.is_empty());
    }

    #[test]
    fn malformed_lines_fail_collection() {
        let artifacts = FakeArtifacts {
            files: HashMap::new(),
        };
        let err = collect(Cursor::new("not json\n"), &config(), &artifacts).expect_err("malformed");
        assert!(matches!(err, BepError::MalformedEvent { line: 1, .. }));
        let err = collect(Cursor::new("[1,2]\n"), &config(), &artifacts).expect_err("array");
        assert!(matches!(err, BepError::MalformedEvent { .. }));
        let err = collect(
            Cursor::new(r#"{"completed": {"success": true}}"#),
            &config(),
            &artifacts,
        )
        .expect_err("no id");
        assert!(matches!(err, BepError::MalformedEvent { .. }));
    }

    #[test]
    fn missing_named_set_fails() {
        let stream = completed("//q:a", true, &group_ref("dx_results", &["ghost"]));
        let artifacts = FakeArtifacts {
            files: HashMap::new(),
        };
        assert_eq!(
            collect(Cursor::new(stream), &config(), &artifacts).expect_err("missing set"),
            BepError::MissingNamedSet {
                id: "ghost".to_owned(),
                line: 1,
            }
        );
    }

    #[test]
    fn remote_uris_fail_without_network_fetch() {
        let stream = [
            named_set("1", &["bytestream://remote/cache/a.pb"]),
            completed("//q:a", true, &group_ref("dx_results", &["1"])),
        ]
        .join("\n");
        struct NoReadReader {
            reads: std::cell::Cell<usize>,
        }
        impl ArtifactReader for NoReadReader {
            fn read_artifact(&self, _path: &Path) -> std::io::Result<Vec<u8>> {
                self.reads.set(self.reads.get() + 1);
                Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "reader must not be reached",
                ))
            }
        }
        let reader = NoReadReader {
            reads: std::cell::Cell::new(0),
        };
        let err = collect(Cursor::new(stream), &config(), &reader).expect_err("remote");
        assert_eq!(
            err,
            BepError::UnsupportedUri {
                uri: "bytestream://remote/cache/a.pb".to_owned()
            }
        );
        assert_eq!(reader.reads.get(), 0);
        // The double itself records calls and fails loudly.
        let probe = NoReadReader {
            reads: std::cell::Cell::new(0),
        };
        assert!(probe.read_artifact(Path::new("/out/a.pb")).is_err());
        assert_eq!(probe.reads.get(), 1);
    }

    #[test]
    fn unreadable_reported_files_fail() {
        let stream = [
            named_set("1", &["file:///out/missing.pb"]),
            completed("//q:a", true, &group_ref("dx_results", &["1"])),
        ]
        .join("\n");
        let artifacts = FakeArtifacts {
            files: HashMap::new(),
        };
        let err = collect(Cursor::new(stream), &config(), &artifacts).expect_err("missing");
        assert!(matches!(err, BepError::UnreadableArtifact { .. }));
    }

    #[test]
    fn duplicate_artifacts_deduplicated() {
        let stream = [
            named_set("1", &["file:///out/a.pb"]),
            named_set("2", &["file:///out/a.pb"]),
            completed("//q:a", true, &group_ref("dx_results", &["1", "2"])),
        ]
        .join("\n");
        struct CountingReader {
            reads: std::cell::Cell<usize>,
        }
        impl ArtifactReader for CountingReader {
            fn read_artifact(&self, _path: &Path) -> std::io::Result<Vec<u8>> {
                self.reads.set(self.reads.get() + 1);
                Ok(vec![7])
            }
        }
        let reader = CountingReader {
            reads: std::cell::Cell::new(0),
        };
        let got = collect(Cursor::new(stream), &config(), &reader).expect("collect");
        assert_eq!(got[0].artifacts.len(), 1);
        assert_eq!(reader.reads.get(), 1);
    }

    #[test]
    fn unknown_events_are_ignored() {
        let stream = [
            r#"{"id": {"progress": {}}, "progress": {"stdout": "noise"}}"#.to_owned(),
            r#"{"id": {"buildFinished": {}}, "finished": {"success": true}}"#.to_owned(),
            named_set("1", &["file:///out/a.pb"]),
            completed("//q:a", true, &group_ref("dx_results", &["1"])),
        ]
        .join("\n");
        let artifacts = FakeArtifacts {
            files: HashMap::from([(PathBuf::from("/out/a.pb"), vec![9])]),
        };
        let got = collect(Cursor::new(stream), &config(), &artifacts).expect("collect");
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn set_definition_order_does_not_matter() {
        // Completion before the named-set definition resolves identically.
        let stream = [
            completed("//q:a", true, &group_ref("dx_results", &["1"])),
            named_set("1", &["file:///out/a.pb"]),
        ]
        .join("\n");
        let artifacts = FakeArtifacts {
            files: HashMap::from([(PathBuf::from("/out/a.pb"), vec![5])]),
        };
        let got = collect(Cursor::new(stream), &config(), &artifacts).expect("collect");
        assert_eq!(got[0].artifacts[0].bytes, vec![5]);
    }

    #[test]
    fn file_uri_forms() {
        assert_eq!(
            file_uri_to_path("file:///out/a.pb").expect("abs"),
            PathBuf::from("/out/a.pb")
        );
        assert_eq!(
            file_uri_to_path("file://localhost/out/a.pb").expect("localhost"),
            PathBuf::from("/out/a.pb")
        );
        assert!(file_uri_to_path("bytestream://x").is_err());
        assert!(file_uri_to_path("file://relative/path").is_err());
        assert!(file_uri_to_path("/plain/path").is_err());
    }
}
