use super::{ArgsError, ReportRequest};

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

pub(crate) fn parse_min_coverage(value: &str) -> Result<u32, ArgsError> {
    match value.parse::<u32>() {
        Ok(percent) if percent <= 100 => Ok(percent),
        _ => Err(ArgsError::BadMinCoverage {
            value: value.to_owned(),
        }),
    }
}
