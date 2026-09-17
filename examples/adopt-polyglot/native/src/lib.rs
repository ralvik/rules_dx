//! Native helper for the polyglot demo: pure functions only.

/// Joins words with a single space.
pub fn join(words: &[&str]) -> String {
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::join;

    #[test]
    fn join_matches_std_join() {
        assert_eq!(join(&["poly", "glot"]), "poly glot");
    }
}
