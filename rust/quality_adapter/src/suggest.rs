//! Machine-applicable suggestion application (Clippy fix mechanism).
//!
//! Clippy offers no in-place fix mode, so the adapter applies
//! `MachineApplicable` suggestion spans to the checked bytes in memory.
//! Application is all or nothing: any out-of-bounds span, any overlap,
//! or a non-UTF-8 result discards every suggestion and the finding stays
//! unfixable. Suggestions never enter results directly; the next
//! convergence round re-checks the patched bytes, so only suggestions
//! that truly resolve their finding can mark it fixable.

use crate::Suggestion;

/// Applies suggestion spans to `base`, returning [`None`] when any span
/// is out of bounds, any two spans overlap, or the result is not UTF-8.
/// Spans apply in total `(start, end, replacement)` order, so the result
/// is deterministic for any input order.
pub fn apply_suggestions(base: &[u8], suggestions: &[Suggestion]) -> Option<Vec<u8>> {
    for suggestion in suggestions {
        if suggestion.start > suggestion.end || suggestion.end > base.len() as u64 {
            return None;
        }
    }
    let mut ordered: Vec<&Suggestion> = suggestions.iter().collect();
    ordered.sort_by(|a, b| (a.start, a.end, &a.replacement).cmp(&(b.start, b.end, &b.replacement)));
    let mut previous_end = 0u64;
    for suggestion in &ordered {
        if suggestion.start < previous_end {
            return None;
        }
        previous_end = suggestion.end;
    }
    let mut out = Vec::with_capacity(base.len());
    let mut cursor = 0u64;
    for suggestion in &ordered {
        out.extend_from_slice(&base[cursor as usize..suggestion.start as usize]);
        out.extend_from_slice(&suggestion.replacement);
        cursor = suggestion.end;
    }
    out.extend_from_slice(&base[cursor as usize..]);
    std::str::from_utf8(&out).ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn suggestion(start: u64, end: u64, replacement: &str) -> Suggestion {
        Suggestion {
            start,
            end,
            replacement: replacement.as_bytes().to_vec(),
        }
    }

    #[test]
    fn applies_non_overlapping_suggestions_in_any_order() {
        let base = b"if x.len() == 0 {";
        let first = suggestion(3, 15, "x.is_empty()");
        let second = suggestion(0, 2, "when");
        let forward = apply_suggestions(base, &[first.clone(), second.clone()]);
        let backward = apply_suggestions(base, &[second, first]);
        assert_eq!(forward, backward);
        assert_eq!(
            forward.expect("valid spans"),
            b"when x.is_empty() {".to_vec()
        );
    }

    #[test]
    fn rejects_overlap_out_of_bounds_and_splits() {
        let base = "caf\u{e9}".as_bytes();
        assert_eq!(
            apply_suggestions(base, &[suggestion(0, 3, "a"), suggestion(2, 5, "b")]),
            None,
            "overlapping spans apply nothing"
        );
        assert_eq!(
            apply_suggestions(base, &[suggestion(0, 99, "a")]),
            None,
            "out-of-bounds spans apply nothing"
        );
        assert_eq!(
            apply_suggestions(base, &[suggestion(4, 4, "a")]),
            None,
            "splitting a multi-byte character is rejected"
        );
    }

    #[test]
    fn allows_adjacent_spans_and_empty_application() {
        let base = b"abcd";
        let applied = apply_suggestions(base, &[suggestion(0, 2, "AB"), suggestion(2, 4, "CD")]);
        assert_eq!(applied.expect("adjacent spans"), b"ABCD".to_vec());
        assert_eq!(
            apply_suggestions(base, &[]).expect("empty applies"),
            b"abcd".to_vec()
        );
    }
}
