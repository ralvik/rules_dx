//! Consolidated `dx status` surface (issue #236).
//!
//! Split from `super` (`lib.rs`): owns `StatusCheck`,
//! `render_status_text`, `render_status_json`, and
//! `default_status_checks`. Re-exported through `super` so the public
//! path stays `dx_adopt::{StatusCheck, render_status_text,
//! render_status_json, default_status_checks}`.

use serde::Serialize;

use super::{version_pin_matches_module, MODULE_VERSION};

/// One diagnostics check in the consolidated `dx status` surface.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StatusCheck {
    /// Check name (toolchain, platform, tools, pin).
    pub name: String,
    /// One of `ok|warn|error`.
    pub status: String,
    /// Human detail.
    pub detail: String,
    /// Actionable hint.
    pub hint: String,
}

#[derive(Serialize)]
struct StatusPayload<'a> {
    checks: &'a [StatusCheck],
}

/// Render text status: one line per check.
pub fn render_status_text(checks: &[StatusCheck]) -> String {
    checks
        .iter()
        .map(|c| format!("{}: {} ({}) hint: {}", c.name, c.status, c.detail, c.hint))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render JSON status (single object, NDJSON-compatible).
pub fn render_status_json(checks: &[StatusCheck]) -> String {
    // Infallible shape (issue #238): strings only, so `serde_json` cannot
    // fail; the fallback names the invariant instead of `expect`.
    serde_json::to_string(&StatusPayload { checks })
        .unwrap_or_else(|err| unreachable!("status JSON serializes: {err:?}"))
}

/// Default local status checks (toolchain + platform + tools + pin).
pub fn default_status_checks(pinned: &str) -> Vec<StatusCheck> {
    let pin_status = if version_pin_matches_module(pinned, MODULE_VERSION) {
        "ok"
    } else {
        "error"
    };
    vec![
        StatusCheck {
            name: "toolchain".to_owned(),
            status: "ok".to_owned(),
            detail: "rust 1.98.0 via rules_rust".to_owned(),
            hint: "bazel build //...".to_owned(),
        },
        StatusCheck {
            name: "platform".to_owned(),
            status: "ok".to_owned(),
            detail: "linux_x86_64 + linux_arm64 glibc plus static musl plus macos_arm64 qualified"
                .to_owned(),
            hint: "see reusable-consumer matrix for windows".to_owned(),
        },
        StatusCheck {
            name: "tools".to_owned(),
            status: "ok".to_owned(),
            detail: "bazel-resolved pinned tools".to_owned(),
            hint: "no ambient tools required".to_owned(),
        },
        StatusCheck {
            name: "pin".to_owned(),
            status: pin_status.to_owned(),
            detail: format!("dx {pinned} vs module {MODULE_VERSION}"),
            hint: "dx version --pin 0.0.0".to_owned(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::super::{default_status_checks, StatusCheck, MODULE_VERSION};
    use super::{render_status_json, render_status_text};

    #[test]
    fn status_renders_text_and_json() {
        let checks = default_status_checks(MODULE_VERSION);
        assert_eq!(checks.len(), 4);
        let text = render_status_text(&checks);
        assert!(text.contains("pin: ok"));
        let json = render_status_json(&checks);
        // Golden pilot (issue #225): full-payload insta snapshot replaces
        // the contains-asserts; a MODULE_VERSION bump intentionally
        // updates this snapshot alongside the pin contract.
        insta::assert_snapshot!(json, @r#"{"checks":[{"name":"toolchain","status":"ok","detail":"rust 1.98.0 via rules_rust","hint":"bazel build //..."},{"name":"platform","status":"ok","detail":"linux_x86_64 + linux_arm64 glibc plus static musl plus macos_arm64 qualified","hint":"see reusable-consumer matrix for windows"},{"name":"tools","status":"ok","detail":"bazel-resolved pinned tools","hint":"no ambient tools required"},{"name":"pin","status":"ok","detail":"dx 0.0.0 vs module 0.0.0","hint":"dx version --pin 0.0.0"}]}"#);
    }

    #[test]
    fn status_json_escapes_quotes_newlines_and_controls() {
        let checks = vec![StatusCheck {
            name: "we\"ird".to_owned(),
            status: "ok".to_owned(),
            detail: "line1\nline2\u{1}".to_owned(),
            hint: "back\\slash".to_owned(),
        }];
        let json = render_status_json(&checks);
        assert!(json.contains("\\\""), "{json}");
        assert!(json.contains("\\n"), "{json}");
        assert!(json.contains("\\\\"), "{json}");
        assert!(json.contains("\\u0001"), "{json}");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("status JSON is valid");
        assert_eq!(parsed["checks"][0]["name"], "we\"ird");
        // Re-serializing via `Value` sorts object keys (BTreeMap), while the
        // struct order stays name,status,detail,hint for byte-stability with
        // the pre-serde rendering; compare values, not bytes.
        let reparsed: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&parsed).expect("reserialize"))
                .expect("reserialized JSON is valid");
        assert_eq!(parsed, reparsed);
    }
}
