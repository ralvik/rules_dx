//! JUnit XML parsing for Bazel-reported test artifacts (issue #388).
//!
//! Rewritten on [`quick_junit::Report`]: deserialization uses the
//! nextest data model instead of the hand-rolled `quick-xml` state
//! machine. Re-exported through [`super::junit`] so the public paths
//! stay `crate::reports::{parse_test_xml}` and
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

/// Parses one Bazel-reported `test.xml` artifact into normalized cases.
///
/// `shard` and `attempt` are the zero-based indices for every case in
/// `bytes` (derived from the BEP identity). Names, durations,
/// `<failure>`, `<error>`, `<skipped>`, `<system-out>`, and
/// `<system-err>` content are preserved via `quick-junit`
/// deserialization (which strips invalid XML chars and ANSI escapes);
/// suite structure is ignored because the caller groups by Bazel target
/// label. Malformed XML fails the whole artifact so the caller can
/// mark collection partial.
pub fn parse_test_xml(
    bytes: &[u8],
    shard: u32,
    attempt: u32,
) -> Result<Vec<JunitCase>, ReportError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| junit_error(format!("test XML is not UTF-8: {e}")))?;
    // Reject documents with no element structure early so empty or
    // whitespace-only artifacts fail closed instead of yielding zero
    // cases that look like a passing suite.
    if !text.contains('<') {
        return Err(junit_error("test XML has no elements"));
    }
    let report = deserialize_report(text)?;

    let mut cases: Vec<JunitCase> = Vec::new();
    for suite in &report.test_suites {
        for case in &suite.test_cases {
            if case.name.as_str().is_empty() {
                return Err(junit_error("testcase without name"));
            }
            let (failure, error, skipped) = match &case.status {
                quick_junit::TestCaseStatus::Success { .. } => (None, None, None),
                quick_junit::TestCaseStatus::NonSuccess {
                    kind,
                    message,
                    description,
                    ..
                } => {
                    let note = JunitMessage {
                        message: message.as_ref().map(|m| m.as_str().to_owned()),
                        text: description
                            .as_ref()
                            .map(|d| d.as_str().to_owned())
                            .unwrap_or_default(),
                    };
                    match kind {
                        quick_junit::NonSuccessKind::Failure => (Some(note), None, None),
                        quick_junit::NonSuccessKind::Error => (None, Some(note), None),
                    }
                }
                quick_junit::TestCaseStatus::Skipped {
                    message,
                    description,
                    ..
                } => (
                    None,
                    None,
                    Some(JunitMessage {
                        message: message.as_ref().map(|m| m.as_str().to_owned()),
                        text: description
                            .as_ref()
                            .map(|d| d.as_str().to_owned())
                            .unwrap_or_default(),
                    }),
                ),
            };
            cases.push(JunitCase {
                name: case.name.as_str().to_owned(),
                classname: case.classname.as_ref().map(|c| c.as_str().to_owned()),
                time: case.time.map(|d| d.as_secs_f64()).unwrap_or(0.0),
                failure,
                error,
                skipped,
                system_out: case.system_out.as_ref().map(|s| s.as_str().to_owned()),
                system_err: case.system_err.as_ref().map(|s| s.as_str().to_owned()),
                shard,
                attempt,
            });
        }
    }
    Ok(cases)
}

/// Deserializes via `quick-junit`, accepting both `<testsuites>` roots
/// and bare `<testsuite>` artifacts (Bazel emits the latter).
///
/// Legacy fixtures (and some Bazel emitters) use nameless `<testsuite>`
/// roots, which quick-junit rejects (`name` is required). Nameless
/// suites are normalized to `name="dx"` so parsing stays total; real
/// artifacts already carry names and are unaffected.
fn deserialize_report(text: &str) -> Result<quick_junit::Report, ReportError> {
    // Jest (and other emitters) write `timestamp="2026-09-19T21:10:02"`
    // without a timezone; quick-junit 0.8 validates timestamps as RFC3339
    // and rejects those artifacts. Timestamps are unused (caller groups by
    // Bazel label), so strip the attribute from suite start tags before
    // deserializing. Restricted to `<testsuite*` start tags so failure
    // text containing `timestamp="..."` is preserved.
    let without_ts = strip_timestamp_attrs(text);
    let normalized = without_ts
        .replace("<testsuite>", "<testsuite name=\"dx\">")
        .replace("<testsuite/>", "<testsuite name=\"dx\"/>");
    match quick_junit::Report::deserialize_from_str(&normalized) {
        Ok(report) => {
            if report.test_suites.is_empty() && normalized.contains("<testcase") {
                // A bare `<testcase>` without a suite wrapper parses as
                // an empty report; nest it so the case is preserved.
                let wrapped = format!(
                    "<testsuites><testsuite name=\"dx\">{normalized}</testsuite></testsuites>"
                );
                quick_junit::Report::deserialize_from_str(&wrapped)
                    .map_err(|e| junit_error(format!("malformed test XML: {e}")))
            } else {
                Ok(report)
            }
        }
        Err(first) => {
            let msg = first.to_string();
            if msg.contains("testsuites") {
                // Bare `<testsuite>` artifact: nest under a synthetic root.
                // Strip a leading XML declaration first: `<?xml ...?>` is
                // only valid at offset zero, so embedding it inside
                // `<testsuites>` would poison the wrapped document.
                let inner = strip_leading_decl(&normalized);
                let wrapped = format!("<testsuites>{inner}</testsuites>");
                quick_junit::Report::deserialize_from_str(&wrapped)
                    .map_err(|e| junit_error(format!("malformed test XML: {e}")))
            } else {
                Err(junit_error(format!("malformed test XML: {first}")))
            }
        }
    }
}

fn strip_leading_decl(text: &str) -> &str {
    let trimmed = text.trim_start();
    if trimmed.starts_with("<?xml") {
        if let Some(end) = trimmed.find("?>") {
            return trimmed[end + 2..].trim_start();
        }
    }
    text
}

fn strip_timestamp_attrs(text: &str) -> String {
    fn remove_one(tag: &mut String) -> bool {
        let mut search_from = 0;
        while let Some(rel) = tag[search_from..].find("timestamp") {
            let pos = search_from + rel;
            if pos > 0 && !tag.as_bytes()[pos - 1].is_ascii_whitespace() {
                search_from = pos + 1;
                continue;
            }
            let mut j = pos + "timestamp".len();
            while j < tag.len() && tag.as_bytes()[j].is_ascii_whitespace() {
                j += 1;
            }
            if j >= tag.len() || tag.as_bytes()[j] != b'=' {
                search_from = pos + 1;
                continue;
            }
            j += 1;
            while j < tag.len() && tag.as_bytes()[j].is_ascii_whitespace() {
                j += 1;
            }
            if j >= tag.len() || (tag.as_bytes()[j] != b'"' && tag.as_bytes()[j] != b'\'') {
                search_from = pos + 1;
                continue;
            }
            let quote = tag.as_bytes()[j];
            j += 1;
            while j < tag.len() && tag.as_bytes()[j] != quote {
                j += 1;
            }
            if j >= tag.len() {
                return false;
            }
            j += 1;
            tag.replace_range(pos..j, "");
            return true;
        }
        false
    }

    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(tag_start) = rest.find("<testsuite") {
        out.push_str(&rest[..tag_start]);
        let tag_rest = &rest[tag_start..];
        let Some(tag_end_rel) = tag_rest.find('>') else {
            out.push_str(tag_rest);
            rest = "";
            break;
        };
        let tag_end = tag_end_rel + 1;
        let mut tag = tag_rest[..tag_end].to_owned();
        while remove_one(&mut tag) {}
        out.push_str(&tag);
        rest = &rest[tag_start + tag_end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn junit_parse_covers_happy_and_error_paths() {
        // Happy: start/end testcase with children, empty testcase, decl/comment.
        // Note: quick-junit allows one main status per testcase, so the
        // third case carries only `<error>` (plus system streams).
        let good = r#"<?xml version="1.0"?><!-- c --><testsuite><testcase name="a" classname="c" time="1.5"><failure message="m">text</failure></testcase><testcase name="b"/><testcase name="c" time="0"><error/><system-out/><system-err/></testcase></testsuite>"#;
        let cases = parse_test_xml(good.as_bytes(), 0, 0).expect("good");
        assert_eq!(cases.len(), 3);
        // Start/end with system-out text and nested markup.
        // quick-junit skips unknown nested elements (e.g. `<b>`) but
        // preserves surrounding text; self-closing unknowns like
        // `<br/>` are not representable and are excluded here.
        let nested = r#"<testsuite><testcase name="a"><failure>text <b>bold</b> moretail</failure></testcase><testcase name="b"><error><![CDATA[blob]]></error></testcase></testsuite>"#;
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
        // Duplicate main statuses fail.
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
    fn junit_parse_rejects_multiple_main_statuses() {
        // quick-junit models one main status per testcase; legacy
        // artifacts carrying both fail closed so collection is partial.
        let xml = b"<testsuite><testcase name=\"a\"><error/><skipped/></testcase></testsuite>";
        assert!(parse_test_xml(xml, 0, 0).is_err());
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
        // Children for every kind plus unknown tags (unknown yields no
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
        // Empty duplicate skipped fails (duplicate main status).
        // Duplicate system-out/err are lenient in quick-junit (last
        // wins) unlike the old state machine.
        let xml = "<testsuite><testcase name=\"a\"><skipped/><skipped/></testcase></testsuite>";
        assert!(parse_test_xml(xml.as_bytes(), 0, 0).is_err());
        for dup in ["<system-out/><system-out/>", "<system-err/><system-err/>"] {
            let xml = format!("<testsuite><testcase name=\"a\">{dup}</testcase></testsuite>");
            assert!(parse_test_xml(xml.as_bytes(), 0, 0).is_ok(), "{dup}");
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
        // End duplicates for failure/error/skipped fail; duplicate
        // system-out/err are lenient (last wins).
        for dup in [
            "<failure></failure><failure></failure>",
            "<error></error><error></error>",
            "<skipped></skipped><skipped></skipped>",
        ] {
            let xml = format!("<testsuite><testcase name=\"a\">{dup}</testcase></testsuite>");
            assert!(parse_test_xml(xml.as_bytes(), 0, 0).is_err(), "{dup}");
        }
        for dup in [
            "<system-out>a</system-out><system-out>b</system-out>",
            "<system-err>a</system-err><system-err>b</system-err>",
        ] {
            let xml = format!("<testsuite><testcase name=\"a\">{dup}</testcase></testsuite>");
            assert!(parse_test_xml(xml.as_bytes(), 0, 0).is_ok(), "{dup}");
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

    #[test]
    fn junit_parse_accepts_jest_timestamp_without_timezone() {
        // Jest emits `timestamp="2026-09-19T21:10:02"` without a timezone;
        // quick-junit validates RFC3339 and would reject it. dx ignores
        // suite timestamps, so the attribute is stripped and cases parse.
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="jest tests" tests="2" failures="0" errors="0" time="0.803">
  <testsuite name="Hello.astro" errors="0" failures="0" skipped="0" timestamp="2026-09-19T21:10:02" time="0.602" tests="2">
    <testcase classname="Hello.astro parses" name="parses" time="0.005">
    </testcase>
    <testcase classname="Hello.astro reports" name="reports" time="0.002">
    </testcase>
  </testsuite>
</testsuites>"#;
        let cases = parse_test_xml(xml.as_bytes(), 0, 0).expect("jest timestamp");
        assert_eq!(cases.len(), 2);
        // Failure text containing timestamp-looking content is preserved.
        let tricky = r#"<testsuite name="a" timestamp="2026-09-19T21:10:02"><testcase name="a"><failure>timestamp="kept"</failure></testcase></testsuite>"#;
        let cases = parse_test_xml(tricky.as_bytes(), 0, 0).expect("tricky");
        assert!(cases[0]
            .failure
            .as_ref()
            .expect("failure")
            .text
            .contains("kept"));
    }
}
