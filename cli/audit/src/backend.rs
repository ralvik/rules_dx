//! Auditor backend planning for `dx audit`.
//!
//! Pure argv planning over audit families and dependency sets, mirroring
//! the resolver-owned backend pattern in `dx_update::backend`: every
//! changed file and invoked operation is attributable to the underlying
//! auditor, never to a private solver. Backends run through the process
//! runner with the workspace as cwd; summaries never render argv, option
//! values, or environment values.
//!
//! V1 backends (all offline, no lockfile/inventory upload):
//! - Secrets (security family, once per invocation): Gitleaks-only
//!   (Trufflehog wont-fix, issue #629; See: `docs/cli/commands/audit-update-bazel.md#dx-audit`) as a checksummed standalone
//!   artifact (`gitleaks detect --source .` with SARIF output,
//!   `--redact`, and `--exit-code 2`), using the flag shapes pinned
//!   in [`crate::secrets`]. Findings-versus-error distinction consults
//!   the SARIF report (see [`crate::secrets`] exit classification);
//!   report redaction is proven by triage surfacing only rule IDs
//!   plus paths (See: `docs/cli/commands/audit-update-bazel.md#dx-audit`, issue #629), never secret values.
//! - Vulnerability (security family, per dependency set): local
//!   OSV-format advisory matching inside `dx_audit` (see
//!   [`crate::vuln`] and [`crate::advisory`]), with identified snapshots
//!   as inputs, 24h cache semantics, refresh-failure behavior, and
//!   offline matching. No subprocess launches for V1 vuln matching: the
//!   matcher reads lockfiles plus snapshot bytes plus snapshot identity
//!   supplied by the caller, so no inventory ever leaves the workspace.
//!   Snapshots refresh automatically on every invocation through
//!   supported upstream database-download tooling: the per-set OSV GCS
//!   sources in [`crate::advisory::advisory_source`] fetched by HTTPS GET
//!   with no query parameters, request body, or telemetry carrying package
//!   names or versions; the derived V1 snapshot bytes
//!   (`.dx/advisory/<set>.json`) plus identity
//!   (`.dx/advisory/<set>.meta.json` with `url`, `sha256`, `retrieved_at`)
//!   are the audited inputs. A missing, invalid, or stale snapshot fails
//!   with `advisory_refresh_failed`, never clean and never a stale
//!   fallback. Package-specific advisory queries and lockfile uploads
//!   never satisfy this contract. Per-ecosystem lockfile
//!   coverage is Cargo (`rust/tests/fixtures/hello/Cargo.lock`), npm
//!   (`pnpm-lock.yaml`, `package-lock.json`, `yarn.lock`), Maven
//!   (`third_party/jvm/maven_install.json`), NuGet
//!   (`third_party/dotnet/paket.lock`), and Go
//!   (`third_party/go/go.mod` via `go_deps.from_file`, parsed by
//!   [`crate::locks::parse_go_mod`]).
//! - License (license family, per dependency set): pure policy
//!   evaluation inside `dx_audit` (see [`crate::license_policy`],
//!   [`crate::license_expr`], [`crate::license_notice`]), with
//!   `licenses.toml` policy-table loading, SPDX-expression evaluation,
//!   notice-text inputs, and SPDX 2.3 JSON reporting (see
//!   [`crate::spdx`]). No subprocess launches.
//!
//! Only secrets launches a subprocess in V1; vuln and license evaluate
//! purely. Backend operation boundaries are pinned here and unit-tested;
//! per-family reporting rides text plus JSON `notice`/`error` events
//! with `command_finished`, and aggregate exit-code selection rides
//! [`crate::outcome`].

use crate::secrets::{
    CONFIG_FLAG, EXIT_CODE_FLAG, REDACT_FLAG, REPORT_FORMAT_FLAG, REPORT_PATH_FLAG, SARIF_FORMAT,
};

/// Qualified secrets tool identity: the checksummed standalone artifact
/// validated in [`crate::secrets::validate_pin`] and pinned per host in
/// [`crate::secrets::HOST_ARTIFACTS`], never an ambient `PATH` lookup.
/// Planned `argv[0]` is always the absolute declared artifact path the
/// CLI resolves (See: `docs/cli/commands/audit-update-bazel.md#dx-audit`).
pub const SECRETS_BINARY: &str = "gitleaks";

/// Secrets subcommand: repository detection over the workspace source.
pub const SECRETS_SUBCOMMAND: &str = "detect";

/// Source flag: scan the runner cwd (the resolved workspace).
pub const SOURCE_FLAG: &str = "--source";

/// Workspace source spelling for the secrets invocation.
pub const WORKSPACE_SOURCE: &str = ".";

/// Exit-code override disambiguating Gitleaks' conflated leaks-or-errors
/// exit: findings exit with the tool default, operational errors exit
/// `2` so [`crate::secrets::classify_exit`] plus SARIF triage can split
/// findings from failure without guessing.
pub const SECRETS_ERROR_EXIT: &str = "2";

/// Planned backend operation for one audit unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendPlan {
    /// Invoke the auditor (`argv[0]` is the absolute hermetic binary).
    Run {
        /// Argument vector passed directly to the runner.
        argv: Vec<String>,
        /// Sanitized environment (spawned cleared; parent never inherited).
        env: Vec<(String, String)>,
    },
    /// No-op success for subprocess-planned units with nothing to do.
    /// Kept for subprocess-planned units only; pure evaluation never
    /// produces a plan.
    Noop,
}

/// Backend planning failure (execution-time per-unit failure, exit 1 for
/// the invocation overall via [`crate::outcome`], never a silent success).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BackendError {
    /// Empty report destination for a subprocess-planned unit.
    #[error("audit backend needs an explicit report destination")]
    MissingReportPath,
    /// Empty auditor tool path: callers resolve the pinned artifact.
    #[error("audit backend needs an explicit hermetic gitleaks binary path")]
    MissingTool,
    /// Bare or relative tool name: `argv[0]` must be absolute so no
    /// ambient `PATH` lookup can substitute an unpinned binary.
    #[error("audit backend needs an absolute gitleaks binary path, got {tool:?}")]
    NonAbsoluteTool { tool: String },
    /// Empty temp directory for the sanitized `TMPDIR` entry.
    #[error("audit backend needs an explicit temp directory for the hermetic environment")]
    MissingTempDir,
}

/// Plans one secrets-audit subprocess invocation: Gitleaks detection
/// over the workspace with SARIF output, redaction, and the exit-code
/// override. Flag order is fixed so action keys stay deterministic.
/// `tool` is the absolute declared artifact path (See: [`crate::secrets::HOST_ARTIFACTS`]);
/// bare names fail closed. `report_path` is the temp SARIF destination
/// the caller parses for findings-versus-error triage; `config` pins
/// `--config` explicitly instead of relying on discovery order.
/// `temp_dir` becomes the sole `TMPDIR` entry (See: [`crate::secrets::hermetic_env`]).
///
/// Trust boundary: `config` is accepted only as the workspace-committed
/// file the caller resolved (see [`crate::secrets::CONFIG_DISCOVERY_ORDER`]);
/// ambient `GITLEAKS_CONFIG` values never reach the child, and an
/// attacker-controlled config can disable rules, so callers warn when a
/// non-default config is selected.
///
/// `offline` forces cache-only: the secrets scan is already hermetic and
/// local (no fetch, no upload), so offline planning is identical and
/// never fails. Threaded for per-backend parity with
/// `dx_update::backend::plan` so air-gapped runs prove the same argv.
/// See: `docs/deploy/offline-bootstrap.md`.
pub fn plan_secrets(
    tool: &str,
    report_path: &str,
    config: Option<&str>,
    temp_dir: &str,
    offline: bool,
) -> Result<BackendPlan, BackendError> {
    let _ = offline;
    if tool.trim().is_empty() {
        return Err(BackendError::MissingTool);
    }
    if !std::path::Path::new(tool).is_absolute() {
        return Err(BackendError::NonAbsoluteTool {
            tool: tool.to_owned(),
        });
    }
    if report_path.trim().is_empty() {
        return Err(BackendError::MissingReportPath);
    }
    if temp_dir.trim().is_empty() {
        return Err(BackendError::MissingTempDir);
    }
    let mut argv = vec![
        tool.to_owned(),
        SECRETS_SUBCOMMAND.to_owned(),
        SOURCE_FLAG.to_owned(),
        WORKSPACE_SOURCE.to_owned(),
        REPORT_FORMAT_FLAG.to_owned(),
        SARIF_FORMAT.to_owned(),
        REPORT_PATH_FLAG.to_owned(),
        report_path.to_owned(),
        REDACT_FLAG.to_owned(),
        EXIT_CODE_FLAG.to_owned(),
        SECRETS_ERROR_EXIT.to_owned(),
    ];
    if let Some(config) = config {
        if !config.trim().is_empty() {
            argv.push(CONFIG_FLAG.to_owned());
            argv.push(config.to_owned());
        }
    }
    Ok(BackendPlan::Run {
        argv,
        env: vec![("TMPDIR".to_owned(), temp_dir.to_owned())],
    })
}

/// Workspace-relative lockfiles audited per dependency set for V1
/// vulnerability matching. Npm audits every present lock shape
/// (`pnpm-lock.yaml`, `package-lock.json`, `yarn.lock`); absent shapes
/// are skipped, so a pnpm-only workspace never fails for a missing
/// sibling lock. Go reads the `go_deps.from_file` module lock
/// (`third_party/go/go.mod`); `go.sum` carries hashes only and is never
/// an audit input.
pub fn vuln_locks(set: &str) -> &'static [&'static str] {
    match set {
        "cargo" => &["rust/tests/fixtures/hello/Cargo.lock"],
        "npm" => &["pnpm-lock.yaml", "package-lock.json", "yarn.lock"],
        "maven" => &["third_party/jvm/maven_install.json"],
        "nuget" => &["third_party/dotnet/paket.lock"],
        "go" => &["third_party/go/go.mod"],
        _ => &[],
    }
}

/// Whether one dependency set is empty for V1 vuln/license audit (no set
/// is empty: every registry set owns a lockfile, so a full audit always
/// assesses).
pub fn is_empty_set(set: &str) -> bool {
    vuln_locks(set).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_plan_pins_gitleaks_sarif_redact_and_exit_split() {
        let plan = plan_secrets(
            "/hermetic/gitleaks",
            "out/gitleaks.sarif",
            None,
            "/tmp/dx",
            false,
        )
        .expect("plans");
        match plan {
            BackendPlan::Run { argv, env } => {
                assert_eq!(argv[0], "/hermetic/gitleaks");
                assert!(argv.contains(&"detect".to_owned()));
                assert!(argv.contains(&"--source".to_owned()));
                assert!(argv.contains(&".".to_owned()));
                assert!(argv.contains(&"--report-format".to_owned()));
                assert!(argv.contains(&"sarif".to_owned()));
                assert!(argv.contains(&"--report-path".to_owned()));
                assert!(argv.contains(&"out/gitleaks.sarif".to_owned()));
                assert!(argv.contains(&"--redact".to_owned()));
                assert!(argv.contains(&"--exit-code".to_owned()));
                assert!(argv.contains(&"2".to_owned()));
                assert!(!argv.contains(&"--config".to_owned()));
                assert_eq!(env, vec![("TMPDIR".to_owned(), "/tmp/dx".to_owned())]);
            }
            BackendPlan::Noop => panic!("secrets runs gitleaks"),
        }
    }

    #[test]
    fn secrets_plan_carries_explicit_config() {
        let plan = plan_secrets(
            "/hermetic/gitleaks",
            "out.sarif",
            Some(".gitleaks.toml"),
            "/tmp/dx",
            false,
        )
        .expect("plans");
        match plan {
            BackendPlan::Run { argv, .. } => {
                assert!(argv.contains(&"--config".to_owned()));
                assert!(argv.contains(&".gitleaks.toml".to_owned()));
            }
            BackendPlan::Noop => panic!("secrets runs"),
        }
    }

    #[test]
    fn secrets_plan_rejects_empty_report_path() {
        assert_eq!(
            plan_secrets("/hermetic/gitleaks", "  ", None, "/tmp/dx", false),
            Err(BackendError::MissingReportPath)
        );
    }

    #[test]
    fn secrets_plan_rejects_bare_and_relative_tools() {
        // Hermetic acquisition: `argv[0]` is always the absolute declared
        // artifact path, never an ambient `PATH` lookup.
        assert_eq!(
            plan_secrets("", "out.sarif", None, "/tmp/dx", false),
            Err(BackendError::MissingTool)
        );
        assert_eq!(
            plan_secrets("gitleaks", "out.sarif", None, "/tmp/dx", false),
            Err(BackendError::NonAbsoluteTool {
                tool: "gitleaks".to_owned()
            })
        );
        assert_eq!(
            plan_secrets("tools/gitleaks", "out.sarif", None, "/tmp/dx", false),
            Err(BackendError::NonAbsoluteTool {
                tool: "tools/gitleaks".to_owned()
            })
        );
        assert_eq!(
            plan_secrets("/hermetic/gitleaks", "out.sarif", None, "  ", false),
            Err(BackendError::MissingTempDir)
        );
    }

    #[test]
    fn secrets_plan_env_is_sanitized_tmpdir_only() {
        // Sanitized invocation: exactly `TMPDIR`, never `PATH` and never
        // ambient `GITLEAKS_*`.
        let plan = plan_secrets(
            "/hermetic/gitleaks",
            "out.sarif",
            None,
            "/tmp/dx-run",
            false,
        )
        .expect("plans");
        match plan {
            BackendPlan::Run { env, .. } => {
                assert_eq!(env, vec![("TMPDIR".to_owned(), "/tmp/dx-run".to_owned())]);
                assert!(!env.iter().any(|(key, _)| key == "PATH"));
            }
            BackendPlan::Noop => panic!("secrets runs"),
        }
    }

    #[test]
    fn vuln_locks_pin_per_set_coverage() {
        assert_eq!(
            vuln_locks("cargo"),
            &["rust/tests/fixtures/hello/Cargo.lock"]
        );
        assert_eq!(
            vuln_locks("npm"),
            &["pnpm-lock.yaml", "package-lock.json", "yarn.lock"]
        );
        assert_eq!(vuln_locks("maven"), &["third_party/jvm/maven_install.json"]);
        assert_eq!(vuln_locks("nuget"), &["third_party/dotnet/paket.lock"]);
        assert_eq!(vuln_locks("go"), &["third_party/go/go.mod"]);
        assert!(!is_empty_set("go"));
        assert!(!is_empty_set("cargo"));
        assert!(is_empty_set("unknown-set"));
    }

    #[test]
    fn frozen_spellings() {
        assert_eq!(SECRETS_BINARY, "gitleaks");
        assert_eq!(SECRETS_SUBCOMMAND, "detect");
        assert_eq!(SOURCE_FLAG, "--source");
        assert_eq!(WORKSPACE_SOURCE, ".");
        assert_eq!(SECRETS_ERROR_EXIT, "2");
    }

    #[test]
    fn secrets_plan_is_cache_only_identical_offline() {
        // See: `docs/deploy/offline-bootstrap.md`. The secrets scan is
        // hermetic and local, so `--offline`/`--frozen` planning is byte
        // -identical and never fails: no fetch, no upload.
        let online = plan_secrets("/hermetic/gitleaks", "out.sarif", None, "/tmp/dx", false)
            .expect("online plans");
        let offline = plan_secrets("/hermetic/gitleaks", "out.sarif", None, "/tmp/dx", true)
            .expect("offline plans");
        assert_eq!(online, offline);
        let online_configed = plan_secrets(
            "/hermetic/gitleaks",
            "out.sarif",
            Some(".gitleaks.toml"),
            "/tmp/dx",
            false,
        )
        .expect("online configed");
        let offline_configed = plan_secrets(
            "/hermetic/gitleaks",
            "out.sarif",
            Some(".gitleaks.toml"),
            "/tmp/dx",
            true,
        )
        .expect("offline configed");
        assert_eq!(online_configed, offline_configed);
    }
}
