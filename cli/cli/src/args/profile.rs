//! Build profile vocabulary (issue #236).
//!
//! Split from `super` (`args.rs`): owns [`Profile`], [`DX_PROFILE_ENV`],
//! and [`resolve_profile`] (issue #179, ADR 0021). The parser
//! ([`super::parser`]) and invocation ([`super::Invocation`]) build on
//! these; `super` re-exports them so `crate::args::{...}` paths are
//! unchanged.

use super::Command;

/// Build profile vocabulary (issue #179, ADR 0021): `--debug` selects
/// `dx_debug` (`dbg`), the bare invocation selects `dx_dev`
/// (`fastbuild`), and `--release` selects `dx_release` (`opt`). There
/// is no `--dev` flag: the bare invocation already means the middle
/// mode and keeps `dx build` short.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Debug,
    Dev,
    Release,
}

impl Profile {
    /// Stable profile name forwarded as `DX_PROFILE` to deploy programs.
    pub fn name(self) -> &'static str {
        match self {
            Profile::Debug => "debug",
            Profile::Dev => "dev",
            Profile::Release => "release",
        }
    }

    /// Shared Bazel config backing the profile (ADR 0021).
    pub fn config(self) -> &'static str {
        match self {
            Profile::Debug => "dx_debug",
            Profile::Dev => "dx_dev",
            Profile::Release => "dx_release",
        }
    }

    /// Required `--config=` flag pinning the profile on a workflow argv.
    pub fn config_flag(self) -> String {
        format!("--config={}", self.config())
    }

    /// Command default: `deploy` defaults to release, every other
    /// command defaults to dev.
    pub fn default_for(command: Command) -> Self {
        match command {
            Command::Deploy => Profile::Release,
            _ => Profile::Dev,
        }
    }

    /// Parses a deploy-target `profile` attribute value (`debug`, `dev`,
    /// `release`): `None` for anything else so analysis diagnostics own
    /// the spelling error.
    pub fn parse_attr(value: &str) -> Option<Self> {
        match value {
            "debug" => Some(Profile::Debug),
            "dev" => Some(Profile::Dev),
            "release" => Some(Profile::Release),
            _ => None,
        }
    }
}

/// Environment variable forwarding the resolved profile to the deploy
/// program (issue #179 item 3).
pub const DX_PROFILE_ENV: &str = "DX_PROFILE";

/// Precedence for the effective profile (issue #179 item 2): the
/// explicit `--debug`/`--release` flag wins over the deploy target
/// `profile` attribute, which wins over the command default. Build,
/// run, and test have no target attribute, so they resolve flag over
/// default; deploy resolves flag over target attribute over the
/// release default.
pub fn resolve_profile(flag: Option<Profile>, attr: Option<Profile>, default: Profile) -> Profile {
    flag.or(attr).unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_vocabulary_maps_to_shared_configs() {
        assert_eq!(Profile::Debug.name(), "debug");
        assert_eq!(Profile::Dev.name(), "dev");
        assert_eq!(Profile::Release.name(), "release");
        assert_eq!(Profile::Debug.config(), "dx_debug");
        assert_eq!(Profile::Dev.config(), "dx_dev");
        assert_eq!(Profile::Release.config(), "dx_release");
        assert_eq!(Profile::Debug.config_flag(), "--config=dx_debug");
        assert_eq!(Profile::Dev.config_flag(), "--config=dx_dev");
        assert_eq!(Profile::Release.config_flag(), "--config=dx_release");
        assert_eq!(DX_PROFILE_ENV, "DX_PROFILE");
    }

    #[test]
    fn profile_attr_parses_deploy_vocabulary() {
        assert_eq!(Profile::parse_attr("debug"), Some(Profile::Debug));
        assert_eq!(Profile::parse_attr("dev"), Some(Profile::Dev));
        assert_eq!(Profile::parse_attr("release"), Some(Profile::Release));
        assert_eq!(Profile::parse_attr("staging"), None);
        assert_eq!(Profile::parse_attr(""), None);
    }

    #[test]
    fn profile_precedence_is_flag_over_attr_over_default() {
        // Explicit flag wins over the deploy target attribute.
        assert_eq!(
            resolve_profile(Some(Profile::Debug), Some(Profile::Release), Profile::Dev),
            Profile::Debug
        );
        // Target attribute wins over the command default.
        assert_eq!(
            resolve_profile(None, Some(Profile::Release), Profile::Dev),
            Profile::Release
        );
        // Bare invocation resolves to the command default.
        assert_eq!(resolve_profile(None, None, Profile::Dev), Profile::Dev);
        assert_eq!(
            resolve_profile(None, None, Profile::Release),
            Profile::Release
        );
    }
}
