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
//! - Secrets (security family, once per invocation): Gitleaks as a
//!   checksummed standalone artifact (`gitleaks detect --source .` with
//!   SARIF output, `--redact`, and `--exit-code 2`), using the flag
//!   shapes pinned in [`crate::secrets`]. Findings-versus-error
//!   distinction consults the SARIF report (see [`crate::secrets`]
//!   exit classification); report-file redaction is fixture-gated.
//! - Vulnerability (security family, per dependency set): local
//!   OSV-format advisory matching inside `dx_audit` (see
//!   [`crate::vuln`] and [`crate::advisory`]), with identified snapshots
//!   as inputs, 24h cache semantics, refresh-failure behavior, and
//!   offline matching. No subprocess launches for V1 vuln: the matcher
//!   reads lockfiles plus snapshot bytes supplied by the caller, so no
//!   inventory ever leaves the workspace. Per-ecosystem lockfile
//!   coverage is Cargo (`rust/tests/fixtures/hello/Cargo.lock`), npm
//!   (`pnpm-lock.yaml`), Maven
//!   (`third_party/jvm/maven_install.json`), NuGet
//!   (`third_party/dotnet/paket.lock`), and Go (empty set, no `go.mod`:
//!   no-op success like `dx_update::backend`).
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

/// Qualified secrets tool binary: the checksummed standalone artifact
/// validated in [`crate::secrets::validate_pin`], never an ambient PATH
/// lookup beyond the pinned artifact resolution the CLI owns.
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
    /// Invoke the auditor (`argv[0]` is the binary).
    Run {
        /// Argument vector passed directly to the runner.
        argv: Vec<String>,
        /// Extra environment (parent environment is always inherited).
        env: Vec<(String, String)>,
    },
    /// No-op success (Go vuln/license with no `go.mod`; pure license
    /// evaluation needs no launch but is modeled by the caller, not here).
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
}

/// Plans one secrets-audit subprocess invocation: Gitleaks detection
/// over the workspace with SARIF output, redaction, and the exit-code
/// override. Flag order is fixed so action keys stay deterministic.
/// `report_path` is the temp SARIF destination the caller parses for
/// findings-versus-error triage; `config` pins `--config` explicitly
/// instead of relying on discovery order.
pub fn plan_secrets(report_path: &str, config: Option<&str>) -> Result<BackendPlan, BackendError> {
    if report_path.trim().is_empty() {
        return Err(BackendError::MissingReportPath);
    }
    let mut argv = vec![
        SECRETS_BINARY.to_owned(),
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
    Ok(BackendPlan::Run { argv, env: vec![] })
}

/// Workspace-relative lockfiles audited per dependency set for V1
/// vulnerability matching. Go has none (empty set, no-op success).
pub fn vuln_locks(set: &str) -> &'static [&'static str] {
    match set {
        "cargo" => &["rust/tests/fixtures/hello/Cargo.lock"],
        "npm" => &["pnpm-lock.yaml"],
        "maven" => &["third_party/jvm/maven_install.json"],
        "nuget" => &["third_party/dotnet/paket.lock"],
        "go" => &[],
        _ => &[],
    }
}

/// Whether one dependency set is empty for V1 vuln/license audit (Go:
/// no `go.mod` in the main workspace, so a full audit is a no-op
/// success with no launch and no file changes).
pub fn is_empty_set(set: &str) -> bool {
    vuln_locks(set).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_plan_pins_gitleaks_sarif_redact_and_exit_split() {
        let plan = plan_secrets("out/gitleaks.sarif", None).expect("plans");
        match plan {
            BackendPlan::Run { argv, env } => {
                assert_eq!(argv[0], "gitleaks");
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
                assert!(env.is_empty());
            }
            BackendPlan::Noop => panic!("secrets runs gitleaks"),
        }
    }

    #[test]
    fn secrets_plan_carries_explicit_config() {
        let plan = plan_secrets("out.sarif", Some(".gitleaks.toml")).expect("plans");
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
            plan_secrets("  ", None),
            Err(BackendError::MissingReportPath)
        );
    }

    #[test]
    fn vuln_locks_pin_per_set_coverage_and_go_empty() {
        assert_eq!(
            vuln_locks("cargo"),
            &["rust/tests/fixtures/hello/Cargo.lock"]
        );
        assert_eq!(vuln_locks("npm"), &["pnpm-lock.yaml"]);
        assert_eq!(vuln_locks("maven"), &["third_party/jvm/maven_install.json"]);
        assert_eq!(vuln_locks("nuget"), &["third_party/dotnet/paket.lock"]);
        assert!(vuln_locks("go").is_empty());
        assert!(is_empty_set("go"));
        assert!(!is_empty_set("cargo"));
    }

    #[test]
    fn frozen_spellings() {
        assert_eq!(SECRETS_BINARY, "gitleaks");
        assert_eq!(SECRETS_SUBCOMMAND, "detect");
        assert_eq!(SOURCE_FLAG, "--source");
        assert_eq!(WORKSPACE_SOURCE, ".");
        assert_eq!(SECRETS_ERROR_EXIT, "2");
    }
}
