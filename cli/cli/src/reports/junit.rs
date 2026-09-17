//! JUnit report parsing and rendering (issue #236).
//!
//! Split from `super` (`reports.rs`): owns `JunitCase`,
//! `JunitMessage`, `parse_test_xml`, `render_junit`, and
//! `junit_infrastructure_case`. Re-exported through `super` so the
//! public path stays `crate::reports::{...}`.

use super::ReportError;

/// One Bazel-reported test case normalized for JUnit rendering.
/// `shard` and `attempt` are zero-based indices derived from the 1-based
/// BEP `testResult` identity (`shard = bep_shard - 1`).
#[derive(Debug, Clone, PartialEq)]
pub struct JunitCase {
    pub name: String,
    pub classname: Option<String>,
    pub time: f64,
    pub failure: Option<JunitMessage>,
    pub error: Option<JunitMessage>,
    pub skipped: Option<JunitMessage>,
    pub system_out: Option<String>,
    pub system_err: Option<String>,
    pub shard: u32,
    pub attempt: u32,
}

/// Message plus body preserved from a Bazel-reported
/// `<failure>`, `<error>`, or `<skipped>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JunitMessage {
    pub message: Option<String>,
    pub text: String,
}

fn junit_error(detail: impl Into<String>) -> ReportError {
    ReportError::InvalidJunit {
        detail: detail.into(),
    }
}

/// Drops characters forbidden in XML 1.0 documents (`<0x20` except
/// `\t`, `\n`, `\r`) so the quick-xml writer always emits well-formed
/// documents even when test names or output capture control bytes.
/// Forbidden codepoints become U+FFFD to keep the output deterministic
/// and visibly lossy rather than silently concatenating.
fn sanitize_xml(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c == '\t' || c == '\n' || c == '\r' || c >= '\u{20}' {
                // `char` never holds surrogates; exclude non-characters
                // that XML 1.0 forbids (`FFFE`/`FFFF` planes).
                let v = c as u32;
                if v == 0xFFFE || v == 0xFFFF {
                    '\u{FFFD}'
                } else {
                    c
                }
            } else {
                '\u{FFFD}'
            }
        })
        .collect()
}

fn format_junit_time(time: f64) -> String {
    if !time.is_finite() || time < 0.0 {
        return "0".to_owned();
    }
    format!("{time:.3}")
}

/// Display name for one case: the Bazel-provided name, plus a
/// zero-based `[shard=…,attempt=…]` suffix when either index is
/// nonzero. Ordering uses the original name plus indices, so an
/// existing identical display name stays disambiguated by indices
/// rather than encounter order.
fn junit_display_name(name: &str, shard: u32, attempt: u32) -> String {
    if shard == 0 && attempt == 0 {
        name.to_owned()
    } else {
        format!("{name} [shard={shard},attempt={attempt}]")
    }
}

fn junit_attr(
    element: &quick_xml::events::BytesStart<'_>,
    name: &[u8],
) -> Result<Option<String>, ReportError> {
    for attr in element.attributes() {
        let attr = attr.map_err(|e| junit_error(format!("malformed testcase attribute: {e}")))?;
        if attr.key.as_ref() == name {
            let value = attr
                .unescape_value()
                .map_err(|e| junit_error(format!("malformed testcase attribute: {e}")))?;
            return Ok(Some(value.into_owned()));
        }
    }
    Ok(None)
}

/// Parses one Bazel-reported `test.xml` artifact into normalized cases.
///
/// `shard` and `attempt` are the zero-based indices for every case in
/// `bytes` (derived from the BEP identity). Names, durations,
/// `<failure>`, `<error>`, `<skipped>`, `<system-out>`, and
/// `<system-err>` content are preserved after safe XML parsing; suite
/// structure is ignored because the caller groups by Bazel target
/// label. Malformed XML fails the whole artifact so the caller can
/// mark collection partial.
pub fn parse_test_xml(
    bytes: &[u8],
    shard: u32,
    attempt: u32,
) -> Result<Vec<JunitCase>, ReportError> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let text = std::str::from_utf8(bytes)
        .map_err(|e| junit_error(format!("test XML is not UTF-8: {e}")))?;
    // Reject documents with no element structure early so empty or
    // whitespace-only artifacts fail closed instead of yielding zero
    // cases that look like a passing suite.
    if !text.contains('<') {
        return Err(junit_error("test XML has no elements"));
    }
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);
    reader.config_mut().check_end_names = true;

    #[derive(Debug)]
    struct ActiveCase {
        name: String,
        classname: Option<String>,
        time: f64,
        failure: Option<JunitMessage>,
        error: Option<JunitMessage>,
        skipped: Option<JunitMessage>,
        system_out: Option<String>,
        system_err: Option<String>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ChildKind {
        Failure,
        Error,
        Skipped,
        SystemOut,
        SystemErr,
    }

    struct ActiveChild {
        kind: ChildKind,
        message: Option<String>,
        text: String,
    }

    let mut cases: Vec<JunitCase> = Vec::new();
    let mut active: Option<ActiveCase> = None;
    let mut child: Option<ActiveChild> = None;
    let mut depth: usize = 0;
    let mut buf = Vec::new();
    loop {
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| junit_error(format!("malformed test XML: {e}")))?;
        match event {
            Event::Eof => break,
            Event::Start(element) => {
                let tag = element.name();
                let tag = tag.as_ref();
                if active.is_none() && tag == b"testcase" {
                    let name = junit_attr(&element, b"name")?
                        .ok_or_else(|| junit_error("testcase without name"))?;
                    if name.is_empty() {
                        return Err(junit_error("testcase without name"));
                    }
                    let classname = junit_attr(&element, b"classname")?;
                    let time = match junit_attr(&element, b"time")? {
                        None => 0.0,
                        Some(raw) => raw.parse::<f64>().map_err(|_| {
                            junit_error(format!("malformed testcase time: {raw:?}"))
                        })?,
                    };
                    if !time.is_finite() || time < 0.0 {
                        return Err(junit_error("malformed testcase time"));
                    }
                    active = Some(ActiveCase {
                        name,
                        classname,
                        time,
                        failure: None,
                        error: None,
                        skipped: None,
                        system_out: None,
                        system_err: None,
                    });
                    depth = 1;
                } else if let Some(current) = active.as_mut() {
                    depth += 1;
                    if depth == 2 && child.is_none() {
                        let kind = if tag == b"failure" {
                            Some(ChildKind::Failure)
                        } else if tag == b"error" {
                            Some(ChildKind::Error)
                        } else if tag == b"skipped" {
                            Some(ChildKind::Skipped)
                        } else if tag == b"system-out" {
                            Some(ChildKind::SystemOut)
                        } else if tag == b"system-err" {
                            Some(ChildKind::SystemErr)
                        } else {
                            None
                        };
                        if let Some(kind) = kind {
                            let message = junit_attr(&element, b"message")?;
                            child = Some(ActiveChild {
                                kind,
                                message,
                                text: String::new(),
                            });
                            let _ = current;
                        }
                    }
                }
                buf.clear();
            }
            Event::Empty(element) => {
                let tag = element.name();
                let tag = tag.as_ref();
                if active.is_none() && tag == b"testcase" {
                    let name = junit_attr(&element, b"name")?
                        .ok_or_else(|| junit_error("testcase without name"))?;
                    if name.is_empty() {
                        return Err(junit_error("testcase without name"));
                    }
                    let classname = junit_attr(&element, b"classname")?;
                    let time = match junit_attr(&element, b"time")? {
                        None => 0.0,
                        Some(raw) => raw.parse::<f64>().map_err(|_| {
                            junit_error(format!("malformed testcase time: {raw:?}"))
                        })?,
                    };
                    if !time.is_finite() || time < 0.0 {
                        return Err(junit_error("malformed testcase time"));
                    }
                    cases.push(JunitCase {
                        name,
                        classname,
                        time,
                        failure: None,
                        error: None,
                        skipped: None,
                        system_out: None,
                        system_err: None,
                        shard,
                        attempt,
                    });
                } else if let Some(current) = active.as_mut() {
                    if depth == 1 {
                        if tag == b"failure" {
                            if current.failure.is_some() {
                                return Err(junit_error("duplicate failure element"));
                            }
                            current.failure = Some(JunitMessage {
                                message: junit_attr(&element, b"message")?,
                                text: String::new(),
                            });
                        } else if tag == b"error" {
                            if current.error.is_some() {
                                return Err(junit_error("duplicate error element"));
                            }
                            current.error = Some(JunitMessage {
                                message: junit_attr(&element, b"message")?,
                                text: String::new(),
                            });
                        } else if tag == b"skipped" {
                            if current.skipped.is_some() {
                                return Err(junit_error("duplicate skipped element"));
                            }
                            current.skipped = Some(JunitMessage {
                                message: junit_attr(&element, b"message")?,
                                text: String::new(),
                            });
                        } else if tag == b"system-out" {
                            if current.system_out.is_some() {
                                return Err(junit_error("duplicate system-out element"));
                            }
                            current.system_out = Some(String::new());
                        } else if tag == b"system-err" {
                            if current.system_err.is_some() {
                                return Err(junit_error("duplicate system-err element"));
                            }
                            current.system_err = Some(String::new());
                        }
                    } else if let Some(open) = child.as_mut() {
                        // Nested empty elements inside failure/error text
                        // contribute no text; the outer child still
                        // closes with its accumulated content.
                        let _ = open;
                    }
                } // LCOV_EXCL_LINE - reason: closing brace of a fully covered nesting level carries no executable region of its own
                buf.clear();
            }
            Event::Text(text) => {
                if let Some(open) = child.as_mut() {
                    let decoded = text
                        .unescape()
                        .map_err(|e| junit_error(format!("malformed test XML text: {e}")))?;
                    open.text.push_str(&decoded);
                }
                buf.clear();
            }
            Event::CData(text) => {
                if let Some(open) = child.as_mut() {
                    // defense-in-depth; parse_test_xml rejects non-UTF-8 documents up front.
                    let decoded = std::str::from_utf8(text.as_ref())
                        .map_err(|_| junit_error("test XML CDATA is not UTF-8"))?; // LCOV_EXCL_LINE - reason: CDATA slices of a valid UTF-8 document are always UTF-8, so this error never fires
                    open.text.push_str(decoded);
                }
                buf.clear();
            }
            Event::End(element) => {
                let tag = element.name();
                let tag = tag.as_ref();
                if let Some(open) = child.take() {
                    if depth == 2 {
                        let current = active
                            .as_mut()
                            .ok_or_else(|| junit_error("test XML child outside testcase"))?;
                        match open.kind {
                            ChildKind::Failure => {
                                if current.failure.is_some() {
                                    return Err(junit_error("duplicate failure element"));
                                }
                                current.failure = Some(JunitMessage {
                                    message: open.message,
                                    text: open.text,
                                });
                            }
                            ChildKind::Error => {
                                if current.error.is_some() {
                                    return Err(junit_error("duplicate error element"));
                                }
                                current.error = Some(JunitMessage {
                                    message: open.message,
                                    text: open.text,
                                });
                            }
                            ChildKind::Skipped => {
                                if current.skipped.is_some() {
                                    return Err(junit_error("duplicate skipped element"));
                                }
                                current.skipped = Some(JunitMessage {
                                    message: open.message,
                                    text: open.text,
                                });
                            }
                            ChildKind::SystemOut => {
                                if current.system_out.is_some() {
                                    return Err(junit_error("duplicate system-out element"));
                                }
                                current.system_out = Some(open.text);
                            }
                            ChildKind::SystemErr => {
                                if current.system_err.is_some() {
                                    return Err(junit_error("duplicate system-err element"));
                                }
                                current.system_err = Some(open.text);
                            }
                        }
                    } else {
                        // Closing a nested element inside child text:
                        // restore the child so the outer end closes it.
                        child = Some(open);
                    }
                    depth = depth.saturating_sub(1);
                    let _ = tag;
                } else if active.is_some() {
                    if depth == 0 {
                        return Err(junit_error("unbalanced test XML")); // LCOV_EXCL_LINE - reason: defense-in-depth; active testcase always sets depth to 1 so depth 0 with active is unreachable
                    }
                    depth -= 1;
                    if depth == 0 {
                        if tag != b"testcase" {
                            return Err(junit_error("unbalanced test XML")); // LCOV_EXCL_LINE - reason: defense-in-depth; quick-xml check_end_names rejects mismatched closes before this guard
                        }
                        // `active` is `Some` in this branch by the guard
                        // above; `if let` keeps this total without a panic
                        // path and without an unreachable error line.
                        if let Some(finished) = active.take() {
                            cases.push(JunitCase {
                                name: finished.name,
                                classname: finished.classname,
                                time: finished.time,
                                failure: finished.failure,
                                error: finished.error,
                                skipped: finished.skipped,
                                system_out: finished.system_out,
                                system_err: finished.system_err,
                                shard,
                                attempt,
                            });
                        }
                    }
                }
                buf.clear();
            }
            _ => {
                buf.clear();
            }
        }
    }
    if active.is_some() || child.is_some() {
        return Err(junit_error("truncated test XML"));
    }
    Ok(cases)
}

/// Writes one JUnit event to a `Vec`-backed writer (issue #238).
///
/// `quick_xml::Writer<Vec<u8>>` only fails on underlying IO; `Vec` writes are
/// infallible, so failure is unreachable. Centralized here so the 40+ call
/// sites carry no `expect`/`unwrap`; the single `unreachable!` documents the
/// invariant instead of repeating proof comments at every site.
fn emit_junit(
    writer: &mut quick_xml::writer::Writer<Vec<u8>>,
    event: quick_xml::events::Event<'_>,
) {
    writer
        .write_event(event)
        .unwrap_or_else(|err| unreachable!("junit writer to Vec is infallible: {err:?}"));
}

/// Finishes a `Vec`-backed JUnit writer as UTF-8 (issue #238).
///
/// All inputs are sanitized XML plus ASCII tags, so the bytes are always valid
/// UTF-8; failure is unreachable.
fn finish_junit(writer: quick_xml::writer::Writer<Vec<u8>>) -> String {
    String::from_utf8(writer.into_inner())
        .unwrap_or_else(|err| unreachable!("junit writer bytes are UTF-8: {err:?}"))
}

fn junit_message_element(kind: &str, message: &JunitMessage) -> String {
    use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
    use quick_xml::writer::Writer;
    let mut writer = Writer::new(Vec::new());
    let mut elem = BytesStart::new(kind);
    if let Some(note) = &message.message {
        let clean_note = sanitize_xml(note);
        elem.push_attribute(("message", clean_note.as_str()));
    }
    if message.text.is_empty() {
        emit_junit(&mut writer, Event::Empty(elem));
    } else {
        let clean_text = sanitize_xml(&message.text);
        emit_junit(&mut writer, Event::Start(elem));
        emit_junit(&mut writer, Event::Text(BytesText::new(&clean_text)));
        emit_junit(&mut writer, Event::End(BytesEnd::new(kind)));
    }
    finish_junit(writer)
}

/// Renders normalized Bazel test cases as one JUnit XML document.
///
/// `suites` groups parsed cases by Bazel target label; every case in
/// one group shares that label. Suites order bytewise by label; cases
/// order bytewise by original name, then shard, then attempt. Retries
/// and shards stay separate cases with zero-based suffixes. Root and
/// suite `tests`, `failures`, `errors`, `skipped`, and `time` counts
/// are recomputed from the normalized cases.
pub fn render_junit(suites: &[(String, Vec<JunitCase>)]) -> String {
    use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
    use quick_xml::writer::Writer;

    let mut ordered: Vec<(String, Vec<JunitCase>)> = suites.to_vec();
    for (_, cases) in &mut ordered {
        cases.sort_by(|a, b| {
            a.name
                .as_bytes()
                .cmp(b.name.as_bytes())
                .then(a.shard.cmp(&b.shard))
                .then(a.attempt.cmp(&b.attempt))
        });
    }
    ordered.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));

    // Precompute per-suite and total counts before streaming the root
    // element (the writer emits start tags before children).
    struct SuiteCounts {
        failures: u64,
        errors: u64,
        skipped: u64,
        time: f64,
    }
    let mut suite_counts: Vec<SuiteCounts> = Vec::with_capacity(ordered.len());
    let mut total_tests = 0u64;
    let mut total_failures = 0u64;
    let mut total_errors = 0u64;
    let mut total_skipped = 0u64;
    let mut total_time = 0.0f64;
    for (_, cases) in &ordered {
        let mut counts = SuiteCounts {
            failures: 0,
            errors: 0,
            skipped: 0,
            time: 0.0,
        };
        for case in cases {
            total_tests += 1;
            counts.time += case.time;
            if case.failure.is_some() {
                counts.failures += 1;
                total_failures += 1;
            }
            if case.error.is_some() {
                counts.errors += 1;
                total_errors += 1;
            }
            if case.skipped.is_some() {
                counts.skipped += 1;
                total_skipped += 1;
            }
        }
        total_time += counts.time;
        suite_counts.push(counts);
    }

    let mut writer = Writer::new(Vec::new());
    emit_junit(
        &mut writer,
        Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)),
    );
    emit_junit(&mut writer, Event::Text(BytesText::from_escaped("\n")));
    let mut root = BytesStart::new("testsuites");
    root.push_attribute(("name", "dx"));
    root.push_attribute(("tests", total_tests.to_string().as_str()));
    root.push_attribute(("failures", total_failures.to_string().as_str()));
    root.push_attribute(("errors", total_errors.to_string().as_str()));
    root.push_attribute(("skipped", total_skipped.to_string().as_str()));
    root.push_attribute(("time", format_junit_time(total_time).as_str()));
    emit_junit(&mut writer, Event::Start(root));
    emit_junit(&mut writer, Event::Text(BytesText::from_escaped("\n")));

    for ((label, cases), counts) in ordered.iter().zip(suite_counts.iter()) {
        emit_junit(&mut writer, Event::Text(BytesText::from_escaped("  ")));
        let mut suite = BytesStart::new("testsuite");
        let clean_label = sanitize_xml(label);
        suite.push_attribute(("name", clean_label.as_str()));
        suite.push_attribute(("tests", cases.len().to_string().as_str()));
        suite.push_attribute(("failures", counts.failures.to_string().as_str()));
        suite.push_attribute(("errors", counts.errors.to_string().as_str()));
        suite.push_attribute(("skipped", counts.skipped.to_string().as_str()));
        suite.push_attribute(("time", format_junit_time(counts.time).as_str()));
        emit_junit(&mut writer, Event::Start(suite));
        emit_junit(&mut writer, Event::Text(BytesText::from_escaped("\n")));
        for case in cases {
            emit_junit(&mut writer, Event::Text(BytesText::from_escaped("    ")));
            let mut testcase = BytesStart::new("testcase");
            let display = sanitize_xml(&junit_display_name(&case.name, case.shard, case.attempt));
            testcase.push_attribute(("name", display.as_str()));
            if let Some(classname) = &case.classname {
                let clean_class = sanitize_xml(classname);
                testcase.push_attribute(("classname", clean_class.as_str()));
            }
            testcase.push_attribute(("time", format_junit_time(case.time).as_str()));
            let has_children = case.failure.is_some()
                || case.error.is_some()
                || case.skipped.is_some()
                || case.system_out.is_some()
                || case.system_err.is_some();
            if !has_children {
                emit_junit(&mut writer, Event::Empty(testcase));
                emit_junit(&mut writer, Event::Text(BytesText::from_escaped("\n")));
                continue;
            }
            emit_junit(&mut writer, Event::Start(testcase));
            emit_junit(&mut writer, Event::Text(BytesText::from_escaped("\n")));
            // Child message elements reuse the writer-backed helper so
            // escaping stays in one place; the helper output is already
            // well-formed XML embedded verbatim.
            if let Some(failure) = &case.failure {
                let rendered = junit_message_element("failure", failure);
                emit_junit(&mut writer, Event::Text(BytesText::from_escaped("      ")));
                emit_junit(
                    &mut writer,
                    Event::Text(BytesText::from_escaped(rendered.as_str())),
                );
                emit_junit(&mut writer, Event::Text(BytesText::from_escaped("\n")));
            }
            if let Some(error) = &case.error {
                let rendered = junit_message_element("error", error);
                emit_junit(&mut writer, Event::Text(BytesText::from_escaped("      ")));
                emit_junit(
                    &mut writer,
                    Event::Text(BytesText::from_escaped(rendered.as_str())),
                );
                emit_junit(&mut writer, Event::Text(BytesText::from_escaped("\n")));
            }
            if let Some(skipped_case) = &case.skipped {
                let rendered = junit_message_element("skipped", skipped_case);
                emit_junit(&mut writer, Event::Text(BytesText::from_escaped("      ")));
                emit_junit(
                    &mut writer,
                    Event::Text(BytesText::from_escaped(rendered.as_str())),
                );
                emit_junit(&mut writer, Event::Text(BytesText::from_escaped("\n")));
            }
            if let Some(out) = &case.system_out {
                let clean_out = sanitize_xml(out);
                emit_junit(&mut writer, Event::Text(BytesText::from_escaped("      ")));
                emit_junit(&mut writer, Event::Start(BytesStart::new("system-out")));
                emit_junit(&mut writer, Event::Text(BytesText::new(&clean_out)));
                emit_junit(&mut writer, Event::End(BytesEnd::new("system-out")));
                emit_junit(&mut writer, Event::Text(BytesText::from_escaped("\n")));
            }
            if let Some(err_text) = &case.system_err {
                let clean_err = sanitize_xml(err_text);
                emit_junit(&mut writer, Event::Text(BytesText::from_escaped("      ")));
                emit_junit(&mut writer, Event::Start(BytesStart::new("system-err")));
                emit_junit(&mut writer, Event::Text(BytesText::new(&clean_err)));
                emit_junit(&mut writer, Event::End(BytesEnd::new("system-err")));
                emit_junit(&mut writer, Event::Text(BytesText::from_escaped("\n")));
            }
            emit_junit(&mut writer, Event::Text(BytesText::from_escaped("    ")));
            emit_junit(&mut writer, Event::End(BytesEnd::new("testcase")));
            emit_junit(&mut writer, Event::Text(BytesText::from_escaped("\n")));
        }
        emit_junit(&mut writer, Event::Text(BytesText::from_escaped("  ")));
        emit_junit(&mut writer, Event::End(BytesEnd::new("testsuite")));
        emit_junit(&mut writer, Event::Text(BytesText::from_escaped("\n")));
    }
    emit_junit(&mut writer, Event::End(BytesEnd::new("testsuites")));
    emit_junit(&mut writer, Event::Text(BytesText::from_escaped("\n")));
    finish_junit(writer)
}

/// Renders one partial-infrastructure suite for JUnit collection that
/// lost at least one `test.xml` artifact. The suite is named
/// `dx.infrastructure` with one error case named `incomplete_results`;
/// callers add it to the normalized suite list before
/// [`render_junit`] so aggregate counts include the error.
pub fn junit_infrastructure_case(detail: &str) -> (String, Vec<JunitCase>) {
    (
        "dx.infrastructure".to_owned(),
        vec![JunitCase {
            name: "incomplete_results".to_owned(),
            classname: Some("dx.infrastructure".to_owned()),
            time: 0.0,
            failure: None,
            error: Some(JunitMessage {
                message: Some("incomplete_results".to_owned()),
                text: detail.to_owned(),
            }),
            skipped: None,
            system_out: None,
            system_err: None,
            shard: 0,
            attempt: 0,
        }],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn junit_case(name: &str) -> JunitCase {
        JunitCase {
            name: name.to_owned(),
            classname: None,
            time: 0.0,
            failure: None,
            error: None,
            skipped: None,
            system_out: None,
            system_err: None,
            shard: 0,
            attempt: 0,
        }
    }

    #[test]
    fn junit_helpers_cover_escapes_times_and_names() {
        assert_eq!(format_junit_time(f64::NAN), "0");
        assert_eq!(format_junit_time(-1.0), "0");
        assert_eq!(format_junit_time(f64::INFINITY), "0");
        assert_eq!(format_junit_time(1.23456), "1.235");
        assert_eq!(junit_display_name("a", 0, 0), "a");
        assert_eq!(junit_display_name("a", 1, 2), "a [shard=1,attempt=2]");
        let doc = render_junit(&[(
            "a&<b>\"'".to_owned(),
            vec![JunitCase {
                name: "n&<m>\"'".to_owned(),
                classname: Some("c&<>".to_owned()),
                time: 1.0,
                failure: Some(JunitMessage {
                    message: Some("msg&<>".to_owned()),
                    text: "body&<>".to_owned(),
                }),
                error: None,
                skipped: None,
                system_out: None,
                system_err: None,
                shard: 0,
                attempt: 0,
            }],
        )]);
        assert!(doc.contains("&amp;"), "{doc}");
        assert!(doc.contains("&lt;"), "{doc}");
        let empty_msg = junit_message_element(
            "failure",
            &JunitMessage {
                message: None,
                text: String::new(),
            },
        );
        assert_eq!(empty_msg, "<failure/>");
        let with_text = junit_message_element(
            "error",
            &JunitMessage {
                message: Some("m".to_owned()),
                text: "t".to_owned(),
            },
        );
        assert!(with_text.contains("message=\"m\""), "{with_text}");
        let (_label, cases) = junit_infrastructure_case("detail");
        assert_eq!(cases[0].name, "incomplete_results");
    }

    #[test]
    fn junit_render_covers_all_child_kinds() {
        let suites = vec![
            ("//b:t".to_owned(), vec![junit_case("b")]),
            (
                "//a:t".to_owned(),
                vec![
                    JunitCase {
                        failure: Some(JunitMessage {
                            message: Some("f".to_owned()),
                            text: "ft".to_owned(),
                        }),
                        ..junit_case("a1")
                    },
                    JunitCase {
                        error: Some(JunitMessage {
                            message: None,
                            text: String::new(),
                        }),
                        skipped: Some(JunitMessage {
                            message: None,
                            text: String::new(),
                        }),
                        system_out: Some("out".to_owned()),
                        system_err: Some("err".to_owned()),
                        shard: 1,
                        attempt: 2,
                        ..junit_case("a2")
                    },
                ],
            ),
        ];
        let doc = render_junit(&suites);
        // Golden pilot (issue #225): full-document insta snapshot replaces
        // the contains-asserts so render changes review as one diff.
        // Under Bazel snapshots never self-update (read-only sources):
        // paste the actual document from the failure diff when the
        // render intentionally changes. Raw string: the XML carries
        // double quotes but no `"#` sequences.
        insta::assert_snapshot!(doc, @r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="dx" tests="3" failures="1" errors="1" skipped="1" time="0.000">
  <testsuite name="//a:t" tests="2" failures="1" errors="1" skipped="1" time="0.000">
    <testcase name="a1" time="0.000">
      <failure message="f">ft</failure>
    </testcase>
    <testcase name="a2 [shard=1,attempt=2]" time="0.000">
      <error/>
      <skipped/>
      <system-out>out</system-out>
      <system-err>err</system-err>
    </testcase>
  </testsuite>
  <testsuite name="//b:t" tests="1" failures="0" errors="0" skipped="0" time="0.000">
    <testcase name="b" time="0.000"/>
  </testsuite>
</testsuites>"#);
    }

    #[test]
    fn junit_writer_escapes_specials_and_sanitizes_controls() {
        // Golden: `&<>"'` plus control bytes must round-trip through the
        // quick-xml writer as well-formed XML (issue #219).
        let tricky = "a&<>\"'\u{0}\u{1}\u{8}\u{b}\u{c}\u{e}b";
        let doc = render_junit(&[(
            format!("suite&<>\"'{tricky}"),
            vec![JunitCase {
                name: tricky.to_owned(),
                classname: Some(format!("cls&<>\"'{tricky}")),
                time: 0.5,
                failure: Some(JunitMessage {
                    message: Some(format!("msg&<>\"'{tricky}")),
                    text: format!("body&<>\"'{tricky}"),
                }),
                error: None,
                skipped: None,
                system_out: Some(format!("out&<>{tricky}")),
                system_err: Some(format!("err&<>{tricky}")),
                shard: 0,
                attempt: 0,
            }],
        )]);
        // Writer escaping for attributes and text.
        assert!(doc.contains("&amp;"), "{doc}");
        assert!(doc.contains("&lt;"), "{doc}");
        // Raw control bytes must never reach the document; they become
        // U+FFFD via `sanitize_xml` while `\t\n\r` would be preserved.
        for raw in ['\u{0}', '\u{1}', '\u{8}', '\u{b}', '\u{c}', '\u{e}'] {
            assert!(!doc.contains(raw), "{doc}");
        }
        assert!(doc.contains("\u{FFFD}"), "{doc}");
        // The writer output must re-parse as XML (well-formedness proof
        // for Jenkins/GitLab ingestion).
        let reparsed = parse_test_xml(doc.as_bytes(), 0, 0).expect("reparse");
        assert_eq!(reparsed.len(), 1);
        assert!(reparsed[0].failure.is_some());
    }

    #[test]
    fn junit_parse_covers_happy_and_error_paths() {
        // Happy: start/end testcase with children, empty testcase, decl/comment.
        let good = r#"<?xml version="1.0"?><!-- c --><testsuite><testcase name="a" classname="c" time="1.5"><failure message="m">text</failure></testcase><testcase name="b"/><testcase name="c" time="0"><error/><skipped/><system-out/><system-err/></testcase></testsuite>"#;
        let cases = parse_test_xml(good.as_bytes(), 0, 0).expect("good");
        assert_eq!(cases.len(), 3);
        // Start/end with system-out text and nested markup.
        let nested = r#"<testsuite><testcase name="a"><failure>text <b>bold</b> more<br/>tail</failure></testcase><testcase name="b"><error><![CDATA[blob]]></error></testcase></testsuite>"#;
        let cases = parse_test_xml(nested.as_bytes(), 0, 0).expect("nested");
        assert_eq!(cases.len(), 2);
        assert!(cases[0]
            .failure
            .as_ref()
            .expect("failure")
            .text
            .contains("text"));
        // Errors: non-utf8, no elements, malformed, missing/empty name, bad time.
        assert!(parse_test_xml(&[0xff], 0, 0).is_err());
        assert!(parse_test_xml(b"hello", 0, 0).is_err());
        assert!(parse_test_xml(b"<testcase", 0, 0).is_err());
        assert!(parse_test_xml(b"<testsuite><testcase/></testsuite>", 0, 0).is_err());
        assert!(parse_test_xml(b"<testsuite><testcase name=\"\"/></testsuite>", 0, 0).is_err());
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\" time=\"bogus\"/></testsuite>",
            0,
            0
        )
        .is_err());
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\" time=\"-1\"/></testsuite>",
            0,
            0
        )
        .is_err());
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\" time=\"inf\"/></testsuite>",
            0,
            0
        )
        .is_err());
        // Duplicate children fail.
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\"><failure/><failure/></testcase></testsuite>",
            0,
            0
        )
        .is_err());
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\"><error/><error/></testcase></testsuite>",
            0,
            0
        )
        .is_err());
        // Unbalanced and truncated fail.
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\"></foo></testcase></testsuite>",
            0,
            0
        )
        .is_err());
        assert!(parse_test_xml(b"<testsuite><testcase name=\"a\">", 0, 0).is_err());
        // Malformed attribute fails; well-formed text succeeds.
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\"><failure message=\"\xff\"/></testcase></testsuite>",
            0,
            0
        )
        .is_err());
        assert!(
            parse_test_xml(
                b"<testsuite><testcase name=\"a\"><failure message=\"m\">ok</failure></testcase></testsuite>",
                0, 0
            )
            .is_ok()
        );
    }

    #[test]
    fn junit_parse_covers_start_and_end_branches() {
        // Start testcase empty name and bad times (Empty variants are in the
        // happy/error test; these hit the Start arms).
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"\"></testcase></testsuite>",
            0,
            0
        )
        .is_err());
        for bad in ["bogus", "-1", "inf", "NaN"] {
            let xml =
                format!("<testsuite><testcase name=\"a\" time=\"{bad}\"></testcase></testsuite>");
            assert!(parse_test_xml(xml.as_bytes(), 0, 0).is_err(), "{bad}");
        }
        // Start children for every kind plus unknown tags (unknown yields no
        // child but still closes cleanly).
        for child in [
            "<failure></failure>",
            "<error></error>",
            "<skipped></skipped>",
            "<system-out>out</system-out>",
            "<system-err>err</system-err>",
            "<foo></foo>",
            "<foo/>",
        ] {
            let xml = format!("<testsuite><testcase name=\"a\">{child}</testcase></testsuite>");
            assert!(parse_test_xml(xml.as_bytes(), 0, 0).is_ok(), "{child}");
        }
        // Empty duplicates for skipped/system-out/system-err.
        for dup in [
            "<skipped/><skipped/>",
            "<system-out/><system-out/>",
            "<system-err/><system-err/>",
        ] {
            let xml = format!("<testsuite><testcase name=\"a\">{dup}</testcase></testsuite>");
            assert!(parse_test_xml(xml.as_bytes(), 0, 0).is_err(), "{dup}");
        }
        // Text and CDATA outside any child cover the no-child arms.
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\">hello</testcase></testsuite>",
            0,
            0
        )
        .is_ok());
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\"><![CDATA[hello]]></testcase></testsuite>",
            0,
            0
        )
        .is_ok());
        // Bad text entity fails unescape.
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\"><failure>&notanentity</failure></testcase></testsuite>",
            0,
            0
        )
        .is_err());
        // End duplicates for every kind via Start/End pairs.
        for dup in [
            "<failure></failure><failure></failure>",
            "<error></error><error></error>",
            "<skipped></skipped><skipped></skipped>",
            "<system-out>a</system-out><system-out>b</system-out>",
            "<system-err>a</system-err><system-err>b</system-err>",
        ] {
            let xml = format!("<testsuite><testcase name=\"a\">{dup}</testcase></testsuite>");
            assert!(parse_test_xml(xml.as_bytes(), 0, 0).is_err(), "{dup}");
        }
        // Children outside testcase are ignored; unbalanced closes fail.
        assert!(parse_test_xml(
            b"<testsuite><failure></failure><testcase name=\"a\"/></testsuite>",
            0,
            0
        )
        .is_ok());
        assert!(parse_test_xml(
            b"<testsuite><testcase name=\"a\"></testcase></testsuite>",
            0,
            0
        )
        .is_ok());
        assert!(parse_test_xml(b"<testsuite><testcase name=\"a\"></testsuite>", 0, 0).is_err());
    }
}
