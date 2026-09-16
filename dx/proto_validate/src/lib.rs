//! Shared validated-codec helpers for proto shards (#72).
//!
//! Five crates repeat the same three patterns: validate-then-encode,
//! decode-then-validate, and sorted-unique key checks over a `BTreeSet` or a
//! `previous` cursor. The per-crate `Error` types stay local; this crate only
//! provides the control flow so every shard keeps its own messages.

use std::collections::BTreeSet;

/// How a sorted-unique sequence broke its contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderViolation {
    Duplicate,
    Unsorted,
}

/// Check one step of a sorted-unique sequence.
///
/// `previous` is the last accepted key (`None` for the first item).
/// Equal keys report [`OrderViolation::Duplicate`]; a key smaller than its
/// predecessor reports [`OrderViolation::Unsorted`].
pub fn check_sorted_next<T: Ord>(previous: Option<&T>, current: &T) -> Result<(), OrderViolation> {
    match previous {
        Some(prev) if prev == current => Err(OrderViolation::Duplicate),
        Some(prev) if prev > current => Err(OrderViolation::Unsorted),
        _ => Ok(()),
    }
}

/// Insert one key into a uniqueness set, mapping a repeat to the caller's
/// error. Reads naturally inside validation loops that also check each item:
///
/// ```ignore
/// let mut seen = std::collections::BTreeSet::new();
/// for entry in &shard.entries {
///     check_value(&entry.value)?;
///     check_unique_insert(&mut seen, &entry.key, |key| Error::Duplicate {
///         key: (*key).clone(),
///     })?;
/// }
/// ```
pub fn check_unique_insert<T: Ord, E>(
    seen: &mut BTreeSet<T>,
    item: T,
    duplicate: impl FnOnce(&T) -> E,
) -> Result<(), E> {
    if seen.contains(&item) {
        return Err(duplicate(&item));
    }
    seen.insert(item);
    Ok(())
}

/// Run `validate` on an already-built message, then encode it.
pub fn encode_with_validation<M: prost::Message, E>(
    msg: &M,
    validate: impl FnOnce(&M) -> Result<(), E>,
) -> Result<Vec<u8>, E> {
    validate(msg)?;
    Ok(msg.encode_to_vec())
}

/// Decode a message, then run `validate` on it. Decode failures map through
/// `map_decode_error` so each crate keeps its own `Error::Decode` shape.
pub fn decode_with_validation<M: prost::Message + Default, E>(
    bytes: &[u8],
    validate: impl Fn(&M) -> Result<(), E>,
    map_decode_error: impl FnOnce(String) -> E,
) -> Result<M, E> {
    let msg = M::decode(bytes).map_err(|error| map_decode_error(error.to_string()))?;
    validate(&msg)?;
    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorted_next_accepts_first_and_ascending() {
        assert!(check_sorted_next::<String>(None, &"a".to_owned()).is_ok());
        assert!(check_sorted_next(Some(&"a".to_owned()), &"b".to_owned()).is_ok());
    }

    #[test]
    fn sorted_next_reports_duplicate_and_unsorted() {
        assert_eq!(
            check_sorted_next(Some(&"a".to_owned()), &"a".to_owned()),
            Err(OrderViolation::Duplicate)
        );
        assert_eq!(
            check_sorted_next(Some(&"b".to_owned()), &"a".to_owned()),
            Err(OrderViolation::Unsorted)
        );
    }

    #[test]
    fn unique_insert_rejects_repeat_with_caller_error() {
        let mut seen = BTreeSet::new();
        assert!(check_unique_insert(&mut seen, "a".to_owned(), |_| "dup").is_ok());
        assert_eq!(
            check_unique_insert(&mut seen, "a".to_owned(), |item| format!("dup:{item}")),
            Err("dup:a".to_owned())
        );
    }
}
