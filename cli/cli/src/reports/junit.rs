//! JUnit report facade (split).
//!
//! Split from `super` (`reports.rs`): the normalized case types live in
//! [`super::junit_types`], XML parsing in [`super::junit_parse`], and
//! document rendering in [`super::junit_render`]. Re-exported through
//! `super` so the public paths stay
//! `crate::reports::{JunitCase, JunitMessage, parse_test_xml,
//! render_junit, junit_infrastructure_case}` and
//! `crate::reports::junit::{...}`.

pub use super::junit_parse::parse_test_xml;
pub use super::junit_render::{junit_infrastructure_case, render_junit};
pub use super::junit_types::{JunitCase, JunitMessage};
