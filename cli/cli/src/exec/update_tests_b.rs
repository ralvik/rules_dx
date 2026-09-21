//! Split from `update.rs`. No behavior change.
//! Originally the inline `mod tests`.
#![allow(unused_imports)]

use super::super::test_support::*;
use super::update_tests_a::*;
use super::*;

#[test]
fn check_clean_passes_without_launching() {
    let harness = Harness::new("update-check-clean");
    harness.write_source(
        ".bazelrc",
        "import %workspace%/tools/bazelrc/preset.bazelrc\ntry-import %workspace%/user.bazelrc\n",
    );
    harness.write_source(
        "tools/bazelrc/preset.bazelrc",
        &dx_adopt::render_preset_fragment(),
    );
    let (code, out, err) = harness.run(&["update", "--check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("Running update --check for preset"), "{out}");
    assert!(out.contains("preset clean"), "{out}");
    assert_eq!(err, "", "{err}");
    assert!(
        harness.seen_env.borrow().is_empty(),
        "check launches nothing"
    );
}

#[test]
fn check_stale_fails_with_diff_and_no_mutation() {
    let harness = Harness::new("update-check-stale");
    harness.write_source(
        ".bazelrc",
        "import %workspace%/tools/bazelrc/preset.bazelrc\ntry-import %workspace%/user.bazelrc\n",
    );
    harness.write_source("tools/bazelrc/preset.bazelrc", "# dirty\n");
    let (code, out, err) = harness.run(&["update", "--check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("stale"), "{out}");
    assert!(out.contains("checked-in"), "{out}");
    assert_eq!(err, "", "{err}");
    // Check never writes.
    assert_eq!(
        std::fs::read_to_string(harness.workspace.join("tools/bazelrc/preset.bazelrc"))
            .expect("read"),
        "# dirty\n"
    );
    assert!(harness.seen_env.borrow().is_empty());
}

#[test]
fn check_missing_fails_closed() {
    let harness = Harness::new("update-check-missing");
    harness.write_source(
        ".bazelrc",
        "import %workspace%/tools/bazelrc/preset.bazelrc\n",
    );
    let (code, out, err) = harness.run(&["update", "--check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("stale"), "{out}");
    assert_eq!(err, "", "{err}");
}

#[test]
fn check_collision_fails_operational() {
    let harness = Harness::new("update-check-collision");
    harness.write_source(
        ".bazelrc",
        "import %workspace%/tools/bazelrc/preset.bazelrc\ncommon --enable_bzlmod\n",
    );
    harness.write_source(
        "tools/bazelrc/preset.bazelrc",
        &dx_adopt::render_preset_fragment(),
    );
    let (code, _, err) = harness.run(&["update", "--check"]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("update_failed"), "{err}");
    assert!(err.contains("duplicates preset"), "{err}");
}

#[test]
fn check_json_reports_stale_and_clean() {
    let clean = Harness::new("update-check-json-clean");
    clean.write_source(
        ".bazelrc",
        "import %workspace%/tools/bazelrc/preset.bazelrc\n",
    );
    clean.write_source(
        "tools/bazelrc/preset.bazelrc",
        &dx_adopt::render_preset_fragment(),
    );
    let (code, out, err) = clean.run(&["update", "--check", "--output=json"]);
    assert_eq!(code, 0, "{out}{err}");
    let events: Vec<serde_json::Value> = out
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .expect("NDJSON");
    assert_eq!(events[0]["event"], serde_json::json!("command_started"));
    assert_eq!(events[0]["mode"], serde_json::json!("check"));
    assert_eq!(
        events.last().expect("finished")["exit_code"],
        serde_json::json!(0)
    );

    let dirty = Harness::new("update-check-json-stale");
    dirty.write_source(
        ".bazelrc",
        "import %workspace%/tools/bazelrc/preset.bazelrc\n",
    );
    dirty.write_source("tools/bazelrc/preset.bazelrc", "# dirty\n");
    let (code, out, err) = dirty.run(&["update", "--check", "--output=json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("\"event\":\"error\""), "{out}");
    assert!(out.contains("update_failed"), "{out}");
}

#[test]
fn default_updates_preset_atomically() {
    // Go is a no-op backend (no launch), so the preset fix is the
    // only mutation; proves default mode regenerates the fragment.
    let harness = Harness::new("update-default-preset");
    harness.write_source(
        ".bazelrc",
        "import %workspace%/tools/bazelrc/preset.bazelrc\ntry-import %workspace%/user.bazelrc\n",
    );
    harness.write_source("tools/bazelrc/preset.bazelrc", "# dirty\n");
    let (code, out, err) = harness.run(&["update", "go"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("updated preset (tools/bazelrc/preset.bazelrc)"),
        "{out}"
    );
    assert_eq!(
        std::fs::read_to_string(harness.workspace.join("tools/bazelrc/preset.bazelrc"))
            .expect("read"),
        dx_adopt::render_preset_fragment()
    );
    assert_eq!(err, "", "{err}");
}

#[test]
fn update_json_never_emits_change_or_mutation() {
    // Wont-fix, Issue #586 (See: `docs/cli/output-protocol.md#mutation`):
    // backends provide no committed-change
    // manifest and Git/BUILD inference is forbidden, so update JSON
    // never emits change/mutation events in any mode.
    let runner = ScriptRunner::new(&[]);
    let (code, out, err) = run_with(&["update", "--output=json"], &runner);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        !out.contains("\"event\":\"change\""),
        "update must not emit change events: {out}"
    );
    assert!(
        !out.contains("\"event\":\"mutation\""),
        "update must not emit mutation events: {out}"
    );
    assert!(
        !out.contains("\"event\":\"diagnostic\""),
        "update must not emit diagnostics: {out}"
    );
    assert!(
        !out.contains("\"event\":\"operation\""),
        "update must not emit operations: {out}"
    );
    let events: Vec<serde_json::Value> = out
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .expect("NDJSON");
    for event in &events {
        let kind = event["event"].as_str().expect("event");
        assert!(
            kind == "command_started"
                || kind == "notice"
                || kind == "error"
                || kind == "command_finished",
            "unexpected update event {kind}: {out}"
        );
    }
}

#[test]
fn update_json_completeness_is_per_set_plus_finished() {
    // Event-completeness contract, Issue #586 (See:
    // `docs/cli/output-protocol.md#mutation`): exactly one terminal
    // per-set event per selected set in sorted order, then an
    // `update_recovery` notice on failure (issue #772, See:
    // `docs/cli/commands/audit-update-bazel.md#dx-update`), then exactly
    // one command_finished. Preceding per-set events stay true with
    // no automatic rollback; nothing is inferred for unattempted sets.
    let runner = ScriptRunner::new(&[("maven", Some(1))]);
    let (code, out, err) = run_with(&["update", "--output=json"], &runner);
    assert_eq!(code, 1, "{out}{err}");
    let events: Vec<serde_json::Value> = out
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .expect("NDJSON");
    let kinds: Vec<&str> = events
        .iter()
        .map(|event| event["event"].as_str().expect("event"))
        .collect();
    assert_eq!(kinds[0], "command_started");
    assert_eq!(kinds[kinds.len() - 1], "command_finished");
    // Five selected sets means five terminal per-set events plus one
    // recovery notice before finished.
    let middle = &events[1..events.len() - 1];
    assert_eq!(middle.len(), 6, "{out}");
    let (per_set, recovery) = (&middle[..5], &middle[5]);
    assert_eq!(
        recovery["code"],
        serde_json::json!("update_recovery"),
        "{out}"
    );
    assert_eq!(recovery["event"], serde_json::json!("notice"), "{out}");
    let recovery_message = recovery["message"].as_str().expect("message");
    assert!(recovery_message.contains("dx update maven"), "{out}");
    assert!(recovery_message.contains("idempotent"), "{out}");
    assert!(err.contains("update_recovery"), "{err}");
    let scopes: Vec<String> = per_set
        .iter()
        .map(|event| {
            let kind = event["event"].as_str().expect("event");
            if kind == "error" {
                // Per-set failures ride `update_failed`
                // errors without a scope; the message names the set.
                assert_eq!(
                    event["code"].as_str().expect("code"),
                    "update_failed",
                    "{out}"
                );
                let message = event["message"].as_str().expect("message");
                assert!(message.contains("maven"), "{out}");
                "maven".to_owned()
            } else {
                assert_eq!(kind, "notice", "{out}");
                assert_eq!(
                    event["code"].as_str().expect("code"),
                    "update_set_success",
                    "{out}"
                );
                event["scope"][0].as_str().expect("scope").to_owned()
            }
        })
        .collect();
    let mut sorted = scopes.clone();
    sorted.sort();
    assert_eq!(scopes, sorted, "per-set events use sorted set order: {out}");
    // Finished keeps results_complete (every set reached a terminal
    // report, including the failure) and omits file-level counts.
    let finished = events.last().expect("finished");
    assert_eq!(finished["exit_code"], serde_json::json!(1));
    assert_eq!(finished["results_complete"], serde_json::json!(true));
    assert!(finished.get("changes").is_none(), "{out}");
    assert!(finished.get("mutations").is_none(), "{out}");
    assert!(finished.get("diagnostics").is_none(), "{out}");
}

#[test]
fn update_json_check_and_dryrun_emit_no_file_events_or_counts() {
    // The wont-fix holds for --check and --dry-run too, Issue #586 (See:
    // `docs/cli/output-protocol.md#mutation`); neither emits file-level
    // events nor counts.
    let harness = Harness::new("update-586-check-json");
    harness.write_source(
        ".bazelrc",
        "import %workspace%/tools/bazelrc/preset.bazelrc\n",
    );
    harness.write_source(
        "tools/bazelrc/preset.bazelrc",
        &dx_adopt::render_preset_fragment(),
    );
    let (code, out, err) = harness.run(&["update", "--check", "--output=json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("\"event\":\"change\""), "{out}");
    assert!(!out.contains("\"event\":\"mutation\""), "{out}");
    let events: Vec<serde_json::Value> = out
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .expect("NDJSON");
    let finished = events.last().expect("finished");
    assert!(finished.get("changes").is_none(), "{out}");
    assert!(finished.get("mutations").is_none(), "{out}");
    assert!(finished.get("diagnostics").is_none(), "{out}");

    let dry = Harness::new("update-586-dryrun-json");
    let (code, out, err) = dry.run(&["update", "--dry-run", "--output=json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("\"event\":\"change\""), "{out}");
    assert!(!out.contains("\"event\":\"mutation\""), "{out}");
}

#[test]
fn update_failure_reports_recovery_in_text_and_json() {
    // Issue #772 (See: `docs/cli/commands/audit-update-bazel.md#dx-update`):
    // a failed run keeps per-set commits and reports the manual recovery
    // (idempotent retry plus `git checkout` restore) in both modes.
    let runner = ScriptRunner::new(&[("maven", Some(1))]);
    let (code, out, err) = run_with(&["update"], &runner);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("update_recovery"), "{err}");
    assert!(err.contains("dx update maven"), "{err}");
    assert!(err.contains("idempotent"), "{err}");
    assert!(err.contains("git checkout --"), "{err}");

    let runner = ScriptRunner::new(&[("maven", Some(1))]);
    let (code, out, err) = run_with(&["update", "--output=json"], &runner);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("\"code\":\"update_recovery\""), "{out}");
    assert!(out.contains("dx update maven"), "{out}");
    assert!(err.contains("update_recovery"), "{err}");
}

#[test]
fn update_success_emits_no_recovery() {
    // Issue #772 (See: `docs/cli/commands/audit-update-bazel.md#dx-update`):
    // clean runs need no recovery hint.
    let runner = ScriptRunner::new(&[]);
    let (code, out, err) = run_with(&["update"], &runner);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!err.contains("update_recovery"), "{err}");
    assert!(!out.contains("update_recovery"), "{out}");

    let runner = ScriptRunner::new(&[]);
    let (code, out, err) = run_with(&["update", "--output=json"], &runner);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("update_recovery"), "{out}");
}
