//! Real-tool fix application (split from `real.rs`). No behavior change.
//! `RealBackend::apply_fix` plus fix scratch helpers, moved verbatim.
//!
//! JVM notes: google-java-format and ktfmt rewrite in place via the
//! shared `run_fix` (re-read on exit 0); ktlint lint fixes via
//! `--format` (re-read on exit 0 or 1 like ESLint); Checkstyle, PMD,
//! and SpotBugs are check-only and return their input. Scala/.NET
//! notes: Scalafmt, CSharpier, and Fantomas rewrite in place;
//! Scalafix, Roslyn, and FSharpLint are check-only. Structured notes:
//! Buf format plus qmlformat rewrite in place; Buf lint plus qmllint
//! are check-only with the provisional sandbox-apply-and-diff fix flow.

use super::*;

impl super::RealBackend {
    /// Applies one fix round to a single file's bytes and returns the
    /// result. Format tools run their in-place fix and the bytes are
    /// re-read; Ruff follows the running capability (`check --fix` for
    /// lint, `format` for format) and its lint fix re-reads on exit 0
    /// or 1 (exit 1 signals remaining unfixable findings after the
    /// fixable ones were applied); ESLint likewise re-reads on exit 0
    /// or 1; Clippy is check-only (its suggestions ride the frozen
    /// upstream diagnostics and cannot track converged bytes, so fixes
    /// never rewrite); Biome lint is
    /// check-only and converges on format; Clippy, Vale, the Markdown
    /// checker, rustc typecheck, Ty, pydoclint, flake8, and pylint return
    /// their input. The native lint cohort (clang-tidy, cppcheck,
    /// staticcheck, govet, errcheck) is check-only with the provisional
    /// sandbox-apply-and-diff fix flow and returns its input; the
    /// native formatters (clang-format, gofumpt) rewrite in place.
    pub fn apply_fix(
        &self,
        tool_id: &str,
        path: &str,
        text: &str,
        capability: &str,
    ) -> Result<String, RunnerError> {
        let tool = self.tool(tool_id)?;
        match tool_id {
            "rustfmt" | "buildifier" | "taplo" | "google_java_format" | "ktfmt" => {
                self.run_fix(tool_id, tool, path, text)
            }
            "ruff" => self.run_ruff_fix(tool, path, text, capability == "format"),
            "vale" | "markdown_check" | "rustc" | "ty" | "pydoclint" | "flake8" | "pylint"
            | "clippy" | "scalafix" | "roslyn" | "fsharplint" | "checkstyle" | "pmd"
            | "spotbugs" | "qmllint" | "clang_tidy" | "cppcheck" | "staticcheck" | "govet"
            | "errcheck" => Ok(text.to_owned()),
            "buf" => {
                if capability == "format" {
                    self.run_buf_format_fix(tool, path, text)
                } else {
                    Ok(text.to_owned())
                }
            }
            "biome" => {
                if capability == "format" {
                    self.run_biome_format_fix(tool, path, text)
                } else {
                    Ok(text.to_owned())
                }
            }
            "prettier" => {
                if capability == "format" {
                    self.run_prettier_fix(tool, path, text)
                } else {
                    Ok(text.to_owned())
                }
            }
            "scalafmt" => self.run_scalafmt_fix(tool, path, text),
            "csharpier" => self.run_csharpier_fix(tool, path, text),
            "fantomas" => self.run_fantomas_fix(tool, path, text),
            "clang_format" => self.run_clang_format_fix(tool, path, text),
            "gofumpt" => self.run_gofumpt_fix(tool, path, text),
            "qmlformat" => self.run_qmlformat_fix(tool, path, text),
            "eslint" => self.run_eslint_fix(tool, path, text),
            "ktlint" => self.run_ktlint_fix(tool, path, text),
            _ => Err(execution(
                tool_id,
                format!("unsupported real tool: {tool_id}"),
            )),
        }
    }

    /// Stages one fix scratch tree with the exact file bytes plus the
    /// tool files, returning the scratch and the file's absolute path.
    fn fix_scratch(
        &self,
        tool_id: &str,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<(Scratch, PathBuf), RunnerError> {
        let scratch = fresh_scratch(&self.scratch_parent, tool_id)?;
        let mut mirrors = vec![MirrorFile {
            mirror_rel: PathBuf::from(path),
            contents: MirrorContents::Bytes(text.as_bytes().to_vec()),
        }];
        mirrors.extend(Self::mirror_tool_files(tool_id, tool));
        write_all(&scratch, tool_id, &mirrors)?;
        let absolute = scratch.root().join(path);
        Ok((scratch, absolute))
    }

    /// Re-reads a fixed file as UTF-8. Re-read failures fail the action;
    /// non-UTF-8 fix output is a tool-output failure, never silent bytes.
    fn reread_fixed(tool_id: &str, absolute: &Path) -> Result<String, RunnerError> {
        let fixed = std::fs::read(absolute)
            .map_err(|err| execution(tool_id, format!("re-read fixed file: {err}")))?;
        String::from_utf8(fixed).map_err(|err| RunnerError::ToolOutput {
            tool_id: tool_id.to_owned(),
            detail: format!("fixed file is not UTF-8: {err}"),
        })
    }

    fn run_fix(
        &self,
        tool_id: &str,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<String, RunnerError> {
        let (scratch, absolute) = self.fix_scratch(tool_id, tool, path, text)?;
        let refs = [absolute.as_path()];
        let config = self.config_abs(tool_id, tool, &scratch)?;
        let cwd_rel = Self::cwd_rel(tool_id, tool.config_rel.as_deref());
        let invocation = match tool_id {
            "rustfmt" => {
                let Some(cfg) = config.as_ref() else {
                    return Err(execution(tool_id, "rustfmt requires a config".to_owned()));
                };
                commands::rustfmt(
                    &tool.binary,
                    &refs,
                    cfg,
                    Self::rustfmt_edition(tool)?,
                    false,
                )
            }
            "buildifier" => commands::buildifier_fix(
                &tool.binary,
                &refs,
                hint_dir(tool.config_rel.as_deref(), &cwd_rel),
            ),
            "google_java_format" => commands::google_java_format_fix(&tool.binary, &refs),
            "ktfmt" => commands::ktfmt_fix(&tool.binary, &refs),
            _ => commands::taplo_format(&tool.binary, &refs, config.as_deref(), false),
        };
        let out = self.run(tool_id, tool, &invocation, &scratch)?;
        if out.code != Some(0) {
            return cleaned(tool_id, scratch, text.to_owned());
        }
        let fixed = Self::reread_fixed(tool_id, &absolute)?;
        cleaned(tool_id, scratch, fixed)
    }

    /// Runs one Ruff fix round: `format` for format pipelines,
    /// `check --fix` for everything else. The format fix re-reads only
    /// on exit 0 like every other format tool; the lint fix re-reads on
    /// exit 0 or 1 because exit 1 signals remaining unfixable findings
    /// after the fixable ones were applied. Any other exit keeps the
    /// input: the check diagnostics report the cause.
    fn run_ruff_fix(
        &self,
        tool: &RealTool,
        path: &str,
        text: &str,
        format: bool,
    ) -> Result<String, RunnerError> {
        const TOOL_ID: &str = "ruff";
        let (scratch, absolute) = self.fix_scratch(TOOL_ID, tool, path, text)?;
        let refs = [absolute.as_path()];
        let config = self.config_abs(TOOL_ID, tool, &scratch)?;
        let invocation = if format {
            commands::ruff_format_fix(&tool.binary, &refs, config.as_deref())
        } else {
            commands::ruff_fix(&tool.binary, &refs, config.as_deref())
        };
        let out = self.run(TOOL_ID, tool, &invocation, &scratch)?;
        let keep_input = if format {
            out.code != Some(0)
        } else {
            out.code != Some(0) && out.code != Some(1)
        };
        if keep_input {
            return cleaned(TOOL_ID, scratch, text.to_owned());
        }
        let fixed = Self::reread_fixed(TOOL_ID, &absolute)?;
        cleaned(TOOL_ID, scratch, fixed)
    }

    /// Runs one Biome format fix round: `format --write` (in-place).
    /// Re-reads only on exit 0 like every other format tool; any other
    /// exit keeps the input and the check diagnostics report the cause.
    fn run_biome_format_fix(
        &self,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<String, RunnerError> {
        const TOOL_ID: &str = "biome";
        let (scratch, absolute) = self.fix_scratch(TOOL_ID, tool, path, text)?;
        let refs = [absolute.as_path()];
        let config_dir = Self::biome_config_dir(tool, &scratch)?;
        let invocation = commands::biome_format_fix(&tool.binary, &refs, &config_dir);
        let out = self.run(TOOL_ID, tool, &invocation, &scratch)?;
        if out.code != Some(0) {
            return cleaned(TOOL_ID, scratch, text.to_owned());
        }
        let fixed = Self::reread_fixed(TOOL_ID, &absolute)?;
        cleaned(TOOL_ID, scratch, fixed)
    }

    /// Runs one Prettier format fix round: `--write` (in-place).
    /// Re-reads only on exit 0; any other exit keeps the input.
    fn run_prettier_fix(
        &self,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<String, RunnerError> {
        const TOOL_ID: &str = "prettier";
        let (scratch, absolute) = self.fix_scratch(TOOL_ID, tool, path, text)?;
        let refs = [absolute.as_path()];
        let invocation = commands::prettier_fix(&tool.binary, &refs);
        let out = self.run(TOOL_ID, tool, &invocation, &scratch)?;
        if out.code != Some(0) {
            return cleaned(TOOL_ID, scratch, text.to_owned());
        }
        let fixed = Self::reread_fixed(TOOL_ID, &absolute)?;
        cleaned(TOOL_ID, scratch, fixed)
    }

    /// Runs one ESLint lint fix round: `-c <config> --fix` (in-place).
    /// Re-reads on exit 0 or 1 because exit 1 signals remaining
    /// unfixable findings after the fixable ones were applied, mirroring
    /// the Ruff lint-fix contract. Any other exit keeps the input; a
    /// missing config fails the action (ESLint has no usable defaults).
    fn run_eslint_fix(
        &self,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<String, RunnerError> {
        const TOOL_ID: &str = "eslint";
        let (scratch, absolute) = self.fix_scratch(TOOL_ID, tool, path, text)?;
        let refs = [absolute.as_path()];
        let config = self.config_abs(TOOL_ID, tool, &scratch)?;
        let Some(cfg) = config.as_deref() else {
            return Err(execution(TOOL_ID, "eslint requires a config".to_owned()));
        };
        let invocation = commands::eslint_fix(&tool.binary, &refs, cfg);
        let out = self.run(TOOL_ID, tool, &invocation, &scratch)?;
        if out.code != Some(0) && out.code != Some(1) {
            return cleaned(TOOL_ID, scratch, text.to_owned());
        }
        let fixed = Self::reread_fixed(TOOL_ID, &absolute)?;
        cleaned(TOOL_ID, scratch, fixed)
    }

    /// Runs one Scalafmt format fix round: in-place rewrite.
    /// Re-reads only on exit 0; any other exit keeps the input.
    fn run_scalafmt_fix(
        &self,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<String, RunnerError> {
        const TOOL_ID: &str = "scalafmt";
        let (scratch, absolute) = self.fix_scratch(TOOL_ID, tool, path, text)?;
        let refs = [absolute.as_path()];
        let config = self.config_abs(TOOL_ID, tool, &scratch)?;
        let invocation = commands::scalafmt_fix(&tool.binary, &refs, config.as_deref());
        let out = self.run(TOOL_ID, tool, &invocation, &scratch)?;
        if out.code != Some(0) {
            return cleaned(TOOL_ID, scratch, text.to_owned());
        }
        let fixed = Self::reread_fixed(TOOL_ID, &absolute)?;
        cleaned(TOOL_ID, scratch, fixed)
    }

    /// Runs one CSharpier format fix round: `format` (in-place).
    /// Re-reads only on exit 0; any other exit keeps the input.
    fn run_csharpier_fix(
        &self,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<String, RunnerError> {
        const TOOL_ID: &str = "csharpier";
        let (scratch, absolute) = self.fix_scratch(TOOL_ID, tool, path, text)?;
        let refs = [absolute.as_path()];
        let config = self.config_abs(TOOL_ID, tool, &scratch)?;
        let invocation = commands::csharpier_fix(&tool.binary, &refs, config.as_deref());
        let out = self.run(TOOL_ID, tool, &invocation, &scratch)?;
        if out.code != Some(0) {
            return cleaned(TOOL_ID, scratch, text.to_owned());
        }
        let fixed = Self::reread_fixed(TOOL_ID, &absolute)?;
        cleaned(TOOL_ID, scratch, fixed)
    }

    /// Runs one Fantomas format fix round: in-place format.
    /// Re-reads only on exit 0; any other exit keeps the input.
    fn run_fantomas_fix(
        &self,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<String, RunnerError> {
        const TOOL_ID: &str = "fantomas";
        let (scratch, absolute) = self.fix_scratch(TOOL_ID, tool, path, text)?;
        let refs = [absolute.as_path()];
        let invocation = commands::fantomas_fix(&tool.binary, &refs);
        let out = self.run(TOOL_ID, tool, &invocation, &scratch)?;
        if out.code != Some(0) {
            return cleaned(TOOL_ID, scratch, text.to_owned());
        }
        let fixed = Self::reread_fixed(TOOL_ID, &absolute)?;
        cleaned(TOOL_ID, scratch, fixed)
    }

    /// Runs one ktlint lint fix round: `--relative --format`
    /// (in-place). Re-reads on exit 0 or 1 because exit 1 signals
    /// remaining unfixable findings after the fixable ones were
    /// applied, mirroring the ESLint/Ruff lint-fix contract. Any other
    /// exit keeps the input.
    fn run_ktlint_fix(
        &self,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<String, RunnerError> {
        const TOOL_ID: &str = "ktlint";
        let (scratch, absolute) = self.fix_scratch(TOOL_ID, tool, path, text)?;
        let refs = [absolute.as_path()];
        let invocation = commands::ktlint_fix(&tool.binary, &refs);
        let out = self.run(TOOL_ID, tool, &invocation, &scratch)?;
        if out.code != Some(0) && out.code != Some(1) {
            return cleaned(TOOL_ID, scratch, text.to_owned());
        }
        let fixed = Self::reread_fixed(TOOL_ID, &absolute)?;
        cleaned(TOOL_ID, scratch, fixed)
    }

    /// Runs one Buf format fix round: `format --write` (in-place).
    /// Re-reads only on exit 0; any other exit keeps the input.
    fn run_buf_format_fix(
        &self,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<String, RunnerError> {
        const TOOL_ID: &str = "buf";
        let (scratch, absolute) = self.fix_scratch(TOOL_ID, tool, path, text)?;
        let refs = [absolute.as_path()];
        let invocation = commands::buf_format_fix(&tool.binary, &refs);
        let out = self.run(TOOL_ID, tool, &invocation, &scratch)?;
        if out.code != Some(0) {
            return cleaned(TOOL_ID, scratch, text.to_owned());
        }
        let fixed = Self::reread_fixed(TOOL_ID, &absolute)?;
        cleaned(TOOL_ID, scratch, fixed)
    }

    /// Runs one clang-format fix round: `-i` (in-place).
    /// Re-reads only on exit 0; any other exit keeps the input.
    fn run_clang_format_fix(
        &self,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<String, RunnerError> {
        const TOOL_ID: &str = "clang_format";
        let (scratch, absolute) = self.fix_scratch(TOOL_ID, tool, path, text)?;
        let refs = [absolute.as_path()];
        let config = self.config_abs(TOOL_ID, tool, &scratch)?;
        let invocation = commands::clang_format_fix(&tool.binary, &refs, config.as_deref());
        let out = self.run(TOOL_ID, tool, &invocation, &scratch)?;
        if out.code != Some(0) {
            return cleaned(TOOL_ID, scratch, text.to_owned());
        }
        let fixed = Self::reread_fixed(TOOL_ID, &absolute)?;
        cleaned(TOOL_ID, scratch, fixed)
    }

    /// Runs one gofumpt fix round: `-w` (in-place).
    /// Re-reads only on exit 0; any other exit keeps the input.
    fn run_gofumpt_fix(
        &self,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<String, RunnerError> {
        const TOOL_ID: &str = "gofumpt";
        let (scratch, absolute) = self.fix_scratch(TOOL_ID, tool, path, text)?;
        let refs = [absolute.as_path()];
        let invocation = commands::gofumpt_fix(&tool.binary, &refs);
        let out = self.run(TOOL_ID, tool, &invocation, &scratch)?;
        if out.code != Some(0) {
            return cleaned(TOOL_ID, scratch, text.to_owned());
        }
        let fixed = Self::reread_fixed(TOOL_ID, &absolute)?;
        cleaned(TOOL_ID, scratch, fixed)
    }

    /// Runs one qmlformat fix round: `-i` (in-place).
    /// Re-reads only on exit 0; any other exit keeps the input.
    fn run_qmlformat_fix(
        &self,
        tool: &RealTool,
        path: &str,
        text: &str,
    ) -> Result<String, RunnerError> {
        const TOOL_ID: &str = "qmlformat";
        let (scratch, absolute) = self.fix_scratch(TOOL_ID, tool, path, text)?;
        let refs = [absolute.as_path()];
        let invocation = commands::qmlformat_fix(&tool.binary, &refs);
        let out = self.run(TOOL_ID, tool, &invocation, &scratch)?;
        if out.code != Some(0) {
            return cleaned(TOOL_ID, scratch, text.to_owned());
        }
        let fixed = Self::reread_fixed(TOOL_ID, &absolute)?;
        cleaned(TOOL_ID, scratch, fixed)
    }
}
