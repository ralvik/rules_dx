//! `dx init` absent-only scaffolding.
//!
//! Split from `super` (`lib.rs`): owns `absent_only_write_allowed`,
//! `init_must_refuse`, `ScaffoldFile`, `DEVCONTAINER_JSON`,
//! `plan_init_files`, and `apply_init`.
//! Re-exported through `super` so the public path stays
//! `dx_adopt::{absent_only_write_allowed, init_must_refuse,
//! ScaffoldFile, DEVCONTAINER_JSON, plan_init_files,
//! apply_init}`.

use std::path::Path;

use super::{AdoptError, DX_VERSION};

/// Whether `dx init` may write one scaffolded file.
///
/// Init writes absent-only: a missing path may be written, an existing path
/// is left untouched (tracked files are never mutated). The caller checks
/// each path independently.
pub fn absent_only_write_allowed(target_exists: bool) -> bool {
    !target_exists
}

/// Whether `dx init` must refuse one scaffolded path.
///
/// An existing file is refused even with force: an unmanaged `.envrc` or any
/// other present file is never overwritten by scaffolding. Absent paths go
/// through [`absent_only_write_allowed`].
pub fn init_must_refuse(target_exists: bool, _force: bool) -> bool {
    target_exists
}

/// One scaffolded file planned by [`plan_init_files`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScaffoldFile {
    /// Workspace-relative path.
    pub path: String,
    /// Exact bytes to write when absent.
    pub content: String,
}

/// `dx init` devcontainer definition (snapshot workflow issue #322).
///
/// Single source for the scaffolded `.devcontainer/devcontainer.json`:
/// the repository's own `.devcontainer/devcontainer.json` is a snapshot of
/// this string held by `//.devcontainer:devcontainer_parity_test`
/// (schema plus byte snapshot, UPDATE_EXPECT refreshes the golden), so the
/// definition we ship is the one we boot. `postCreateCommand` runs the user-path bootstrap
/// (`bazel run //dx:env`, then `dx setup`) instead of a full build, so
/// container create pays only the managed-environment setup.
pub const DEVCONTAINER_JSON: &str = concat!(
    "{\n",
    "  \"name\": \"rules_dx\",\n",
    "  \"image\": \"mcr.microsoft.com/devcontainers/base:ubuntu\",\n",
    "  \"features\": {\n",
    "    \"ghcr.io/devcontainers/features/bazel:1\": {}\n",
    "  },\n",
    "  \"customizations\": {\n",
    "    \"vscode\": {\n",
    "      \"extensions\": [\"rust-lang.rust-analyzer\"]\n",
    "    }\n",
    "  },\n",
    "  \"postCreateCommand\": \"bazel run //dx:env && bazel run //cli/cli:dx -- setup\"\n",
    "}\n",
);

/// Plan the `dx init` scaffold for a module name.
///
/// All writes are absent-only; the caller refuses existing paths even with
/// force (see [`init_must_refuse`]). Contents are pinned (no network) and
/// point editors at `.dx` projections, checked-in native configs, and
/// managed `.dx/bin` tools.
pub fn plan_init_files(module_name: &str) -> Vec<ScaffoldFile> {
    let module = if module_name.is_empty() {
        "my_project"
    } else {
        module_name
    };
    vec![
        ScaffoldFile {
            path: ".dx/version".to_owned(),
            content: format!("{DX_VERSION}\n"),
        },
        ScaffoldFile {
            path: "dx.local.toml".to_owned(),
            content: "# Local-only overrides (gitignored). See dx hooks status.\n[hooks]\n".to_owned(),
        },
        ScaffoldFile {
            path: "dx.hooks.toml".to_owned(),
            content: "[hooks]\npre_commit = [\"format --check\", \"lint --check\"]\npre_push = [\"typecheck --check\", \"generate --check\"]\nbudget_secs = 120\n"
                .to_owned(),
        },
        ScaffoldFile {
            path: ".devcontainer/devcontainer.json".to_owned(),
            content: DEVCONTAINER_JSON.to_owned(),
        },
        ScaffoldFile {
            path: ".vscode/settings.json".to_owned(),
            content: "{\"rust-analyzer.check.command\":\"bazel\",\"python.defaultInterpreterPath\":\".dx/setups/current/.venv/bin/python\",\"typescript.tsdk\":\".dx/setups/current/node_modules/typescript/lib\",\"go.toolsManagement.checkForUpdates\":\"off\"}\n"
                .to_owned(),
        },
        ScaffoldFile {
            path: ".vscode/extensions.json".to_owned(),
            content: "{\"recommendations\":[\"rust-lang.rust-analyzer\",\"ms-python.python\",\"bradlc.vscode-tailwindcss\"]}\n"
                .to_owned(),
        },
        ScaffoldFile {
            path: ".github/workflows/ci.yml".to_owned(),
            content: format!(
                "# Caller template: pins the qualified reusable workflow at a reviewed commit.\nname: ci\non:\n  push: {{}}\n  pull_request: {{}}\njobs:\n  dx:\n    uses: {module}/.github/workflows/reusable-consumer.yml@<reviewed-commit>\n"
            ),
        },
        ScaffoldFile {
            path: "MODULE.bazel.snippet".to_owned(),
            content: format!(
                "# Add to MODULE.bazel:\nbazel_dep(name = \"rules_dx\", version = \"{DX_VERSION}\")\n# module: {module}\n"
            ),
        },
    ]
}

/// Apply the init scaffold under `root`, writing absent-only.
///
/// Returns the written workspace-relative paths. Existing files are left
/// untouched and reported as refusals in the returned `Vec` prefix
/// `refused:` entries follow written entries after a `---` separator.
pub fn apply_init(root: &Path, module_name: &str) -> Result<Vec<String>, AdoptError> {
    let mut written = Vec::new();
    let mut refused = Vec::new();
    for file in plan_init_files(module_name) {
        let dest = root.join(&file.path);
        if dest.exists() {
            refused.push(format!("refused:{}", file.path));
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AdoptError::CreateParent {
                parent: parent.display().to_string(),
                detail: e.to_string(),
            })?;
        }
        dx_atomic_fs::write_atomic(&dest, file.content.as_ref()).map_err(|e| {
            AdoptError::WriteFile {
                path: dest.display().to_string(),
                detail: e.to_string(),
            }
        })?;
        written.push(file.path);
    }
    written.push("---".to_owned());
    written.extend(refused);
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::super::{
        absent_only_write_allowed, apply_init, init_must_refuse, plan_init_files, DEVCONTAINER_JSON,
    };
    use super::DEVCONTAINER_JSON as LOCAL_DEVCONTAINER;

    #[test]
    fn scaffold_reexports_match_local_definitions() {
        assert_eq!(DEVCONTAINER_JSON, LOCAL_DEVCONTAINER);
    }

    #[test]
    fn init_writes_only_absent_paths() {
        assert!(absent_only_write_allowed(false));
        assert!(!absent_only_write_allowed(true));
    }

    #[test]
    fn init_refuses_existing_paths_even_with_force() {
        assert!(init_must_refuse(true, false));
        assert!(init_must_refuse(true, true));
        assert!(!init_must_refuse(false, false));
        assert!(!init_must_refuse(false, true));
    }

    #[test]
    fn init_plans_eight_absent_only_files() {
        let files = plan_init_files("demo");
        assert_eq!(files.len(), 8);
        assert!(files.iter().any(|f| f.path == ".dx/version"));
        assert!(files
            .iter()
            .any(|f| f.path == ".devcontainer/devcontainer.json"));
        assert!(files.iter().any(|f| f.path == ".vscode/settings.json"));
    }

    #[test]
    fn devcontainer_scaffold_runs_bootstrap_not_full_build() {
        // Issue #183 (snapshot workflow #322): the scaffolded definition
        // must stay admissible and bootstrap-shaped. The snapshot with the
        // checked-in definition lives in
        // //.devcontainer:devcontainer_parity_test; this pins the
        // contract fields here so scaffold drift fails at the source.
        let files = plan_init_files("demo");
        let scaffold = files
            .iter()
            .find(|f| f.path == ".devcontainer/devcontainer.json")
            .expect("devcontainer scaffold");
        assert_eq!(scaffold.content, DEVCONTAINER_JSON);
        let parsed: serde_json::Value =
            serde_json::from_str(&scaffold.content).expect("valid JSON");
        assert_eq!(parsed["name"], serde_json::Value::from("rules_dx"));
        assert_eq!(
            parsed["image"],
            serde_json::Value::from("mcr.microsoft.com/devcontainers/base:ubuntu")
        );
        let post_create = parsed["postCreateCommand"]
            .as_str()
            .expect("postCreateCommand string");
        assert!(
            post_create.contains("bazel run //dx:env"),
            "bootstrap first: {post_create}"
        );
        assert!(
            !post_create.contains("bazel build //..."),
            "no full build on create: {post_create}"
        );
        assert!(super::super::devcontainer_is_admissible(true, true, false));
    }

    #[test]
    fn apply_init_writes_absent_only_and_refuses_existing() {
        let scratch = dx_test_scratch::scratch("dx-adopt-init-");
        let root = scratch.path().to_path_buf();
        std::fs::create_dir_all(&root).expect("tmp");
        let first = apply_init(&root, "demo").expect("init");
        assert!(first.iter().any(|p| p == ".dx/version"));
        assert!(root.join(".dx/version").exists());
        std::fs::write(root.join(".dx/version"), "custom\n").expect("custom");
        let second = apply_init(&root, "demo").expect("init again");
        assert!(second.iter().any(|p| p == "refused:.dx/version"));
        assert_eq!(
            std::fs::read_to_string(root.join(".dx/version")).expect("read"),
            "custom\n"
        );
        scratch.close().expect("cleanup");
    }
}
