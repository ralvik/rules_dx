//! Feature-gated digest extras (`digest_extra` feature).
use super::digest;

/// Hashes two values in order.
pub fn digest_pair(first: &str, second: &str) -> u64 {
    digest((first, second))
}

#[cfg(test)]
mod tests {
    use super::digest_pair;

    #[test]
    fn pair_is_order_sensitive() {
        assert_ne!(digest_pair("a", "b"), digest_pair("b", "a"));
    }
}
