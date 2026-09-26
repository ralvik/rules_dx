use dx_output::{ColorMode, LogLevel, OutputMode, Threshold};

use super::profile::{resolve_profile, Profile};
use super::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportRequest {
    pub format: String,
    pub destination: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub command: Command,
    pub check: bool,
    pub debug: bool,
    pub release: bool,
    pub workspace: Option<String>,
    pub dry_run: bool,
    pub quiet: bool,
    pub verbose: bool,
    pub log_level: Option<LogLevel>,
    pub color: ColorMode,
    pub output: OutputMode,
    pub reports: Vec<ReportRequest>,
    pub fail_on: Threshold,
    pub min_coverage: Option<u32>,
    pub targets: Vec<String>,
    pub bazel_options: Vec<String>,
    pub bazel_clean: bool,
    pub pin: Option<String>,
    pub rollback: bool,
    pub configured: bool,
    pub from: Option<String>,
    pub to: Option<String>,
    pub here: bool,
    pub serve: bool,
    pub port: Option<u16>,
    pub host: Option<String>,
    pub open: bool,
    pub offline: bool,
}

impl Invocation {
    pub fn mode(self) -> &'static str {
        if self.check {
            "check"
        } else {
            "default"
        }
    }

    pub fn profile_flag(&self) -> Option<Profile> {
        if self.debug {
            Some(Profile::Debug)
        } else if self.release {
            Some(Profile::Release)
        } else {
            None
        }
    }

    pub fn profile(&self) -> Profile {
        resolve_profile(
            self.profile_flag(),
            None,
            Profile::default_for(self.command),
        )
    }
}

pub fn here_scope(workspace: &std::path::Path, cwd: &std::path::Path) -> Result<String, String> {
    use std::path::Component;
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
    let mut parts: Vec<String> = Vec::new();
    for component in rel.components() {
        match component {
            Component::Normal(part) => {
                let text = part.to_str().ok_or_else(|| {
                    format!(
                        "current directory {} is not valid UTF-8: --here needs a UTF-8 path",
                        cwd.display()
                    )
                })?;
                // Normalize Windows separators inside a part (defensive;
                // `components` already splits on both separators).
                for piece in text.replace('\\', "/").split('/') {
                    if !piece.is_empty() && piece != "." {
                        parts.push(piece.to_owned());
                    }
                }
            }
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) | Component::ParentDir => {
                return Err(format!(
                    "current directory {} is outside workspace {}: --here needs a directory under the workspace",
                    cwd.display(),
                    workspace.display()
                ));
            }
        }
    }
    if parts.is_empty() {
        return Ok("//...".to_owned());
    }
    Ok(parts.join("/"))
}

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
    if !next.targets.is_empty() {
        return Err(
            "option \"--here/--cwd\" cannot be combined with explicit scopes".to_owned(),
        );
    }
    next.targets = vec![scope];
    next.here = false;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invocation_for(command: Command, targets: &[&str]) -> Invocation {
        use dx_output::{ColorMode, OutputMode, Threshold};
        Invocation {
            command,
            check: false,
            debug: false,
            release: false,
            workspace: None,
            dry_run: false,
            quiet: false,
            verbose: false,
            log_level: None,
            color: ColorMode::Auto,
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
            serve: false,
            port: None,
            host: None,
            open: false,
            offline: false,
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

        // Workspace root becomes `//...` so `owning_sets` stays ALL.
        let resolved =
            apply_here(&invocation_for(Command::Lint, &[]), workspace, root).expect("resolves");
        assert_eq!(resolved.targets, vec!["//...".to_owned()]);

        // Security/license resolve like every other graph-scope command.
        let resolved =
            apply_here(&invocation_for(Command::Security, &[]), workspace, subdir)
                .expect("resolves");
        assert_eq!(resolved.targets, vec!["cli/cli".to_owned()]);
        let resolved =
            apply_here(&invocation_for(Command::License, &[]), workspace, root).expect("resolves");
        assert_eq!(resolved.targets, vec!["//...".to_owned()]);

        // Explicit scopes never combine; unsupported commands fail closed.
        assert!(apply_here(
            &invocation_for(Command::Lint, &["//a:one"]),
            workspace,
            subdir
        )
        .is_err());
        assert!(apply_here(
            &invocation_for(Command::Security, &["//a:one"]),
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

    #[test]
    fn here_scope_canonicalizes_and_bare_stays_repo_wide() {
        let workspace = std::path::Path::new("/ws");
        assert_eq!(
            here_scope(workspace, std::path::Path::new("/ws")),
            Ok("//...".to_owned())
        );
        assert_eq!(
            here_scope(workspace, std::path::Path::new("/ws/cli/cli")),
            Ok("cli/cli".to_owned())
        );
        // Canonicalization: trailing slashes, dot segments, and doubled
        // separators never reach `classify`.
        assert_eq!(
            here_scope(workspace, std::path::Path::new("/ws/cli/cli/")),
            Ok("cli/cli".to_owned())
        );
        assert_eq!(
            here_scope(workspace, std::path::Path::new("/ws/./cli/cli")),
            Ok("cli/cli".to_owned())
        );
        // Bare invocations in a subdir stay `//...`: only `--here`/`--cwd`
        // selects the directory tree, never the no-flag default.
        let mut bare = invocation_for(Command::Lint, &[]);
        bare.here = false;
        let resolved =
            apply_here(&bare, workspace, std::path::Path::new("/ws/cli/cli")).expect("passthrough");
        assert!(
            resolved.targets.is_empty(),
            "bare subdir must stay empty (=//...)"
        );
        // `--cwd` is the same flag as `--here` (visible alias).
        let aliased = crate::args::parse(
            &["lint", "--cwd"]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
        )
        .expect("cwd alias parses");
        assert!(aliased.here, "--cwd must set here");
    }
}
