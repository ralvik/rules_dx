//! Nested local module with a stdlib-only reference.
use std::collections::HashMap;

/// Counts word occurrences.
pub fn histogram<'a>(words: &[&'a str]) -> HashMap<&'a str, usize> {
    let mut counts = HashMap::new();
    for word in words {
        *counts.entry(*word).or_insert(0) += 1;
    }
    counts
}
