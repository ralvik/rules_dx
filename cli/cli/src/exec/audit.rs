//! Audit command execution: live auditor backends with per-family reporting.

use super::common::*;
use crate::args::{Command, Invocation};
use crate::reports::{plan_reports, Destination};
use dx_output::{
    command_finished, command_started, error_event, notice_event, report_event, write_event,
    DiagnosticEvent, FinishedCounts, NoticeEvent, OutputMode, Severity, Snapshot, Threshold,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// Re-exported for guards that machine-check live execution wiring.
#[allow(dead_code)]
fn _live_wiring_pins() {
    let _ = CODE_AUDIT_FAILED;
}

/// Day-granularity audit clock (keep): stays on
/// `chrono::Utc::now` because the gates compare fixed-width `YYYY-MM-DD` UTC
/// days with no `tzdb`/zone arithmetic, so the `jiff` `Timestamp::now` plus
/// `tz::TimeZone::UTC` rewrite pays bundle plus churn for no gate gain;
/// re-evaluate on `jiff 1.0`.
fn today_utc() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

fn resolve_audit_sets(scopes: &[String]) -> Result<Vec<dx_update::sets::SetId>, String> {
    let mut union: BTreeSet<dx_update::sets::SetId> = BTreeSet::new();
    for scope in scopes {
        let owners = dx_update::selector::owning_sets(scope);
        if owners.is_empty() {
            return Err(format!(
                "no owning dependency set for {scope:?} (python and non-dependency paths are out of V1 audit scope)"
            ));
        }
        for set in owners {
            union.insert(set);
        }
    }
    let mut ordered = Vec::new();
    for set in dx_update::sets::SetId::ALL {
        if union.contains(&set) {
            ordered.push(set);
        }
    }
    Ok(ordered)
}

fn read_workspace_text(workspace: &Path, rel: &str) -> Result<String, String> {
    match std::fs::read(workspace.join(rel)) {
        Err(error) => Err(format!("could not read {rel}: {error}")),
        Ok(bytes) => match String::from_utf8(bytes) {
            Err(_) => Err(format!("could not read {rel}: not valid UTF-8")),
            Ok(text) => Ok(text),
        },
    }
}

fn advisory_path(set: dx_update::sets::SetId) -> String {
    dx_audit::advisory::snapshot_rel(set.name())
}

fn advisory_identity_path(set: dx_update::sets::SetId) -> String {
    dx_audit::advisory::identity_rel(set.name())
}

fn load_advisories(
    workspace: &Path,
    set: dx_update::sets::SetId,
    today: &str,
) -> Result<Vec<dx_audit::vuln::Advisory>, String> {
    // Issue #628: never empty clean. A missing, empty, invalid, or stale
    // snapshot fails with `advisory_refresh_failed`, never a clean result
    // and never a stale fallback. Snapshots refresh automatically via
    // supported upstream database-download tooling (per-set OSV GCS zips
    // fetched by HTTPS GET with no inventory in the request); the derived
    // bytes plus identity are the audited inputs. Live CLI performs no
    // network fetch and no lockfile upload.
    let code = dx_audit::advisory::CODE_ADVISORY_REFRESH_FAILED;
    let source = dx_audit::advisory::advisory_source(set.name()).ok_or_else(|| {
        format!(
            "{code}: could not obtain current advisory data for {}: unsupported set",
            set.name()
        )
    })?;
    let rel = advisory_path(set);
    let full = workspace.join(&rel);
    if !full.is_file() {
        return Err(format!(
            "{code}: could not obtain current advisory data for {}: missing {rel} (refresh via {source})",
            set.name()
        ));
    }
    let bytes = std::fs::read(&full).map_err(|error| {
        format!(
            "{code}: could not obtain current advisory data for {}: could not read {rel}: {error}",
            set.name()
        )
    })?;
    let text = String::from_utf8(bytes.clone()).map_err(|_| {
        format!(
            "{code}: could not obtain current advisory data for {}: {rel} is not valid UTF-8",
            set.name()
        )
    })?;
    if text.trim().is_empty() {
        return Err(format!(
            "{code}: could not obtain current advisory data for {}: empty {rel}",
            set.name()
        ));
    }
    let meta_rel = advisory_identity_path(set);
    if !workspace.join(&meta_rel).is_file() {
        return Err(format!(
            "{code}: could not obtain current advisory data for {}: missing {meta_rel}",
            set.name()
        ));
    }
    let meta_text = read_workspace_text(workspace, &meta_rel).map_err(|detail| {
        format!(
            "{code}: could not obtain current advisory data for {}: {detail}",
            set.name()
        )
    })?;
    let snapshot = dx_audit::advisory::parse_identity(&meta_text).map_err(|detail| {
        format!(
            "{code}: could not obtain current advisory data for {}: {detail}",
            set.name()
        )
    })?;
    dx_audit::advisory::validate_snapshot(&snapshot).map_err(|error| {
        format!(
            "{code}: could not obtain current advisory data for {}: invalid advisory identity: {error}",
            set.name()
        )
    })?;
    if snapshot.set != set.name() {
        return Err(format!(
            "{code}: could not obtain current advisory data for {}: identity set {:?} does not match",
            set.name(),
            snapshot.set
        ));
    }
    if dx_audit::advisory::freshness(&snapshot, today) != dx_audit::advisory::Freshness::Fresh {
        return Err(format!(
            "{code}: could not obtain current advisory data for {}: stale snapshot {} (want {today})",
            set.name(),
            snapshot.retrieved_at
        ));
    }
    if !dx_audit::advisory::identity_matches_bytes(&snapshot, &bytes) {
        return Err(format!(
            "{code}: could not obtain current advisory data for {}: identity sha256 does not match {rel}",
            set.name()
        ));
    }
    dx_audit::vuln::parse_snapshot(&text).map_err(|detail| {
        format!(
            "{code}: could not obtain current advisory data for {}: {detail}",
            set.name()
        )
    })
}

fn lock_texts_for_set(
    workspace: &Path,
    set: dx_update::sets::SetId,
) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    for rel in dx_audit::backend::vuln_locks(set.name()) {
        let full = workspace.join(rel);
        if !full.is_file() {
            // Npm owns three competing lock shapes; a workspace carries
            // whichever its package manager writes. Absent shapes are
            // skipped so a pnpm-only workspace never fails for a missing
            // sibling lock; every other set keeps required-lock behavior.
            if set == dx_update::sets::SetId::Npm {
                missing.push((*rel).to_owned());
                continue;
            }
            return Err(format!("could not read {rel}: no such file"));
        }
        let text = read_workspace_text(workspace, rel)?;
        out.push(((*rel).to_owned(), text));
    }
    if out.is_empty() && set == dx_update::sets::SetId::Npm {
        return Err(format!(
            "could not read {}: no such file",
            missing
                .first()
                .cloned()
                .unwrap_or_else(|| "pnpm-lock.yaml".to_owned())
        ));
    }
    Ok(out)
}

fn parse_locked_for_set(
    set: dx_update::sets::SetId,
    locks: &[(String, String)],
) -> Result<Vec<dx_audit::vuln::LockedPackage>, String> {
    let mut all = Vec::new();
    for (rel, text) in locks {
        let mut packages = match set {
            dx_update::sets::SetId::Cargo => dx_audit::locks::parse_cargo_lock(text),
            dx_update::sets::SetId::Npm => match rel.as_str() {
                "package-lock.json" => dx_audit::locks::parse_package_lock(text),
                "yarn.lock" => dx_audit::locks::parse_yarn_lock(text),
                _ => dx_audit::locks::parse_pnpm_lock(text),
            },
            dx_update::sets::SetId::Maven => dx_audit::locks::parse_maven_install(text),
            dx_update::sets::SetId::NuGet => dx_audit::locks::parse_paket_lock(text),
            dx_update::sets::SetId::Go => dx_audit::locks::parse_go_mod(text),
        }
        .map_err(|detail| format!("could not parse {rel}: {detail}"))?;
        all.append(&mut packages);
    }
    all.sort_by(|a, b| (&a.set, &a.name, &a.version).cmp(&(&b.set, &b.name, &b.version)));
    all.dedup_by(|b, a| a.set == b.set && a.name == b.name && a.version == b.version);
    Ok(all)
}

fn default_license_policy() -> dx_audit::license_policy::LicensePolicy {
    use std::collections::BTreeSet;
    let list =
        |items: &[&str]| -> BTreeSet<String> { items.iter().copied().map(str::to_owned).collect() };
    dx_audit::license_policy::LicensePolicy {
        tables: dx_audit::license_policy::PolicyTables {
            blocked: list(&["AGPL-3.0-only", "AGPL-3.0-or-later", "SSPL-1.0"]),
            allow: list(&[
                "MIT",
                "Apache-2.0",
                "BSD-2-Clause",
                "BSD-3-Clause",
                "ISC",
                "Unlicense",
            ]),
            review: list(&[
                "LGPL-2.1-only",
                "LGPL-2.1-or-later",
                "LGPL-3.0-only",
                "LGPL-3.0-or-later",
                "MPL-2.0",
                "EPL-2.0",
                "CDDL-1.0",
            ]),
            deny: list(&[
                "GPL-2.0-only",
                "GPL-2.0-or-later",
                "GPL-3.0-only",
                "GPL-3.0-or-later",
            ]),
        },
        sets: Vec::new(),
        distribution: dx_audit::license_policy::Distribution::default(),
        exceptions: Vec::new(),
        inventory: Vec::new(),
    }
}

fn load_license_policy(
    workspace: &Path,
) -> Result<dx_audit::license_policy::LicensePolicy, String> {
    let rel = "licenses.toml";
    let full = workspace.join(rel);
    if !full.is_file() {
        return Ok(default_license_policy());
    }
    match read_workspace_text(workspace, rel) {
        Err(detail) => Err(detail),
        Ok(text) => {
            if text.trim().is_empty() {
                return Ok(default_license_policy());
            }
            dx_audit::license_policy::load_licenses_toml(&text)
                .map_err(|error| format!("invalid licenses.toml: {error}"))
        }
    }
}

fn tier_for_roots(
    policy: &dx_audit::license_policy::LicensePolicy,
    roots: &[String],
) -> dx_audit::license_expr::Tier {
    use dx_audit::license_expr::Tier;
    for root in roots {
        if policy.distribution.tier_of(root) == Tier::Distributed {
            return Tier::Distributed;
        }
    }
    Tier::Internal
}

fn severity_for_level(level: &str) -> Severity {
    match level {
        "warning" => Severity::Warning,
        "info" => Severity::Info,
        _ => Severity::Error,
    }
}

fn meets_audit_threshold(level: &str, fail_on: Threshold) -> bool {
    let severity = severity_for_level(level);
    dx_output::meets_threshold(severity, fail_on)
}

struct SecurityResult {
    status: dx_audit::outcome::FamilyStatus,
    diagnostics: Vec<DiagnosticEvent>,
    detail: String,
}

struct LicenseResult {
    status: dx_audit::outcome::FamilyStatus,
    packages: Vec<dx_audit::spdx::SpdxPackage>,
    diagnostics: Vec<DiagnosticEvent>,
    detail: String,
}

fn run_secrets(
    workspace: &Path,
    runner: &dyn dx_process::Runner,
    temp_dir: &Path,
    pid: u32,
    nonce: u64,
) -> (
    Vec<dx_audit::secrets::SecretFinding>,
    Option<String>,
    Vec<DiagnosticEvent>,
) {
    let report_file: PathBuf = temp_dir.join(format!("audit-gitleaks-{pid}-{nonce}.sarif"));
    let report_arg = report_file.to_string_lossy().into_owned();
    let config = if workspace.join(".gitleaks.toml").is_file() {
        Some(".gitleaks.toml")
    } else {
        None
    };
    let plan = match dx_audit::backend::plan_secrets(&report_arg, config) {
        Ok(dx_audit::backend::BackendPlan::Run { argv, env }) => (argv, env),
        Ok(dx_audit::backend::BackendPlan::Noop) => (Vec::new(), Vec::new()),
        Err(error) => {
            return (
                Vec::new(),
                Some(format!("failed to plan secrets audit: {error}")),
                Vec::new(),
            );
        }
    };
    let (argv, extra) = plan;
    if argv.is_empty() {
        return (Vec::new(), None, Vec::new());
    }
    let env_refs: Vec<(&str, &str)> = extra
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let status = match runner.run(&argv, workspace, &env_refs) {
        Err(error) => {
            return (
                Vec::new(),
                Some(format!("failed to launch secrets auditor: {error}")),
                Vec::new(),
            );
        }
        Ok(status) => status,
    };
    let code = status.code;
    let sarif_text = std::fs::read(&report_file)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok());
    match code {
        Some(0) => match sarif_text {
            None => (Vec::new(), None, Vec::new()),
            Some(text) => match dx_audit::secrets::triage_sarif(&text) {
                Err(detail) => (
                    Vec::new(),
                    Some(format!("invalid gitleaks SARIF: {detail}")),
                    Vec::new(),
                ),
                Ok(findings) => {
                    if findings.is_empty() {
                        (Vec::new(), None, Vec::new())
                    } else {
                        let diagnostics = findings
                            .iter()
                            .map(|finding| DiagnosticEvent {
                                severity: Severity::Error,
                                tool: "gitleaks".to_owned(),
                                message: format!("{}: {}", finding.rule, finding.message),
                                rule: Some(finding.rule.clone()),
                                path: finding.path.clone(),
                                range: None,
                                snapshot: Snapshot::Terminal,
                                fixable: false,
                                resolution: None,
                            })
                            .collect();
                        (findings, None, diagnostics)
                    }
                }
            },
        },
        Some(1) | Some(2) => match sarif_text {
            None => (
                Vec::new(),
                Some("secrets auditor reported leaks-or-errors without a SARIF report".to_owned()),
                Vec::new(),
            ),
            Some(text) => match dx_audit::secrets::triage_sarif(&text) {
                Err(detail) => (
                    Vec::new(),
                    Some(format!("invalid gitleaks SARIF: {detail}")),
                    Vec::new(),
                ),
                Ok(findings) => {
                    if findings.is_empty() {
                        (
                            Vec::new(),
                            Some(format!(
                                "secrets auditor exited {} with no SARIF results",
                                code.unwrap_or(1)
                            )),
                            Vec::new(),
                        )
                    } else {
                        let diagnostics = findings
                            .iter()
                            .map(|finding| DiagnosticEvent {
                                severity: Severity::Error,
                                tool: "gitleaks".to_owned(),
                                message: format!("{}: {}", finding.rule, finding.message),
                                rule: Some(finding.rule.clone()),
                                path: finding.path.clone(),
                                range: None,
                                snapshot: Snapshot::Terminal,
                                fixable: false,
                                resolution: None,
                            })
                            .collect();
                        (findings, None, diagnostics)
                    }
                }
            },
        },
        Some(other) => (
            Vec::new(),
            Some(format!("secrets auditor exited {other}")),
            Vec::new(),
        ),
        None => (
            Vec::new(),
            Some("secrets auditor terminated by signal".to_owned()),
            Vec::new(),
        ),
    }
}

struct SecurityInputs<'a> {
    workspace: &'a Path,
    runner: &'a dyn dx_process::Runner,
    temp_dir: &'a Path,
    pid: u32,
    nonce: u64,
    sets: &'a [dx_update::sets::SetId],
    fail_on: Threshold,
    today: &'a str,
}

fn run_security(inputs: SecurityInputs<'_>) -> SecurityResult {
    let SecurityInputs {
        workspace,
        runner,
        temp_dir,
        pid,
        nonce,
        sets,
        fail_on,
        today,
    } = inputs;
    let (secret_findings, secrets_incomplete, mut diagnostics) =
        run_secrets(workspace, runner, temp_dir, pid, nonce);
    let mut vuln_findings_all: Vec<dx_audit::vuln::VulnFinding> = Vec::new();
    let mut unassessed_all: Vec<dx_audit::vuln::Unassessed> = Vec::new();
    let mut incomplete: Option<String> = secrets_incomplete;
    for set in sets {
        if dx_audit::backend::is_empty_set(set.name()) {
            continue;
        }
        let locks = match lock_texts_for_set(workspace, *set) {
            Err(detail) => {
                let message = format!("failed to assess {}: {detail}", set.name());
                if incomplete.is_none() {
                    incomplete = Some(message.clone());
                }
                continue;
            }
            Ok(locks) => locks,
        };
        let packages = match parse_locked_for_set(*set, &locks) {
            Err(detail) => {
                if incomplete.is_none() {
                    incomplete = Some(detail.clone());
                }
                continue;
            }
            Ok(packages) => packages,
        };
        let advisories = match load_advisories(workspace, *set, today) {
            Err(detail) => {
                if incomplete.is_none() {
                    incomplete = Some(format!("failed to assess {}: {detail}", set.name()));
                }
                continue;
            }
            Ok(advisories) => advisories,
        };
        let (findings, unassessed) = dx_audit::vuln::match_packages(&packages, &advisories);
        let (unexempted, problems) = dx_audit::vuln::apply_exceptions(&findings, &[], today);
        if !problems.is_empty() {
            let first = problems
                .first()
                .map(|problem| problem.to_string())
                .unwrap_or_else(|| "exception validation failed".to_owned());
            if incomplete.is_none() {
                incomplete = Some(format!("failed to assess {}: {first}", set.name()));
            }
        }
        for finding in &unexempted {
            let lock_path = dx_audit::backend::vuln_locks(set.name())
                .first()
                .copied()
                .unwrap_or("unknown lockfile");
            let fixed = if finding.fixed.is_empty() {
                "no fixed version available".to_owned()
            } else {
                format!("fixed in {}", finding.fixed.join(", "))
            };
            diagnostics.push(DiagnosticEvent {
                severity: severity_for_level(finding.level),
                tool: "vuln".to_owned(),
                message: format!(
                    "{}@{} vulnerable to {} (severity {}; {fixed})",
                    finding.package, finding.version, finding.advisory, finding.severity
                ),
                rule: Some(format!(
                    "{}/{}",
                    dx_audit::vuln::VULN_RULE_PREFIX,
                    finding.advisory
                )),
                path: Some(lock_path.to_owned()),
                range: None,
                snapshot: Snapshot::Terminal,
                fixable: false,
                resolution: None,
            });
        }
        for item in &unassessed {
            if incomplete.is_none() {
                incomplete = Some(format!(
                    "incomplete assessment: {} in {} ({})",
                    item.package, item.set, item.reason
                ));
            }
        }
        vuln_findings_all.extend(unexempted);
        unassessed_all.extend(unassessed);
    }
    let failing_secrets = !secret_findings.is_empty();
    let failing_vuln = vuln_findings_all
        .iter()
        .any(|finding| meets_audit_threshold(finding.level, fail_on));
    let has_incomplete = incomplete.is_some() || !unassessed_all.is_empty();
    let status = if has_incomplete {
        dx_audit::outcome::FamilyStatus::Incomplete
    } else if failing_secrets || failing_vuln {
        dx_audit::outcome::FamilyStatus::Findings
    } else {
        dx_audit::outcome::FamilyStatus::Clean
    };
    let detail = if has_incomplete {
        incomplete
            .clone()
            .unwrap_or_else(|| "incomplete assessment".to_owned())
    } else if failing_secrets || failing_vuln {
        format!(
            "{} secret findings, {} vulnerability findings",
            secret_findings.len(),
            vuln_findings_all
                .iter()
                .filter(|finding| meets_audit_threshold(finding.level, fail_on))
                .count()
        )
    } else {
        "clean".to_owned()
    };
    SecurityResult {
        status,
        diagnostics,
        detail,
    }
}

fn run_license(
    workspace: &Path,
    sets: &[dx_update::sets::SetId],
    roots: &[String],
    fail_on: Threshold,
    today: &str,
) -> LicenseResult {
    let policy = match load_license_policy(workspace) {
        Err(detail) => {
            return LicenseResult {
                status: dx_audit::outcome::FamilyStatus::Incomplete,
                packages: Vec::new(),
                diagnostics: Vec::new(),
                detail: detail.clone(),
            };
        }
        Ok(policy) => policy,
    };
    if let Err(error) = policy.tables.validate() {
        let detail = format!("invalid license policy: {error}");
        return LicenseResult {
            status: dx_audit::outcome::FamilyStatus::Incomplete,
            packages: Vec::new(),
            diagnostics: Vec::new(),
            detail: detail.clone(),
        };
    }
    for adjustment in &policy.sets {
        if let Err(error) = policy.tables.validate_set(adjustment) {
            let detail = format!("invalid license policy: {error}");
            return LicenseResult {
                status: dx_audit::outcome::FamilyStatus::Incomplete,
                packages: Vec::new(),
                diagnostics: Vec::new(),
                detail: detail.clone(),
            };
        }
    }
    for entry in &policy.inventory {
        if let Err(error) = dx_audit::license_policy::validate_license_inventory(entry) {
            let detail = format!("invalid license policy: {error}");
            return LicenseResult {
                status: dx_audit::outcome::FamilyStatus::Incomplete,
                packages: Vec::new(),
                diagnostics: Vec::new(),
                detail: detail.clone(),
            };
        }
    }
    let tier = tier_for_roots(&policy, roots);
    let mut finding_count: usize = 0;
    let mut diagnostics: Vec<DiagnosticEvent> = Vec::new();
    let mut packages_all: Vec<dx_audit::spdx::SpdxPackage> = Vec::new();
    let mut incomplete: Option<String> = None;
    let mut licensed_all: Vec<dx_audit::locks::LicensedPackage> = Vec::new();
    // Per-package notice-text presence from the committed inventory,
    // keyed by `set/name@version` for the notice evaluation below.
    // Absent entries mean no words (fail closed in `distributed` when
    // the license requires reproduction).
    let mut notice_present: std::collections::BTreeMap<(String, String, String), bool> =
        std::collections::BTreeMap::new();
    for set in sets {
        if dx_audit::backend::is_empty_set(set.name()) {
            continue;
        }
        let locks = match lock_texts_for_set(workspace, *set) {
            Err(detail) => {
                if incomplete.is_none() {
                    incomplete = Some(format!("failed to assess {}: {detail}", set.name()));
                }
                continue;
            }
            Ok(locks) => locks,
        };
        let locked = match parse_locked_for_set(*set, &locks) {
            Err(detail) => {
                if incomplete.is_none() {
                    incomplete = Some(detail.clone());
                }
                continue;
            }
            Ok(locked) => locked,
        };
        let mut licensed = match *set {
            dx_update::sets::SetId::Cargo => {
                let bazel_text =
                    read_workspace_text(workspace, "cargo-bazel-lock.json").unwrap_or_default();
                dx_audit::locks::cargo_licenses(&bazel_text, &locked)
            }
            dx_update::sets::SetId::Npm => {
                if let Some((_, package_lock_text)) =
                    locks.iter().find(|(rel, _)| rel == "package-lock.json")
                {
                    dx_audit::locks::npm_licenses(package_lock_text, &locked)
                } else {
                    dx_audit::locks::unknown_licenses(&locked, set.name())
                }
            }
            _ => dx_audit::locks::inventory_licenses(&locked, &policy.inventory, set.name()),
        };
        // Committed inventory overrides automatic identities where it
        // matches (explicit curator data wins); for Maven/NuGet/Go the
        // inventory is the only source, already resolved above.
        if *set == dx_update::sets::SetId::Cargo || *set == dx_update::sets::SetId::Npm {
            let inventoried =
                dx_audit::locks::inventory_licenses(&locked, &policy.inventory, set.name());
            let by_key: std::collections::BTreeMap<(String, String), String> = inventoried
                .into_iter()
                .map(|entry| ((entry.name, entry.version), entry.license))
                .collect();
            for entry in &mut licensed {
                if let Some(license) = by_key.get(&(entry.name.clone(), entry.version.clone())) {
                    if license != "UNKNOWN" {
                        entry.license = license.clone();
                    }
                }
            }
        }
        for entry in &licensed {
            let probe = dx_audit::vuln::LockedPackage {
                name: entry.name.clone(),
                version: entry.version.clone(),
                set: entry.set.clone(),
                is_git: false,
                is_private: false,
            };
            let present = dx_audit::locks::inventory_text_present(&probe, &policy.inventory);
            notice_present.insert(
                (entry.set.clone(), entry.name.clone(), entry.version.clone()),
                present,
            );
        }
        licensed_all.extend(licensed);
    }
    licensed_all.sort_by(|a, b| (&a.set, &a.name, &a.version).cmp(&(&b.set, &b.name, &b.version)));
    let mut spdx_index = 1usize;
    for licensed in &licensed_all {
        let expr = dx_audit::license_expr::parse_license(&licensed.license);
        let lookup = |id: &str| -> dx_audit::license_expr::IdClass {
            use dx_audit::license_expr::IdClass;
            if policy.tables.blocked.contains(id) {
                return IdClass::Blocked;
            }
            if policy.tables.allow.contains(id) {
                return IdClass::Allow;
            }
            if policy.tables.review.contains(id) {
                return IdClass::Review;
            }
            if policy.tables.deny.contains(id) {
                return IdClass::Deny;
            }
            for adjustment in &policy.sets {
                if adjustment.set == licensed.set && adjustment.review.contains(id) {
                    return IdClass::Review;
                }
            }
            IdClass::Unlisted
        };
        let mut approved_hit = false;
        for exception in &policy.exceptions {
            if dx_audit::license_policy::validate_license_exception(exception, today).is_err() {
                continue;
            }
            if !dx_audit::license_policy::license_exception_covers(
                exception,
                &dx_audit::license_policy::LicenseFinding {
                    package: licensed.name.clone(),
                    set: licensed.set.clone(),
                    license: licensed.license.clone(),
                    version: licensed.version.clone(),
                },
            ) {
                continue;
            }
            approved_hit = true;
            break;
        }
        let approved = |name: &str| -> bool {
            if name != licensed.license {
                return false;
            }
            approved_hit
        };
        let outcome = dx_audit::license_expr::evaluate(&expr, tier, &lookup, &approved);
        let fails = dx_audit::license_expr::fails_in_tier(outcome, tier);
        let level = if fails { "error" } else { "info" };
        // Per-package notice-text evaluation: the words come from the
        // committed inventory (`text_present`), absent means no words.
        // `missing-notice-text` fails in `distributed` unless the same
        // exception that approves the license approves it.
        let text_present = notice_present
            .get(&(
                licensed.set.clone(),
                licensed.name.clone(),
                licensed.version.clone(),
            ))
            .copied()
            .unwrap_or(false);
        let notice_input = dx_audit::license_notice::NoticeInput {
            package: licensed.name.clone(),
            license: licensed.license.clone(),
            text_present,
        };
        let notice_outcome =
            dx_audit::license_notice::evaluate_notice(&notice_input, tier, &approved);
        let notice_fails = dx_audit::license_notice::notice_fails(notice_outcome, tier);
        let spdx_license = if licensed.license.trim().is_empty() {
            "NOASSERTION".to_owned()
        } else {
            licensed.license.clone()
        };
        packages_all.push(dx_audit::spdx::spdx_package(
            &licensed.set,
            &licensed.name,
            &licensed.version,
            &spdx_license,
            spdx_index,
        ));
        spdx_index += 1;
        if fails && meets_audit_threshold(level, fail_on) {
            let lock_path = dx_audit::backend::vuln_locks(&licensed.set)
                .first()
                .copied()
                .unwrap_or("unknown lockfile");
            finding_count += 1;
            diagnostics.push(DiagnosticEvent {
                severity: Severity::Error,
                tool: "license".to_owned(),
                message: format!(
                    "{}@{} license {} denied in {}",
                    licensed.name,
                    licensed.version,
                    licensed.license,
                    match tier {
                        dx_audit::license_expr::Tier::Distributed => "distributed",
                        dx_audit::license_expr::Tier::Internal => "internal",
                    }
                ),
                rule: Some(format!("license/{}", licensed.license)),
                path: Some(lock_path.to_owned()),
                range: None,
                snapshot: Snapshot::Terminal,
                fixable: false,
                resolution: None,
            });
        }
        if notice_fails && meets_audit_threshold("error", fail_on) {
            let lock_path = dx_audit::backend::vuln_locks(&licensed.set)
                .first()
                .copied()
                .unwrap_or("unknown lockfile");
            finding_count += 1;
            diagnostics.push(DiagnosticEvent {
                severity: Severity::Error,
                tool: "license".to_owned(),
                message: format!(
                    "{}@{} license {} missing-notice-text in {}",
                    licensed.name,
                    licensed.version,
                    licensed.license,
                    match tier {
                        dx_audit::license_expr::Tier::Distributed => "distributed",
                        dx_audit::license_expr::Tier::Internal => "internal",
                    }
                ),
                rule: Some("license/missing-notice-text".to_owned()),
                path: Some(lock_path.to_owned()),
                range: None,
                snapshot: Snapshot::Terminal,
                fixable: false,
                resolution: None,
            });
        }
    }
    for exception in &policy.exceptions {
        if let Err(error) = dx_audit::license_policy::validate_license_exception(exception, today) {
            if incomplete.is_none() {
                incomplete = Some(format!("invalid license exception: {error}"));
            }
            continue;
        }
        let refs: Vec<dx_audit::license_policy::LicenseFinding> = licensed_all
            .iter()
            .map(|licensed| dx_audit::license_policy::LicenseFinding {
                package: licensed.name.clone(),
                set: licensed.set.clone(),
                license: licensed.license.clone(),
                version: licensed.version.clone(),
            })
            .collect();
        if let Err(error) = dx_audit::license_policy::check_applies(exception, &refs) {
            if incomplete.is_none() {
                incomplete = Some(format!("invalid license exception: {error}"));
            }
        }
    }
    let has_incomplete = incomplete.is_some();
    let status = if has_incomplete {
        dx_audit::outcome::FamilyStatus::Incomplete
    } else if finding_count > 0 {
        dx_audit::outcome::FamilyStatus::Findings
    } else {
        dx_audit::outcome::FamilyStatus::Clean
    };
    let detail = if has_incomplete {
        incomplete
            .clone()
            .unwrap_or_else(|| "incomplete assessment".to_owned())
    } else if finding_count > 0 {
        format!("{finding_count} license findings")
    } else {
        "clean".to_owned()
    };
    LicenseResult {
        status,
        packages: packages_all,
        diagnostics,
        detail,
    }
}

/// Runs `dx audit` live: family selection and scope defaults
/// through `dx_audit`, dependency-set resolution through the approved
/// `dx_update` registry, then qualified auditors per family over resolved
/// scopes with per-family reporting. `--dry-run` prints the planned
/// families and scopes and exits `0` without launching. Audit is
/// non-mutating: advisory refresh changes analysis inputs, never
/// manifests, lockfiles, or projections.
pub(crate) fn execute_audit(invocation: &Invocation, env: Env<'_>) -> i32 {
    debug_assert!(
        invocation.command == Command::Audit,
        "audit dispatch guards commands"
    );
    let Env {
        workspace,
        runner,
        temp_dir,
        pid,
        nonce,
        out,
        err,
        ..
    } = env;
    let request = match dx_audit::plan_audit(&invocation.targets) {
        Ok(request) => request,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let planned_reports = match plan_reports(
        invocation.command,
        &invocation.reports,
        &invocation.output,
        invocation.dry_run,
    ) {
        Ok(planned) => planned,
        Err(error) => return pre_exec(err, &error.to_string()),
    };
    let families = request
        .families
        .iter()
        .map(|family| family.as_str())
        .collect::<Vec<_>>()
        .join("+");
    let effective = request.effective_scopes();
    let scopes = effective.join(", ");
    let summary = format!("Running audit {families} for {scopes}");
    let verbose =
        matches!(invocation.output, OutputMode::Text { quiet: false }) && !invocation.quiet;
    if invocation.dry_run {
        if invocation.output == OutputMode::Json {
            if let Ok(event) = command_started(invocation.command.name(), true, "default") {
                let _ = write_event(out, &event);
            }
            let finished = command_finished(0, &FinishedCounts::default());
            let _ = write_event(out, &finished);
        } else if verbose {
            let _ = writeln!(out, "{summary}");
        }
        return 0;
    }
    let sets = match resolve_audit_sets(&effective) {
        Ok(sets) => sets,
        Err(detail) => return pre_exec(err, &detail),
    };
    if invocation.output == OutputMode::Json {
        if let Ok(event) = command_started(invocation.command.name(), false, "default") {
            let _ = write_event(out, &event);
        }
    } else if verbose {
        let _ = writeln!(out, "{summary}");
    }
    let today = today_utc();
    let mut outcomes: Vec<dx_audit::outcome::FamilyOutcome> = Vec::new();
    let mut sarif_diagnostics: Vec<DiagnosticEvent> = Vec::new();
    let mut sarif_tools: BTreeSet<String> = BTreeSet::new();
    let mut spdx_packages: Vec<dx_audit::spdx::SpdxPackage> = Vec::new();
    let mut family_messages: BTreeMap<String, String> = BTreeMap::new();
    let mut any_incomplete = false;
    for family in &request.families {
        match *family {
            dx_audit::AuditFamily::Security => {
                let result = run_security(SecurityInputs {
                    workspace,
                    runner,
                    temp_dir,
                    pid,
                    nonce,
                    sets: &sets,
                    fail_on: invocation.fail_on,
                    today: &today,
                });
                sarif_tools.insert("gitleaks".to_owned());
                sarif_tools.insert("vuln".to_owned());
                sarif_diagnostics.extend(result.diagnostics.clone());
                if result.status == dx_audit::outcome::FamilyStatus::Incomplete {
                    any_incomplete = true;
                }
                family_messages.insert("security".to_owned(), result.detail.clone());
                outcomes.push(dx_audit::outcome::FamilyOutcome {
                    family: *family,
                    status: result.status,
                });
            }
            dx_audit::AuditFamily::License => {
                let result = run_license(workspace, &sets, &effective, invocation.fail_on, &today);
                sarif_tools.insert("license".to_owned());
                sarif_diagnostics.extend(result.diagnostics.clone());
                spdx_packages.extend(result.packages.clone());
                if result.status == dx_audit::outcome::FamilyStatus::Incomplete {
                    any_incomplete = true;
                }
                family_messages.insert("license".to_owned(), result.detail.clone());
                outcomes.push(dx_audit::outcome::FamilyOutcome {
                    family: *family,
                    status: result.status,
                });
            }
        }
    }
    let report = dx_audit::outcome::AuditReport::aggregate(outcomes);
    let exit = dx_audit::outcome::exit_code(&report);
    let sarif_complete = !any_incomplete && report.incomplete().is_empty();
    let mut reports_ok = true;
    for planned in &planned_reports {
        let name = planned.format.name();
        if name == "sarif" {
            let tools: Vec<String> = sarif_tools.iter().cloned().collect();
            let snapshots: BTreeMap<String, String> = BTreeMap::new();
            let document = match crate::reports::render_sarif(
                &tools,
                &sarif_diagnostics,
                &snapshots,
                sarif_complete,
            ) {
                Ok(document) => document,
                Err(error) => {
                    reports_ok = false;
                    let detail = format!("failed to render SARIF report: {error}");
                    let _ = writeln!(err, "dx: report_failed: {detail}");
                    if invocation.output == OutputMode::Json {
                        if let Ok(event) =
                            dx_output::error_event(CODE_REPORT_FAILED, &detail, None, None, None)
                        {
                            let _ = write_event(out, &event);
                        }
                    }
                    continue;
                }
            };
            let written = match &planned.destination {
                Destination::Stdout => out
                    .write_all(document.as_bytes())
                    .and_then(|()| out.write_all(b"\n"))
                    .is_ok(),
                Destination::File(destination) => {
                    let target = workspace.join(destination);
                    let parent_ok = target
                        .parent()
                        .is_none_or(|parent| parent.as_os_str().is_empty() || parent.is_dir());
                    if !parent_ok {
                        false
                    } else {
                        std::fs::write(&target, document.as_bytes()).is_ok()
                    }
                }
            };
            if !written {
                reports_ok = false;
                let detail = format!(
                    "failed to write sarif report to {}",
                    planned.destination.display()
                );
                let _ = writeln!(err, "dx: report_failed: {detail}");
                if invocation.output == OutputMode::Json {
                    if let Ok(event) =
                        dx_output::error_event(CODE_REPORT_FAILED, &detail, None, None, None)
                    {
                        let _ = write_event(out, &event);
                    }
                }
                continue;
            }
            if invocation.output == OutputMode::Json {
                if let Ok(event) =
                    report_event("sarif", planned.destination.display(), sarif_complete)
                {
                    let _ = write_event(out, &event);
                }
            } else if matches!(invocation.output, OutputMode::Text { .. }) && !invocation.quiet {
                if planned.destination != Destination::Stdout {
                    let _ = writeln!(
                        out,
                        "Wrote sarif report to {}.",
                        planned.destination.display()
                    );
                }
            } else if invocation.output == OutputMode::Diff {
                let _ = writeln!(
                    err,
                    "Wrote sarif report to {}.",
                    planned.destination.display()
                );
            }
        } else if name == "spdx" {
            let namespace = format!("https://dx-audit.local/{pid}-{nonce}");
            let contains: Vec<(String, String)> = Vec::new();
            let document =
                dx_audit::spdx::render_spdx(&effective, &spdx_packages, &contains, &namespace);
            let written = match &planned.destination {
                Destination::Stdout => out
                    .write_all(document.as_bytes())
                    .and_then(|()| out.write_all(b"\n"))
                    .is_ok(),
                Destination::File(destination) => {
                    let target = workspace.join(destination);
                    let parent_ok = target
                        .parent()
                        .is_none_or(|parent| parent.as_os_str().is_empty() || parent.is_dir());
                    if !parent_ok {
                        false
                    } else {
                        std::fs::write(&target, document.as_bytes()).is_ok()
                    }
                }
            };
            if !written {
                reports_ok = false;
                let detail = format!(
                    "failed to write spdx report to {}",
                    planned.destination.display()
                );
                let _ = writeln!(err, "dx: report_failed: {detail}");
                if invocation.output == OutputMode::Json {
                    if let Ok(event) =
                        dx_output::error_event(CODE_REPORT_FAILED, &detail, None, None, None)
                    {
                        let _ = write_event(out, &event);
                    }
                }
                continue;
            }
            if invocation.output == OutputMode::Json {
                if let Ok(event) =
                    report_event("spdx", planned.destination.display(), !any_incomplete)
                {
                    let _ = write_event(out, &event);
                }
            } else if matches!(invocation.output, OutputMode::Text { .. }) && !invocation.quiet {
                if planned.destination != Destination::Stdout {
                    let _ = writeln!(
                        out,
                        "Wrote spdx report to {}.",
                        planned.destination.display()
                    );
                }
            } else if invocation.output == OutputMode::Diff {
                let _ = writeln!(
                    err,
                    "Wrote spdx report to {}.",
                    planned.destination.display()
                );
            }
        }
    }
    if !reports_ok {
        if invocation.output == OutputMode::Json {
            let finished = command_finished(
                1,
                &FinishedCounts {
                    results_complete: Some(false),
                    ..FinishedCounts::default()
                },
            );
            let _ = write_event(out, &finished);
        } else if verbose {
            let _ = writeln!(out, "dx audit: report write failed");
        }
        return 1;
    }
    if invocation.output == OutputMode::Json {
        for outcome in &report.outcomes {
            let family_name = outcome.family.as_str();
            let message = family_messages
                .get(family_name)
                .cloned()
                .unwrap_or_else(|| "audit complete".to_owned());
            match outcome.status {
                dx_audit::outcome::FamilyStatus::Clean => {
                    if let Ok(event) = notice_event(&NoticeEvent {
                        level: "info".to_owned(),
                        code: format!("audit_{family_name}_clean"),
                        message,
                        related_command: Some("audit".to_owned()),
                        scope: Some(effective.clone()),
                        path: None,
                        language: None,
                        import: None,
                    }) {
                        let _ = write_event(out, &event);
                    }
                }
                dx_audit::outcome::FamilyStatus::Findings
                | dx_audit::outcome::FamilyStatus::Incomplete => {
                    if let Ok(event) = error_event(
                        CODE_AUDIT_FAILED,
                        &format!("audit {family_name}: {message}"),
                        None,
                        None,
                        Some("execute"),
                    ) {
                        let _ = write_event(out, &event);
                    }
                    let _ = writeln!(
                        err,
                        "dx: {CODE_AUDIT_FAILED}: audit {family_name}: {message}"
                    );
                }
            }
        }
        let finished = command_finished(
            exit,
            &FinishedCounts {
                results_complete: Some(sarif_complete),
                ..FinishedCounts::default()
            },
        );
        let _ = write_event(out, &finished);
        return exit;
    }
    for outcome in &report.outcomes {
        let family_name = outcome.family.as_str();
        let message = family_messages
            .get(family_name)
            .cloned()
            .unwrap_or_default();
        match outcome.status {
            dx_audit::outcome::FamilyStatus::Clean => {
                if verbose {
                    let _ = writeln!(out, "audit {family_name}: clean");
                }
            }
            dx_audit::outcome::FamilyStatus::Findings
            | dx_audit::outcome::FamilyStatus::Incomplete => {
                let _ = writeln!(
                    err,
                    "dx: {CODE_AUDIT_FAILED}: audit {family_name}: {message}"
                );
            }
        }
    }
    if verbose {
        let clean = report
            .outcomes
            .iter()
            .filter(|outcome| outcome.status == dx_audit::outcome::FamilyStatus::Clean)
            .count();
        let failed = report.outcomes.len().saturating_sub(clean);
        let _ = writeln!(out, "dx audit: {clean} clean, {failed} failed");
    }
    exit
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use dx_process::{ChildStatus, Runner};
    use std::cell::RefCell;
    use std::io;
    use std::path::Path;
    use std::rc::Rc;

    struct AuditRunner {
        calls: Rc<RefCell<Vec<Vec<String>>>>,
        code: Option<i32>,
        sarif: Option<String>,
    }

    impl AuditRunner {
        fn clean() -> Self {
            AuditRunner {
                calls: Rc::new(RefCell::new(Vec::new())),
                code: Some(0),
                sarif: None,
            }
        }

        fn with_sarif(code: Option<i32>, sarif: &str) -> Self {
            AuditRunner {
                calls: Rc::new(RefCell::new(Vec::new())),
                code,
                sarif: Some(sarif.to_owned()),
            }
        }
    }

    impl Runner for AuditRunner {
        fn run(
            &self,
            argv: &[String],
            _cwd: &Path,
            _env: &[(&str, &str)],
        ) -> io::Result<ChildStatus> {
            self.calls.borrow_mut().push(argv.to_vec());
            if let Some(sarif) = &self.sarif {
                for (index, arg) in argv.iter().enumerate() {
                    if arg == "--report-path" {
                        if let Some(path) = argv.get(index + 1) {
                            let _ = std::fs::write(path, sarif.as_bytes());
                        }
                    }
                }
            }
            Ok(ChildStatus { code: self.code })
        }
    }

    fn run_with(
        argv: &[&str],
        runner: &AuditRunner,
        setup: &dyn Fn(&Harness),
    ) -> (i32, String, String) {
        use crate::args::parse;
        use crate::exec::{execute, Env};
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let words: Vec<String> = argv.iter().map(|word| (*word).to_owned()).collect();
        let invocation = parse(&words).expect("parse");
        let harness = Harness::new(&format!(
            "audit-live-{}-{}-{}",
            std::thread::current()
                .name()
                .unwrap_or("test")
                .replace(':', "_"),
            argv.join("-").replace('/', "_"),
            id
        ));
        setup(&harness);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = execute(
            &invocation,
            Env {
                workspace: &harness.workspace,
                runner,
                query_runner: &harness.query,
                temp_dir: &harness.temp,
                pid: std::process::id(),
                nonce: 0,
                out: &mut out,
                err: &mut err,
                ci: false,
            },
        );
        (
            code,
            String::from_utf8(out).expect("stdout"),
            String::from_utf8(err).expect("stderr"),
        )
    }

    fn clean_workspace(harness: &Harness) {
        harness.write_source(
            "rust/tests/fixtures/hello/Cargo.lock",
            "[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
        );
        harness.write_source(
            "cargo-bazel-lock.json",
            r#"{"packages": {"serde 1.0.100": {"license": "MIT"}}}"#,
        );
        harness.write_source(
            "pnpm-lock.yaml",
            "lockfileVersion: '9.0'\n\npackages:\n\n  'react@18.2.0':\n    resolution: {integrity: sha512-abc}\n",
        );
        harness.write_source("third_party/jvm/maven_install.json", r#"{"artifacts": {}}"#);
        harness.write_source(
            "third_party/dotnet/paket.lock",
            "NUGET\n  remote: https://api.nuget.org/v3/index.json\n",
        );
        write_go_mod(harness);
        harness.write_source(
            "licenses.toml",
            "[policy.distributed]\nallow = [\"MIT\"]\nreview = []\ndeny = []\n",
        );
    }

    /// Minimal `go_deps.from_file` module lock for live-audit harnesses:
    /// the Go set is always assessed (never an empty clean), so every
    /// repository-wide audit fixture must carry it.
    fn write_go_mod(harness: &Harness) {
        harness.write_source(
            "third_party/go/go.mod",
            "module rules_dx/third_party/go\n\ngo 1.24.12\n\nrequire (\n\tgithub.com/bazelbuild/buildtools v0.0.0-20250930140053-2eb4fccefb52 // indirect\n\tgithub.com/google/go-cmp v0.6.0\n\tgithub.com/pmezard/go-difflib v1.0.0\n)\n",
        );
    }

    /// Fresh identified advisory snapshot for one set (issue #628): the
    /// derived bytes plus identity (`url`, `sha256`, `retrieved_at`) are
    /// the audited inputs. Snapshots refresh via supported upstream
    /// database-download tooling (per-set OSV GCS zips, no inventory
    /// upload); live CLI performs no network fetch.
    fn write_advisory(harness: &Harness, set: &str, json: &str) {
        let today = super::today_utc();
        let url = dx_audit::advisory::advisory_source(set)
            .expect("supported set needs a source")
            .to_owned();
        let sha = dx_digest::sha256_hex(json.as_bytes());
        harness.write_source(&format!(".dx/advisory/{set}.json"), json);
        let meta = serde_json::json!({
            "set": set,
            "url": url,
            "sha256": sha,
            "retrieved_at": today,
            "path": format!(".dx/advisory/{set}.json"),
        });
        harness.write_source(&format!(".dx/advisory/{set}.meta.json"), &meta.to_string());
    }

    fn write_all_empty_advisories(harness: &Harness) {
        for set in ["cargo", "npm", "maven", "nuget", "go"] {
            write_advisory(harness, set, "[]");
        }
    }

    #[test]
    fn audit_dry_run_plans_families_without_launching() {
        let harness = Harness::new("audit-dryrun");
        let (code, out, err) = harness.run(&["audit", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("Running audit security+license for //..."),
            "{out}"
        );
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );

        let harness = Harness::new("audit-dryrun-family");
        let (code, out, err) = harness.run(&["audit", "security", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Running audit security for //..."), "{out}");
        assert_eq!(err, "", "{err}");
        assert!(
            harness.seen_env.borrow().is_empty(),
            "dry-run launches nothing"
        );
    }

    #[test]
    fn audit_live_clean_runs_gitleaks_and_exits_zero() {
        let runner = AuditRunner::clean();
        let (code, out, err) = run_with(&["audit", "security"], &runner, &|harness| {
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
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("Running audit security for //..."), "{out}");
        assert!(out.contains("audit security: clean"), "{out}");
        assert_eq!(err, "", "{err}");
        assert_eq!(runner.calls.borrow().len(), 1);
        assert!(runner.calls.borrow()[0].contains(&"gitleaks".to_owned()));
        assert!(runner.calls.borrow()[0].contains(&"--redact".to_owned()));
    }

    #[test]
    fn audit_live_secrets_findings_fail_with_redacted_summary() {
        // Issue #629: an unredacted SARIF (secrets in message.text,
        // fingerprints, snippets, and properties) still yields a
        // redacted summary: rule IDs and counts only, never values.
        // Sentinels are assembled at runtime so the file never stores
        // a push-protected token shape verbatim.
        let github = format!("{}{}", "ghp_", "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8");
        let generic = format!("{}{}", "sk-live-", "51H7x9yQ2wE4rT6yU8iO0p");
        let sarif = format!(
            "{{\"version\": \"2.1.0\", \"runs\": [{{\"tool\": {{\"driver\": {{\"name\": \"gitleaks\"}}}}, \"results\": [{{\"ruleId\": \"gitleaks/aws-key\", \"message\": {{\"text\": \"leaked AKIAIOSFODNN7EXAMPLE in src/app.py\"}}, \"fingerprints\": {{\"secret\": \"AKIAIOSFODNN7EXAMPLE\"}}, \"partialFingerprints\": {{\"secret/v1\": \"{github}\"}}, \"properties\": {{\"secret\": \"{generic}\"}}, \"locations\": [{{\"physicalLocation\": {{\"artifactLocation\": {{\"uri\": \"src/app.py\"}}, \"region\": {{\"snippet\": {{\"text\": \"key = 'AKIAIOSFODNN7EXAMPLE'\"}}}}}}}}]}}]}}]}}"
        );
        let runner = AuditRunner::with_sarif(Some(1), &sarif);
        let (code, out, err) = run_with(&["audit", "security"], &runner, &|harness| {
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
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("audit_failed"), "{err}");
        assert!(err.contains("audit security"), "{err}");
        // Findings fail the audit, but secret values never reach output.
        for secret in ["AKIAIOSFODNN7EXAMPLE", github.as_str(), generic.as_str()] {
            assert!(!out.contains(secret), "{out}");
            assert!(!err.contains(secret), "{err}");
        }
        // The invocation still pins redaction on the auditor argv.
        assert!(runner.calls.borrow()[0].contains(&"--redact".to_owned()));
    }

    #[test]
    fn audit_live_vuln_findings_fail_and_git_is_incomplete() {
        let runner = AuditRunner::clean();
        let (code, _out, err) = run_with(&["audit", "security"], &runner, &|harness| {
            harness.write_source(
                "rust/tests/fixtures/hello/Cargo.lock",
                "[[package]]\nname = \"git-dep\"\nversion = \"0.1.0\"\nsource = \"git+https://github.com/example/git-dep#abc123\"\n",
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
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("audit_failed"), "{err}");
        assert!(err.contains("incomplete"), "{err}");
    }

    #[test]
    fn audit_live_npm_git_and_sibling_locks_are_incomplete() {
        // `package-lock.json` git entries fail as incomplete while
        // absent `yarn.lock` siblings are skipped, never required.
        let runner = AuditRunner::clean();
        let (code, _out, err) = run_with(&["audit", "security"], &runner, &|harness| {
            harness.write_source(
                "rust/tests/fixtures/hello/Cargo.lock",
                "[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
            );
            harness.write_source("pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
            harness.write_source(
                "package-lock.json",
                r#"{"name":"root","lockfileVersion":3,"packages":{"":{"name":"root"},"node_modules/git-dep":{"version":"github:user/repo#abc123"}}}"#,
            );
            harness.write_source("third_party/jvm/maven_install.json", r#"{"artifacts": {}}"#);
            harness.write_source(
                "third_party/dotnet/paket.lock",
                "NUGET\n  remote: https://api.nuget.org/v3/index.json\n",
            );
            write_go_mod(harness);
            write_all_empty_advisories(harness);
        });
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("audit_failed"), "{err}");
        assert!(err.contains("incomplete"), "{err}");
        assert!(err.contains("git-dep"), "{err}");
    }

    #[test]
    fn audit_live_npm_pnpm_git_resolution_is_incomplete() {
        // Pnpm `resolution: {type: git}` entries fail as incomplete,
        // never dropped and never clean.
        let runner = AuditRunner::clean();
        let (code, _out, err) = run_with(&["audit", "security"], &runner, &|harness| {
            harness.write_source(
                "rust/tests/fixtures/hello/Cargo.lock",
                "[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
            );
            harness.write_source(
                "pnpm-lock.yaml",
                "lockfileVersion: '9.0'\npackages:\n  'git-dep@github:user/repo#abc123':\n    resolution: {repo: 'https://github.com/user/repo.git', commit: abc123}\n",
            );
            harness.write_source("third_party/jvm/maven_install.json", r#"{"artifacts": {}}"#);
            harness.write_source(
                "third_party/dotnet/paket.lock",
                "NUGET\n  remote: https://api.nuget.org/v3/index.json\n",
            );
            write_go_mod(harness);
            write_all_empty_advisories(harness);
        });
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("audit_failed"), "{err}");
        assert!(err.contains("incomplete"), "{err}");
        assert!(err.contains("git-dep"), "{err}");
    }

    #[test]
    fn audit_live_go_advisory_findings_fail_instead_of_empty_clean() {
        let runner = AuditRunner::clean();
        let (code, _out, err) = run_with(
            &["audit", "security", "//go/tests/fixtures/hello:hello"],
            &runner,
            &|harness| {
                write_go_mod(harness);
                write_advisory(
                    harness,
                    "go",
                    r#"[{"id":"GHSA-go-test-0001","package":"github.com/google/go-cmp","versions":">=v0.5.0, <v0.7.0","severity":"high","fixed":["v0.7.0"],"set":"go"}]"#,
                );
            },
        );
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("audit_failed"), "{err}");
        assert!(err.contains("1 vulnerability findings"), "{err}");
    }

    #[test]
    fn audit_live_missing_advisory_fails_never_empty_clean() {
        // Issue #628: a missing snapshot means current data could not be
        // obtained, never clean and never a lockfile upload.
        let runner = AuditRunner::clean();
        let (code, out, err) = run_with(
            &["audit", "security", "//rust/tests/fixtures/hello:hello"],
            &runner,
            &|harness| {
                harness.write_source(
                    "rust/tests/fixtures/hello/Cargo.lock",
                    "[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
                );
            },
        );
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("audit_failed"), "{err}");
        assert!(
            err.contains(dx_audit::advisory::CODE_ADVISORY_REFRESH_FAILED),
            "{err}"
        );
        assert!(!out.contains("audit security: clean"), "{out}");
    }

    #[test]
    fn audit_live_stale_advisory_fails_without_stale_fallback() {
        // A `retrieved_at` older than today is stale and fails without
        // analyzing the stale bytes.
        let runner = AuditRunner::clean();
        let (code, out, err) = run_with(
            &["audit", "security", "//rust/tests/fixtures/hello:hello"],
            &runner,
            &|harness| {
                harness.write_source(
                    "rust/tests/fixtures/hello/Cargo.lock",
                    "[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
                );
                let json = "[]";
                let url = dx_audit::advisory::advisory_source("cargo")
                    .expect("source")
                    .to_owned();
                let sha = dx_digest::sha256_hex(json.as_bytes());
                harness.write_source(".dx/advisory/cargo.json", json);
                let meta = serde_json::json!({
                    "set": "cargo",
                    "url": url,
                    "sha256": sha,
                    "retrieved_at": "2000-01-01",
                    "path": ".dx/advisory/cargo.json",
                });
                harness.write_source(".dx/advisory/cargo.meta.json", &meta.to_string());
            },
        );
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("audit_failed"), "{err}");
        assert!(
            err.contains(dx_audit::advisory::CODE_ADVISORY_REFRESH_FAILED),
            "{err}"
        );
        assert!(err.contains("stale"), "{err}");
        assert!(!out.contains("audit security: clean"), "{out}");
    }

    #[test]
    fn audit_live_tampered_advisory_fails_on_sha_mismatch() {
        // Identity `sha256` must match the exact snapshot bytes; a
        // mismatch fails closed, never analyzed.
        let runner = AuditRunner::clean();
        let (code, out, err) = run_with(
            &["audit", "security", "//rust/tests/fixtures/hello:hello"],
            &runner,
            &|harness| {
                harness.write_source(
                    "rust/tests/fixtures/hello/Cargo.lock",
                    "[[package]]\nname = \"serde\"\nversion = \"1.0.100\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
                );
                harness.write_source(".dx/advisory/cargo.json", "[]");
                let today = super::today_utc();
                let url = dx_audit::advisory::advisory_source("cargo")
                    .expect("source")
                    .to_owned();
                let meta = serde_json::json!({
                    "set": "cargo",
                    "url": url,
                    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "retrieved_at": today,
                    "path": ".dx/advisory/cargo.json",
                });
                harness.write_source(".dx/advisory/cargo.meta.json", &meta.to_string());
            },
        );
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("audit_failed"), "{err}");
        assert!(
            err.contains(dx_audit::advisory::CODE_ADVISORY_REFRESH_FAILED),
            "{err}"
        );
        assert!(!out.contains("audit security: clean"), "{out}");
    }

    #[test]
    fn audit_live_license_clean_and_denied() {
        let runner = AuditRunner::clean();
        let (code, out, err) = run_with(&["audit", "license"], &runner, &|_harness| {
            // Placeholder replaced below by clean_workspace setup.
        });
        let _ = (code, out, err);
        let runner = AuditRunner::clean();
        let (code, out, err) = run_with(&["audit", "license"], &runner, &clean_workspace);
        assert_eq!(
            code, 1,
            "{out}{err} clean cargo license but missing notice plus UNKNOWN npm must fail distributed"
        );
        assert!(err.contains("audit_failed"), "{err}");

        let runner = AuditRunner::clean();
        let (code, out, err) = run_with(
            &["audit", "license", "//rust/tests/fixtures/hello:hello"],
            &runner,
            &|harness| {
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
            },
        );
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("audit license: clean"), "{out}");
    }

    #[test]
    fn audit_live_license_per_ecosystem_ids_and_notice_texts() {
        // npm via `package-lock.json` license plus inventory words: clean
        // when both identify, missing-notice-text fails distributed when
        // words are absent.
        let runner = AuditRunner::clean();
        let (code, out, err) = run_with(
            &[
                "audit",
                "license",
                "//javascript/tests/fixtures/hello:hello",
            ],
            &runner,
            &|harness| {
                harness.write_source(
                    "package-lock.json",
                    r#"{"name":"root","lockfileVersion":3,"packages":{"":{"name":"root"},"node_modules/react":{"version":"18.2.0","license":"MIT"}}}"#,
                );
                harness.write_source(
                    "licenses.toml",
                    "[policy.distributed]\nallow = [\"MIT\"]\nreview = []\ndeny = []\n\n[[inventory]]\npackage = \"react\"\nset = \"npm\"\nlicense = \"MIT\"\nversions = \"18.2.0\"\ntext_present = true\n",
                );
            },
        );
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("audit license: clean"), "{out}");

        // Same npm package without words fails (MIT is allow-listed, so
        // the failure is the notice check firing, not the license table).
        let runner = AuditRunner::clean();
        let (code, _out, err) = run_with(
            &[
                "audit",
                "license",
                "//javascript/tests/fixtures/hello:hello",
            ],
            &runner,
            &|harness| {
                harness.write_source(
                    "package-lock.json",
                    r#"{"name":"root","lockfileVersion":3,"packages":{"":{"name":"root"},"node_modules/react":{"version":"18.2.0","license":"MIT"}}}"#,
                );
                harness.write_source(
                    "licenses.toml",
                    "[policy.distributed]\nallow = [\"MIT\"]\nreview = []\ndeny = []\n",
                );
            },
        );
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("audit_failed"), "{err}");

        // Maven via inventory: clean with words, denied UNKNOWN without.
        let runner = AuditRunner::clean();
        let (code, out, err) = run_with(
            &["audit", "license", "//third_party/jvm:maven_install"],
            &runner,
            &|harness| {
                harness.write_source(
                    "third_party/jvm/maven_install.json",
                    r#"{"artifacts": {"junit:junit": {"version": "4.13.2"}}}"#,
                );
                harness.write_source(
                    "licenses.toml",
                    "[policy.distributed]\nallow = [\"MIT\"]\nreview = [\"EPL-1.0\"]\ndeny = []\n\n[[exception]]\npackage = \"junit:junit\"\nset = \"maven\"\nlicense = \"EPL-1.0\"\nversions = \"4.13.2\"\nreason = \"Test approval.\"\nexpires = \"2027-03-01\"\n\n[[inventory]]\npackage = \"junit:junit\"\nset = \"maven\"\nlicense = \"EPL-1.0\"\nversions = \"4.13.2\"\ntext_present = true\n",
                );
            },
        );
        // Review still fails distributed without approval; with the
        // exception above plus words it passes via approval.
        assert_eq!(code, 0, "{out}{err} {code}");
        assert!(out.contains("audit license: clean"), "{out}");

        // NuGet via inventory with words: clean.
        let runner = AuditRunner::clean();
        let (code, out, err) = run_with(
            &["audit", "license", "//csharp/tests/fixtures/hello:hello"],
            &runner,
            &|harness| {
                harness.write_source(
                    "third_party/dotnet/paket.lock",
                    "NUGET\n  remote: https://api.nuget.org/v3/index.json\n    FSharp.Core (10.1.201)\n",
                );
                harness.write_source(
                    "licenses.toml",
                    "[policy.distributed]\nallow = [\"MIT\"]\nreview = []\ndeny = []\n\n[[inventory]]\npackage = \"FSharp.Core\"\nset = \"nuget\"\nlicense = \"MIT\"\nversions = \"10.1.201\"\ntext_present = true\n",
                );
            },
        );
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("audit license: clean"), "{out}");

        // Go via inventory with words: clean; without words the BSD
        // notice fails distributed, inventoried internal stays clean.
        let runner = AuditRunner::clean();
        let (code, out, err) = run_with(
            &["audit", "license", "//go/tests/fixtures/hello:hello"],
            &runner,
            &|harness| {
                harness.write_source(
                    "third_party/go/go.mod",
                    "module example.com/root\n\ngo 1.24.12\n\nrequire example.com/hello v1.0.0\n",
                );
                harness.write_source(
                    "licenses.toml",
                    "[policy.distributed]\nallow = [\"BSD-3-Clause\"]\nreview = []\ndeny = []\n\n[[inventory]]\npackage = \"example.com/hello\"\nset = \"go\"\nlicense = \"BSD-3-Clause\"\nversions = \"v1.0.0\"\ntext_present = true\n",
                );
            },
        );
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("audit license: clean"), "{out}");

        // Same Go package without words fails (BSD is allow-listed, so
        // the failure is the notice check firing).
        let runner = AuditRunner::clean();
        let (code, _out, err) = run_with(
            &["audit", "license", "//go/tests/fixtures/hello:hello"],
            &runner,
            &|harness| {
                harness.write_source(
                    "third_party/go/go.mod",
                    "module example.com/root\n\ngo 1.24.12\n\nrequire example.com/hello v1.0.0\n",
                );
                harness.write_source(
                    "licenses.toml",
                    "[policy.distributed]\nallow = [\"BSD-3-Clause\"]\nreview = []\ndeny = []\n\n[[inventory]]\npackage = \"example.com/hello\"\nset = \"go\"\nlicense = \"BSD-3-Clause\"\nversions = \"v1.0.0\"\n",
                );
            },
        );
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("audit_failed"), "{err}");
    }

    #[test]
    fn audit_live_target_scopes_to_owning_set_only() {
        let runner = AuditRunner::clean();
        let (code, out, err) = run_with(
            &["audit", "license", "//go/tests/fixtures/hello:hello"],
            &runner,
            &|harness| {
                write_go_mod(harness);
                // Uninventoried Go licenses stay `UNKNOWN` (fail closed in
                // `distributed`): scope the root internal so the inventory
                // stays clean, like `clean_workspace` does for cargo plus
                // MIT with words.
                harness.write_source(
                    "licenses.toml",
                    "[policy.distributed]\nallow = [\"MIT\"]\nreview = []\ndeny = []\n\n[distribution]\ninternal = [\"//go/tests/fixtures/hello:hello\"]\n",
                );
            },
        );
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("audit license: clean"), "{out}");
        assert_eq!(runner.calls.borrow().len(), 0);
    }

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
            let text =
                std::fs::read_to_string(harness.workspace.join("out.spdx.json")).expect("spdx");
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
}
