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
    format!(".dx/advisory/{}.json", set.name())
}

fn load_advisories(
    workspace: &Path,
    set: dx_update::sets::SetId,
) -> Result<Vec<dx_audit::vuln::Advisory>, String> {
    let rel = advisory_path(set);
    let full = workspace.join(&rel);
    if !full.is_file() {
        return Ok(Vec::new());
    }
    match read_workspace_text(workspace, &rel) {
        Err(detail) => Err(detail),
        Ok(text) => {
            if text.trim().is_empty() {
                return Ok(Vec::new());
            }
            dx_audit::vuln::parse_snapshot(&text)
        }
    }
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
            missing.first().cloned().unwrap_or_else(|| "pnpm-lock.yaml".to_owned())
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
        let advisories = match load_advisories(workspace, *set) {
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
    let tier = tier_for_roots(&policy, roots);
    let mut finding_count: usize = 0;
    let mut diagnostics: Vec<DiagnosticEvent> = Vec::new();
    let mut packages_all: Vec<dx_audit::spdx::SpdxPackage> = Vec::new();
    let mut incomplete: Option<String> = None;
    let mut licensed_all: Vec<dx_audit::locks::LicensedPackage> = Vec::new();
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
        let licensed = match *set {
            dx_update::sets::SetId::Cargo => {
                let bazel_text =
                    read_workspace_text(workspace, "cargo-bazel-lock.json").unwrap_or_default();
                dx_audit::locks::cargo_licenses(&bazel_text, &locked)
            }
            _ => dx_audit::locks::unknown_licenses(&locked, set.name()),
        };
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
            if exception.package != licensed.name || exception.set != licensed.set {
                continue;
            }
            let names_match = exception.license == licensed.license;
            if !names_match {
                continue;
            }
            if dx_audit::license_policy::validate_license_exception(exception, today).is_err() {
                continue;
            }
            if !dx_audit::vuln::version_affected(
                &licensed.set,
                &exception.versions,
                &licensed.version,
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
        let sarif = r#"{"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "gitleaks"}}, "results": [{"ruleId": "gitleaks/generic-api-key", "message": {"text": "Generic API Key"}, "locations": [{"physicalLocation": {"artifactLocation": {"uri": "src/app.py"}}}]}]}]}"#;
        let runner = AuditRunner::with_sarif(Some(1), sarif);
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
        });
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("audit_failed"), "{err}");
        assert!(err.contains("audit security"), "{err}");
        assert!(!out.contains("AKIA"), "{out}");
        assert!(!err.contains("AKIA"), "{err}");
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
                harness.write_source(
                    ".dx/advisory/go.json",
                    r#"[{"id":"GHSA-go-test-0001","package":"github.com/google/go-cmp","versions":">=v0.5.0, <v0.7.0","severity":"high","fixed":["v0.7.0"],"set":"go"}]"#,
                );
            },
        );
        assert_eq!(code, 1, "{err}");
        assert!(err.contains("audit_failed"), "{err}");
        assert!(err.contains("1 vulnerability findings"), "{err}");
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
            "{out}{err} clean cargo but UNKNOWN npm must fail distributed"
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
                    "[policy.distributed]\nallow = [\"MIT\"]\nreview = []\ndeny = []\n",
                );
            },
        );
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.contains("audit license: clean"), "{out}");
    }

    #[test]
    fn audit_live_target_scopes_to_owning_set_only() {
        let runner = AuditRunner::clean();
        let (code, out, err) = run_with(
            &["audit", "license", "//go/tests/fixtures/hello:hello"],
            &runner,
            &|harness| {
                write_go_mod(harness);
                // The Go set reports `UNKNOWN` licenses (V1, pending
                // per-ecosystem qualification): scope the root internal so
                // the inventory stays clean, like `clean_workspace` does
                // for cargo plus MIT.
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
            "[policy.distributed]\nallow = [\"MIT\"]\nreview = []\ndeny = []\n",
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
        let sarif = std::fs::read_to_string(harness.workspace.join("out.sarif")).expect("sarif");
        let value: serde_json::Value = serde_json::from_str(&sarif).expect("sarif JSON");
        assert_eq!(value["version"], serde_json::json!("2.1.0"));
        let spdx = std::fs::read_to_string(harness.workspace.join("out.spdx.json")).expect("spdx");
        let spdx_value: serde_json::Value = serde_json::from_str(&spdx).expect("spdx JSON");
        assert_eq!(spdx_value["spdxVersion"], serde_json::json!("SPDX-2.3"));
    }
}
