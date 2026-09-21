//! Shared invocation types.
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
/// repository scope (`//...`), unless `here` selects the current directory
/// tree instead (`//path/...`; `//...` at the root). `bazel_clean` is set only by
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
    /// `--verbose`: structured `tracing` diagnostics on
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
    /// `dx migrate --from <version>`: source version (Migrate only).
    pub from: Option<String>,
    /// `dx migrate --to <version>`: target version
    /// (Migrate only).
    pub to: Option<String>,
    /// `dx <command> --here` (`--cwd` alias): select the current directory
    /// tree instead of `//...` (cwd-scope commands only; never implicit).
    pub here: bool,
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

    /// Effective profile precedence: explicit flag over
    /// the command default. Deploy resolves flag over the target
    /// `profile` attribute over the release default; the
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

/// Workspace-relative scope spelling for `--here` (`--cwd` alias,
/// See: `docs/cli/target-resolution.md`, issue #699): the current directory tree as a directory scope for the
/// existing resolution (`//path/...` via `classify`; `//...` at the root
/// directly so both the `classify` directory path and the audit
/// `owning_sets` label path resolve repository-wide). Never implicit:
/// only called when `here` is set.
/// Fails when `cwd` is outside `workspace` or not UTF-8.
pub fn here_scope(workspace: &std::path::Path, cwd: &std::path::Path) -> Result<String, String> {
    let rel = cwd.strip_prefix(workspace).map_err(|_| {
        format!(
            "current directory {} is outside workspace {}: --here needs a directory under the workspace",
            cwd.display(),
            workspace.display()
        )
    })?;
    if rel.as_os_str().is_empty() {
        return Ok("//...".to_owned());
    }
    let text = rel.to_str().ok_or_else(|| {
        format!(
            "current directory {} is not valid UTF-8: --here needs a UTF-8 path",
            cwd.display()
        )
    })?;
    if text.is_empty() {
        return Ok("//...".to_owned());
    }
    // Normalize Windows separators so the scope stays workspace-relative
    // for `classify` on every host.
    Ok(text.replace('\\', "/"))
}

/// Consumes `--here` into explicit targets: replaces an empty scope (or
/// one audit family selector) with the [`here_scope`] directory spelling
/// and clears `here`, so downstream resolution reuses the existing
/// directory-scope path verbatim. Parsing already rejects `--here` with
/// explicit scopes (audit allows one family); this rechecks and fails
/// closed on misuse.
pub fn apply_here(
    invocation: &Invocation,
    workspace: &std::path::Path,
    cwd: &std::path::Path,
) -> Result<Invocation, String> {
    if !invocation.here {
        return Ok(invocation.clone());
    }
    if !invocation.command.supports_here() {
        return Err(format!(
            "option \"--here\" is not supported by dx {}",
            invocation.command.name()
        ));
    }
    let scope = here_scope(workspace, cwd)?;
    let mut next = invocation.clone();
    if invocation.command == super::command::Command::Audit {
        if next.targets.is_empty() {
            next.targets = vec![scope];
        } else if next.targets.len() == 1
            && (next.targets[0] == "security" || next.targets[0] == "license")
        {
            let family = next.targets[0].clone();
            next.targets = vec![family, scope];
        } else {
            return Err(
                "option \"--here/--cwd\" cannot be combined with explicit scopes".to_owned(),
            );
        }
    } else {
        if !next.targets.is_empty() {
            return Err(
                "option \"--here/--cwd\" cannot be combined with explicit scopes".to_owned(),
            );
        }
        next.targets = vec![scope];
    }
    next.here = false;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invocation_for(command: Command, targets: &[&str]) -> Invocation {
        use dx_output::{OutputMode, Threshold};
        Invocation {
            command,
            check: false,
            debug: false,
            release: false,
            workspace: None,
            dry_run: false,
            quiet: false,
            verbose: false,
            output: OutputMode::Text { quiet: false },
            reports: Vec::new(),
            fail_on: Threshold::Warning,
            min_coverage: None,
            targets: targets.iter().map(ToString::to_string).collect(),
            bazel_options: Vec::new(),
            bazel_clean: false,
            pin: None,
            rollback: false,
            configured: false,
            from: None,
            to: None,
            here: true,
        }
    }

    #[test]
    fn here_scope_maps_cwd_to_directory_spelling() {
        let workspace = std::path::Path::new("/ws");
        // Workspace root selects repository-wide `//...` directly so both
        // the `classify` directory path and the audit `owning_sets` label
        // path resolve without a filesystem probe.
        assert_eq!(
            here_scope(workspace, std::path::Path::new("/ws")),
            Ok("//...".to_owned())
        );
        assert_eq!(
            here_scope(workspace, std::path::Path::new("/ws/cli/cli")),
            Ok("cli/cli".to_owned())
        );
        assert!(here_scope(workspace, std::path::Path::new("/other")).is_err());
    }

    #[test]
    fn apply_here_consumes_flag_into_explicit_targets() {
        let workspace = std::path::Path::new("/ws");
        let subdir = std::path::Path::new("/ws/cli/cli");
        let root = std::path::Path::new("/ws");

        // Quality command in a subdir becomes the relative directory scope.
        let resolved =
            apply_here(&invocation_for(Command::Lint, &[]), workspace, subdir).expect("resolves");
        assert!(!resolved.here);
        assert_eq!(resolved.targets, vec!["cli/cli".to_owned()]);

        // Workspace root becomes `//...` so audit `owning_sets` stays ALL.
        let resolved =
            apply_here(&invocation_for(Command::Lint, &[]), workspace, root).expect("resolves");
        assert_eq!(resolved.targets, vec!["//...".to_owned()]);

        // Audit keeps one family selector plus the directory scope.
        let resolved = apply_here(
            &invocation_for(Command::Audit, &["security"]),
            workspace,
            subdir,
        )
        .expect("resolves");
        assert_eq!(
            resolved.targets,
            vec!["security".to_owned(), "cli/cli".to_owned()]
        );
        let resolved =
            apply_here(&invocation_for(Command::Audit, &[]), workspace, root).expect("resolves");
        assert_eq!(resolved.targets, vec!["//...".to_owned()]);

        // Explicit scopes never combine; unsupported commands fail closed.
        assert!(apply_here(
            &invocation_for(Command::Lint, &["//a:one"]),
            workspace,
            subdir
        )
        .is_err());
        assert!(apply_here(
            &invocation_for(Command::Audit, &["security", "//a:one"]),
            workspace,
            subdir
        )
        .is_err());
        assert!(apply_here(&invocation_for(Command::Clean, &[]), workspace, subdir).is_err());
        assert!(apply_here(
            &invocation_for(Command::Lint, &[]),
            workspace,
            std::path::Path::new("/other")
        )
        .is_err());

        // Without the flag the invocation passes through untouched.
        let mut plain = invocation_for(Command::Lint, &[]);
        plain.here = false;
        let resolved = apply_here(&plain, workspace, subdir).expect("passthrough");
        assert!(resolved.targets.is_empty());
        assert!(!resolved.here);
    }
}
