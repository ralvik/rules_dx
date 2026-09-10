//! Real-tool finding model shared by the M04 initial adapters.
//!
//! Each adapter parses its tool's check output into [`Finding`] values over
//! tool-native 1-based line/column positions, then [`place_finding`]
//! converts those positions into the half-open UTF-8 byte ranges the frozen
//! result schema requires. Findings without a tool-given extent become
//! point ranges at the reported position: the range locates the finding,
//! the message describes it. Nothing here spawns processes; execution lives
//! in `exec`, command shapes in `commands`, per-tool grammars in `parsers`.

use quality_result::proto::{Diagnostic, Severity};

pub mod commands;
pub mod exec;
pub mod parsers;
pub mod suggest;

/// Tool-native severity. Maps onto the frozen [`Severity`] one to one;
/// unknown tool severities are a parse error, never a silent downgrade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSeverity {
    Warning,
    Error,
    Info,
}

impl ToolSeverity {
    /// Converts to the frozen result-schema severity.
    pub fn proto(self) -> i32 {
        match self {
            ToolSeverity::Warning => Severity::Warning as i32,
            ToolSeverity::Error => Severity::Error as i32,
            ToolSeverity::Info => Severity::Info as i32,
        }
    }
}

/// One-based line/column position as reported by a tool. Columns count
/// Unicode scalar values, matching the probed Buildifier and Clippy
/// behavior on non-ASCII lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPosition {
    pub line: u64,
    pub column: u64,
}

/// One byte-range replacement against the exact bytes the tool checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub start: u64,
    pub end: u64,
    pub replacement: Vec<u8>,
}

/// One parsed tool finding before range placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub tool_id: String,
    pub rule_id: String,
    pub message: String,
    pub severity: ToolSeverity,
    pub start: TextPosition,
    /// End position when the tool reports an extent; [`None`] places a
    /// point range at `start`.
    pub end: Option<TextPosition>,
    /// Machine-applicable replacements offered by the tool, all against
    /// the checked bytes of the finding's file.
    pub suggestions: Vec<Suggestion>,
}

/// Placement failure. Every variant is an action failure, never a skipped
/// finding: positions come from the bytes just checked, so an unmappable
/// position means the adapter, not the source, is broken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceError {
    UnmappablePosition { line: u64, column: u64 },
    InvertedRange,
}

impl std::fmt::Display for PlaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlaceError::UnmappablePosition { line, column } => {
                write!(f, "position {line}:{column} is outside the checked bytes")
            }
            PlaceError::InvertedRange => write!(f, "end position precedes start position"),
        }
    }
}

impl std::error::Error for PlaceError {}

/// Converts a 1-based line/column position to a UTF-8 byte offset into
/// `text`. A column one past the last character (the newline or end of
/// file) is valid; anything further out returns [`None`].
pub fn line_col_to_byte(text: &str, line: u64, column: u64) -> Option<u64> {
    if line == 0 || column == 0 {
        return None;
    }
    let mut offset = 0u64;
    for (index, content) in text.split_inclusive('\n').enumerate() {
        if index as u64 + 1 == line {
            let body = content.strip_suffix('\n').unwrap_or(content);
            let mut byte = 0u64;
            let mut chars = 0u64;
            for ch in body.chars() {
                if chars + 1 == column {
                    return Some(offset + byte);
                }
                byte += ch.len_utf8() as u64;
                chars += 1;
            }
            if chars + 1 == column {
                return Some(offset + byte);
            }
            return None;
        }
        offset += content.len() as u64;
    }
    None
}

/// Places a parsed finding onto exact file bytes as a normalized
/// [`Diagnostic`] with `fixable` cleared. Range presence is guaranteed;
/// the convergence pass marks fixability, never the parsers.
pub fn place_finding(finding: &Finding, path: &str, text: &str) -> Result<Diagnostic, PlaceError> {
    let start = line_col_to_byte(text, finding.start.line, finding.start.column).ok_or(
        PlaceError::UnmappablePosition {
            line: finding.start.line,
            column: finding.start.column,
        },
    )?;
    let end = match finding.end {
        Some(position) => line_col_to_byte(text, position.line, position.column).ok_or(
            PlaceError::UnmappablePosition {
                line: position.line,
                column: position.column,
            },
        )?,
        None => start,
    };
    if end < start {
        return Err(PlaceError::InvertedRange);
    }
    Ok(Diagnostic {
        severity: finding.severity.proto(),
        message: finding.message.clone(),
        tool_id: finding.tool_id.clone(),
        rule_id: finding.rule_id.clone(),
        path: path.to_owned(),
        start_byte: Some(start),
        end_byte: Some(end),
        fixable: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAFE: &str = "fn main() {\n    let caf\u{e9} = 1;\n}\n";

    #[test]
    fn line_col_covers_ascii_positions() {
        assert_eq!(line_col_to_byte("ab\ncd\n", 1, 1), Some(0));
        assert_eq!(line_col_to_byte("ab\ncd\n", 1, 3), Some(2));
        assert_eq!(line_col_to_byte("ab\ncd\n", 2, 1), Some(3));
        assert_eq!(line_col_to_byte("ab\ncd\n", 2, 3), Some(5));
    }

    #[test]
    fn line_col_counts_characters_not_bytes() {
        // Line 2 is `    let caf\u{e9} = 1;`: `c` sits at character
        // column 9 but byte offset 20, and `\u{e9}` at column 12 but
        // byte offset 23.
        assert_eq!(line_col_to_byte(CAFE, 2, 9), Some(20));
        assert_eq!(line_col_to_byte(CAFE, 2, 12), Some(23));
    }

    #[test]
    fn line_col_rejects_zero_and_out_of_range() {
        assert_eq!(line_col_to_byte(CAFE, 0, 1), None);
        assert_eq!(line_col_to_byte(CAFE, 1, 0), None);
        assert_eq!(line_col_to_byte(CAFE, 4, 1), None);
        assert_eq!(line_col_to_byte(CAFE, 2, 99), None);
    }

    #[test]
    fn place_finding_builds_point_and_covering_ranges() {
        let point = Finding {
            tool_id: "rustfmt".to_owned(),
            rule_id: String::new(),
            message: "file is not formatted".to_owned(),
            severity: ToolSeverity::Warning,
            start: TextPosition { line: 2, column: 1 },
            end: None,
            suggestions: Vec::new(),
        };
        let placed = place_finding(&point, "src/main.rs", CAFE).expect("mappable");
        assert_eq!(placed.start_byte, Some(12));
        assert_eq!(placed.end_byte, Some(12));
        assert_eq!(placed.severity, Severity::Warning as i32);
        assert!(!placed.fixable);

        let covering = Finding {
            end: Some(TextPosition {
                line: 2,
                column: 12,
            }),
            ..point
        };
        let placed = place_finding(&covering, "src/main.rs", CAFE).expect("mappable");
        assert_eq!((placed.start_byte, placed.end_byte), (Some(12), Some(23)));
    }

    #[test]
    fn severity_and_error_display_are_stable() {
        assert_eq!(ToolSeverity::Warning.proto(), Severity::Warning as i32);
        assert_eq!(ToolSeverity::Error.proto(), Severity::Error as i32);
        assert_eq!(ToolSeverity::Info.proto(), Severity::Info as i32);
        assert_eq!(
            PlaceError::UnmappablePosition { line: 9, column: 1 }.to_string(),
            "position 9:1 is outside the checked bytes"
        );
        assert_eq!(
            PlaceError::InvertedRange.to_string(),
            "end position precedes start position"
        );
    }

    #[test]
    fn place_finding_rejects_bad_positions() {
        let base = Finding {
            tool_id: "taplo".to_owned(),
            rule_id: String::new(),
            message: "expected value".to_owned(),
            severity: ToolSeverity::Error,
            start: TextPosition { line: 9, column: 1 },
            end: None,
            suggestions: Vec::new(),
        };
        assert_eq!(
            place_finding(&base, "a.toml", CAFE),
            Err(PlaceError::UnmappablePosition { line: 9, column: 1 })
        );
        let inverted = Finding {
            start: TextPosition { line: 2, column: 5 },
            end: Some(TextPosition { line: 2, column: 3 }),
            ..base.clone()
        };
        assert_eq!(
            place_finding(&inverted, "a.toml", CAFE),
            Err(PlaceError::InvertedRange)
        );
        let bad_end = Finding {
            start: TextPosition { line: 1, column: 1 },
            end: Some(TextPosition { line: 9, column: 1 }),
            ..base.clone()
        };
        assert_eq!(
            place_finding(&bad_end, "a.toml", CAFE),
            Err(PlaceError::UnmappablePosition { line: 9, column: 1 })
        );
    }
}
