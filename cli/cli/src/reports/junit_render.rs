//! JUnit report rendering on `quick-junit` (issue #388).
//!
//! Replaces the hand-rolled `quick-xml` writer (`sanitize_xml`,
//! `format_junit_time`, `emit_junit`, `finish_junit`,
//! `junit_message_element`) with the nextest data model
//! ([`quick_junit::Report`], [`quick_junit::TestSuite`],
//! [`quick_junit::TestCase`]). Invalid XML chars and ANSI escapes are
//! stripped by [`quick_junit::XmlString`]; times render as seconds with
//! three decimals via the crate. Re-exported through [`super::junit`]
//! so the public paths stay
//! `crate::reports::{render_junit, junit_infrastructure_case}` and
//! `crate::reports::junit::{render_junit, junit_infrastructure_case}`.
//! Shares the normalized case types from [`super::junit_types`].

use super::junit_types::{JunitCase, JunitMessage};

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

fn junit_duration(time: f64) -> std::time::Duration {
    if time.is_finite() && time >= 0.0 {
        std::time::Duration::try_from_secs_f64(time).unwrap_or(std::time::Duration::ZERO)
    } else {
        std::time::Duration::ZERO
    }
}

fn junit_status(case: &JunitCase) -> quick_junit::TestCaseStatus {
    if let Some(failure) = &case.failure {
        let mut status =
            quick_junit::TestCaseStatus::non_success(quick_junit::NonSuccessKind::Failure);
        if let Some(note) = &failure.message {
            status.set_message(note.as_str());
        }
        if !failure.text.is_empty() {
            status.set_description(failure.text.as_str());
        }
        status
    } else if let Some(error) = &case.error {
        let mut status =
            quick_junit::TestCaseStatus::non_success(quick_junit::NonSuccessKind::Error);
        if let Some(note) = &error.message {
            status.set_message(note.as_str());
        }
        if !error.text.is_empty() {
            status.set_description(error.text.as_str());
        }
        status
    } else if let Some(skipped) = &case.skipped {
        let mut status = quick_junit::TestCaseStatus::skipped();
        if let Some(note) = &skipped.message {
            status.set_message(note.as_str());
        }
        if !skipped.text.is_empty() {
            status.set_description(skipped.text.as_str());
        }
        status
    } else {
        quick_junit::TestCaseStatus::success()
    }
}

/// Renders normalized Bazel test cases as one JUnit XML document.
///
/// `suites` groups parsed cases by Bazel target label; every case in
/// one group shares that label. Suites order bytewise by label; cases
/// order bytewise by original name, then shard, then attempt. Retries
/// and shards stay separate cases with zero-based suffixes. Root and
/// suite `tests`, `failures`, `errors`, `skipped`, and `time` counts
/// are aggregated by `quick-junit` from the normalized cases.
pub fn render_junit(suites: &[(String, Vec<JunitCase>)]) -> String {
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

    let mut report = quick_junit::Report::new("dx");
    let mut total_time = std::time::Duration::ZERO;
    for (label, cases) in &ordered {
        let mut suite = quick_junit::TestSuite::new(label.as_str());
        let mut suite_time = std::time::Duration::ZERO;
        for case in cases {
            let display = junit_display_name(&case.name, case.shard, case.attempt);
            let mut test_case = quick_junit::TestCase::new(display, junit_status(case));
            if let Some(classname) = &case.classname {
                test_case.set_classname(classname.as_str());
            }
            let duration = junit_duration(case.time);
            test_case.set_time(duration);
            suite_time += duration;
            if let Some(out) = &case.system_out {
                test_case.set_system_out(out.as_str());
            }
            if let Some(err_text) = &case.system_err {
                test_case.set_system_err(err_text.as_str());
            }
            suite.add_test_case(test_case);
        }
        suite.set_time(suite_time);
        total_time += suite_time;
        report.add_test_suite(suite);
    }
    report.set_time(total_time);
    report
        .to_string()
        .unwrap_or_else(|err| unreachable!("junit report serialization is infallible: {err:?}"))
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
    use super::super::junit_parse::parse_test_xml;
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
        assert_eq!(junit_duration(f64::NAN), std::time::Duration::ZERO);
        assert_eq!(junit_duration(-1.0), std::time::Duration::ZERO);
        assert_eq!(junit_duration(f64::INFINITY), std::time::Duration::ZERO);
        assert_eq!(
            junit_duration(1.23456),
            std::time::Duration::try_from_secs_f64(1.23456).expect("duration")
        );
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
        // Failure-first precedence: a case carrying both failure and
        // error renders the failure (quick-junit holds one main status).
        let doc = render_junit(&[(
            "s".to_owned(),
            vec![JunitCase {
                failure: Some(JunitMessage {
                    message: Some("f".to_owned()),
                    text: "ft".to_owned(),
                }),
                error: Some(JunitMessage {
                    message: Some("e".to_owned()),
                    text: "et".to_owned(),
                }),
                ..junit_case("both")
            }],
        )]);
        assert!(doc.contains("<failure"), "{doc}");
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
                        system_out: Some("out".to_owned()),
                        system_err: Some("err".to_owned()),
                        shard: 1,
                        attempt: 2,
                        ..junit_case("a2")
                    },
                    JunitCase {
                        skipped: Some(JunitMessage {
                            message: None,
                            text: String::new(),
                        }),
                        ..junit_case("a3")
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
<testsuites name="dx" tests="4" skipped="1" failures="1" errors="1" time="0.000">
    <testsuite name="//a:t" tests="3" skipped="1" errors="1" failures="1" time="0.000">
        <testcase name="a1" time="0.000">
            <failure message="f">ft</failure>
        </testcase>
        <testcase name="a2 [shard=1,attempt=2]" time="0.000">
            <error/>
            <system-out>out</system-out>
            <system-err>err</system-err>
        </testcase>
        <testcase name="a3" time="0.000">
            <skipped/>
        </testcase>
    </testsuite>
    <testsuite name="//b:t" tests="1" skipped="0" errors="0" failures="0" time="0.000">
        <testcase name="b" time="0.000"/>
    </testsuite>
</testsuites>"#);
    }

    #[test]
    fn junit_writer_escapes_specials_and_strips_controls() {
        // Golden: `&<>"'` plus control bytes must round-trip through the
        // quick-junit writer as well-formed XML (issue #219).
        // quick-junit strips invalid XML chars (plus ANSI escapes) rather
        // than replacing with U+FFFD.
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
        // Raw control bytes must never reach the document; quick-junit
        // strips them while `\t\n\r` would be preserved.
        for raw in ['\u{0}', '\u{1}', '\u{8}', '\u{b}', '\u{c}', '\u{e}'] {
            assert!(!doc.contains(raw), "{doc}");
        }
        // The writer output must re-parse as XML (well-formedness proof
        // for Jenkins/GitLab ingestion).
        let reparsed = parse_test_xml(doc.as_bytes(), 0, 0).expect("reparse");
        assert_eq!(reparsed.len(), 1);
        assert!(reparsed[0].failure.is_some());
    }
}
