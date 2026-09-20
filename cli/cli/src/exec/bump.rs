//! Bump command execution: explicit widen-one-requirement.

use super::common::*;
use crate::args::{Command, Invocation};
use crate::reports::plan_reports;
use dx_output::{
    command_finished, command_started, notice_event, write_event, FinishedCounts, NoticeEvent,
    OutputMode,
};

/// Runs `dx bump <selector> <version>`: validates the single
/// `set:package` selector plus new version through `dx_bump`, mutating
/// without confirmation. `--dry-run` prints the planned widen and exits
/// `0` without touching the tree; live execution rewrites exactly one
/// declared requirement atomically (never batch) and reports the
/// resolver-owned follow-up (`dx update <set>` for Cargo/npm/Go/Maven/NuGet,
/// preset flag-diff review plus build for Bazel/GitHub Actions). Usage errors
/// exit `2` before any write; widen failures exit `1` with `bump_failed`.
pub(crate) fn execute_bump(invocation: &Invocation, env: Env<'_>) -> i32 {
    debug_assert!(
        invocation.command == Command::Bump,
        "bump dispatch guards commands"
    );
    let Env {
        workspace,
        out,
        err,
        ..
    } = env;
    match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(_) => {}
        Err(error) => return pre_exec(err, &error.to_string()),
    }
    if invocation.targets.len() != 2 {
        return pre_exec(err, "bump needs exactly <selector> <version>");
    }
    let request = match dx_bump::BumpRequest::parse(&invocation.targets[0], &invocation.targets[1])
    {
        Ok(request) => request,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let summary = request.summary();
    let verbose =
        matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet;
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, "default") {
                let _ = write_event(out, &event);
            }
            if let Ok(event) = notice_event(&NoticeEvent {
                level: "info".to_owned(),
                code: "bump_planned".to_owned(),
                message: summary,
                related_command: Some("bump".to_owned()),
                scope: Some(vec![request.selector.clone()]),
                path: Some(request.target_manifest().to_owned()),
                language: None,
                import: None,
            }) {
                let _ = write_event(out, &event);
            }
            let finished = command_finished(0, &FinishedCounts::default());
            let _ = write_event(out, &finished);
        } else if verbose {
            let _ = writeln!(out, "{summary}");
        }
        return 0;
    }
    if invocation.output == OutputMode::Json {
        if let Ok(event) = command_started(invocation.command.name(), false, "default") {
            let _ = write_event(out, &event);
        }
    } else if verbose {
        let _ = writeln!(out, "{summary}");
    }
    // Live: read the owning manifest, plan the single-requirement edit
    // over its bytes, and commit atomically. Any failure leaves the tree
    // untouched and reports `bump_failed` (exit 1).
    let manifest = request.target_manifest();
    let original = match std::fs::read(workspace.join(manifest)) {
        Ok(bytes) => bytes,
        Err(_) => {
            return operational(
                invocation,
                out,
                err,
                CODE_BUMP_FAILED,
                &format!(
                    "failed to widen {}: cannot read {manifest}",
                    request.selector
                ),
            );
        }
    };
    let text = match String::from_utf8(original) {
        Ok(text) => text,
        Err(_) => {
            return operational(
                invocation,
                out,
                err,
                CODE_BUMP_FAILED,
                &format!(
                    "failed to widen {}: {manifest} is not valid UTF-8",
                    request.selector
                ),
            );
        }
    };
    let widened = match request.plan_edit(&text) {
        Ok(widened) => widened,
        Err(error) => {
            return operational(
                invocation,
                out,
                err,
                CODE_BUMP_FAILED,
                &format!("failed to widen {}: {error}", request.selector),
            );
        }
    };
    if dx_atomic_fs::write_atomic(&workspace.join(manifest), widened.as_bytes()).is_err() {
        return operational(
            invocation,
            out,
            err,
            CODE_BUMP_FAILED,
            &format!(
                "failed to widen {}: cannot write {manifest}",
                request.selector
            ),
        );
    }
    let next = if request.needs_update_refresh() {
        format!(
            "then run `dx update {}` for resolver-owned lock refresh",
            request.set.name()
        )
    } else {
        "then run preset flag-diff review plus `bazel build //...`".to_owned()
    };
    let message = format!(
        "widened {} to {} in {manifest} ({next})",
        request.selector,
        request.version.display()
    );
    if invocation.output == OutputMode::Json {
        if let Ok(event) = notice_event(&NoticeEvent {
            level: "info".to_owned(),
            code: "bump_widened".to_owned(),
            message: message.clone(),
            related_command: Some("bump".to_owned()),
            scope: Some(vec![request.selector.clone()]),
            path: Some(manifest.to_owned()),
            language: None,
            import: None,
        }) {
            let _ = write_event(out, &event);
        }
        let finished = command_finished(
            0,
            &FinishedCounts {
                results_complete: Some(true),
                ..FinishedCounts::default()
            },
        );
        let _ = write_event(out, &finished);
        return 0;
    }
    if verbose {
        let _ = writeln!(out, "{message}");
    }
    0
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;

    #[test]
    fn dry_run_plans_without_writing() {
        let harness = Harness::new("bump-dryrun");
        harness.write_source(
            "rust/tests/fixtures/hello/Cargo.toml",
            "[dependencies]\nanyhow = \"1\"\n",
        );
        let (code, out, err) = harness.run(&["bump", "cargo:anyhow", "1.2.3", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Widen cargo:anyhow"), "{out}");
        assert_eq!(err, "", "{err}");
        // Dry-run writes nothing.
        assert_eq!(
            std::fs::read_to_string(
                harness
                    .workspace
                    .join("rust/tests/fixtures/hello/Cargo.toml")
            )
            .expect("read"),
            "[dependencies]\nanyhow = \"1\"\n"
        );
    }

    #[test]
    fn dry_run_rejects_bad_selector_and_version() {
        let harness = Harness::new("bump-dryrun-bad-selector");
        let (code, _, err) = harness.run(&["bump", "crates", "1.2.3", "--dry-run"]);
        assert_eq!(code, 2, "{err}");
        let harness = Harness::new("bump-dryrun-bad-version");
        let (code, _, err) = harness.run(&["bump", "cargo:anyhow", "nope!!!", "--dry-run"]);
        assert_eq!(code, 2, "{err}");
    }

    #[test]
    fn live_widens_one_cargo_requirement_atomically() {
        let harness = Harness::new("bump-live-cargo");
        harness.write_source(
            "rust/tests/fixtures/hello/Cargo.toml",
            "[dependencies]\nanyhow = \"1\"\nserde = \"1\"\n",
        );
        let (code, out, err) = harness.run(&["bump", "cargo:anyhow", "1.2.3"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("widened cargo:anyhow to 1.2.3"), "{out}");
        assert!(err.is_empty(), "{err}");
        let widened = std::fs::read_to_string(
            harness
                .workspace
                .join("rust/tests/fixtures/hello/Cargo.toml"),
        )
        .expect("read");
        assert!(widened.contains("anyhow = \"1.2.3\""), "{widened}");
        assert!(widened.contains("serde = \"1\""), "{widened}");
    }

    #[test]
    fn live_missing_requirement_fails_without_writing() {
        let harness = Harness::new("bump-live-missing");
        harness.write_source(
            "rust/tests/fixtures/hello/Cargo.toml",
            "[dependencies]\nserde = \"1\"\n",
        );
        let (code, _, err) = harness.run(&["bump", "cargo:anyhow", "1.2.3"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("bump_failed"), "{err}");
        assert!(err.contains("no declared requirement"), "{err}");
        assert_eq!(
            std::fs::read_to_string(
                harness
                    .workspace
                    .join("rust/tests/fixtures/hello/Cargo.toml")
            )
            .expect("read"),
            "[dependencies]\nserde = \"1\"\n"
        );
    }

    #[test]
    fn live_github_tag_needs_sha_resolution() {
        let harness = Harness::new("bump-live-gha-tag");
        harness.write_source(
            ".github/workflows/ci.yml",
            "      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7\n",
        );
        let (code, _, err) = harness.run(&["bump", "github-actions:actions/checkout", "v5"]);
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("bump_failed"), "{err}");
        assert!(err.contains("needs SHA resolution"), "{err}");
    }

    #[test]
    fn live_widens_one_maven_artifact_atomically() {
        let harness = Harness::new("bump-live-maven");
        harness.write_source(
            "MODULE.bazel",
            "maven.install(\n    artifacts = [\n        \"junit:junit:4.13.2\",\n        \"org.junit.jupiter:junit-jupiter-api:6.1.3\",\n    ],\n)\n",
        );
        let (code, out, err) = harness.run(&["bump", "maven:junit:junit", "4.13.3"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("widened maven:junit:junit to 4.13.3"), "{out}");
        assert!(out.contains("dx update maven"), "{out}");
        assert!(err.is_empty(), "{err}");
        let widened =
            std::fs::read_to_string(harness.workspace.join("MODULE.bazel")).expect("read");
        assert!(widened.contains("\"junit:junit:4.13.3\""), "{widened}");
        assert!(
            widened.contains("\"org.junit.jupiter:junit-jupiter-api:6.1.3\""),
            "{widened}"
        );
    }

    #[test]
    fn live_widens_one_nuget_requirement_atomically() {
        let harness = Harness::new("bump-live-nuget");
        harness.write_source(
            "third_party/dotnet/paket.dependencies",
            "source https://api.nuget.org/v3/index.json\nframework: net10.0\n\nnuget FSharp.Core 10.1.201\nnuget xunit.v3 4.0.0\n",
        );
        let (code, out, err) = harness.run(&["bump", "nuget:FSharp.Core", "10.1.202"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("widened nuget:FSharp.Core to 10.1.202"),
            "{out}"
        );
        assert!(out.contains("dx update nuget"), "{out}");
        assert!(err.is_empty(), "{err}");
        let widened = std::fs::read_to_string(
            harness
                .workspace
                .join("third_party/dotnet/paket.dependencies"),
        )
        .expect("read");
        assert!(widened.contains("nuget FSharp.Core 10.1.202"), "{widened}");
        assert!(widened.contains("nuget xunit.v3 4.0.0"), "{widened}");
    }

    #[test]
    fn live_json_emits_planned_or_widened_and_finished() {
        let harness = Harness::new("bump-live-json");
        harness.write_source(".bazelversion", "9.2.0\n");
        let (code, out, err) =
            harness.run(&["bump", "bazel:.bazelversion", "9.3.0", "--output=json"]);
        assert_eq!(code, 0, "{out}{err}");
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
        assert!(kinds.contains(&"notice"));
        assert_eq!(
            events.last().expect("finished")["exit_code"],
            serde_json::json!(0)
        );
        assert_eq!(err, "", "{err}");
        assert_eq!(
            std::fs::read_to_string(harness.workspace.join(".bazelversion")).expect("read"),
            "9.3.0\n"
        );
    }
}
