//! Consolidated `dx status` surface.
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

/// Render legacy JSON payload (single object, not the NDJSON envelope).
/// See: `docs/cli/output-protocol.md#status` for the CLI envelope.
pub fn render_status_json(
    checks: &[StatusCheck],
) -> Result<String, dx_fingerprint::FingerprintError> {
    // Single owner for string-only JSON shapes (typed, no `unreachable!`).
    // See: `cli/fingerprint/src/lib.rs` (`dx_fingerprint::to_json`).
    dx_fingerprint::to_json(&StatusPayload { checks })
}

/// Default local status checks (toolchain + platform + tools + pin).
/// See: `docs/cli/commands/status-version.md#dx-status`.
///
/// Authority: CLI-owned diagnostics vocabulary reporting Bazel/MODULE facts
/// statically (no Bazel subprocess; startup and dry-run stay cheap).
/// Toolchain/platform/tools details mirror their Bazel sources below;
/// live Bazel resolution stays open and is never claimed here.
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
            // Mirrors `MODULE.bazel` (`rules_rust 0.74.0`, `versions = ["1.98.0"]`).
            // See: `docs/product/support-matrix.md#minimal-required-core`.
            detail: "rust 1.98.0 via rules_rust 0.74.0 (MODULE.bazel)".to_owned(),
            hint: "bazel build //...".to_owned(),
        },
        StatusCheck {
            name: "platform".to_owned(),
            status: "ok".to_owned(),
            detail:
                "linux_x86_64 + linux_arm64 glibc plus macos_arm64 plus windows_x86_64 qualified"
                    .to_owned(),
            hint: "see support-matrix for out-of-v1".to_owned(),
        },
        StatusCheck {
            name: "tools".to_owned(),
            status: "ok".to_owned(),
            // Pinned tool hub; Bazel acquires declared artifacts lazily.
            // See: `docs/tools/tool-acquisition.md`.
            detail: "bazel-resolved pinned tools (//quality/artifacts)".to_owned(),
            hint: "no ambient tools required".to_owned(),
        },
        StatusCheck {
            name: "pin".to_owned(),
            status: pin_status.to_owned(),
            detail: format!("dx {pinned} vs module {MODULE_VERSION}"),
            hint: format!("dx version --pin {MODULE_VERSION}"),
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
        // Single source: the pin hint must track `MODULE_VERSION`, and the
        // toolchain/tools details must name their Bazel sources, never bare
        // hardcoded claims.
        // See: `docs/cli/commands/status-version.md#dx-status`.
        let pin = checks.iter().find(|c| c.name == "pin").expect("pin check");
        assert_eq!(pin.hint, format!("dx version --pin {MODULE_VERSION}"));
        let toolchain = checks
            .iter()
            .find(|c| c.name == "toolchain")
            .expect("toolchain check");
        assert!(
            toolchain.detail.contains("MODULE.bazel"),
            "{}",
            toolchain.detail
        );
        let tools = checks
            .iter()
            .find(|c| c.name == "tools")
            .expect("tools check");
        assert!(
            tools.detail.contains("//quality/artifacts"),
            "{}",
            tools.detail
        );
        let json = render_status_json(&checks).expect("status json");
        // Golden pilot: full-payload insta snapshot replaces
        // the contains-asserts; a MODULE_VERSION bump intentionally
        // updates this snapshot alongside the pin contract.
        insta::assert_snapshot!(json, @r#"{"checks":[{"name":"toolchain","status":"ok","detail":"rust 1.98.0 via rules_rust 0.74.0 (MODULE.bazel)","hint":"bazel build //..."},{"name":"platform","status":"ok","detail":"linux_x86_64 + linux_arm64 glibc plus macos_arm64 plus windows_x86_64 qualified","hint":"see support-matrix for out-of-v1"},{"name":"tools","status":"ok","detail":"bazel-resolved pinned tools (//quality/artifacts)","hint":"no ambient tools required"},{"name":"pin","status":"ok","detail":"dx 0.0.0 vs module 0.0.0","hint":"dx version --pin 0.0.0"}]}"#);
    }

    #[test]
    fn status_json_escapes_quotes_newlines_and_controls() {
        let checks = vec![StatusCheck {
            name: "we\"ird".to_owned(),
            status: "ok".to_owned(),
            detail: "line1\nline2\u{1}".to_owned(),
            hint: "back\\slash".to_owned(),
        }];
        let json = render_status_json(&checks).expect("status json");
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
