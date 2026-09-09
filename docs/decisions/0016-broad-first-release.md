# ADR 0016: Broad First Release

## Status

Accepted.

## Context

The initial delivery plan restricted application foundations while already planning broad quality
tool parity. The user requires v1 to include broadly working languages, tools, and workflows when
existing upstream implementations allow hermetic integration without building entire stacks.

## Decision

This record captures the user's broad-first-release direction. The authoritative admission
and maintenance policy is [Product Scope](../product/scope.md#first-release-admission); this
record does not restate its thresholds. Required core/framework scope, additional-foundation
admission, quality-tool independence, patch/packaging routes, and support gates follow that
policy.

Prefer upstream-supported automatic workflows per the cross-language policy in
[Automatic Workflows](../product/scope.md#automatic-workflows); this record adds no separate
automation rule.

The user also requires 100% coverage of non-ignored executable first-party lines per the
authoritative policy in [Testing](../testing/README.md#coverage); this record adds no separate
coverage rule.

## Consequences

Rust-first dogfooding remains the delivery strategy. The support inventory and delivery gates
distinguish required core/frameworks, admitted additional foundations, and approved deferrals,
without weakening independent quality-tool obligations.
Remaining upstream selections, feasibility evidence, coverage instrumentation, and API choices remain open;
this decision does not claim any implementation or approve an implementation milestone.
The 2026-09-08 admit/defer/exclude dispositions are recorded in
[ADR 0019](0019-first-release-additional-foundations.md).

Blanket post-v1 language deferral and host-installed tools as a shortcut are rejected. Windows
retains hermetic acquisition under [ADR 0014](0014-tested-platform-release-stack.md#decision). Conflicts
with existing generation or platform contracts require explicit decisions rather than hidden
exceptions. There is no shipped implementation or compatibility migration at this design stage.
