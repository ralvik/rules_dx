//! Reporting and review-thread planning for consumer CI (WP1 slice 5).
//!
//! Split from `super` (`lib.rs`): owns [`Finding`], [`OwnedThread`],
//! [`ThreadPlan`], [`plan_threads`] (failure-contributing findings win new
//! threads in deterministic ID order; existing threads are never deleted
//! or resolved to rotate others into view; full reports retain every
//! finding), [`Summary`], [`plan_summary`], and [`reporting_gate`]
//! (required publication failure fails CI separately; intentionally
//! inapplicable outputs are not failures). Re-exported through `super`
//! so the public paths stay `dx_ci::{Finding, OwnedThread, ThreadPlan,
//! plan_threads, Summary, plan_summary, reporting_gate}`. Distinct from
//! the revision, selection, scheduling, supersession, fork/aggregate,
//! rerun, caller, pin, audit, artifact, metadata, and preset modules.

/// One diff-mappable finding with a stable identity.
///
/// `location` is `Some` only for findings with a valid PR-diff location;
/// locationless findings stay in full reports and summary counts and never
/// receive invented review locations or inline annotations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    /// Stable finding identity across reruns (check + rule + path digest).
    pub id: String,
    /// Whether this finding contributes to its check's failure.
    pub contributes_to_failure: bool,
    /// Valid PR-diff location, if mappable.
    pub location: Option<String>,
}

/// One integration-owned review thread.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedThread {
    /// Finding this thread reports.
    pub finding: String,
    /// Whether a human has replied (replies pin resolve-over-delete).
    pub has_human_replies: bool,
}

/// Planned thread updates for one PR assessment.
///
/// Presentation is fixed: review threads for diff-mapped findings plus one
/// integration-owned summary; there is no review-comment opt-out or
/// annotation-only mode, and findings already shown in threads never gain
/// duplicate inline annotations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreadPlan {
    /// Threads to keep as-is (finding still present, no repost).
    pub keep: Vec<String>,
    /// Finding IDs to open new threads for (deterministic priority order).
    pub create: Vec<String>,
    /// Bot-only threads without replies whose finding is confirmed gone.
    pub delete: Vec<String>,
    /// Threads with human replies whose finding is confirmed gone.
    pub resolve: Vec<String>,
    /// Diff-mapped findings omitted only by the fixed per-PR limit.
    pub omitted_due_to_limit: Vec<String>,
    /// Findings without a valid diff location (reports/counts only).
    pub unmappable: Vec<String>,
}

/// Plan review-thread updates.
///
/// `findings` are the current complete assessment's findings;
/// `existing` are the integration-owned threads; `confirmed_gone` are
/// finding IDs a later complete assessment of the relevant check and scope
/// confirmed absent (skipped, disabled, cancelled, or incomplete checks,
/// missing reports, and locations moving outside the diff never confirm
/// absence — callers must not list them here). `limit` is the fixed
/// per-PR thread cap (numeric value frozen elsewhere): failure-contributing
/// findings win new threads in deterministic ID order, existing threads are
/// never deleted or resolved to rotate others into view, and full reports
/// retain every finding. Intentional truncation is not a failure.
pub fn plan_threads(
    findings: &[Finding],
    existing: &[OwnedThread],
    confirmed_gone: &[String],
    limit: usize,
) -> ThreadPlan {
    let present: std::collections::BTreeSet<&str> =
        findings.iter().map(|finding| finding.id.as_str()).collect();
    let existing_ids: std::collections::BTreeSet<&str> = existing
        .iter()
        .map(|thread| thread.finding.as_str())
        .collect();
    let mut keep = Vec::new();
    for thread in existing {
        if present.contains(thread.finding.as_str()) {
            keep.push(thread.finding.clone());
        }
    }
    let mut delete = Vec::new();
    let mut resolve = Vec::new();
    for thread in existing {
        if present.contains(thread.finding.as_str()) {
            continue;
        }
        if !confirmed_gone.iter().any(|id| id == &thread.finding) {
            continue;
        }
        if thread.has_human_replies {
            resolve.push(thread.finding.clone());
        } else {
            delete.push(thread.finding.clone());
        }
    }
    keep.sort();
    delete.sort();
    resolve.sort();
    let mut candidates: Vec<&Finding> = findings
        .iter()
        .filter(|finding| finding.location.is_some())
        .filter(|finding| !existing_ids.contains(finding.id.as_str()))
        .collect();
    candidates.sort_by(|a, b| {
        b.contributes_to_failure
            .cmp(&a.contributes_to_failure)
            .then_with(|| a.id.cmp(&b.id))
    });
    let used = keep.len();
    let budget = limit.saturating_sub(used.min(limit));
    let mut create = Vec::new();
    let mut omitted_due_to_limit = Vec::new();
    for finding in candidates {
        if create.len() < budget {
            create.push(finding.id.clone());
        } else {
            omitted_due_to_limit.push(finding.id.clone());
        }
    }
    let mut unmappable: Vec<String> = findings
        .iter()
        .filter(|finding| finding.location.is_none())
        .map(|finding| finding.id.clone())
        .collect();
    unmappable.sort();
    ThreadPlan {
        keep,
        create,
        delete,
        resolve,
        omitted_due_to_limit,
        unmappable,
    }
}

/// Compact per-PR summary (one integration-owned updated comment).
///
/// Carries every selected check's status, truthful finding counts, the
/// analyzed revision/run identity, and completeness — never the exhaustive
/// finding list. Disabled, blocked, skipped, cancelled, or incomplete
/// results are never shown as passed. Retries and concurrent completions
/// must neither duplicate the summary nor let older results replace newer
/// ones (see [`super::may_publish`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Summary {
    /// Analyzed snapshot identity.
    pub validated: String,
    /// Total findings retained in full reports.
    pub finding_count: usize,
    /// Findings omitted only by the fixed per-PR thread limit.
    pub limit_omitted: usize,
    /// Findings without a valid diff location.
    pub unmappable: usize,
}

/// Build the compact summary counts.
///
/// `full_findings` is every finding in the detailed reports; limit-omitted
/// and unmappable counts come from [`plan_threads`].
pub fn plan_summary(validated: &str, full_findings: usize, plan: &ThreadPlan) -> Summary {
    Summary {
        validated: validated.to_owned(),
        finding_count: full_findings,
        limit_omitted: plan.omitted_due_to_limit.len(),
        unmappable: plan.unmappable.len(),
    }
}

/// Reporting gate: required publication failure fails CI separately.
///
/// Passing analysis with failed required check, review-comment, or summary
/// publication is not success; command outcomes are preserved alongside the
/// reporting failure. Intentionally inapplicable outputs (for example PR
/// comments for queue runs) are not failures — callers pass
/// `reporting_required: false` there.
pub fn reporting_gate(
    analysis_passed: bool,
    reporting_required: bool,
    reporting_succeeded: bool,
) -> bool {
    if reporting_required && !reporting_succeeded {
        return false;
    }
    analysis_passed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(id: &str, failure: bool, located: bool) -> Finding {
        Finding {
            id: id.to_owned(),
            contributes_to_failure: failure,
            location: if located {
                Some(format!("file.rs:{id}"))
            } else {
                None
            },
        }
    }

    #[test]
    fn unchanged_findings_keep_threads_without_repost() {
        let findings = vec![finding("f1", true, true)];
        let existing = vec![OwnedThread {
            finding: "f1".to_owned(),
            has_human_replies: false,
        }];
        let plan = plan_threads(&findings, &existing, &[], 10);
        assert_eq!(plan.keep, vec!["f1".to_owned()]);
        assert!(plan.create.is_empty());
    }

    #[test]
    fn unmappable_findings_never_gain_threads() {
        let findings = vec![finding("f1", true, false)];
        let plan = plan_threads(&findings, &[], &[], 10);
        assert!(plan.create.is_empty());
        assert_eq!(plan.unmappable, vec!["f1".to_owned()]);
        let summary = plan_summary("merge-a", 1, &plan);
        assert_eq!(summary.unmappable, 1);
        assert_eq!(summary.finding_count, 1);
    }

    #[test]
    fn failure_contributors_win_limited_threads_deterministically() {
        let findings = vec![
            finding("b-non-failing", false, true),
            finding("a-failing", true, true),
            finding("c-failing", true, true),
        ];
        let plan = plan_threads(&findings, &[], &[], 2);
        assert_eq!(
            plan.create,
            vec!["a-failing".to_owned(), "c-failing".to_owned()]
        );
        assert_eq!(plan.omitted_due_to_limit, vec!["b-non-failing".to_owned()]);
        let rerun = plan_threads(&findings, &[], &[], 2);
        assert_eq!(plan, rerun);
    }

    #[test]
    fn existing_threads_never_rotate_for_new_findings() {
        let existing = vec![OwnedThread {
            finding: "old".to_owned(),
            has_human_replies: false,
        }];
        let findings = vec![finding("old", true, true), finding("new", true, true)];
        let plan = plan_threads(&findings, &existing, &[], 1);
        assert_eq!(plan.keep, vec!["old".to_owned()]);
        assert!(plan.create.is_empty());
        assert_eq!(plan.omitted_due_to_limit, vec!["new".to_owned()]);
    }

    #[test]
    fn confirmed_gone_deletes_bot_only_and_resolves_replied() {
        let existing = vec![
            OwnedThread {
                finding: "gone-quiet".to_owned(),
                has_human_replies: false,
            },
            OwnedThread {
                finding: "gone-discussed".to_owned(),
                has_human_replies: true,
            },
            OwnedThread {
                finding: "unconfirmed".to_owned(),
                has_human_replies: false,
            },
        ];
        let findings = vec![];
        let confirmed = vec!["gone-quiet".to_owned(), "gone-discussed".to_owned()];
        let plan = plan_threads(&findings, &existing, &confirmed, 10);
        assert_eq!(plan.delete, vec!["gone-quiet".to_owned()]);
        assert_eq!(plan.resolve, vec!["gone-discussed".to_owned()]);
        assert!(plan.keep.is_empty());
    }

    #[test]
    fn reporting_failure_fails_ci_despite_passing_analysis() {
        assert!(reporting_gate(true, true, true));
        assert!(!reporting_gate(true, true, false));
        assert!(!reporting_gate(false, true, true));
        // Intentionally inapplicable outputs (queue-run PR comments) are
        // not failures.
        assert!(reporting_gate(true, false, false));
    }
}
