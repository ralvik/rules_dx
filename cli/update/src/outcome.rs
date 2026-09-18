//! Per-set update outcome aggregation (issue #19).
//!
//! Pure planning for the accepted continuation policy in the update
//! contract (`docs/cli/commands/audit-update-bazel.md`): failure in one
//! dependency set does not stop updates to independent selected sets.
//! Successful changes are preserved, each failure is reported, and the
//! run returns an overall failure status when any selected update fails.
//! Operations depending on a failed update are reported as blocked, not
//! run and never reported successful. This is not a repository-wide
//! transaction or rollback.
//!
//! Independence follows the approved upstream integration, not label
//! distinctness: sets sharing a lockfile or resolver workspace cannot be
//! treated as independent merely because they have different Bazel
//! labels. Set identity lives in [`super::sets`] and backend operation
//! boundaries in [`super::backend`]; this module aggregates over injected
//! per-set results and an injected depends-on relation only, so outcome
//! combination stays deterministic and unit-testable without any updater.
//!
//! Out of scope here: parallel-execution scheduling. Aggregate exit-status
//! selection over these reports lives in [`super::report`]. Continued
//! updates imply no parallelism and no new mutation-event API.

use std::collections::{BTreeMap, BTreeSet};

/// Terminal result of one selected dependency set's update attempt, as
/// reported by the upstream integration. `Blocked` is never an input:
/// the planner derives it for dependents of failed sets that were not
/// attempted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetStatus {
    /// The set updated; its changes are preserved.
    Success,
    /// The set failed; its failure is reported and the run fails overall.
    Failed,
}

/// One selected set's reported result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetOutcome {
    /// Owning dependency-set identity (selector spelling, verbatim).
    pub set: String,
    /// Reported terminal status (success or failure only).
    pub status: SetStatus,
}

/// Aggregated per-set report: attempted results plus planner-derived
/// blocked entries for unattempted dependents of failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReportedStatus {
    /// Attempted and updated; changes preserved.
    Success,
    /// Attempted and failed; reported, fails the run overall.
    Failed,
    /// Not attempted because a dependency failed; reported as blocked,
    /// never as successful.
    Blocked,
}

/// One entry in the aggregated report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportedOutcome {
    /// Dependency-set identity.
    pub set: String,
    /// Aggregated status, including derived `Blocked`.
    pub status: ReportedStatus,
}

/// Aggregated update run: per-set report plus the overall failure flag.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateReport {
    /// One entry per selected set, in sorted set order for determinism.
    pub outcomes: Vec<ReportedOutcome>,
    /// True when any selected update failed (blocked alone never sets
    /// this; only an actual failure does, and any failure implies it).
    pub overall_failure: bool,
}

impl UpdateReport {
    /// Preserved successes: sets that updated and keep their changes.
    pub fn successes(&self) -> Vec<String> {
        self.outcomes
            .iter()
            .filter(|outcome| outcome.status == ReportedStatus::Success)
            .map(|outcome| outcome.set.clone())
            .collect()
    }

    /// Reported failures.
    pub fn failures(&self) -> Vec<String> {
        self.outcomes
            .iter()
            .filter(|outcome| outcome.status == ReportedStatus::Failed)
            .map(|outcome| outcome.set.clone())
            .collect()
    }

    /// Dependents reported as blocked without running.
    pub fn blocked(&self) -> Vec<String> {
        self.outcomes
            .iter()
            .filter(|outcome| outcome.status == ReportedStatus::Blocked)
            .map(|outcome| outcome.set.clone())
            .collect()
    }
}

/// Aggregation failures. Missing results are a caller error, never a
/// silent success: the planner refuses to guess an outcome the upstream
/// integration did not report.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum AggregateError {
    /// A selected set has no attempted result and no failed dependency
    /// to explain the gap. Dependents of failures report `Blocked`
    /// instead; only an unexplained gap errors.
    #[error("update set {set:?} has no reported result; refusing to guess")]
    MissingResult { set: String },
}

/// Aggregate one update run.
///
/// * `selected` lists every selected set (sorted output follows this
///   membership, deduplicated).
/// * `results` carries the attempted per-set outcomes. Sets with no
///   result that depend (transitively) on a failed set are reported as
///   `Blocked` and never run; a selected set with no result and no
///   failed dependency is an [`AggregateError::MissingResult`] caller
///   error rather than a guessed success.
/// * `depends_on` maps each set to the sets it directly depends on
///   (operation ordering under the upstream integration). Only
///   failed-ancestor propagation is computed here; scheduling and
///   parallelism are explicitly out of scope.
///
/// Independent failures never remove other sets' results: every
/// attempted outcome is preserved verbatim in the report.
pub fn aggregate(
    selected: &[String],
    results: &[SetOutcome],
    depends_on: &BTreeMap<String, Vec<String>>,
) -> Result<UpdateReport, AggregateError> {
    let mut selected_sets = BTreeSet::new();
    for set in selected {
        selected_sets.insert(set.clone());
    }
    let mut reported: BTreeMap<String, ReportedStatus> = BTreeMap::new();
    for result in results {
        if !selected_sets.contains(&result.set) {
            continue;
        }
        let status = match result.status {
            SetStatus::Success => ReportedStatus::Success,
            SetStatus::Failed => ReportedStatus::Failed,
        };
        reported.insert(result.set.clone(), status);
    }
    let failed: BTreeSet<String> = reported
        .iter()
        .filter_map(|(set, status)| {
            if *status == ReportedStatus::Failed {
                Some(set.clone())
            } else {
                None
            }
        })
        .collect();
    // Selected sets with no attempted result are fail-closed. Dependents
    // of failures report `Blocked` and must not run; an unexplained gap
    // errors so missing evidence can never read as clean.
    for set in &selected_sets {
        if reported.contains_key(set) {
            continue;
        }
        if depends_on_failed(set, depends_on, &failed) {
            reported.insert(set.clone(), ReportedStatus::Blocked);
        } else {
            debug_assert!(results_missing(set, results));
            return Err(AggregateError::MissingResult { set: set.clone() });
        }
    }
    let overall_failure = reported
        .values()
        .any(|status| *status == ReportedStatus::Failed);
    let mut outcomes: Vec<ReportedOutcome> = reported
        .into_iter()
        .map(|(set, status)| ReportedOutcome { set, status })
        .collect();
    outcomes.sort_by(|left, right| left.set.cmp(&right.set));
    Ok(UpdateReport {
        outcomes,
        overall_failure,
    })
}

/// True when any transitive dependency of `set` (through `depends_on`)
/// is in `failed`. Cycles terminate via the visited set and count as
/// depending on the failure only when the failure is actually reached.
/// Public so callers can explain *why* an entry reports blocked.
pub fn depends_on_failed(
    set: &str,
    depends_on: &BTreeMap<String, Vec<String>>,
    failed: &BTreeSet<String>,
) -> bool {
    let mut visited = BTreeSet::new();
    let mut stack: Vec<String> = depends_on.get(set).cloned().unwrap_or_default();
    while let Some(next) = stack.pop() {
        if !visited.insert(next.clone()) {
            continue;
        }
        if failed.contains(&next) {
            return true;
        }
        if let Some(parents) = depends_on.get(&next) {
            stack.extend(parents.iter().cloned());
        }
    }
    false
}

/// Selected sets surface as blocked (never successful) when the caller
/// supplies no result for them; this keeps missing evidence fail-closed.
/// Public so callers can distinguish a missing result from a
/// dependency-failure block.
pub fn results_missing(set: &str, results: &[SetOutcome]) -> bool {
    !results.iter().any(|result| result.set == set)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sets(words: &[&str]) -> Vec<String> {
        words.iter().copied().map(str::to_owned).collect()
    }

    fn outcomes(pairs: &[(&str, SetStatus)]) -> Vec<SetOutcome> {
        pairs
            .iter()
            .map(|(set, status)| SetOutcome {
                set: (*set).to_owned(),
                status: *status,
            })
            .collect()
    }

    fn empty_deps() -> BTreeMap<String, Vec<String>> {
        BTreeMap::new()
    }

    #[test]
    fn independent_failure_preserves_success_and_fails_overall() {
        let report = aggregate(
            &sets(&["cargo-lock", "npm-root"]),
            &outcomes(&[
                ("cargo-lock", SetStatus::Success),
                ("npm-root", SetStatus::Failed),
            ]),
            &empty_deps(),
        )
        .expect("complete results aggregate");
        assert!(report.overall_failure);
        assert_eq!(report.successes(), vec!["cargo-lock".to_owned()]);
        assert_eq!(report.failures(), vec!["npm-root".to_owned()]);
        assert!(report.blocked().is_empty());
    }

    #[test]
    fn all_success_passes_overall() {
        let report = aggregate(
            &sets(&["cargo-lock", "npm-root"]),
            &outcomes(&[
                ("cargo-lock", SetStatus::Success),
                ("npm-root", SetStatus::Success),
            ]),
            &empty_deps(),
        )
        .expect("complete results aggregate");
        assert!(!report.overall_failure);
        assert!(report.failures().is_empty());
        assert!(report.blocked().is_empty());
    }

    #[test]
    fn dependent_of_failure_reports_blocked_not_successful() {
        let mut deps = BTreeMap::new();
        deps.insert("app-set".to_owned(), vec!["base-set".to_owned()]);
        let report = aggregate(
            &sets(&["base-set", "app-set"]),
            &outcomes(&[("base-set", SetStatus::Failed)]),
            &deps,
        )
        .expect("dependent blocks");
        assert!(report.overall_failure);
        assert_eq!(report.failures(), vec!["base-set".to_owned()]);
        assert_eq!(report.blocked(), vec!["app-set".to_owned()]);
        assert!(report.successes().is_empty());
    }

    #[test]
    fn transitive_dependent_reports_blocked() {
        let mut deps = BTreeMap::new();
        deps.insert("mid".to_owned(), vec!["base".to_owned()]);
        deps.insert("top".to_owned(), vec!["mid".to_owned()]);
        let report = aggregate(
            &sets(&["base", "mid", "top"]),
            &outcomes(&[("base", SetStatus::Failed)]),
            &deps,
        )
        .expect("transitive dependents block");
        assert_eq!(report.failures(), vec!["base".to_owned()]);
        assert_eq!(report.blocked(), vec!["mid".to_owned(), "top".to_owned()]);
    }

    #[test]
    fn unrelated_set_still_attempts_despite_failure_elsewhere() {
        let mut deps = BTreeMap::new();
        deps.insert("app-set".to_owned(), vec!["base-set".to_owned()]);
        let report = aggregate(
            &sets(&["base-set", "app-set", "other-set"]),
            &outcomes(&[
                ("base-set", SetStatus::Failed),
                ("other-set", SetStatus::Success),
            ]),
            &deps,
        )
        .expect("independent set attempts");
        assert!(report.overall_failure);
        assert_eq!(report.successes(), vec!["other-set".to_owned()]);
        assert_eq!(report.blocked(), vec!["app-set".to_owned()]);
    }

    #[test]
    fn missing_result_without_failed_dependency_errors() {
        let error = aggregate(&sets(&["cargo-lock"]), &[], &empty_deps())
            .expect_err("unexplained gap must error");
        assert_eq!(
            error,
            AggregateError::MissingResult {
                set: "cargo-lock".to_owned()
            }
        );
    }

    #[test]
    fn unselected_results_are_ignored() {
        let report = aggregate(
            &sets(&["cargo-lock"]),
            &outcomes(&[
                ("cargo-lock", SetStatus::Success),
                ("elsewhere", SetStatus::Failed),
            ]),
            &empty_deps(),
        )
        .expect("unselected results ignored");
        assert!(!report.overall_failure);
        assert_eq!(report.successes(), vec!["cargo-lock".to_owned()]);
    }

    #[test]
    fn report_order_is_sorted_and_deterministic() {
        let report = aggregate(
            &sets(&["b-set", "a-set"]),
            &outcomes(&[("b-set", SetStatus::Success), ("a-set", SetStatus::Success)]),
            &empty_deps(),
        )
        .expect("complete results aggregate");
        let order: Vec<&str> = report
            .outcomes
            .iter()
            .map(|outcome| outcome.set.as_str())
            .collect();
        assert_eq!(order, vec!["a-set", "b-set"]);
    }
}
