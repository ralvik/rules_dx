//! Secrets-audit invocation planning (M26 WP1 slice 3).
//!
//! Pure qualification planning for the selected initial secrets
//! integration (Gitleaks) per the audit contract: a checksummed
//! standalone artifact with SARIF output and secret-value redaction.
//! This module plans over injected pin records and argument strings
//! only, so artifact identity, report wiring, and exit classification
//! stay deterministic and unit-testable without network access, an
//! auditor binary, or any Bazel integration.
//!
//! Research observations from the contract (unproven mappings, not
//! pins): upstream `v8.30.1` with per-OS/arch archives, `--report-format
//! json|csv|junit|sarif|template`, `--report-path`, `--redact` for
//! logs/stdout, TOML discovery (`--config`, `GITLEAKS_CONFIG`,
//! `GITLEAKS_CONFIG_TOML`, `.gitleaks.toml`, else built-in defaults),
//! and a conflated exit `1` for leaks or errors with an `--exit-code`
//! override. Report-file redaction, findings-versus-operational-error
//! distinction, and silent-`0` cases need fixtures before any adapter
//! claims working audit support.
//!
//! Out of scope here (O11 qualification): actual byte acquisition and
//! digest verification against upstream, SARIF parsing, report-file
//! redaction proofs, adapter/registry wiring, and the `secrets`
//! policy-family registry amendment. Those arrive in later M26 slices;
//! this crate only records which artifact identity and flag shape a
//! future adapter must satisfy.

/// Tool identifier the future adapter must resolve as a checksummed
/// standalone artifact, never an ambient PATH lookup.
pub const GITLEAKS_TOOL: &str = "gitleaks";

/// Observed upstream version from contract research. Not a pin: recheck
/// the latest stable and re-pin exact bytes at implementation.
pub const OBSERVED_VERSION: &str = "8.30.1";

/// Frozen report format for the secrets family: SARIF 2.1.0 via the
/// shared `--report` contract.
pub const SARIF_FORMAT: &str = "sarif";

/// Flag requesting secret-value redaction. Always present in a planned
/// invocation; whether redaction also covers the report file (versus
/// logs/stdout only) is fixture-gated under O11, so adapters must prove
/// it rather than assume it.
pub const REDACT_FLAG: &str = "--redact";

/// Flag selecting the report format.
pub const REPORT_FORMAT_FLAG: &str = "--report-format";

/// Flag selecting the report destination.
pub const REPORT_PATH_FLAG: &str = "--report-path";

/// Flag overriding the conflated leaks-or-errors exit code.
pub const EXIT_CODE_FLAG: &str = "--exit-code";

/// Flag pinning the Gitleaks TOML configuration explicitly.
pub const CONFIG_FLAG: &str = "--config";

/// Upstream config discovery order (highest precedence first), recorded
/// here so the future adapter cannot invent a second mechanism. The
/// explicit flag wins; built-in defaults apply only when every earlier
/// source is absent.
pub const CONFIG_DISCOVERY_ORDER: &[&str] = &[
    "--config flag",
    "GITLEAKS_CONFIG",
    "GITLEAKS_CONFIG_TOML",
    ".gitleaks.toml",
    "built-in defaults",
];

/// Checksummed standalone artifact identity a future adapter must
/// satisfy. Field shapes mirror the checked-in quality-artifact metadata
/// schema (`quality/artifacts/*.bzl`) so the future pin cannot drift
/// from the acquisition contract; no bytes are fetched here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactPin {
    /// Tool identifier (must be [`GITLEAKS_TOOL`]).
    pub tool: String,
    /// Upstream version the bytes were recorded from.
    pub upstream_version: String,
    /// Immutable download URL for the exact artifact.
    pub url: String,
    /// Lowercase hex SHA-256 of the exact artifact bytes.
    pub sha256: String,
    /// Artifact size in bytes (must be nonzero).
    pub size: u64,
}

/// Artifact identity failures. Every variant fails qualification; none
/// falls back to an ambient tool or an unpinned download.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PinProblem {
    /// Wrong tool: only Gitleaks qualifies through this module.
    #[error("gitleaks pin names wrong tool {tool:?}")]
    WrongTool { tool: String },
    /// Empty version, URL, or digest.
    #[error("gitleaks pin missing {field}")]
    MissingField { field: &'static str },
    /// URL is not an immutable `https://` reference.
    #[error("gitleaks pin has non-https URL {url:?}")]
    BadUrl { url: String },
    /// Digest is not 64 lowercase hex characters.
    #[error("gitleaks pin has invalid sha256 {value:?}; want 64 lowercase hex")]
    BadDigest { value: String },
    /// Size is zero: no empty artifact is a valid pin.
    #[error("gitleaks pin has invalid size {size}; want nonzero")]
    BadSize { size: u64 },
}

/// Validate a checksummed standalone pin without fetching anything: the
/// tool must be Gitleaks, version/URL/digest must be present, the URL
/// must be `https://`, the digest must be 64 lowercase hex characters,
/// and the size must be nonzero. Byte identity against upstream is
/// proven at implementation by the artifact regeneration/verification
/// command, not here.
pub fn validate_pin(pin: &ArtifactPin) -> Result<(), PinProblem> {
    if pin.tool != GITLEAKS_TOOL {
        return Err(PinProblem::WrongTool {
            tool: pin.tool.clone(),
        });
    }
    for (field, value) in [
        ("upstream_version", pin.upstream_version.as_str()),
        ("url", pin.url.as_str()),
        ("sha256", pin.sha256.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(PinProblem::MissingField { field });
        }
    }
    // Fail-closed URL shape: the literal `https://` prefix stays the gate
    // (so an uppercase scheme or bare `https:foo` never newly qualifies)
    // and `Url::parse` additionally rejects malformed absolute URLs that
    // the prefix alone would accept.
    if !(pin.url.starts_with("https://") && url::Url::parse(&pin.url).is_ok()) {
        return Err(PinProblem::BadUrl {
            url: pin.url.clone(),
        });
    }
    // Decode round-trip pins the 64-lowercase-hex digest form via the
    // single digest owner (`dx_digest::is_hex`): `hex` accepts any
    // even-length hex, so the re-encode comparison (not the decode alone)
    // is what rejects uppercase and wrong lengths.
    let valid_digest = dx_digest::is_hex(&pin.sha256);
    if !valid_digest {
        return Err(PinProblem::BadDigest {
            value: pin.sha256.clone(),
        });
    }
    if pin.size == 0 {
        return Err(PinProblem::BadSize { size: pin.size });
    }
    Ok(())
}

/// Planned secrets-audit report wiring: SARIF destination plus the
/// redaction and exit-code overrides the future adapter must pass.
/// Only the contract-observed flags are modeled; no subcommand or
/// extra flag is invented here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecretsReport {
    /// SARIF report destination (the `--report-path` value).
    pub report_path: String,
    /// Explicit `--config` value, if the invocation pins configuration
    /// instead of relying on discovery order.
    pub config: Option<String>,
    /// `--exit-code` override disambiguating the conflated leaks/errors
    /// exit, if the qualified adapter selects one.
    pub exit_code: Option<u8>,
}

/// Report wiring failures: SARIF is mandatory and destinations must be
/// explicit file paths, never empty.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReportProblem {
    /// Empty report destination.
    #[error("secrets report needs an explicit --report-path destination")]
    MissingPath,
}

impl SecretsReport {
    /// Deterministic report flag fragment for the future adapter
    /// invocation: SARIF format, explicit path, always redacted, plus
    /// any pinned config and exit-code override. Flag order is fixed so
    /// action keys stay deterministic.
    pub fn argv(&self) -> Result<Vec<String>, ReportProblem> {
        if self.report_path.trim().is_empty() {
            return Err(ReportProblem::MissingPath);
        }
        let mut argv = vec![
            REPORT_FORMAT_FLAG.to_owned(),
            SARIF_FORMAT.to_owned(),
            REPORT_PATH_FLAG.to_owned(),
            self.report_path.clone(),
            REDACT_FLAG.to_owned(),
        ];
        if let Some(config) = &self.config {
            argv.push(CONFIG_FLAG.to_owned());
            argv.push(config.clone());
        }
        if let Some(code) = self.exit_code {
            argv.push(EXIT_CODE_FLAG.to_owned());
            argv.push(code.to_string());
        }
        Ok(argv)
    }
}

/// Planned classification of a Gitleaks process exit. Exit `1` is
/// conflated upstream (leaks or errors), so it never classifies itself:
/// the future adapter must consult the SARIF report and operational
/// evidence (fixtures pending) before reporting leaks or failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecretsOutcome {
    /// Exit `0`: no findings reported. Silent-`0` semantics still need
    /// fixtures before an adapter claims complete coverage.
    Clean,
    /// Exit `1`: leaks or errors; consult the SARIF report before
    /// claiming either. Never auto-pass, never auto-claim leaks.
    NeedsFindingErrorTriage,
    /// Any other exit: operational failure, not a finding.
    Failed,
}

/// Classify one process exit code without executing anything. The
/// SARIF report is the disambiguating evidence for exit `1`; its
/// parsing and redaction proofs arrive with the adapter fixtures.
pub fn classify_exit(code: i32) -> SecretsOutcome {
    match code {
        0 => SecretsOutcome::Clean,
        1 => SecretsOutcome::NeedsFindingErrorTriage,
        _ => SecretsOutcome::Failed,
    }
}

/// One triaged secrets finding from a Gitleaks SARIF report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecretFinding {
    /// SARIF rule ID (`gitleaks/<rule>` or the raw rule ID).
    pub rule: String,
    /// Human message (rule ID plus location, never a secret value: the
    /// invocation always passes `--redact` and fixtures prove the report
    /// carries no plaintext secret).
    pub message: String,
    /// Workspace-relative artifact URI when the report names one.
    pub path: Option<String>,
}

/// Triage one Gitleaks SARIF report (JSON text) into secret findings.
///
/// Counts typed `Sarif::runs[].results[]` via [`serde_sarif::sarif`]
/// instead of hand-walked `Value`; each result becomes one finding
/// with tool `gitleaks` downstream. Malformed JSON or a missing
/// `runs` array fails closed with the document error (callers map this to
/// incomplete, never clean). An empty results list is clean, not a
/// coverage failure; silent-`0` semantics (exit `0` with no results)
/// still mean clean here because the SARIF report is the disambiguating
/// evidence the exit classification requires.
///
/// Redaction is proven by construction plus fixtures: the planned argv
/// always carries `--redact` (see `backend::plan_secrets`), and the
/// redaction fixture below asserts a representative SARIF report carries
/// no `secret:` plaintext field. Live execution never logs secret values;
/// summaries render only rule IDs and counts.
pub fn triage_sarif(text: &str) -> Result<Vec<SecretFinding>, String> {
    let document: serde_sarif::sarif::Sarif =
        serde_json::from_str(text).map_err(|error| format!("invalid gitleaks SARIF: {error}"))?;
    let mut findings = Vec::new();
    for run in &document.runs {
        let results = run.results.as_ref().cloned().unwrap_or_default();
        for result in &results {
            let rule = result
                .rule_id
                .as_deref()
                .unwrap_or("gitleaks/secret")
                .to_owned();
            let message = result
                .message
                .text
                .as_deref()
                .unwrap_or("secret detected")
                .to_owned();
            // Never surface secret values: messages are rule/location
            // text only; any `secret:` field in the report is ignored.
            let path = result
                .locations
                .as_ref()
                .and_then(|locations| locations.first())
                .and_then(|location| location.physical_location.as_ref())
                .and_then(|physical| physical.artifact_location.as_ref())
                .and_then(|artifact| artifact.uri.clone());
            findings.push(SecretFinding {
                rule,
                message,
                path,
            });
        }
    }
    findings.sort_by(|a, b| (&a.rule, &a.message, &a.path).cmp(&(&b.rule, &b.message, &b.path)));
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin() -> ArtifactPin {
        ArtifactPin {
            tool: "gitleaks".to_owned(),
            upstream_version: "8.30.1".to_owned(),
            url: "https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_linux_x64.tar.gz".to_owned(),
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                .to_owned(),
            size: 12345678,
        }
    }

    #[test]
    fn valid_pin_passes() {
        validate_pin(&pin()).expect("valid pin qualifies");
    }

    #[test]
    fn wrong_tool_fails() {
        let mut bad = pin();
        bad.tool = "trufflehog".to_owned();
        assert_eq!(
            validate_pin(&bad),
            Err(PinProblem::WrongTool {
                tool: "trufflehog".to_owned()
            })
        );
    }

    #[test]
    fn empty_fields_fail() {
        let mut bad = pin();
        bad.upstream_version = "  ".to_owned();
        assert_eq!(
            validate_pin(&bad),
            Err(PinProblem::MissingField {
                field: "upstream_version"
            })
        );
        let mut bad = pin();
        bad.url = String::new();
        assert!(validate_pin(&bad).is_err());
        let mut bad = pin();
        bad.sha256 = String::new();
        assert!(validate_pin(&bad).is_err());
    }

    #[test]
    fn non_https_url_fails() {
        let mut bad = pin();
        bad.url = "http://example.com/gitleaks.tar.gz".to_owned();
        assert_eq!(
            validate_pin(&bad),
            Err(PinProblem::BadUrl {
                url: "http://example.com/gitleaks.tar.gz".to_owned()
            })
        );
    }

    #[test]
    fn malformed_digests_fail() {
        for digest in [
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b85",
            "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
            "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
            "not-a-digest",
        ] {
            let mut bad = pin();
            bad.sha256 = digest.to_owned();
            assert!(
                matches!(validate_pin(&bad), Err(PinProblem::BadDigest { .. })),
                "{digest} must fail"
            );
        }
    }

    #[test]
    fn zero_size_fails() {
        let mut bad = pin();
        bad.size = 0;
        assert_eq!(validate_pin(&bad), Err(PinProblem::BadSize { size: 0 }));
    }

    #[test]
    fn report_argv_pins_sarif_redact_and_path() {
        let report = SecretsReport {
            report_path: "bazel-out/gitleaks.sarif".to_owned(),
            config: None,
            exit_code: None,
        };
        let argv = report.argv().expect("plans");
        assert_eq!(
            argv,
            vec![
                "--report-format".to_owned(),
                "sarif".to_owned(),
                "--report-path".to_owned(),
                "bazel-out/gitleaks.sarif".to_owned(),
                "--redact".to_owned(),
            ]
        );
    }

    #[test]
    fn report_argv_carries_config_and_exit_code_overrides() {
        let report = SecretsReport {
            report_path: "out.sarif".to_owned(),
            config: Some(".gitleaks.toml".to_owned()),
            exit_code: Some(2),
        };
        let argv = report.argv().expect("plans");
        assert!(argv.contains(&"--config".to_owned()));
        assert!(argv.contains(&".gitleaks.toml".to_owned()));
        assert!(argv.contains(&"--exit-code".to_owned()));
        assert!(argv.contains(&"2".to_owned()));
        // SARIF + redact stay mandatory alongside overrides.
        assert!(argv.contains(&"sarif".to_owned()));
        assert!(argv.contains(&"--redact".to_owned()));
    }

    #[test]
    fn missing_report_path_fails_closed() {
        let report = SecretsReport {
            report_path: "  ".to_owned(),
            config: None,
            exit_code: None,
        };
        assert_eq!(report.argv(), Err(ReportProblem::MissingPath));
    }

    #[test]
    fn exit_classification_is_fail_closed_on_one() {
        assert_eq!(classify_exit(0), SecretsOutcome::Clean);
        // Conflated leaks-or-errors: triage via SARIF, never auto-pass.
        assert_eq!(classify_exit(1), SecretsOutcome::NeedsFindingErrorTriage);
        assert_eq!(classify_exit(2), SecretsOutcome::Failed);
        assert_eq!(classify_exit(-1), SecretsOutcome::Failed);
    }

    #[test]
    fn sarif_triage_counts_results_and_never_surfaces_secrets() {
        let clean = r#"{"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "gitleaks"}}, "results": []}]}"#;
        assert!(triage_sarif(clean).expect("clean").is_empty());
        let leaks = r#"{"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "gitleaks"}}, "results": [{"ruleId": "gitleaks/generic-api-key", "message": {"text": "Generic API Key"}, "locations": [{"physicalLocation": {"artifactLocation": {"uri": "src/app.py"}}}]}, {"ruleId": "gitleaks/aws-key", "message": {"text": "AWS key"}, "fingerprint": "secret:AKIAIOSFODNN7EXAMPLE"}]}]}"#;
        let findings = triage_sarif(leaks).expect("leaks");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].rule, "gitleaks/aws-key");
        assert_eq!(findings[1].path, Some("src/app.py".to_owned()));
        // Secret values never surface in triaged messages.
        for finding in &findings {
            assert!(!finding.message.contains("AKIAIOSFODNN7EXAMPLE"));
        }
        assert!(triage_sarif("not json").is_err());
        assert!(triage_sarif(r#"{"version": "2.1.0"}"#).is_err());
    }

    #[test]
    fn sarif_triage_handles_empty_and_multi_run_fixtures() {
        let empty_runs = r#"{"version": "2.1.0", "$schema": "https://json.schemastore.org/sarif-2.1.0.json", "runs": []}"#;
        let typed: serde_sarif::sarif::Sarif =
            serde_json::from_str(empty_runs).expect("schema-valid empty runs");
        assert!(typed.runs.is_empty());
        assert!(triage_sarif(empty_runs).expect("empty runs").is_empty());

        let multi = r#"{"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "gitleaks"}}, "results": [{"ruleId": "gitleaks/a", "message": {"text": "A"}}]}, {"tool": {"driver": {"name": "gitleaks"}}, "results": [{"ruleId": "gitleaks/b", "message": {"text": "B"}, "locations": [{"physicalLocation": {"artifactLocation": {"uri": "src/b.py"}}}]}]}]}"#;
        let typed_multi: serde_sarif::sarif::Sarif =
            serde_json::from_str(multi).expect("schema-valid multi-run");
        assert_eq!(typed_multi.runs.len(), 2);
        let findings = triage_sarif(multi).expect("multi-run");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].rule, "gitleaks/a");
        assert_eq!(findings[1].path, Some("src/b.py".to_owned()));
    }

    #[test]
    fn frozen_spellings_and_discovery_order() {
        assert_eq!(GITLEAKS_TOOL, "gitleaks");
        assert_eq!(SARIF_FORMAT, "sarif");
        assert_eq!(REDACT_FLAG, "--redact");
        assert_eq!(
            CONFIG_DISCOVERY_ORDER,
            &[
                "--config flag",
                "GITLEAKS_CONFIG",
                "GITLEAKS_CONFIG_TOML",
                ".gitleaks.toml",
                "built-in defaults",
            ]
        );
    }
}
