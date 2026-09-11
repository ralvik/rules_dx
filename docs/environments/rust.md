# Rust Environment

Rust contributes provider-derived plans to the private environment collection described by
[Developer Environments](environment.md). Shared identity, installation, selection, and retention
follow [Managed Environment State](managed-state.md).

## Toolchain And Source Projection

The plan uses the exact Rust toolchain selected for the configured target and the authoritative
crate, dependency, generated-source, proc-macro, and build-script information exposed by the pinned
`rules_rs` and patched `rules_rust` providers. It does not reconstruct Cargo metadata, scan the
checkout, or synthesize a static crate graph.

The repository-default plan covers the declared packages selected by the approved repository-root
strategy. A focused target plan contains only its configured transitive closure. Both plans preserve
the upstream package and crate boundaries.

## IDE Integration

Rust IDE support reuses the patched upstream rust-analyzer discovery and Bazel flycheck pipeline.
Repository-default operation remains package-scoped; focused operation is constrained to the
selected target closure and uses a separate IDE Bazel output base.

Dynamic IDE refresh may invoke Bazel, but it never invokes `dx codegen`, changes the selected
environment or generated-code generation, or writes a project-owned crate graph.

## Evidence

The focused Rust foundation proves provider completeness, proc macros, build-script outputs,
generated-source visibility, version selection, required platforms, and exact-target isolation. The
focused environment plan is the provider-derived `rust_env_plan` rule over one `dx_rust_*` wrapper
(`rust/env/plan.bzl`, pinned by `//rust/env:env_plan_tests`); IDE reuse is pinned by upstream
`gen_rust_project` + `flycheck` acquisition (`//rust/ide:ide_acquisition_test`) with focused
exact-target projection recorded in the [M12 completion report](../milestones/M12-completion-report.md#wp3-compiler-diagnostics-ide-coverage-and-environment-projection).
The public `dx env` repository/root/exact-target orchestration and atomic selection are delivered by the
repository-workflow milestone.
