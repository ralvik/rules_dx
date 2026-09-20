//! Canonical JSON rendering for plan fingerprints.
//!
//! `dx_codegen` and `dx_env_plan` fingerprint the same way: project merged
//! records onto a `Serialize` view of strings, booleans, and vecs, then
//! render it as JSON. That shape cannot fail to serialize (no maps with
//! non-string keys, no non-finite floats), so this crate owns the single
//! infallible call site instead of repeating `.expect(...)` at every
//! fingerprint function (see also).

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

/// Renders a fingerprint view as canonical JSON.
///
/// The view types are strings, booleans, and vecs thereof, which
/// `serde_json` always serializes; failure is unreachable.
pub fn to_json<T: serde::Serialize>(view: &T) -> String {
    serde_json::to_string(view)
        .unwrap_or_else(|err| unreachable!("fingerprint JSON serializes: {err:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_string_bool_vec_shapes() {
        #[derive(serde::Serialize)]
        struct View<'a> {
            name: &'a str,
            read_only: bool,
            tags: Vec<&'a str>,
        }
        let view = View {
            name: "a\"b",
            read_only: true,
            tags: vec!["x", "y"],
        };
        assert_eq!(
            to_json(&view),
            r#"{"name":"a\"b","read_only":true,"tags":["x","y"]}"#
        );
    }
}
