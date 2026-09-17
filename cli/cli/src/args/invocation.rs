//! Shared invocation types (issue #236).
//!
//! Split from `super` (`args.rs`): owns [`ReportRequest`] and
//! [`Invocation`] plus the profile/mode helpers. The parser
//! ([`super::parser`]) constructs these; `super` re-exports them so
//! `crate::args::{Invocation, ReportRequest}` paths are unchanged.

use dx_output::{OutputMode, Threshold};

use super::profile::{resolve_profile, Profile};
use super::Command;

/// One `--report <format>=<destination>` request. Format support is
/// validated against the command registry during planning; parsing only
/// checks the `format=destination` shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportRequest {
    pub format: String,
    pub destination: String,
}

/// Parsed `dx` invocation: command mode, global options, explicit scope,
/// and Bazel command options after `--`. An empty `targets` selects the
/// repository scope (`//...`). `bazel_clean` is set only by
/// `dx clean --bazel` (additionally forward `bazel clean` after pruning).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub command: Command,
    pub check: bool,
    /// `--debug` (build/run/test only).
    pub debug: bool,
    /// `--release` (build/run/test only).
    pub release: bool,
    pub workspace: Option<String>,
    pub dry_run: bool,
    pub quiet: bool,
    /// `--verbose` (issue #222): structured `tracing` diagnostics on
    /// stderr; orthogonal to `--quiet` (which suppresses human summaries).
    /// Default stays byte-identical (warn+error only).
    pub verbose: bool,
    pub output: OutputMode,
    pub reports: Vec<ReportRequest>,
    pub fail_on: Threshold,
    /// `dx coverage --min-coverage <percent>`: required line-coverage
    /// percent over the collected LCOV (Coverage only; `None` collects
    /// without enforcing a threshold).
    pub min_coverage: Option<u32>,
    pub targets: Vec<String>,
    pub bazel_options: Vec<String>,
    pub bazel_clean: bool,
    /// `dx version --pin <version>`: re-pin target (Version only).
    pub pin: Option<String>,
    /// `dx version --rollback`: re-pin the recorded previous release
    /// (Version only; rejected together with `--pin`).
    pub rollback: bool,
    /// Inspect wrappers use `cquery` instead of `query` (Owners, Deps,
    /// Why only).
    pub configured: bool,
}

impl Invocation {
    /// `command_started` mode: `check` for `--check`, else `default`.
    pub fn mode(self) -> &'static str {
        if self.check {
            "check"
        } else {
            "default"
        }
    }

    /// Explicit `--debug`/`--release` flag as a [`Profile`]: `None` for
    /// the bare invocation (which resolves to the command default).
    /// Parsing rejects both flags together, so the arms are exclusive.
    pub fn profile_flag(&self) -> Option<Profile> {
        if self.debug {
            Some(Profile::Debug)
        } else if self.release {
            Some(Profile::Release)
        } else {
            None
        }
    }

    /// Effective profile under issue #179 precedence: explicit flag over
    /// the command default. Deploy resolves flag over the target
    /// `profile` attribute over the release default (issue #180); the
    /// target attribute is read during execution via cquery, so this
    /// returns flag over command default and execution refines it.
    /// Build/run/test have no target attribute.
    pub fn profile(&self) -> Profile {
        resolve_profile(
            self.profile_flag(),
            None,
            Profile::default_for(self.command),
        )
    }
}
