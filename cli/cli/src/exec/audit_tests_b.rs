//! Split from `audit.rs`. No behavior change.
//! Originally the inline `mod tests`.

use super::super::test_support::*;
use super::audit_tests_a::*;

#[test]
fn audit_live_unowned_scope_fails_usage() {
    let harness = Harness::new("audit-unowned");
    let (code, _out, err) = harness.run(&["audit", "python/tests/fixtures/hello/hello.py"]);
    assert_eq!(code, 2, "{err}");
}

#[test]
fn audit_live_json_emits_per_family_lifecycle() {
    let runner = AuditRunner::clean();
    let (code, out, err) = run_with(
        &["audit", "security", "--output=json"],
        &runner,
        &|harness| {
            harness.write_source(
            "rust/tests/fixtures/hello/Cargo.lock",
            "[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
        );
            harness.write_source("pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
            harness.write_source("third_party/jvm/maven_install.json", r#"{"artifacts": {}}"#);
            harness.write_source(
                "third_party/dotnet/paket.lock",
                "NUGET\n  remote: https://api.nuget.org/v3/index.json\n",
            );
            write_go_mod(harness);
            write_all_empty_advisories(harness);
        },
    );
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
}

#[test]
fn audit_live_json_failure_emits_error_and_finished_one() {
    let sarif = r#"{"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "gitleaks"}}, "results": [{"ruleId": "gitleaks/aws-key", "message": {"text": "AWS key"}}]}]}"#;
    let runner = AuditRunner::with_sarif(Some(1), sarif);
    let (code, out, err) = run_with(&["audit", "--output=json"], &runner, &|harness| {
        harness.write_source(
            "rust/tests/fixtures/hello/Cargo.lock",
            "[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
        );
        harness.write_source("pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
        harness.write_source("third_party/jvm/maven_install.json", r#"{"artifacts": {}}"#);
        harness.write_source(
            "third_party/dotnet/paket.lock",
            "NUGET\n  remote: https://api.nuget.org/v3/index.json\n",
        );
        write_go_mod(harness);
        write_all_empty_advisories(harness);
        harness.write_source(
            "cargo-bazel-lock.json",
            r#"{"packages": {"serde 1.0.100": {"license": "MIT"}}}"#,
        );
        harness.write_source(
            "licenses.toml",
            "[policy.distributed]\nallow = [\"MIT\"]\nreview = []\ndeny = []\n",
        );
    });
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("audit_failed"), "{err}");
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
    assert!(kinds.contains(&"error"));
    assert_eq!(
        events.last().expect("finished")["exit_code"],
        serde_json::json!(1)
    );
}

#[test]
fn audit_dry_run_json_emits_lifecycle() {
    let harness = Harness::new("audit-dryrun-json");
    let (code, out, err) = harness.run(&["audit", "--dry-run", "--output=json"]);
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
    assert_eq!(kinds, vec!["command_started", "command_finished"]);
    assert_eq!(
        events.last().expect("finished")["exit_code"],
        serde_json::json!(0)
    );
    assert!(
        harness.seen_env.borrow().is_empty(),
        "dry-run launches nothing"
    );
}

#[test]
fn audit_update_dry_run_quiet_prints_nothing() {
    for argv in [
        vec!["audit", "--dry-run", "--quiet"],
        vec!["update", "--dry-run", "--quiet"],
    ] {
        let name = format!("dryrun-quiet-{}", argv[0]);
        let harness = Harness::new(&name);
        let (code, out, err) = harness.run(&argv);
        assert_eq!(code, 0, "{out}{err}");
        assert_eq!(out, "", "{out}");
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );
    }
}

#[test]
fn audit_reports_sarif_and_spdx_to_files() {
    use crate::args::parse;
    use crate::exec::{execute, Env};
    let runner = AuditRunner::clean();
    // clean_workspace uses UNKNOWN npm which fails distributed; use cargo-only scope for clean reports.
    let harness = Harness::new("audit-reports-cargo");
    harness.write_source(
        "rust/tests/fixtures/hello/Cargo.lock",
        "[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
    );
    harness.write_source(
        "cargo-bazel-lock.json",
        r#"{"packages": {"serde 1.0.100": {"license": "MIT"}}}"#,
    );
    harness.write_source(
        "licenses.toml",
        "[policy.distributed]\nallow = [\"MIT\"]\nreview = []\ndeny = []\n\n[[inventory]]\npackage = \"serde\"\nset = \"cargo\"\nlicense = \"MIT\"\nversions = \"1.0.100\"\ntext_present = true\n",
    );
    let invocation = parse(&[
        "audit".to_owned(),
        "license".to_owned(),
        "//rust/tests/fixtures/hello:hello".to_owned(),
        "--report=sarif=out.sarif".to_owned(),
        "--report=spdx=out.spdx.json".to_owned(),
    ])
    .expect("parse");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = execute(
        &invocation,
        Env {
            workspace: &harness.workspace,
            runner: &runner,
            query_runner: &harness.query,
            temp_dir: &harness.temp,
            pid: std::process::id(),
            nonce: 0,
            out: &mut out,
            err: &mut err,
            ci: false,
        },
    );
    assert_eq!(runner.calls.borrow().len(), 0);
    // License-only run launches no subprocess; reports still write.
    let out_text = String::from_utf8(out).expect("stdout");
    let err_text = String::from_utf8(err).expect("stderr");
    assert_eq!(code, 0, "{out_text}{err_text}");
    // Issue #632: SARIF run shape golden for a clean license-only
    // run: SARIF 2.1.0, one deterministically ordered `license` run,
    // empty results kept, no partial-invocation marker when complete.
    let sarif = std::fs::read_to_string(harness.workspace.join("out.sarif")).expect("sarif");
    let value: serde_json::Value = serde_json::from_str(&sarif).expect("sarif JSON");
    assert_eq!(value["version"], serde_json::json!("2.1.0"));
    assert_eq!(
        value["$schema"],
        serde_json::json!("https://json.schemastore.org/sarif-2.1.0.json")
    );
    let runs = value["runs"].as_array().expect("runs");
    assert_eq!(runs.len(), 1, "{value}");
    assert_eq!(
        runs[0]["tool"]["driver"]["name"],
        serde_json::json!("license")
    );
    assert_eq!(runs[0]["results"], serde_json::json!([]));
    assert_eq!(runs[0]["tool"]["driver"]["rules"], serde_json::json!([]));
    assert!(
        runs[0].get("invocations").is_none(),
        "complete run carries no unsuccessful invocation: {value}"
    );
    // Typed SARIF parses: the live document is schema-valid.
    let typed: serde_sarif::sarif::Sarif = serde_json::from_str(&sarif).expect("typed SARIF");
    assert_eq!(typed.runs.len(), 1);
    assert_eq!(typed.runs[0].tool.driver.name, "license");
    // Issue #632: live SPDX golden for the same clean run: exactly
    // one document per invocation with the frozen envelope, purl
    // package identity, DESCRIBES from the audited root, and no
    // CONTAINS edges in V1 (no lock-graph projection yet).
    let spdx = std::fs::read_to_string(harness.workspace.join("out.spdx.json")).expect("spdx");
    let spdx_value: serde_json::Value = serde_json::from_str(&spdx).expect("spdx JSON");
    assert_eq!(spdx_value["spdxVersion"], serde_json::json!("SPDX-2.3"));
    assert_eq!(spdx_value["dataLicense"], serde_json::json!("CC0-1.0"));
    assert_eq!(spdx_value["SPDXID"], serde_json::json!("SPDXRef-DOCUMENT"));
    assert_eq!(spdx_value["name"], serde_json::json!("dx-audit-license"));
    let namespace = spdx_value["documentNamespace"].as_str().expect("namespace");
    assert!(
        namespace.starts_with("https://dx-audit.local/"),
        "invocation-unique namespace: {namespace}"
    );
    let packages = spdx_value["packages"].as_array().expect("packages");
    assert_eq!(packages.len(), 1, "{spdx_value}");
    assert_eq!(
        packages[0]["SPDXID"],
        serde_json::json!("SPDXRef-Package-1")
    );
    assert_eq!(packages[0]["name"], serde_json::json!("serde"));
    assert_eq!(packages[0]["versionInfo"], serde_json::json!("1.0.100"));
    assert_eq!(packages[0]["licenseConcluded"], serde_json::json!("MIT"));
    assert_eq!(packages[0]["licenseDeclared"], serde_json::json!("MIT"));
    assert_eq!(
        packages[0]["copyrightText"],
        serde_json::json!("NOASSERTION")
    );
    assert_eq!(
        packages[0]["externalRefs"],
        serde_json::json!([{
            "referenceCategory": "PACKAGE-MANAGER",
            "referenceType": "purl",
            "referenceLocator": "pkg:cargo/serde@1.0.100",
        }]),
        "{spdx_value}"
    );
    let rels = spdx_value["relationships"]
        .as_array()
        .expect("relationships");
    assert_eq!(
        rels,
        &vec![serde_json::json!({
            "spdxElementId": "//rust/tests/fixtures/hello:hello",
            "relationshipType": "DESCRIBES",
            "relatedSpdxElement": "SPDXRef-DOCUMENT",
        })],
        "one DESCRIBES per audited root, no V1 CONTAINS: {spdx_value}"
    );
}

#[test]
fn audit_sarif_run_shape_pins_family_tools_and_ordering() {
    use crate::args::parse;
    use crate::exec::{execute, Env};
    // License-only clean run carries exactly the `license` run.
    let runner = AuditRunner::clean();
    let harness = Harness::new("audit-sarif-license-shape");
    harness.write_source(
        "rust/tests/fixtures/hello/Cargo.lock",
        "[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
    );
    harness.write_source(
        "cargo-bazel-lock.json",
        r#"{"packages": {"serde 1.0.100": {"license": "MIT"}}}"#,
    );
    harness.write_source(
        "licenses.toml",
        "[policy.distributed]\nallow = [\"MIT\"]\nreview = []\ndeny = []\n\n[[inventory]]\npackage = \"serde\"\nset = \"cargo\"\nlicense = \"MIT\"\nversions = \"1.0.100\"\ntext_present = true\n",
    );
    let invocation = parse(&[
        "audit".to_owned(),
        "license".to_owned(),
        "//rust/tests/fixtures/hello:hello".to_owned(),
        "--report=sarif=out.sarif".to_owned(),
    ])
    .expect("parse");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = execute(
        &invocation,
        Env {
            workspace: &harness.workspace,
            runner: &runner,
            query_runner: &harness.query,
            temp_dir: &harness.temp,
            pid: std::process::id(),
            nonce: 0,
            out: &mut out,
            err: &mut err,
            ci: false,
        },
    );
    assert_eq!(
        code,
        0,
        "{}{}",
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap()
    );
    let sarif = std::fs::read_to_string(harness.workspace.join("out.sarif")).expect("sarif");
    let value: serde_json::Value = serde_json::from_str(&sarif).expect("sarif JSON");
    let names: Vec<&str> = value["runs"]
        .as_array()
        .expect("runs")
        .iter()
        .map(|run| run["tool"]["driver"]["name"].as_str().expect("driver"))
        .collect();
    assert_eq!(names, vec!["license"]);
    // Security-only clean run carries exactly `gitleaks` plus `vuln`
    // in deterministic bytewise order with empty runs kept.
    let runner = AuditRunner::clean();
    let harness = Harness::new("audit-sarif-security-shape");
    harness.write_source(
        "rust/tests/fixtures/hello/Cargo.lock",
        "[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
    );
    harness.write_source("pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
    harness.write_source("third_party/jvm/maven_install.json", r#"{"artifacts": {}}"#);
    harness.write_source(
        "third_party/dotnet/paket.lock",
        "NUGET\n  remote: https://api.nuget.org/v3/index.json\n",
    );
    write_go_mod(&harness);
    write_all_empty_advisories(&harness);
    let invocation = parse(&[
        "audit".to_owned(),
        "security".to_owned(),
        "--report=sarif=out.sarif".to_owned(),
    ])
    .expect("parse");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = execute(
        &invocation,
        Env {
            workspace: &harness.workspace,
            runner: &runner,
            query_runner: &harness.query,
            temp_dir: &harness.temp,
            pid: std::process::id(),
            nonce: 1,
            out: &mut out,
            err: &mut err,
            ci: false,
        },
    );
    assert_eq!(
        code,
        0,
        "{}{}",
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap()
    );
    let sarif = std::fs::read_to_string(harness.workspace.join("out.sarif")).expect("sarif");
    let value: serde_json::Value = serde_json::from_str(&sarif).expect("sarif JSON");
    assert_eq!(value["version"], serde_json::json!("2.1.0"));
    let runs = value["runs"].as_array().expect("runs");
    assert_eq!(runs.len(), 2, "{value}");
    assert_eq!(
        runs[0]["tool"]["driver"]["name"],
        serde_json::json!("gitleaks")
    );
    assert_eq!(runs[1]["tool"]["driver"]["name"], serde_json::json!("vuln"));
    for run in runs {
        assert_eq!(run["results"], serde_json::json!([]), "{value}");
        assert!(run.get("invocations").is_none(), "complete: {value}");
    }
    // Findings keep their stable tool/rule identity with
    // path-only locations (no byte ranges, hence no regions).
    let sarif_text = r#"{"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "gitleaks"}}, "results": [{"ruleId": "gitleaks/aws-key", "message": {"text": "AWS key"}}]}]}"#;
    let runner = AuditRunner::with_sarif(Some(1), sarif_text);
    let (code, _out, _err) = run_with(&["audit", "security"], &runner, &|harness| {
        harness.write_source(
            "rust/tests/fixtures/hello/Cargo.lock",
            "[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
        );
        harness.write_source("pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
        harness.write_source("third_party/jvm/maven_install.json", r#"{"artifacts": {}}"#);
        harness.write_source(
            "third_party/dotnet/paket.lock",
            "NUGET\n  remote: https://api.nuget.org/v3/index.json\n",
        );
        write_go_mod(harness);
        write_all_empty_advisories(harness);
    });
    assert_eq!(code, 1);
}

#[test]
fn audit_sarif_partial_marks_unsuccessful_while_retaining_findings() {
    use crate::args::parse;
    use crate::exec::{execute, Env};
    // Issue #632: partial collection marks every run unsuccessful
    // while retaining validated findings. Secrets finding (gitleaks)
    // plus a missing advisory snapshot (vuln incomplete) yields one
    // retained result and `executionSuccessful=false` in both runs.
    let github = format!("{}{}", "ghp_", "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8");
    let sarif_in = format!(
        "{{\"version\": \"2.1.0\", \"runs\": [{{\"tool\": {{\"driver\": {{\"name\": \"gitleaks\"}}}}, \"results\": [{{\"ruleId\": \"gitleaks/aws-key\", \"message\": {{\"text\": \"leaked {github}\"}}, \"locations\": [{{\"physicalLocation\": {{\"artifactLocation\": {{\"uri\": \"src/app.py\"}}}}}}]}}]}}]}}"
    );
    let runner = AuditRunner::with_sarif(Some(1), &sarif_in);
    let harness = Harness::new("audit-sarif-partial");
    harness.write_source(
        "rust/tests/fixtures/hello/Cargo.lock",
        "[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
    );
    harness.write_source("pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
    harness.write_source("third_party/jvm/maven_install.json", r#"{"artifacts": {}}"#);
    harness.write_source(
        "third_party/dotnet/paket.lock",
        "NUGET\n  remote: https://api.nuget.org/v3/index.json\n",
    );
    write_go_mod(&harness);
    // No advisory snapshots: every vuln set is incomplete, so the
    // SARIF document is partial even though the secrets finding is
    // validated.
    let invocation = parse(&[
        "audit".to_owned(),
        "security".to_owned(),
        "--report=sarif=out.sarif".to_owned(),
    ])
    .expect("parse");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = execute(
        &invocation,
        Env {
            workspace: &harness.workspace,
            runner: &runner,
            query_runner: &harness.query,
            temp_dir: &harness.temp,
            pid: std::process::id(),
            nonce: 2,
            out: &mut out,
            err: &mut err,
            ci: false,
        },
    );
    assert_eq!(
        code,
        1,
        "{}{}",
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap()
    );
    let sarif = std::fs::read_to_string(harness.workspace.join("out.sarif")).expect("sarif");
    let value: serde_json::Value = serde_json::from_str(&sarif).expect("sarif JSON");
    assert_eq!(value["version"], serde_json::json!("2.1.0"));
    let runs = value["runs"].as_array().expect("runs");
    assert_eq!(runs.len(), 2, "{value}");
    assert_eq!(
        runs[0]["tool"]["driver"]["name"],
        serde_json::json!("gitleaks")
    );
    assert_eq!(runs[1]["tool"]["driver"]["name"], serde_json::json!("vuln"));
    // Validated secrets finding is retained in the partial document.
    let gitleaks_results = runs[0]["results"].as_array().expect("results");
    assert_eq!(gitleaks_results.len(), 1, "{value}");
    assert_eq!(
        gitleaks_results[0]["ruleId"],
        serde_json::json!("gitleaks/aws-key")
    );
    // Secret values never reach the partial report either.
    assert!(!sarif.contains(&github), "{sarif}");
    // Every run records the unsuccessful invocation.
    for run in runs {
        assert_eq!(
            run["invocations"],
            serde_json::json!([{"executionSuccessful": false}]),
            "partial run must mark unsuccessful: {value}"
        );
    }
    // Rules stay stable per run; locations stay path-only.
    assert_eq!(
        runs[0]["tool"]["driver"]["rules"],
        serde_json::json!([{"id": "gitleaks/aws-key"}])
    );
    assert_eq!(
        gitleaks_results[0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        serde_json::json!("src/app.py")
    );
}

#[test]
fn audit_spdx_live_golden_is_single_deterministic_document() {
    use crate::args::parse;
    use crate::exec::{execute, Env};
    // Issue #632: live SPDX emission is one deterministic document
    // per invocation, never one per package/set/root. Two runs over
    // the same inputs with different temp nonces agree on packages
    // plus relationships; only the invocation namespace differs.
    // Partial reports stay non-authoritative: an incomplete license
    // run still emits a valid SPDX shape but the report event plus
    // `command_finished` carry `results_complete=false`.
    fn emit(nonce: u64) -> (serde_json::Value, i32, String) {
        let runner = AuditRunner::clean();
        let harness = Harness::new(&format!("audit-spdx-determinism-{nonce}"));
        harness.write_source(
            "rust/tests/fixtures/hello/Cargo.lock",
            "[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
        );
        harness.write_source(
            "cargo-bazel-lock.json",
            r#"{"packages": {"serde 1.0.100": {"license": "MIT"}}}"#,
        );
        harness.write_source(
            "licenses.toml",
            "[policy.distributed]\nallow = [\"MIT\"]\nreview = []\ndeny = []\n\n[[inventory]]\npackage = \"serde\"\nset = \"cargo\"\nlicense = \"MIT\"\nversions = \"1.0.100\"\ntext_present = true\n",
        );
        let invocation = parse(&[
            "audit".to_owned(),
            "license".to_owned(),
            "//rust/tests/fixtures/hello:hello".to_owned(),
            "--report=spdx=out.spdx.json".to_owned(),
        ])
        .expect("parse");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute(
            &invocation,
            Env {
                workspace: &harness.workspace,
                runner: &runner,
                query_runner: &harness.query,
                temp_dir: &harness.temp,
                pid: 999,
                nonce,
                out: &mut out,
                err: &mut err,
                ci: false,
            },
        );
        let text = std::fs::read_to_string(harness.workspace.join("out.spdx.json")).expect("spdx");
        let mut value: serde_json::Value = serde_json::from_str(&text).expect("spdx JSON");
        // Normalize the invocation-unique namespace before comparing.
        value["documentNamespace"] = serde_json::json!("NORMALIZED");
        (value, code, text)
    }
    let (first, first_code, first_text) = emit(10);
    let (second, second_code, second_text) = emit(11);
    assert_eq!(first_code, 0);
    assert_eq!(second_code, 0);
    assert_eq!(first, second, "packages plus relationships deterministic");
    // The raw texts differ only in the namespace line.
    assert_ne!(first_text, second_text);
    let first_ns = serde_json::from_str::<serde_json::Value>(&first_text).expect("json")
        ["documentNamespace"]
        .as_str()
        .expect("ns")
        .to_owned();
    let second_ns = serde_json::from_str::<serde_json::Value>(&second_text).expect("json")
        ["documentNamespace"]
        .as_str()
        .expect("ns")
        .to_owned();
    assert!(
        first_ns.starts_with("https://dx-audit.local/"),
        "{first_ns}"
    );
    assert!(
        second_ns.starts_with("https://dx-audit.local/"),
        "{second_ns}"
    );
    assert_ne!(first_ns, second_ns);
}

#[test]
fn audit_partial_reports_are_not_authoritative() {
    // Issue #632 alternative rejected: a partial SARIF/SPDX report
    // (marked `executionSuccessful=false` / `results_complete=false`)
    // must not be uploaded as an authoritative replacement scan.
    // Live JSON report events plus `command_finished` gate
    // authoritative upload on `results_complete=true`.
    let runner = AuditRunner::clean();
    let (code, out, err) = run_with(
        &[
            "audit",
            "security",
            "--output=json",
            "--report=sarif=out.sarif",
        ],
        &runner,
        &|harness| {
            harness.write_source(
                "rust/tests/fixtures/hello/Cargo.lock",
                "[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
            );
            // Missing advisory snapshots: incomplete, never clean.
        },
    );
    assert_eq!(code, 1, "{out}{err}");
    let events: Vec<serde_json::Value> = out
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .expect("NDJSON");
    let report = events
        .iter()
        .find(|event| event["event"] == serde_json::json!("report"))
        .expect("report event");
    assert_eq!(report["format"], serde_json::json!("sarif"));
    assert_eq!(report["results_complete"], serde_json::json!(false));
    let finished = events.last().expect("finished");
    assert_eq!(finished["event"], serde_json::json!("command_finished"));
    assert_eq!(finished["results_complete"], serde_json::json!(false));
    assert_eq!(finished["exit_code"], serde_json::json!(1));
}
