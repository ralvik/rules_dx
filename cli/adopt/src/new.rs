//! `dx new` minimal project scaffolding per language.
//!
//! Split from `super` (`lib.rs`): owns `SUPPORTED_NEW_LANGUAGES`,
//! `normalize_new_language`, `new_is_known_language`, `plan_new_files`,
//! and `apply_new`. Re-exported through `super` so the public path stays
//! `dx_adopt::{plan_new_files, apply_new, ...}`.
//!
//! Templates stay stdlib-only so `dx generate` mappings accept them
//! without ecosystem manifests/locks: one hello source plus the minimal
//! foreign-layout marker (`Cargo.toml`, `pyproject.toml`, `package.json`,
//! `go.mod`, `pom.xml`, `build.sbt`, SDK-style project, or C++ pair).
//! See: `docs/cli/commands/new-upgrade.md`.

use std::path::Path;

use super::{AdoptError, ScaffoldFile};

/// Languages `dx new` scaffolds, matching the `dx generate` mappings.
///
/// `c`/`cc` are aliases for `cpp` (C/C++ share the `cc` Gazelle path);
/// `c#`/`f#` spellings are accepted for ergonomics and normalize to
/// `csharp`/`fsharp`.
pub const SUPPORTED_NEW_LANGUAGES: &[&str] = &[
    "rust",
    "python",
    "javascript",
    "typescript",
    "go",
    "java",
    "kotlin",
    "scala",
    "csharp",
    "fsharp",
    "c",
    "cc",
    "cpp",
];

/// Normalize a user-supplied language to its canonical template key.
///
/// Returns `None` for unsupported spellings (rejected pre-exec, never
/// defaulted silently).
pub fn normalize_new_language(language: &str) -> Option<&'static str> {
    match language {
        "rust" => Some("rust"),
        "python" => Some("python"),
        "javascript" => Some("javascript"),
        "typescript" => Some("typescript"),
        "go" => Some("go"),
        "java" => Some("java"),
        "kotlin" => Some("kotlin"),
        "scala" => Some("scala"),
        "csharp" | "c#" => Some("csharp"),
        "fsharp" | "f#" => Some("fsharp"),
        "c" | "cc" | "cpp" => Some("cpp"),
        _ => None,
    }
}

/// Whether `language` is a known `dx new` template.
pub fn new_is_known_language(language: &str) -> bool {
    normalize_new_language(language).is_some()
}

/// Default project directory when `dx new <lang>` omits the name.
pub fn default_new_name() -> &'static str {
    "my_project"
}

/// Plan the `dx new` scaffold: absent-only repo wiring under `<name>/`
/// (the `dx init` files, prefixed) plus one minimal stdlib-only project
/// for `language`.
///
/// All writes are absent-only like `dx init`; the caller refuses existing
/// paths (there is no overwrite flag). Contents are pinned (no network).
pub fn plan_new_files(language: &str, name: &str) -> Result<Vec<ScaffoldFile>, AdoptError> {
    let canonical =
        normalize_new_language(language).ok_or_else(|| AdoptError::NewUnknownLanguage {
            language: language.to_owned(),
        })?;
    let project = if name.is_empty() {
        default_new_name()
    } else {
        name
    };
    let mut files = Vec::new();
    for file in super::plan_init_files(project) {
        files.push(ScaffoldFile {
            path: format!("{project}/{}", file.path),
            content: file.content,
        });
    }
    for (suffix, content) in new_language_files(canonical, project) {
        files.push(ScaffoldFile {
            path: format!("{project}/{suffix}"),
            content,
        });
    }
    Ok(files)
}

/// Minimal stdlib-only sources per canonical language.
///
/// Each pair mirrors the corresponding `examples/adopt-*` foreign layout
/// and `*/tests/fixtures/hello/` seed shape at stdlib-only scope, so the
/// Gazelle extensions own BUILD decisions on first `dx generate`.
fn new_language_files(canonical: &str, project: &str) -> Vec<(String, String)> {
    match canonical {
        "rust" => vec![
            (
                "Cargo.toml".to_owned(),
                format!(
                    "[package]\nname = \"{project}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"
                ),
            ),
            (
                "src/main.rs".to_owned(),
                "fn main() {\n    println!(\"hello world\");\n}\n".to_owned(),
            ),
        ],
        "python" => vec![
            (
                "pyproject.toml".to_owned(),
                format!(
                    "[project]\nname = \"{project}\"\nversion = \"0.1.0\"\nrequires-python = \">=3.12\"\ndependencies = []\n"
                ),
            ),
            (
                "hello.py".to_owned(),
                "\"\"\"Greeting helper with no dependencies.\"\"\"\n\n\ndef greet(name):\n    \"\"\"Return a greeting for name.\"\"\"\n    return f\"Hello, {{name}}!\"\n".to_owned(),
            ),
        ],
        "javascript" => vec![
            (
                "package.json".to_owned(),
                format!(
                    "{{\n  \"name\": \"{project}\",\n  \"private\": true,\n  \"type\": \"module\"\n}}\n"
                ),
            ),
            (
                "hello.js".to_owned(),
                "export function hello(name) {\n\treturn `hello ${name}`;\n}\n".to_owned(),
            ),
        ],
        "typescript" => vec![
            (
                "package.json".to_owned(),
                format!(
                    "{{\n  \"name\": \"{project}\",\n  \"private\": true,\n  \"type\": \"module\"\n}}\n"
                ),
            ),
            (
                "tsconfig.json".to_owned(),
                "{\n  \"compilerOptions\": {\n    \"module\": \"ESNext\",\n    \"target\": \"ES2022\",\n    \"strict\": true\n  }\n}\n"
                    .to_owned(),
            ),
            (
                "hello.ts".to_owned(),
                "export function hello(name: string): string {\n\treturn `hello ${name}`;\n}\n"
                    .to_owned(),
            ),
        ],
        "go" => vec![
            (
                "go.mod".to_owned(),
                format!("module {project}\n\ngo 1.26\n"),
            ),
            (
                "hello.go".to_owned(),
                "package hello\n\n// Hello returns a greeting for name.\nfunc Hello(name string) string {\n\treturn \"hello \" + name\n}\n"
                    .to_owned(),
            ),
        ],
        "java" => vec![
            (
                "pom.xml".to_owned(),
                format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<project xmlns=\"http://maven.apache.org/POM/4.0.0\">\n  <modelVersion>4.0.0</modelVersion>\n  <groupId>example.com</groupId>\n  <artifactId>{project}</artifactId>\n  <version>0.1.0</version>\n  <packaging>jar</packaging>\n</project>\n"
                ),
            ),
            (
                "Hello.java".to_owned(),
                "package hello;\n\npublic class Hello {\n  public static String hello(String name) {\n    return \"hello \" + name;\n  }\n}\n"
                    .to_owned(),
            ),
        ],
        "kotlin" => vec![
            (
                "pom.xml".to_owned(),
                format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<project xmlns=\"http://maven.apache.org/POM/4.0.0\">\n  <modelVersion>4.0.0</modelVersion>\n  <groupId>example.com</groupId>\n  <artifactId>{project}</artifactId>\n  <version>0.1.0</version>\n  <packaging>jar</packaging>\n</project>\n"
                ),
            ),
            (
                "Hello.kt".to_owned(),
                "package hello\n\nobject Hello {\n  fun hello(name: String): String {\n    return \"hello \" + name\n  }\n}\n"
                    .to_owned(),
            ),
        ],
        "scala" => vec![
            (
                "build.sbt".to_owned(),
                format!(
                    "ThisBuild / scalaVersion := \"2.13.18\"\nThisBuild / organization := \"example.com\"\nlazy val root = (project in file(\".\")).settings(name := \"{project}\")\n"
                ),
            ),
            (
                "Hello.scala".to_owned(),
                "package hello\n\nobject Hello {\n  def hello(name: String): String = \"hello \" + name\n}\n"
                    .to_owned(),
            ),
        ],
        "csharp" => vec![
            (
                format!("{project}.csproj"),
                "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net10.0</TargetFramework>\n    <Nullable>enable</Nullable>\n  </PropertyGroup>\n</Project>\n"
                    .to_owned(),
            ),
            (
                "Hello.cs".to_owned(),
                "namespace Hello;\n\npublic static class Greeter\n{\n    public static string Greet(string name)\n    {\n        return \"hello \" + name;\n    }\n}\n"
                    .to_owned(),
            ),
        ],
        "fsharp" => vec![
            (
                format!("{project}.fsproj"),
                "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net10.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <Compile Include=\"Library.fs\" />\n  </ItemGroup>\n</Project>\n"
                    .to_owned(),
            ),
            (
                "Library.fs".to_owned(),
                "module Hello\n\nlet greet name = \"hello \" + name\n".to_owned(),
            ),
        ],
        _ => vec![
            (
                "hello.cc".to_owned(),
                "#include \"hello.h\"\n\nstd::string Hello(const std::string& name) {\n  return \"hello \" + name;\n}\n"
                    .to_owned(),
            ),
            (
                "hello.h".to_owned(),
                "#pragma once\n\n#include <string>\n\nstd::string Hello(const std::string& name);\n"
                    .to_owned(),
            ),
        ],
    }
}

/// Apply the `dx new` scaffold under `root`, writing absent-only.
///
/// Returns the written workspace-relative paths with `refused:` entries
/// after a `---` separator, mirroring [`super::apply_init`].
pub fn apply_new(root: &Path, language: &str, name: &str) -> Result<Vec<String>, AdoptError> {
    let mut written = Vec::new();
    let mut refused = Vec::new();
    for file in plan_new_files(language, name)? {
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
    use super::super::DX_VERSION;
    use super::*;

    #[test]
    fn new_aliases_normalize_to_canonical_templates() {
        assert_eq!(normalize_new_language("rust"), Some("rust"));
        assert_eq!(normalize_new_language("c"), Some("cpp"));
        assert_eq!(normalize_new_language("cc"), Some("cpp"));
        assert_eq!(normalize_new_language("cpp"), Some("cpp"));
        assert_eq!(normalize_new_language("c#"), Some("csharp"));
        assert_eq!(normalize_new_language("f#"), Some("fsharp"));
        assert_eq!(normalize_new_language("ruby"), None);
        assert!(new_is_known_language("go"));
        assert!(!new_is_known_language("swift"));
        assert_eq!(SUPPORTED_NEW_LANGUAGES.len(), 13);
    }

    #[test]
    fn new_plans_init_wiring_plus_language_sources() {
        // See: `docs/cli/commands/new-upgrade.md`.
        for lang in ["rust", "go", "java", "cpp", "python", "typescript"] {
            let files = plan_new_files(lang, "demo").expect("plans");
            assert!(
                files.iter().any(|f| f.path == "demo/.dx/version"),
                "{lang} carries init wiring"
            );
            assert!(
                files.iter().any(|f| f.path == "demo/.vscode/settings.json"),
                "{lang} carries editor wiring"
            );
        }
        let rust = plan_new_files("rust", "demo").expect("rust");
        assert!(rust.iter().any(|f| f.path == "demo/Cargo.toml"));
        assert!(rust.iter().any(|f| f.path == "demo/src/main.rs"));
        let go = plan_new_files("go", "demo").expect("go");
        assert!(go.iter().any(|f| f.path == "demo/go.mod"));
        let java = plan_new_files("java", "demo").expect("java");
        assert!(java.iter().any(|f| f.path == "demo/Hello.java"));
        let cpp = plan_new_files("c", "demo").expect("c alias");
        assert!(cpp.iter().any(|f| f.path == "demo/hello.cc"));
        let version = rust
            .iter()
            .find(|f| f.path == "demo/.dx/version")
            .expect("version");
        assert_eq!(version.content, format!("{DX_VERSION}\n"));
        assert!(plan_new_files("ruby", "demo").is_err());
        assert_eq!(
            plan_new_files("ruby", "demo").unwrap_err().to_string(),
            "unknown language for dx new: ruby (want one of rust, python, javascript, typescript, go, java, kotlin, scala, csharp, fsharp, c, cc, cpp)"
        );
    }

    #[test]
    fn new_templates_are_stdlib_only_for_generate() {
        // No invented ecosystem semantics: templates carry no external
        // dependencies, so `dx generate` owns BUILD decisions.
        let rust = plan_new_files("rust", "demo").expect("rust");
        let cargo = rust
            .iter()
            .find(|f| f.path == "demo/Cargo.toml")
            .expect("cargo");
        assert!(!cargo.content.contains("anyhow"));
        let python = plan_new_files("python", "demo").expect("python");
        let pyproject = python
            .iter()
            .find(|f| f.path == "demo/pyproject.toml")
            .expect("pyproject");
        assert!(pyproject.content.contains("dependencies = []"));
        let js = plan_new_files("javascript", "demo").expect("js");
        let package = js
            .iter()
            .find(|f| f.path == "demo/package.json")
            .expect("package");
        assert!(!package.content.contains("jest"));
    }

    #[test]
    fn apply_new_writes_absent_only_and_refuses_existing() {
        let scratch = dx_test_scratch::scratch("dx-adopt-new-");
        let root = scratch.path().to_path_buf();
        std::fs::create_dir_all(&root).expect("tmp");
        let first = apply_new(&root, "go", "demo").expect("new");
        assert!(first.iter().any(|p| p == "demo/go.mod"));
        assert!(root.join("demo/go.mod").exists());
        assert!(root.join("demo/.dx/version").exists());
        let second = apply_new(&root, "go", "demo").expect("new again");
        assert!(second.iter().any(|p| p == "refused:demo/go.mod"));
        scratch.close().expect("cleanup");
    }
}
