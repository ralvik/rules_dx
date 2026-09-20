//! Quality standard-report writing: SARIF documents over
//! current findings with snapshot line regions, written atomically
//! after validation.
//!
//! Extracted from [`super::quality`] without behavior change: the
//! execution root still owns Bazel launch, result collection, mutation
//! plus status projection ([`super::quality_apply`]), diff-patch
//! rendering ([`super::quality_patch`]), event emission, and exit-code
//! selection; this module owns only standard-report snapshot collection,
//! SARIF rendering, atomic writes, and the `report_failed` diagnostics.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::Path;

use dx_apply::{FileSystem, RealFileSystem};
use dx_digest::blake3 as digest;
use dx_output::{report_event, write_event, DiagnosticEvent, OutputMode};

use super::common::CODE_REPORT_FAILED;
use super::results::Collected;
use crate::reports::{render_sarif, Destination, PlannedReport, ReportError};

/// Shared inputs for one standard-report write pass. Bundled so the
/// entry point stays under the clippy argument limit without changing
/// call-site behavior.
pub(crate) struct StandardReports<'a> {
    pub(crate) workspace: &'a Path,
    pub(crate) collected: &'a Collected,
    pub(crate) status: &'a [DiagnosticEvent],
    pub(crate) planned: &'a [PlannedReport],
    pub(crate) output: &'a OutputMode,
    pub(crate) stdout_report: bool,
}

/// Writes every planned standard report and returns whether all
/// reports succeeded.
///
/// Snapshot bytes are read from `workspace` and validated against the
/// terminal digests before SARIF rendering; missing, stale, or
/// non-UTF-8 snapshots fail that report with `report_failed` while
/// other reports still attempt. File destinations write atomically and
/// require an existing parent directory; stdout destinations own stdout.
/// Human notes follow the output-mode routing (`report_event` in JSON
/// mode, stdout text, stderr for diff).
pub(crate) fn write_standard_reports(
    inputs: StandardReports<'_>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> bool {
    let StandardReports {
        workspace,
        collected,
        status,
        planned,
        output,
        stdout_report,
    } = inputs;
    let fs = RealFileSystem;
    let mut reports_ok = true;
    for planned in planned {
        let mut snapshots = BTreeMap::new();
        let mut snapshot_result: Result<(), ReportError> = Ok(());
        let mut needed: BTreeSet<&str> = BTreeSet::new();
        for finding in status {
            if let Some(path) = &finding.path {
                if finding.range.is_some() {
                    needed.insert(path.as_str());
                }
            } // LCOV_EXCL_LINE - reason: closing brace of a fully covered nesting level carries no executable region of its own.
        }
        for path in needed {
            match std::fs::read(workspace.join(path)) {
                Err(_) => {
                    snapshot_result = Err(ReportError::MissingSnapshot {
                        path: path.to_owned(),
                    });
                    break;
                }
                Ok(bytes) => {
                    if let Some(expected) = collected.terminal_digests.get(path) {
                        if digest(&bytes) != *expected {
                            snapshot_result = Err(ReportError::MissingSnapshot {
                                path: path.to_owned(),
                            });
                            break;
                        }
                    } // LCOV_EXCL_LINE - reason: closing brace of a fully covered guard carries no executable region of its own.
                    match String::from_utf8(bytes) {
                        Ok(text) => {
                            snapshots.insert(path.to_owned(), text);
                        }
                        Err(_) => {
                            snapshot_result = Err(ReportError::MissingSnapshot {
                                path: path.to_owned(),
                            });
                            break;
                        }
                    }
                }
            }
        }
        let document = match snapshot_result {
            Err(error) => Err(error),
            Ok(()) => render_sarif(&collected.tools, status, &snapshots, collected.complete),
        };
        match document {
            Ok(document) => {
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
                        parent_ok && fs.write_atomic(&target, document.as_bytes()).is_ok()
                    }
                };
                if !written {
                    reports_ok = false;
                    let detail = format!(
                        "failed to write {} report to {}",
                        planned.format.name(),
                        planned.destination.display()
                    );
                    let _ = writeln!(err, "dx: report_failed: {detail}");
                    if *output == OutputMode::Json {
                        if let Ok(event) =
                            dx_output::error_event(CODE_REPORT_FAILED, &detail, None, None, None)
                        {
                            let _ = write_event(out, &event);
                        }
                    } // LCOV_EXCL_LINE - reason: closing brace of a fully covered error-reporting guard carries no executable region of its own.
                    continue;
                }
                if *output == OutputMode::Json {
                    if let Ok(event) = report_event(
                        planned.format.name(),
                        planned.destination.display(),
                        collected.complete,
                    ) {
                        let _ = write_event(out, &event);
                    }
                } else if matches!(output, OutputMode::Text { .. }) && !stdout_report {
                    let _ = writeln!(
                        out,
                        "Wrote {} report to {}.",
                        planned.format.name(),
                        planned.destination.display()
                    );
                } else if *output == OutputMode::Diff {
                    let _ = writeln!(
                        err,
                        "Wrote {} report to {}.",
                        planned.format.name(),
                        planned.destination.display()
                    );
                }
            }
            Err(error) => {
                reports_ok = false;
                let detail = format!("failed to render SARIF report: {error}");
                let _ = writeln!(err, "dx: report_failed: {detail}");
                if *output == OutputMode::Json {
                    if let Ok(event) =
                        dx_output::error_event(CODE_REPORT_FAILED, &detail, None, None, None)
                    {
                        let _ = write_event(out, &event);
                    }
                }
            }
        }
    }
    reports_ok
}
