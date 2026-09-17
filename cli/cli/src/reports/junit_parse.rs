//! JUnit XML parsing for Bazel-reported test artifacts (issue #236).
//!
//! Split from [`super::junit`]: owns [`parse_test_xml`] plus its
//! attribute helper. Re-exported through [`super::junit`] so the public
//! paths stay `crate::reports::{parse_test_xml}` and
//! `crate::reports::junit::{parse_test_xml}`. Shares the normalized
//! case types ([`JunitCase`], [`JunitMessage`]) from
//! [`super::junit_types`] with the rendering side in
//! [`super::junit_render`].

use super::junit_types::{JunitCase, JunitMessage};
use super::ReportError;

fn junit_error(detail: impl Into<String>) -> ReportError {
    ReportError::InvalidJunit {
        detail: detail.into(),
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

#[cfg(test)]
mod tests {
    use super::*;

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
