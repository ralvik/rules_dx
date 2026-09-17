//! Foreign-tree API crate: hashing helpers with a feature-gated extra.
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Hashes one value with the default hasher.
pub fn digest<T: Hash>(value: T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(feature = "digest_extra")]
pub mod extra;

#[cfg(test)]
mod tests {
    use super::digest;

    #[test]
    fn digest_is_stable_for_equal_inputs() {
        assert_eq!(digest("worker"), digest("worker"));
    }

    #[test]
    fn digest_differs_for_distinct_inputs() {
        assert_ne!(digest("api"), digest("worker"));
    }
}
