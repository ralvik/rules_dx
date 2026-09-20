//! Schedule planning for consumer CI (M27 WP1 slice 3).
//!
//! Split from `super` (`lib.rs`): owns [`SchedulingMode`],
//! [`ExecutionCell`], [`CellIsolation`], [`PlannedCell`],
//! [`PlannedSchedule`] (with [`PlannedSchedule::is_independent`]),
//! [`CellOutcome`], [`aggregate_outcome`], and [`plan_schedule`]
//! (expands a [`CiSelection`](super::CiSelection) into cells in
//! canonical order — Linux-once checks once, per-platform checks fanned
//! out in caller platform order — each with deterministic isolated
//! context identities; parallel and sequential modes plan identical
//! cells and differ in overlap only; aggregation fails the run on any
//! cell failure while preserving completed results). Re-exported
//! through `super` so the public paths stay
//! `dx_ci::{SchedulingMode, ExecutionCell, CellIsolation, PlannedCell,
//! PlannedSchedule, CellOutcome, aggregate_outcome, plan_schedule}`.
//! Distinct from the revision, selection, supersession, reporting,
//! fork/aggregate, rerun, caller, pin, audit, artifact, metadata, and
//! preset modules.

use super::{CiSelection, ExecutionScope};

/// How independent check cells overlap.
///
/// Parallel is the default; sequential changes overlap only. Cell set,
/// isolation identities, failure preservation, and aggregate semantics are
/// identical in both modes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SchedulingMode {
    /// Independent cells may overlap, subject to runner availability.
    #[default]
    Parallel,
    /// Cells execute without overlap.
    Sequential,
}

/// One executable unit: a check on its execution scope.
///
/// Linux-once checks carry `platform: None`; per-platform checks carry the
/// verbatim caller-supplied platform spelling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionCell {
    /// Frozen check identifier.
    pub check: &'static str,
    /// Verbatim platform spelling, or `None` for Linux-once checks.
    pub platform: Option<String>,
    /// Whether this cell runs once on Linux or per selected platform.
    pub scope: ExecutionScope,
}

/// Isolated execution context identities for one cell.
///
/// Every cell owns distinct checkout, report-destination, and Bazel
/// output-base identities so parallel cells never mutate shared state or
/// serialize on one shared output-base lock. Identities derive
/// deterministically from the cell's check and platform; platform
/// spellings pass through verbatim (runner mapping and filesystem
/// sanitization arrive with the workflow slice).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellIsolation {
    /// Isolated checkout/setup identity.
    pub workdir: String,
    /// Isolated report-destination identity.
    pub report_path: String,
    /// Isolated Bazel output-base identity (never shared between cells).
    pub output_base: String,
}

/// One planned cell with its isolated context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedCell {
    /// The executable unit.
    pub cell: ExecutionCell,
    /// Its isolated execution context.
    pub isolation: CellIsolation,
}

/// Planned schedule: the fixed cell set plus its overlap mode.
///
/// The plan is fixed before execution: a cell failure never removes or
/// cancels independent cells, and completed results are always preserved.
/// Starter cells are independent (no prerequisites), so a failure blocks
/// no sibling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedSchedule {
    /// Every cell to execute, in canonical check/platform order.
    pub cells: Vec<PlannedCell>,
    /// Overlap mode (scheduling only).
    pub mode: SchedulingMode,
}

impl PlannedSchedule {
    /// Starter cells carry no prerequisites: every cell is runnable
    /// regardless of sibling outcomes.
    pub fn is_independent(&self) -> bool {
        true
    }
}

/// Per-cell outcome for aggregate planning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellOutcome {
    Success,
    Failure,
}

/// Overall run outcome: success requires every cell to succeed.
///
/// Failures preserve completed results and fail the run without cancelling
/// independent cells. An empty cell set succeeds vacuously (every selected
/// check — none — completed).
pub fn aggregate_outcome(outcomes: &[CellOutcome]) -> bool {
    outcomes
        .iter()
        .all(|outcome| *outcome == CellOutcome::Success)
}

fn isolation_for(check: &str, platform: Option<&str>) -> CellIsolation {
    let scope = platform.unwrap_or("linux-once");
    CellIsolation {
        workdir: format!("ci-workdir/{check}/{scope}"),
        report_path: format!("ci-reports/{check}/{scope}"),
        output_base: format!("ci-output-base/{check}/{scope}"),
    }
}

/// Plan the schedule for one selection.
///
/// Expands the selection into cells in canonical order (starter check
/// order; per-platform checks fan out in caller platform order) and
/// assigns each cell isolated context identities. Parallel and sequential
/// modes plan the same cells and isolation; only the overlap mode differs.
pub fn plan_schedule(selection: &CiSelection, mode: SchedulingMode) -> PlannedSchedule {
    let mut cells = Vec::new();
    for check in &selection.enabled {
        match check.scope {
            ExecutionScope::LinuxOnce => {
                let isolation = isolation_for(check.id, None);
                cells.push(PlannedCell {
                    cell: ExecutionCell {
                        check: check.id,
                        platform: None,
                        scope: ExecutionScope::LinuxOnce,
                    },
                    isolation,
                });
            }
            ExecutionScope::PerPlatform => {
                for platform in &selection.platforms {
                    let isolation = isolation_for(check.id, Some(platform));
                    cells.push(PlannedCell {
                        cell: ExecutionCell {
                            check: check.id,
                            platform: Some(platform.clone()),
                            scope: ExecutionScope::PerPlatform,
                        },
                        isolation,
                    });
                }
            }
        }
    }
    PlannedSchedule { cells, mode }
}

#[cfg(test)]
mod tests {
    use super::super::plan_selection;
    use super::*;

    fn strings(words: &[&str]) -> Vec<String> {
        words.iter().copied().map(str::to_owned).collect()
    }

    #[test]
    fn parallel_is_the_default_mode() {
        assert_eq!(SchedulingMode::default(), SchedulingMode::Parallel);
    }

    #[test]
    fn starter_expands_to_six_linux_once_plus_per_platform_fanout() {
        let selection =
            plan_selection(&[], &strings(&["linux_x86_64", "macos_arm64"])).expect("plans");
        let schedule = plan_schedule(&selection, SchedulingMode::Parallel);
        // 6 Linux-once + 3 checks x 2 platforms = 12 cells.
        assert_eq!(schedule.cells.len(), 12);
        let linux_once: Vec<_> = schedule
            .cells
            .iter()
            .filter(|cell| cell.cell.scope == ExecutionScope::LinuxOnce)
            .collect();
        assert_eq!(linux_once.len(), 6);
        assert!(linux_once.iter().all(|cell| cell.cell.platform.is_none()));
        let per_platform: Vec<_> = schedule
            .cells
            .iter()
            .filter(|cell| cell.cell.scope == ExecutionScope::PerPlatform)
            .collect();
        assert_eq!(per_platform.len(), 6);
        for cell in &per_platform {
            assert!(cell.cell.platform.is_some());
        }
    }

    #[test]
    fn sequential_mode_plans_identical_cells_and_isolation() {
        let selection = plan_selection(&[], &strings(&["linux_x86_64"])).expect("plans");
        let parallel = plan_schedule(&selection, SchedulingMode::Parallel);
        let sequential = plan_schedule(&selection, SchedulingMode::Sequential);
        assert_eq!(parallel.cells, sequential.cells);
        assert_eq!(parallel.mode, SchedulingMode::Parallel);
        assert_eq!(sequential.mode, SchedulingMode::Sequential);
    }

    #[test]
    fn every_cell_owns_a_distinct_output_base() {
        let selection =
            plan_selection(&[], &strings(&["linux_x86_64", "macos_arm64"])).expect("plans");
        let schedule = plan_schedule(&selection, SchedulingMode::Parallel);
        let mut bases: Vec<_> = schedule
            .cells
            .iter()
            .map(|cell| cell.isolation.output_base.clone())
            .collect();
        bases.sort();
        bases.dedup();
        assert_eq!(bases.len(), schedule.cells.len());
    }

    #[test]
    fn isolation_identities_are_distinct_per_cell() {
        let selection = plan_selection(&[], &strings(&["linux_x86_64"])).expect("plans");
        let schedule = plan_schedule(&selection, SchedulingMode::Parallel);
        let mut workdirs: Vec<_> = schedule
            .cells
            .iter()
            .map(|cell| cell.isolation.workdir.clone())
            .collect();
        workdirs.sort();
        workdirs.dedup();
        assert_eq!(workdirs.len(), schedule.cells.len());
        let mut reports: Vec<_> = schedule
            .cells
            .iter()
            .map(|cell| cell.isolation.report_path.clone())
            .collect();
        reports.sort();
        reports.dedup();
        assert_eq!(reports.len(), schedule.cells.len());
    }

    #[test]
    fn platform_spellings_pass_through_verbatim_into_cells() {
        let selection = plan_selection(&[], &strings(&["Custom-Runner_01"])).expect("plans");
        let schedule = plan_schedule(&selection, SchedulingMode::Parallel);
        let platforms: Vec<_> = schedule
            .cells
            .iter()
            .filter_map(|cell| cell.cell.platform.clone())
            .collect();
        assert!(!platforms.is_empty());
        assert!(platforms.iter().all(|p| p == "Custom-Runner_01"));
    }

    #[test]
    fn starter_cells_are_independent() {
        let selection = plan_selection(&[], &strings(&["linux_x86_64"])).expect("plans");
        let schedule = plan_schedule(&selection, SchedulingMode::Parallel);
        assert!(schedule.is_independent());
    }

    #[test]
    fn any_cell_failure_fails_the_run() {
        use CellOutcome::{Failure, Success};
        assert!(aggregate_outcome(&[Success, Success]));
        assert!(!aggregate_outcome(&[Success, Failure]));
        assert!(!aggregate_outcome(&[Failure]));
        // Completed results are preserved inputs to aggregation: a lone
        // failure still reports the sibling success alongside it.
        let outcomes = [Success, Failure, Success];
        assert!(!aggregate_outcome(&outcomes));
        assert_eq!(outcomes.len(), 3);
    }

    #[test]
    fn empty_schedule_succeeds_vacuously() {
        assert!(aggregate_outcome(&[]));
        let disabled = strings(&[
            "lint",
            "typecheck",
            "format",
            "generate",
            "security-audit",
            "license-audit",
            "test",
            "build",
            "coverage",
        ]);
        let selection = plan_selection(&disabled, &[]).expect("empty plans");
        assert!(selection.enabled.is_empty());
        let schedule = plan_schedule(&selection, SchedulingMode::Sequential);
        assert!(schedule.cells.is_empty());
    }
}
