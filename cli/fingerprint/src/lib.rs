//! Canonical JSON rendering for plan fingerprints.
//!
//! Contract: `docs/environments/codegen.md`.
//!
//! `dx_codegen` and `dx_env_plan` fingerprint the same way: project merged
//! records onto a `Serialize` view of strings, booleans, and vecs, then
//! render it as JSON. This crate owns the single typed call site
//! ([`to_json`]/[`to_json_ascii_pretty`]) instead of repeating
//! `.expect(...)`/`unreachable!` at every fingerprint function, and owns
//! the Python `ensure_ascii` escape rule (see also).

// Infallible paths must not `expect`/`unwrap`/`unreachable`/`todo` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(
    not(test),
    deny(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::unreachable,
        clippy::todo
    )
)]

/// Fingerprint rendering failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FingerprintError {
    /// The fingerprint view failed to serialize as JSON.
    #[error("fingerprint JSON serializes: {detail}")]
    Json { detail: String },
}

impl From<serde_json::Error> for FingerprintError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json {
            detail: error.to_string(),
        }
    }
}

/// Renders a fingerprint view as canonical JSON.
///
/// The view types are strings, booleans, and vecs thereof, which
/// `serde_json` always serializes for the pinned shapes; serialization
/// failure is a typed [`FingerprintError`] (with the serde detail as
/// display, no panic) so callers propagate instead of trapping.
pub fn to_json<T: serde::Serialize>(view: &T) -> Result<String, FingerprintError> {
    serde_json::to_string(view).map_err(FingerprintError::from)
}

/// Re-escapes non-ASCII as `\uXXXX` (surrogate pairs above U+FFFF).
///
/// Single owner for Python `json.dumps(ensure_ascii=True)` parity:
/// `serde_json` emits raw UTF-8 while Python escapes non-ASCII, so SBOM
/// pretty output re-escapes after rendering.
/// See: `deploy/release/src/lib.rs` (`render_pretty`).
pub fn ensure_ascii(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        let code = c as u32;
        if code < 0x80 {
            out.push(c);
        } else if code <= 0xFFFF {
            out.push_str(&format!("\\u{code:04x}"));
        } else {
            let v = code - 0x10000;
            let high = 0xD800 + (v >> 10);
            let low = 0xDC00 + (v & 0x3FF);
            out.push_str(&format!("\\u{high:04x}\\u{low:04x}"));
        }
    }
    out
}

/// Renders `value` as Python `json.dumps(indent=2, sort_keys=True,
/// ensure_ascii=True)` bytes (pretty plus trailing newline).
///
/// Single owner for the SBOM/provenance wire bytes; `deploy/release`
/// delegates here so the ASCII-escape rule cannot drift.
/// See: `deploy/release/src/lib.rs` (`render_pretty`).
pub fn to_json_ascii_pretty(value: &serde_json::Value) -> Result<String, FingerprintError> {
    let pretty = serde_json::to_string_pretty(value).map_err(FingerprintError::from)?;
    Ok(ensure_ascii(&pretty) + "\n")
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
            to_json(&view).expect("fingerprint renders"),
            r#"{"name":"a\"b","read_only":true,"tags":["x","y"]}"#
        );
    }

    #[test]
    fn ascii_escape_matches_python_ensure_ascii() {
        // Parity with Python `json.dumps(ensure_ascii=True)`: BMP escapes
        // as `\uXXXX`, astral as a surrogate pair, ASCII untouched.
        assert_eq!(ensure_ascii("caf\u{e9}"), "caf\\u00e9");
        assert_eq!(ensure_ascii("a\u{1F600}b"), "a\\ud83d\\ude00b");
        assert_eq!(ensure_ascii("plain"), "plain");
        let value = serde_json::json!({"name": "caf\u{e9}"});
        let text = to_json_ascii_pretty(&value).expect("pretty ascii");
        assert!(text.contains("caf\\u00e9"));
        assert!(!text.contains("caf\u{e9}"));
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn json_error_is_typed_with_display() {
        // A map with non-string keys cannot serialize: the typed error
        // keeps the display (no panic) so callers propagate.
        use std::collections::HashMap;
        let mut bad: HashMap<Vec<u8>, String> = HashMap::new();
        bad.insert(vec![0xff], "x".to_owned());
        let err = to_json(&bad).expect_err("non-string keys must fail");
        assert!(err.to_string().contains("fingerprint JSON serializes"));
    }
}
