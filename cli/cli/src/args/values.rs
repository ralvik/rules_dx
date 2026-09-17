//! Small value helpers for invocation parsing (issue #236): scope-shape
//! errors plus `--report` and `--min-coverage` value parsing.
//!
//! Extracted from [`super::parser`] without behavior change: the
//! parsing root still owns the full `parse` validation (scope shapes,
//! per-command option ownership, output-contract gates, profile
//! flags); this module owns only the pure value mappings it calls.

use super::{ArgsError, ReportRequest};

/// Maps one scope positional onto its shape-specific parse failure:
/// empty scopes name the repository-wide default, package-relative
/// labels name the `//` qualification, and anything else keeps the
/// generic label/path guidance (typo paths and `@` scopes fail later
/// in resolution with `PathNotFound`/`ExternalScope` context).
pub(crate) fn scope_error(scope: &str) -> ArgsError {
    if scope.is_empty() {
        ArgsError::EmptyScope
    } else if scope.starts_with(':') {
        ArgsError::RelativeLabel {
            scope: scope.to_owned(),
        }
    } else {
        ArgsError::InvalidScope {
            scope: scope.to_owned(),
        }
    }
}

/// Parses one `--report` value into its `format=destination` shape.
pub(crate) fn parse_report(value: &str) -> Result<ReportRequest, ArgsError> {
    match value.split_once('=') {
        Some((format, destination)) if !format.is_empty() && !destination.is_empty() => {
            Ok(ReportRequest {
                format: format.to_owned(),
                destination: destination.to_owned(),
            })
        }
        _ => Err(ArgsError::BadReport {
            value: value.to_owned(),
        }),
    }
}

/// Parses a `--min-coverage` value into an integer percent 0-100.
pub(crate) fn parse_min_coverage(value: &str) -> Result<u32, ArgsError> {
    match value.parse::<u32>() {
        Ok(percent) if percent <= 100 => Ok(percent),
        _ => Err(ArgsError::BadMinCoverage {
            value: value.to_owned(),
        }),
    }
}
