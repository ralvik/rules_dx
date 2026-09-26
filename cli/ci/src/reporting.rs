#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    pub id: String,
    pub contributes_to_failure: bool,
    pub location: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedThread {
    pub finding: String,
    pub has_human_replies: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreadPlan {
    pub keep: Vec<String>,
    pub create: Vec<String>,
    pub delete: Vec<String>,
    pub resolve: Vec<String>,
    pub omitted_due_to_limit: Vec<String>,
    pub unmappable: Vec<String>,
}

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Summary {
    pub validated: String,
    pub finding_count: usize,
    pub limit_omitted: usize,
    pub unmappable: usize,
}

pub fn plan_summary(validated: &str, full_findings: usize, plan: &ThreadPlan) -> Summary {
    Summary {
        validated: validated.to_owned(),
        finding_count: full_findings,
        limit_omitted: plan.omitted_due_to_limit.len(),
        unmappable: plan.unmappable.len(),
    }
}

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
