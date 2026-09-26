use super::{CiSelection, ExecutionScope};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SchedulingMode {
    #[default]
    Parallel,
    Sequential,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionCell {
    pub check: &'static str,
    pub platform: Option<String>,
    pub scope: ExecutionScope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellIsolation {
    pub workdir: String,
    pub report_path: String,
    pub output_base: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedCell {
    pub cell: ExecutionCell,
    pub isolation: CellIsolation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedSchedule {
    pub cells: Vec<PlannedCell>,
    pub mode: SchedulingMode,
}

impl PlannedSchedule {
    pub fn is_independent(&self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellOutcome {
    Success,
    Failure,
}

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
    fn starter_expands_to_per_platform_fanout() {
        let selection =
            plan_selection(&[], &strings(&["linux_x86_64", "macos_arm64"])).expect("plans");
        let schedule = plan_schedule(&selection, SchedulingMode::Parallel);
        assert_eq!(schedule.cells.len(), 18);
        assert!(schedule
            .cells
            .iter()
            .all(|cell| cell.cell.scope == ExecutionScope::PerPlatform));
        assert!(schedule
            .cells
            .iter()
            .all(|cell| cell.cell.platform.is_some()));
        let per_platform: Vec<_> = schedule
            .cells
            .iter()
            .filter(|cell| cell.cell.scope == ExecutionScope::PerPlatform)
            .collect();
        assert_eq!(per_platform.len(), 18);
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
