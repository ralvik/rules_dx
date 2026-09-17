//! BEP test-action output collection (`testResult` events).
//!
//! Owns the `collect_test_outputs` domain of the streaming BEP collector:
//! one record per `testActionOutput` entry of every `testResult` event,
//! sorted by label bytes, then name bytes, then 1-based run, shard, and
//! attempt. Entries without a label, name, or `file://` URI fail the whole
//! collection with a JSON-path-annotated [`crate::BepError::MalformedEvent`];
//! unknown event kinds are ignored. Remote URIs fail with
//! [`crate::BepError::UnsupportedUri`]: the CLI never fetches
//! unmaterialized outputs over the network.
//!
//! Shared collection shape (malformed-line helper, `file://` URI parsing,
//! [`crate::BepError`]) stays in the facade and is shared via `pub(crate)`
//! re-exports; the output-group/named-set collection (`collect`,
//! `RawFile`/`RawSet`/`PendingTarget`) stays in the facade.

use std::io::BufRead;
use std::path::PathBuf;

use serde_json::Value;

use super::{file_uri_to_path, malformed, BepError};

/// One test-action output file reported by a BEP `testResult` event:
/// the owning test label plus the output entry name (such as
/// `test.xml`, `test.log`, or `test.lcov`) and the local path parsed
/// from its reported `file://` URI. `run`, `shard`, and `attempt` carry
/// the 1-based BEP `testResult` identity (`id.testResult.{run,shard,
/// attempt}`, defaulting to 1 when absent) so JUnit normalization can
/// order retries/shards deterministically and suffix zero-based
/// `[shard=…,attempt=…]` display names. Bytes are read later through
/// [`crate::ArtifactReader`] by the report collector that owns the name
/// contract, so collection here stays name-agnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestOutputFile {
    pub label: String,
    pub name: String,
    pub exec_path: PathBuf,
    pub run: u32,
    pub shard: u32,
    pub attempt: u32,
}

/// Typed view of a BEP `id.testResult` object: the owning test label
/// plus the 1-based run/shard/attempt identity. Unknown fields are
/// ignored so newer Bazel servers stay forward compatible; only the
/// consumed shape is validated, with failures annotated by JSON path
/// (see [`malformed`]).
///
/// `serde_path_to_error` evaluation (issue #231): rejected. The path
/// annotation here needs no new dependency and no manifest/lock churn,
/// while derived `Deserialize` would still require `default` on every
/// forward-compatible field and could not express the "absent means 1"
/// identity rule or the "missing `testResult` means ignore" filter as
/// directly as this targeted parser.
struct TestResultId<'a> {
    label: &'a str,
    run: u32,
    shard: u32,
    attempt: u32,
}

impl<'a> TestResultId<'a> {
    fn parse(id: &'a serde_json::Map<String, Value>, line: u64) -> Result<Self, BepError> {
        let label = id
            .get("label")
            .and_then(Value::as_str)
            .ok_or_else(|| malformed(line, "id.testResult.label", "test result without label"))?;
        Ok(TestResultId {
            label,
            run: test_index(id, "run", "id.testResult.run", line)?,
            shard: test_index(id, "shard", "id.testResult.shard", line)?,
            attempt: test_index(id, "attempt", "id.testResult.attempt", line)?,
        })
    }
}

/// Typed view of one `testResult.testActionOutput` entry: the output
/// name plus its reported URI. Parsing stays name-agnostic: the report
/// collector that owns the name contract reads bytes later through
/// [`crate::ArtifactReader`].
struct TestActionOutput<'a> {
    name: &'a str,
    uri: &'a str,
}

impl<'a> TestActionOutput<'a> {
    fn parse(file: &'a Value, index: usize, line: u64) -> Result<Self, BepError> {
        let base = format!("testResult.testActionOutput[{index}]");
        let entry = file
            .as_object()
            .ok_or_else(|| malformed(line, &base, "test action output must be an object"))?;
        let name = entry.get("name").and_then(Value::as_str).ok_or_else(|| {
            malformed(
                line,
                &format!("{base}.name"),
                "test action output without name",
            )
        })?;
        let uri = entry.get("uri").and_then(Value::as_str).ok_or_else(|| {
            malformed(
                line,
                &format!("{base}.uri"),
                "test action output without uri",
            )
        })?;
        Ok(TestActionOutput { name, uri })
    }
}

/// Collects test-action outputs from one BEP JSON stream.
///
/// `reader` yields one JSON build event per line. Returns one record
/// per `testActionOutput` entry of every `testResult` event, sorted by
/// label bytes, then name bytes, then 1-based run, shard, and attempt.
/// Entries without a label, name, or `file://` URI fail the whole
/// collection with a JSON-path-annotated [`BepError::MalformedEvent`]
/// (such as `testResult.testActionOutput[2].uri`); unknown event kinds
/// are ignored. Remote URIs fail with [`BepError::UnsupportedUri`]: the
/// CLI never fetches unmaterialized outputs over the network.
pub fn collect_test_outputs(reader: impl BufRead) -> Result<Vec<TestOutputFile>, BepError> {
    let mut outputs = Vec::new();
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
        let result_id = id.get("testResult").and_then(Value::as_object);
        let Some(result_id) = result_id else {
            continue;
        };
        let label = TestResultId::parse(result_id, line_no)?;
        let result = object.get("testResult").and_then(Value::as_object);
        let Some(files) = result.and_then(|result| result.get("testActionOutput")) else {
            continue;
        };
        let files = files.as_array().ok_or_else(|| {
            malformed(
                line_no,
                "testResult.testActionOutput",
                "testActionOutput must be an array",
            )
        })?;
        for (index, file) in files.iter().enumerate() {
            let entry = TestActionOutput::parse(file, index, line_no)?;
            outputs.push(TestOutputFile {
                label: label.label.to_owned(),
                name: entry.name.to_owned(),
                exec_path: file_uri_to_path(entry.uri)?,
                run: label.run,
                shard: label.shard,
                attempt: label.attempt,
            });
        }
    }
    outputs.sort_by(|a, b| {
        a.label
            .as_bytes()
            .cmp(b.label.as_bytes())
            .then(a.name.as_bytes().cmp(b.name.as_bytes()))
            .then(a.run.cmp(&b.run))
            .then(a.shard.cmp(&b.shard))
            .then(a.attempt.cmp(&b.attempt))
    });
    Ok(outputs)
}

/// Parses a 1-based BEP `testResult` identity field (`run`, `shard`,
/// or `attempt`). Absent fields default to 1; present fields must be
/// integers >= 1, otherwise the whole collection fails with a
/// JSON-path-annotated [`BepError::MalformedEvent`] at `path`.
fn test_index(
    result_id: &serde_json::Map<String, Value>,
    field: &str,
    path: &str,
    line: u64,
) -> Result<u32, BepError> {
    match result_id.get(field) {
        None => Ok(1),
        Some(value) => {
            let raw = value.as_u64().ok_or_else(|| {
                malformed(
                    line,
                    path,
                    &format!("test result {field} must be a positive integer"),
                )
            })?;
            u32::try_from(raw)
                .ok()
                .filter(|index| *index >= 1)
                .ok_or_else(|| {
                    malformed(
                        line,
                        path,
                        &format!("test result {field} must be a positive integer"),
                    )
                })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn test_result(label: &str, outputs: &[(&str, &str)]) -> String {
        let entries = outputs
            .iter()
            .map(|(name, uri)| format!(r#"{{"name":{name:?},"uri":{uri:?}}}"#))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"id":{{"testResult":{{"label":{label:?},"run":1,"shard":1,"attempt":1}}}},"testResult":{{"status":"PASSED","testActionOutput":[{entries}]}}}}"#
        )
    }

    #[test]
    fn test_outputs_collect_sorted_records() {
        let stream = [
            test_result(
                "//z:t",
                &[
                    ("test.xml", "file:///out/z/test.xml"),
                    ("test.log", "file:///out/z/test.log"),
                ],
            ),
            r#"{"id":{"targetCompleted":{"label":"//z:t"}},"completed":{"success":true}}"#
                .to_owned(),
            test_result("//a:t", &[("test.xml", "file:///out/a/test.xml")]),
        ]
        .join("\n");
        let got = collect_test_outputs(Cursor::new(stream)).expect("collect");
        assert_eq!(
            got,
            vec![
                TestOutputFile {
                    label: "//a:t".to_owned(),
                    name: "test.xml".to_owned(),
                    exec_path: PathBuf::from("/out/a/test.xml"),
                    run: 1,
                    shard: 1,
                    attempt: 1,
                },
                TestOutputFile {
                    label: "//z:t".to_owned(),
                    name: "test.log".to_owned(),
                    exec_path: PathBuf::from("/out/z/test.log"),
                    run: 1,
                    shard: 1,
                    attempt: 1,
                },
                TestOutputFile {
                    label: "//z:t".to_owned(),
                    name: "test.xml".to_owned(),
                    exec_path: PathBuf::from("/out/z/test.xml"),
                    run: 1,
                    shard: 1,
                    attempt: 1,
                },
            ]
        );
    }

    fn test_result_with_identity(
        label: &str,
        run: u32,
        shard: u32,
        attempt: u32,
        outputs: &[(&str, &str)],
    ) -> String {
        let entries = outputs
            .iter()
            .map(|(name, uri)| format!(r#"{{"name":{name:?},"uri":{uri:?}}}"#))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"id":{{"testResult":{{"label":{label:?},"run":{run},"shard":{shard},"attempt":{attempt}}}}},"testResult":{{"status":"PASSED","testActionOutput":[{entries}]}}}}"#
        )
    }

    #[test]
    fn test_outputs_capture_run_shard_attempt() {
        let stream = [
            test_result_with_identity("//a:t", 2, 3, 4, &[("test.xml", "file:///out/a.xml")]),
            test_result_with_identity("//a:t", 1, 1, 2, &[("test.xml", "file:///out/b.xml")]),
            test_result_with_identity("//a:t", 1, 1, 1, &[("test.xml", "file:///out/c.xml")]),
        ]
        .join("\n");
        let got = collect_test_outputs(Cursor::new(stream)).expect("collect");
        assert_eq!(
            got.iter()
                .map(|output| (output.run, output.shard, output.attempt))
                .collect::<Vec<_>>(),
            vec![(1, 1, 1), (1, 1, 2), (2, 3, 4)],
            "records sort by run, shard, then attempt"
        );
        assert_eq!(got[0].exec_path, PathBuf::from("/out/c.xml"));
        assert_eq!(got[2].exec_path, PathBuf::from("/out/a.xml"));
    }

    #[test]
    fn test_outputs_default_missing_identity_to_one() {
        let stream = r#"{"id":{"testResult":{"label":"//a:t"}},"testResult":{"status":"PASSED","testActionOutput":[{"name":"test.xml","uri":"file:///out/a.xml"}]}}"#;
        let got = collect_test_outputs(Cursor::new(stream)).expect("collect");
        assert_eq!(got.len(), 1);
        assert_eq!((got[0].run, got[0].shard, got[0].attempt), (1, 1, 1));
    }

    #[test]
    fn test_outputs_reject_non_positive_identity() {
        for field in ["run", "shard", "attempt"] {
            let stream = format!(
                r#"{{"id":{{"testResult":{{"label":"//a:t","{field}":0}}}},"testResult":{{"status":"PASSED","testActionOutput":[{{"name":"test.xml","uri":"file:///out/a.xml"}}]}}}}"#
            );
            assert!(
                collect_test_outputs(Cursor::new(stream)).is_err(),
                "{field}=0 must fail"
            );
            let stream = format!(
                r#"{{"id":{{"testResult":{{"label":"//a:t","{field}":"one"}}}},"testResult":{{"status":"PASSED","testActionOutput":[{{"name":"test.xml","uri":"file:///out/a.xml"}}]}}}}"#
            );
            assert!(
                collect_test_outputs(Cursor::new(stream)).is_err(),
                "{field} string must fail"
            );
        }
    }

    #[test]
    fn test_outputs_without_entries_or_results_are_ignored() {
        let stream = [
            r#"{"id":{"progress":{}}}"#.to_owned(),
            test_result("//a:t", &[]),
        ]
        .join("\n");
        let got = collect_test_outputs(Cursor::new(stream)).expect("collect");
        assert!(got.is_empty());
    }

    #[test]
    fn test_output_shape_failures_fail_collection() {
        for stream in [
            "not json".to_owned(),
            r#"{"no-id":true}"#.to_owned(),
            r#"{"id":{"testResult":{}},"testResult":{"testActionOutput":[]}}"#.to_owned(),
            test_result("//a:t", &[("test.xml", "bytestream://remote/1")]),
            // Non-object event, non-array testActionOutput, non-object entry,
            // missing name, and missing uri all fail closed.
            "[]".to_owned(),
            r#"{"id":{"testResult":{"label":"//a:t"}},"testResult":{"testActionOutput":{}}}"#
                .to_owned(),
            r#"{"id":{"testResult":{"label":"//a:t"}},"testResult":{"testActionOutput":["x"]}}"#
                .to_owned(),
            r#"{"id":{"testResult":{"label":"//a:t"}},"testResult":{"testActionOutput":[{"uri":"file:///out/a.xml"}]}}"#
                .to_owned(),
            r#"{"id":{"testResult":{"label":"//a:t"}},"testResult":{"testActionOutput":[{"name":"test.xml"}]}}"#
                .to_owned(),
        ] {
            assert!(
                collect_test_outputs(Cursor::new(stream)).is_err(),
                "must fail closed"
            );
        }
    }

    #[test]
    fn test_output_malformed_reasons_carry_json_paths() {
        // Malformed-line corpus (issue #231): every `testResult`
        // shape failure names the offending field by JSON path while
        // keeping the legacy human-readable wording.
        let cases = [
            (
                r#"{"id":{"testResult":{}},"testResult":{"testActionOutput":[]}}"#,
                "id.testResult.label",
                "test result without label",
            ),
            (
                r#"{"id":{"testResult":{"label":"//a:t","run":0}},"testResult":{"testActionOutput":[]}}"#,
                "id.testResult.run",
                "must be a positive integer",
            ),
            (
                r#"{"id":{"testResult":{"label":"//a:t","shard":"one"}},"testResult":{"testActionOutput":[]}}"#,
                "id.testResult.shard",
                "must be a positive integer",
            ),
            (
                r#"{"id":{"testResult":{"label":"//a:t","attempt":4294967296}},"testResult":{"testActionOutput":[]}}"#,
                "id.testResult.attempt",
                "must be a positive integer",
            ),
            (
                r#"{"id":{"testResult":{"label":"//a:t"}},"testResult":{"testActionOutput":{}}}"#,
                "testResult.testActionOutput",
                "must be an array",
            ),
            (
                r#"{"id":{"testResult":{"label":"//a:t"}},"testResult":{"testActionOutput":["x"]}}"#,
                "testResult.testActionOutput[0]",
                "must be an object",
            ),
            (
                r#"{"id":{"testResult":{"label":"//a:t"}},"testResult":{"testActionOutput":[{"uri":"file:///out/a.xml"}]}}"#,
                "testResult.testActionOutput[0].name",
                "without name",
            ),
            (
                r#"{"id":{"testResult":{"label":"//a:t"}},"testResult":{"testActionOutput":[{"name":"test.xml"}]}}"#,
                "testResult.testActionOutput[0].uri",
                "without uri",
            ),
        ];
        for (stream, path, wording) in cases {
            let err = collect_test_outputs(Cursor::new(stream)).expect_err("must fail closed");
            let BepError::MalformedEvent { line, reason } = err else {
                panic!("want MalformedEvent, got {err:?}");
            };
            assert_eq!(line, 1);
            assert!(
                reason.contains(path),
                "reason {reason:?} must name JSON path {path}"
            );
            assert!(
                reason.contains(wording),
                "reason {reason:?} must keep wording {wording:?}"
            );
        }
    }

    #[test]
    fn test_outputs_unreadable_stream_fails() {
        struct FailRead;
        impl std::io::Read for FailRead {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "boom"))
            }
        }
        assert!(collect_test_outputs(std::io::BufReader::new(FailRead)).is_err());
    }
}
