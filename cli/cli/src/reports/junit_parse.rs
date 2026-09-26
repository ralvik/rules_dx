use super::junit_types::{JunitCase, JunitMessage};
use super::ReportError;

fn junit_error(detail: impl Into<String>) -> ReportError {
    ReportError::InvalidJunit {
        detail: detail.into(),
    }
}

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

fn deserialize_report(text: &str) -> Result<quick_junit::Report, ReportError> {
    // Jest (and other emitters) write `timestamp="2026-09-19T21:10:02"`
    // without a timezone; quick-junit 0.8 validates timestamps as RFC3339
    // and rejects those artifacts. Timestamps are unused (caller groups by
    // Bazel label), so strip the attribute from start/empty tags before
    // deserializing. Typed `quick-xml` events keep failure text, CDATA,
    // and comments containing `timestamp="..."` byte-identical.
    let without_ts = strip_timestamp_attrs(text);
    // Bazel emitters occasionally write negative testcase durations
    // (e.g. `time="-2"` from clock skew); quick-junit rejects negatives as
    // malformed durations. Duration is informational (pass/fail rides the
    // failure/error tags), so clamp negatives to zero before
    // deserializing. Non-numeric times ("bogus", "inf") still fail.
    // Restricted to `<testsuite*`/`<testcase*` start tags so failure text
    // containing `time="-..."` is preserved.
    let without_negative = clamp_negative_times(&without_ts);
    let normalized = without_negative
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
    use quick_xml::events::{BytesStart, Event};
    use quick_xml::reader::Reader;
    use quick_xml::writer::Writer;
    use std::io::Cursor;

    // Fast path: without the substring there is no attribute to drop.
    if !text.contains("timestamp") {
        return text.to_owned();
    }

    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text_start = false;
    reader.config_mut().trim_text_end = false;
    reader.config_mut().expand_empty_elements = false;

    let mut writer = Writer::new(Cursor::new(Vec::with_capacity(text.len())));
    let mut stripped_any = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let content: &[u8] = &e;
                let name_len = e.name().as_ref().len();
                if let Some(filtered) = remove_timestamp_from_tag(content, name_len) {
                    stripped_any = true;
                    match String::from_utf8(filtered) {
                        Ok(s) => {
                            let elem = BytesStart::from_content(s, name_len);
                            if writer.write_event(Event::Start(elem)).is_err() {
                                return text.to_owned();
                            }
                        }
                        Err(_) => {
                            if writer.write_event(Event::Start(e)).is_err() {
                                return text.to_owned();
                            }
                        }
                    }
                } else if writer.write_event(Event::Start(e)).is_err() {
                    return text.to_owned();
                }
            }
            Ok(Event::Empty(e)) => {
                let content: &[u8] = &e;
                let name_len = e.name().as_ref().len();
                if let Some(filtered) = remove_timestamp_from_tag(content, name_len) {
                    stripped_any = true;
                    match String::from_utf8(filtered) {
                        Ok(s) => {
                            let elem = BytesStart::from_content(s, name_len);
                            if writer.write_event(Event::Empty(elem)).is_err() {
                                return text.to_owned();
                            }
                        }
                        Err(_) => {
                            if writer.write_event(Event::Empty(e)).is_err() {
                                return text.to_owned();
                            }
                        }
                    }
                } else if writer.write_event(Event::Empty(e)).is_err() {
                    return text.to_owned();
                }
            }
            Ok(Event::Eof) => break,
            Ok(event) => {
                if writer.write_event(event).is_err() {
                    return text.to_owned();
                }
            }
            Err(_) => {
                // Malformed XML during the strip pass: fall back to the
                // original so the caller still fails closed via `junit_error`.
                return text.to_owned();
            }
        }
    }

    if !stripped_any {
        return text.to_owned();
    }
    let bytes = writer.into_inner().into_inner();
    String::from_utf8(bytes).unwrap_or_else(|_| text.to_owned())
}

fn is_xml_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\r' | b'\n' | b'\t')
}

fn remove_timestamp_from_tag(content: &[u8], name_len: usize) -> Option<Vec<u8>> {
    if name_len > content.len() {
        return None;
    }
    // Quick pre-check to avoid scanning tags that cannot match.
    let mut has_candidate = false;
    if content.len() >= b"timestamp".len() {
        for window in content.windows(b"timestamp".len()) {
            if window == b"timestamp" {
                has_candidate = true;
                break;
            }
        }
    }
    if !has_candidate {
        return None;
    }

    let mut remove_ranges: Vec<(usize, usize)> = Vec::new();
    let mut pos = name_len;
    while pos < content.len() {
        let ws_start = pos;
        while pos < content.len() && is_xml_whitespace(content[pos]) {
            pos += 1;
        }
        if pos >= content.len() {
            break;
        }
        if content[pos] == b'/' || content[pos] == b'>' || content[pos] == b'?' {
            break;
        }
        let key_start = pos;
        while pos < content.len()
            && content[pos] != b'='
            && !is_xml_whitespace(content[pos])
            && content[pos] != b'/'
            && content[pos] != b'>'
        {
            pos += 1;
        }
        let key_end = pos;
        if key_start == key_end {
            pos += 1;
            continue;
        }
        let key = &content[key_start..key_end];
        while pos < content.len() && is_xml_whitespace(content[pos]) {
            pos += 1;
        }
        if pos >= content.len() || content[pos] != b'=' {
            continue;
        }
        pos += 1;
        while pos < content.len() && is_xml_whitespace(content[pos]) {
            pos += 1;
        }
        if pos >= content.len() {
            break;
        }
        let quote = content[pos];
        if quote != b'"' && quote != b'\'' {
            while pos < content.len()
                && !is_xml_whitespace(content[pos])
                && content[pos] != b'>'
                && content[pos] != b'/'
            {
                pos += 1;
            }
            continue;
        }
        pos += 1;
        let mut closed = false;
        while pos < content.len() {
            if content[pos] == quote {
                closed = true;
                break;
            }
            pos += 1;
        }
        if !closed {
            return None;
        }
        pos += 1;
        let attr_end = pos;
        if key == b"timestamp" {
            remove_ranges.push((ws_start, attr_end));
        }
    }

    if remove_ranges.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(content.len());
    let mut cursor = 0;
    for (start, end) in remove_ranges {
        if start > cursor {
            out.extend_from_slice(&content[cursor..start]);
        }
        cursor = end;
    }
    if cursor < content.len() {
        out.extend_from_slice(&content[cursor..]);
    }
    Some(out)
}

fn clamp_negative_times(text: &str) -> String {
    fn clamp_one(tag: &mut String) -> bool {
        let mut search_from = 0;
        while let Some(rel) = tag[search_from..].find("time") {
            let pos = search_from + rel;
            if pos > 0 && !tag.as_bytes()[pos - 1].is_ascii_whitespace() {
                search_from = pos + 1;
                continue;
            }
            let mut j = pos + "time".len();
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
            if j >= tag.len() || tag.as_bytes()[j] != b'"' {
                search_from = pos + 1;
                continue;
            }
            j += 1;
            if j < tag.len() && tag.as_bytes()[j] == b'-' {
                let value_start = j;
                while j < tag.len() && tag.as_bytes()[j] != b'"' {
                    j += 1;
                }
                if j >= tag.len() {
                    return false;
                }
                tag.replace_range(value_start..j, "0");
                return true;
            }
            search_from = pos + 1;
        }
        false
    }

    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let suite_pos = rest.find("<testsuite");
        let case_pos = rest.find("<testcase");
        let tag_start = match (suite_pos, case_pos) {
            (Some(s), Some(c)) => Some(s.min(c)),
            (Some(s), None) => Some(s),
            (None, Some(c)) => Some(c),
            (None, None) => None,
        };
        let Some(tag_start) = tag_start else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..tag_start]);
        let tag_rest = &rest[tag_start..];
        let Some(tag_end_rel) = tag_rest.find('>') else {
            out.push_str(tag_rest);
            break;
        };
        let tag_end = tag_end_rel + 1;
        let mut tag = tag_rest[..tag_end].to_owned();
        while clamp_one(&mut tag) {}
        out.push_str(&tag);
        rest = &rest[tag_start + tag_end..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_attribute_scanner_preserves_unrelated_and_truncated_bytes() {
        for input in [
            "testcase timestamp",
            "testcase timestamp=",
            "testcase timestamp=unquoted",
            "testcase timestamp='unterminated",
            "testcase note='timestamp'",
            "testcase timestamp >",
            "testcase timestamp /",
            "testcase timestamp ?",
            "testcase = timestamp",
            "testcase timestamp   ",
        ] {
            assert_eq!(
                remove_timestamp_from_tag(input.as_bytes(), 8),
                None,
                "{input}"
            );
        }
        assert_eq!(remove_timestamp_from_tag(b"short", 99), None);
        assert_eq!(
            remove_timestamp_from_tag(b"testcase timestamp='ignored' name='kept'", 8),
            Some(b"testcase name='kept'".to_vec())
        );
    }

    #[test]
    fn negative_duration_normalization_preserves_results() {
        let cases = parse_test_xml(
            b"<testsuite><testcase name=\"standalone\" time=\"-2\"/></testsuite>",
            2,
            3,
        )
        .expect("testcase");
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].name, "standalone");
        for (input, expected) in [
            (
                "<testcase runtime=\"-1\" time = \"-2\"/>",
                "<testcase runtime=\"-1\" time = \"0\"/>",
            ),
            ("<testcase time='-1'/>", "<testcase time='-1'/>"),
            ("<testcase time=\"-1>", "<testcase time=\"-1>"),
            (
                "<testcase time other=\"x\"/>",
                "<testcase time other=\"x\"/>",
            ),
        ] {
            assert_eq!(clamp_negative_times(input), expected);
        }
        assert_eq!(
            strip_leading_decl("<?xml unterminated"),
            "<?xml unterminated"
        );
        for bad in [
            "<testsuite timestamp=\"unterminated",
            "<testsuite timestamp=noquote>",
            "<testsuite timestamp=\"x\" broken>",
        ] {
            assert!(parse_test_xml(bad.as_bytes(), 0, 0).is_err(), "{bad}");
        }
    }

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
        // Bazel clock skew writes negative durations; they clamp to zero
        // instead of failing (duration is informational, status rides the
        // failure/error tags).
        let negative = parse_test_xml(
            b"<testsuite><testcase name=\"a\" time=\"-1\"/></testsuite>",
            0,
            0,
        )
        .expect("negative time clamps");
        assert_eq!(negative.len(), 1);
        assert_eq!(negative[0].time, 0.0);
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
        for bad in ["bogus", "inf", "NaN"] {
            let xml =
                format!("<testsuite><testcase name=\"a\" time=\"{bad}\"></testcase></testsuite>");
            assert!(parse_test_xml(xml.as_bytes(), 0, 0).is_err(), "{bad}");
        }
        // Negative durations clamp to zero (clock skew, see above).
        let xml = "<testsuite><testcase name=\"a\" time=\"-1\"></testcase></testsuite>";
        let clamped = parse_test_xml(xml.as_bytes(), 0, 0).expect("negative time clamps");
        assert_eq!(clamped.len(), 1);
        assert_eq!(clamped[0].time, 0.0);
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

    #[test]
    fn junit_strip_timestamp_uses_typed_events() {
        // Single quotes and whitespace around `=` are real attributes.
        let single = r#"<testsuite name="a" timestamp = '2026-09-19T21:10:02'><testcase name="a"/></testsuite>"#;
        let stripped = strip_timestamp_attrs(single);
        assert!(!stripped.contains("2026-09-19T21:10:02"));
        assert!(stripped.contains(r#"name="a""#));
        assert!(parse_test_xml(single.as_bytes(), 0, 0).is_ok());

        // `>` inside another attribute value must not truncate the tag scan.
        let gt = r#"<testsuite name="a>b" timestamp="2026-09-19T21:10:02"><testcase name="a"/></testsuite>"#;
        let stripped = strip_timestamp_attrs(gt);
        assert!(!stripped.contains("2026-09-19T21:10:02"));
        assert!(stripped.contains(r#"name="a>b""#));
        assert!(parse_test_xml(gt.as_bytes(), 0, 0).is_ok());

        // `timestamp`-like text inside another attribute value is preserved.
        let value_lookalike = r#"<testsuite name='a timestamp="kept" b' timestamp="2026-09-19T21:10:02"><testcase name="a"/></testsuite>"#;
        let stripped = strip_timestamp_attrs(value_lookalike);
        assert!(!stripped.contains("2026-09-19T21:10:02"));
        assert!(stripped.contains(r#"timestamp="kept""#));
        assert!(parse_test_xml(value_lookalike.as_bytes(), 0, 0).is_ok());

        // CDATA containing a testsuite-like tag passes through byte-identical.
        let cdata_inner = r#"<testsuite timestamp="2026-09-19T21:10:02">"#;
        let cdata = r#"<testsuite name="a" timestamp="2026-09-19T21:10:02"><testcase name="a"><failure><![CDATA[<testsuite timestamp="2026-09-19T21:10:02">]]></failure></testcase></testsuite>"#;
        let stripped = strip_timestamp_attrs(cdata);
        assert!(stripped.contains(cdata_inner));
        let cases = parse_test_xml(cdata.as_bytes(), 0, 0).expect("cdata");
        assert!(cases[0]
            .failure
            .as_ref()
            .expect("failure")
            .text
            .contains("testsuite"));

        // Comments containing a testsuite-like tag are preserved.
        let comment = r#"<!-- <testsuite timestamp="2026-09-19T21:10:02"> -->"#;
        let with_comment = format!(
            r#"<testsuite name="a" timestamp="2026-09-19T21:10:02">{comment}<testcase name="a"/></testsuite>"#
        );
        let stripped = strip_timestamp_attrs(&with_comment);
        assert!(stripped.contains(comment));
        assert!(parse_test_xml(with_comment.as_bytes(), 0, 0).is_ok());

        // Timestamps on the testsuites root and testcase tags are also dropped.
        let roots = r#"<testsuites timestamp="2026-09-19T21:10:02"><testsuite><testcase name="a"/></testsuite></testsuites>"#;
        assert!(parse_test_xml(roots.as_bytes(), 0, 0).is_ok());
        let case_ts =
            r#"<testsuite><testcase name="a" timestamp="2026-09-19T21:10:02"/></testsuite>"#;
        assert!(parse_test_xml(case_ts.as_bytes(), 0, 0).is_ok());
    }
}
