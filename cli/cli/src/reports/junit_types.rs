//! JUnit normalized case types (issue #236 split).
//!
//! Split from [`super::junit`]: owns [`JunitCase`] and [`JunitMessage`],
//! the normalized Bazel-reported test-case shape shared by the parsing
//! side ([`super::junit_parse`]) and the rendering side
//! ([`super::junit_render`]). Re-exported through [`super::junit`] so the
//! public paths stay `crate::reports::{JunitCase, JunitMessage}` and
//! `crate::reports::junit::{JunitCase, JunitMessage}`.

/// One Bazel-reported test case normalized for JUnit rendering.
/// `shard` and `attempt` are zero-based indices derived from the 1-based
/// BEP `testResult` identity (`shard = bep_shard - 1`).
#[derive(Debug, Clone, PartialEq)]
pub struct JunitCase {
    pub name: String,
    pub classname: Option<String>,
    pub time: f64,
    pub failure: Option<JunitMessage>,
    pub error: Option<JunitMessage>,
    pub skipped: Option<JunitMessage>,
    pub system_out: Option<String>,
    pub system_err: Option<String>,
    pub shard: u32,
    pub attempt: u32,
}

/// Message plus body preserved from a Bazel-reported
/// `<failure>`, `<error>`, or `<skipped>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JunitMessage {
    pub message: Option<String>,
    pub text: String,
}
