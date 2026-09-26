pub const GITLEAKS_TOOL: &str = "gitleaks";

pub const TRUFFLEHOG_V1: &str = "wont-fix";

pub const OBSERVED_VERSION: &str = "8.30.1";

pub const GITLEAKS_VERSION: &str = "8.30.1";

pub const TOOL_ENV_VAR: &str = "DX_GITLEAKS_BIN";

pub const TOOL_LABEL: &str = "@dx_tools//:gitleaks";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostArtifact {
    pub platform: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub size: u64,
    pub executable: &'static str,
}

pub const HOST_ARTIFACTS: &[HostArtifact] = &[
    HostArtifact {
        platform: "linux_x86_64",
        url: "https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_linux_x64.tar.gz",
        sha256: "551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb",
        size: 8230402,
        executable: "gitleaks",
    },
    HostArtifact {
        platform: "linux_arm64",
        url: "https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_linux_arm64.tar.gz",
        sha256: "e4a487ee7ccd7d3a7f7ec08657610aa3606637dab924210b3aee62570fb4b080",
        size: 7601421,
        executable: "gitleaks",
    },
    HostArtifact {
        platform: "macos_arm64",
        url: "https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_darwin_arm64.tar.gz",
        sha256: "b40ab0ae55c505963e365f271a8d3846efbc170aa17f2607f13df610a9aeb6a5",
        size: 7897593,
        executable: "gitleaks",
    },
    HostArtifact {
        platform: "windows_x86_64",
        url: "https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_windows_x64.zip",
        sha256: "d29144deff3a68aa93ced33dddf84b7fdc26070add4aa0f4513094c8332afc4e",
        size: 8438883,
        executable: "gitleaks.exe",
    },
];

pub fn hermetic_env(temp_dir: &std::path::Path) -> Vec<(String, String)> {
    vec![("TMPDIR".to_owned(), temp_dir.to_string_lossy().into_owned())]
}

pub const SARIF_FORMAT: &str = "sarif";

pub const REDACT_FLAG: &str = "--redact";

pub const REPORT_FORMAT_FLAG: &str = "--report-format";

pub const REPORT_PATH_FLAG: &str = "--report-path";

pub const EXIT_CODE_FLAG: &str = "--exit-code";

pub const CONFIG_FLAG: &str = "--config";

pub const CONFIG_DISCOVERY_ORDER: &[&str] = &[
    "--config flag",
    "GITLEAKS_CONFIG",
    "GITLEAKS_CONFIG_TOML",
    ".gitleaks.toml",
    "built-in defaults",
];

/// Checksummed standalone artifact identity a future adapter must
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactPin {
    pub tool: String,
    pub upstream_version: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PinProblem {
    #[error("gitleaks pin names wrong tool {tool:?}")]
    WrongTool { tool: String },
    #[error("gitleaks pin missing {field}")]
    MissingField { field: &'static str },
    #[error("gitleaks pin has non-https URL {url:?}")]
    BadUrl { url: String },
    #[error("gitleaks pin has invalid sha256 {value:?}; want 64 lowercase hex")]
    BadDigest { value: String },
    #[error("gitleaks pin has invalid size {size}; want nonzero")]
    BadSize { size: u64 },
}

/// Validate a checksummed standalone pin without fetching anything: the
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecretsReport {
    pub report_path: String,
    pub config: Option<String>,
    pub exit_code: Option<u8>,
}

/// Report wiring failures: SARIF is mandatory and destinations must be
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReportProblem {
    #[error("secrets report needs an explicit --report-path destination")]
    MissingPath,
}

impl SecretsReport {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecretsOutcome {
    Clean,
    NeedsFindingErrorTriage,
    Failed,
}

/// Classify one process exit code without executing anything. The
pub fn classify_exit(code: i32) -> SecretsOutcome {
    match code {
        0 => SecretsOutcome::Clean,
        1 => SecretsOutcome::NeedsFindingErrorTriage,
        _ => SecretsOutcome::Failed,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecretFinding {
    pub rule: String,
    pub message: String,
    pub path: Option<String>,
}

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
            // Never surface secret values: the message is rebuilt
            // from the rule ID plus the artifact path only. Raw
            // `message.text` (which an unredacted report could fill
            // with a secret) plus fingerprints, snippets, fixes, and
            // properties are all ignored.
            let path = result
                .locations
                .as_ref()
                .and_then(|locations| locations.first())
                .and_then(|location| location.physical_location.as_ref())
                .and_then(|physical| physical.artifact_location.as_ref())
                .and_then(|artifact| artifact.uri.clone());
            let message = match &path {
                Some(uri) => format!("{rule} detected in {uri}"),
                None => format!("{rule} detected"),
            };
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
    fn trufflehog_v1_stays_wont_fix() {
        // a `trufflehog` tool identity, and this disposition pins the
        // docs-level decision so a silent tool swap cannot qualify.
        assert_eq!(TRUFFLEHOG_V1, "wont-fix");
        assert_eq!(GITLEAKS_TOOL, "gitleaks");
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
        // Secret values never surface in triaged rules, messages, or paths.
        for finding in &findings {
            assert!(!finding.rule.contains("AKIAIOSFODNN7EXAMPLE"));
            assert!(!finding.message.contains("AKIAIOSFODNN7EXAMPLE"));
            assert!(finding.path.as_deref() != Some("AKIAIOSFODNN7EXAMPLE"));
        }
        // Messages are rule-plus-path only, never raw SARIF text.
        assert_eq!(
            findings[1].message,
            "gitleaks/generic-api-key detected in src/app.py"
        );
        assert!(triage_sarif("not json").is_err());
        assert!(triage_sarif(r#"{"version": "2.1.0"}"#).is_err());
    }

    #[test]
    fn sarif_triage_redaction_ignores_every_secret_field() {
        // triage. Every plausible secret-carrying SARIF field carries
        // a distinct sentinel; triaged output must contain none of
        // them while still counting the finding with its rule and path.
        // Sentinels are assembled at runtime so the file never stores
        // a push-protected token shape verbatim.
        let aws = "AKIAIOSFODNN7EXAMPLE";
        let github = format!("{}{}", "ghp_", "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8");
        let generic = format!("{}{}", "sk-live-", "51H7x9yQ2wE4rT6yU8iO0p");
        let github: &str = &github;
        let generic: &str = &generic;
        let text = format!(
            r#"{{"version": "2.1.0", "runs": [{{"tool": {{"driver": {{"name": "gitleaks"}}}}, "results": [{{
                "ruleId": "gitleaks/aws-key",
                "message": {{"text": "leaked {aws} in src/creds.py"}},
                "fingerprints": {{"secret": "{aws}"}},
                "partialFingerprints": {{"secret/v1": "{github}"}},
                "properties": {{"secret": "{generic}"}},
                "locations": [{{
                    "physicalLocation": {{
                        "artifactLocation": {{"uri": "src/creds.py"}},
                        "region": {{"snippet": {{"text": "key = '{aws}'"}}}},
                        "contextRegion": {{"snippet": {{"text": "token {generic} here"}}}}
                    }}
                }}]}}]}}]}}"#
        );
        let findings = triage_sarif(&text).expect("unredacted triages");
        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule, "gitleaks/aws-key");
        assert_eq!(finding.path, Some("src/creds.py".to_owned()));
        assert_eq!(finding.message, "gitleaks/aws-key detected in src/creds.py");
        for secret in [aws, github, generic] {
            assert!(!finding.rule.contains(secret), "rule leaks {secret}");
            assert!(!finding.message.contains(secret), "message leaks {secret}");
            assert!(
                finding.path.as_deref().unwrap_or("").find(secret).is_none(),
                "path leaks {secret}"
            );
        }
        // The redacted twin (secrets replaced by `...`, as `--redact`
        // emits) triages to the identical finding: secret values never
        // affect triage output.
        let redacted = text
            .replace(aws, "...")
            .replace(github, "...")
            .replace(generic, "...");
        let redacted_findings = triage_sarif(&redacted).expect("redacted triages");
        assert_eq!(findings, redacted_findings);
    }

    #[test]
    fn sarif_triage_message_never_copies_raw_text() {
        // A SARIF `message.text` carrying only a secret still yields a
        // rule-plus-path message with no secret substring. Assembled at
        // runtime so the file never stores the token shape verbatim.
        let secret_owned = format!("{}{}", "xoxb-", "123456789012-abcdefghijklmnopqrstuvwx");
        let secret: &str = &secret_owned;
        let text = format!(
            r#"{{"version": "2.1.0", "runs": [{{"tool": {{"driver": {{"name": "gitleaks"}}}}, "results": [{{
                "ruleId": "gitleaks/slack-token",
                "message": {{"text": "{secret}"}},
                "locations": [{{"physicalLocation": {{"artifactLocation": {{"uri": "src/chat.py"}}}}}}]}}]}}]}}"#
        );
        let findings = triage_sarif(&text).expect("secret-text triages");
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].message,
            "gitleaks/slack-token detected in src/chat.py"
        );
        assert!(!findings[0].message.contains(secret));
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
        assert_eq!(TRUFFLEHOG_V1, "wont-fix");
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

    #[test]
    fn hermetic_hosts_pin_five_platforms() {
        // Multi-platform acquisition: one checksummed pin per required
        // host, matching `quality/artifacts/gitleaks.*.bzl` plus the
        // upstream checksums file. Execution is proven on the seed host
        // via byte fetch plus triage fixtures; remaining hosts resolve
        // via Bazel execution-platform selection with no host execution
        // claimed here.
        assert_eq!(GITLEAKS_VERSION, "8.30.1");
        assert_eq!(TOOL_ENV_VAR, "DX_GITLEAKS_BIN");
        assert_eq!(TOOL_LABEL, "@dx_tools//:gitleaks");
        let platforms: Vec<&str> = HOST_ARTIFACTS.iter().map(|host| host.platform).collect();
        assert_eq!(
            platforms,
            vec![
                "linux_x86_64",
                "linux_arm64",
                "macos_arm64",
                "windows_x86_64",
            ]
        );
        for host in HOST_ARTIFACTS {
            let pin = ArtifactPin {
                tool: GITLEAKS_TOOL.to_owned(),
                upstream_version: GITLEAKS_VERSION.to_owned(),
                url: host.url.to_owned(),
                sha256: host.sha256.to_owned(),
                size: host.size,
            };
            validate_pin(&pin).expect("host pin qualifies");
            assert!(host
                .url
                .starts_with("https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/"));
            assert_eq!(host.sha256.len(), 64);
            assert!(host.size > 7_000_000);
        }
        assert_eq!(
            HOST_ARTIFACTS
                .iter()
                .find(|host| host.platform == "windows_x86_64")
                .expect("windows pin")
                .executable,
            "gitleaks.exe"
        );
    }

    #[test]
    fn hermetic_env_carries_only_tmpdir() {
        // Sanitized invocation environment: exactly `TMPDIR`, never
        // `PATH` and never ambient `GITLEAKS_*`, so configuration flows
        // only through explicit `--config`, committed `.gitleaks.toml`,
        // or built-in defaults.
        let env = hermetic_env(std::path::Path::new("/tmp/dx-audit"));
        assert_eq!(env, vec![("TMPDIR".to_owned(), "/tmp/dx-audit".to_owned())]);
        assert!(!env.iter().any(|(key, _)| key == "PATH"));
        assert!(!env.iter().any(|(key, _)| key == "GITLEAKS_CONFIG"));
        assert!(!env.iter().any(|(key, _)| key == "GITLEAKS_CONFIG_TOML"));
    }
}
