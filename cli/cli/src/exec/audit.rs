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
    // Issue #628 (See: `docs/cli/commands/audit-update-bazel.md#dx-audit`): never empty clean. A missing, empty, invalid, or stale
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
    let tool_path = match runner.gitleaks_tool() {
        Some(path) => path.to_string_lossy().into_owned(),
        None => {
            return (
                Vec::new(),
                Some(
                    "secrets auditor unavailable: set DX_GITLEAKS_BIN to the pinned @dx_tools//:gitleaks artifact; ambient PATH lookup is rejected"
                        .to_owned(),
                ),
                Vec::new(),
            );
        }
    };
    let temp_arg = temp_dir.to_string_lossy().into_owned();
    let plan = match dx_audit::backend::plan_secrets(&tool_path, &report_arg, config, &temp_arg) {
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
    let status = match runner.run_hermetic(&argv, workspace, &env_refs) {
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

#[path = "audit_license.rs"]
mod audit_license;
use audit_license::*;

#[cfg(test)]
#[path = "audit_tests_a.rs"]
mod audit_tests_a;
#[cfg(test)]
#[path = "audit_tests_b.rs"]
mod audit_tests_b;
