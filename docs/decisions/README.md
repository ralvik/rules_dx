# Decision Records

Decision records use zero-padded numbers and these statuses: Accepted, Provisional, Superseded, or
Rejected. Records carry a `Date:` line only where the decision date is evidenced; the table
shows `—` otherwise. This log is rewritten in place until the first release.
Accepted records define durable current constraints. Provisional records require
validation and are not stable commitments.

| Record | Status | Date | Domain |
| --- | --- | --- | --- |
| [0001: Bazel Owns Execution](0001-bazel-owns-execution.md) | Accepted | — | Execution and CI |
| [0002: Targets, Not Files](0002-targets-not-files.md) | Accepted | — | Target model |
| [0003: Initial Action Granularity](0003-action-granularity.md) | Accepted | — | Quality actions |
| [0004: Naming](0004-naming.md) | Accepted | — | Public naming |
| [0005: Explicit Command Mutation Semantics](0005-mutating-operations.md) | Accepted | — | CLI mutation |
| [0006: CLI Command Surface](0006-cli-command-surface.md) | Superseded (check/fix/clean surface by 0018; docs surface by 0020) | 2026-09-09 | CLI surface |
| [0007: Tool Integration Model](0007-tool-integration-model.md) | Accepted | — | Quality integrations |
| [0008: Dependency Currency](0008-dependency-currency.md) | Accepted | — | Dependency governance |
| [0009: Project-Owned Starlark Testing](0009-starlark-testing.md) | Accepted | — | Testing |
| [0010: Python Foundation](0010-python-foundation.md) | Accepted | — | Python foundation |
| [0011: Configuration Composition](0011-configuration-composition.md) | Accepted | — | Configuration |
| [0012: Language Toolchain Versions](0012-language-toolchain-versions.md) | Accepted | — | Toolchains |
| [0013: Rust and JavaScript/TypeScript Foundations](0013-rust-javascript-typescript-foundations.md) | Accepted | — | Language foundations |
| [0014: Tested Platform Release Stack](0014-tested-platform-release-stack.md) | Accepted | — | Release platform |
| [0015: First-Party Gazelle Extensions](0015-first-party-gazelle-extensions.md) | Accepted | — | Generation |
| [0016: Broad First Release](0016-broad-first-release.md) | Accepted | — | V1 scope and project coverage |
| [0017: dx Watch Loop](0017-dx-watch.md) | Accepted | 2026-09-22 | CLI iteration |
| [0018: Umbrella Check/Fix And Managed-State Cleanup](0018-umbrella-check-fix-cleanup-clean.md) | Accepted | — | CLI surface |
| [0019: First-Release Additional-Foundation Dispositions](0019-first-release-additional-foundations.md) | Superseded (Ruby/PowerShell admitted, Bandit re-selected, Swift exclusion re-evidenced by 0032) | 2026-09-08 | V1 scope dispositions |
| [0020: Remove The `dx docs` Placeholder Command](0020-remove-dx-docs-placeholder.md) | Accepted | — | CLI surface |
| [0021: Build Profiles](0021-build-profiles.md) | Accepted | — | Configuration |
| [0022: No Standing Benchmarking](0022-no-benchmarking.md) | Accepted | — | Performance and testing |
| [0023: Rust Libraries Stay Internal](0023-rust-libraries-internal.md) | Accepted | — | Rust libraries |
| [0024: Selective `dx update` Per-Set Support](0024-selective-update.md) | Accepted | — | Dependency currency |
| [0025: `dx migrate` Upgrade Scope](0025-migrate-upgrade-scope.md) | Accepted | — | CLI surface |
| [0026: Rust Product Code Boundary And Migration Umbrella](0026-rust-product-code.md) | Accepted | — | Product runtime |
| [0027: Depcheck Checker Rust Delivery](0027-depcheck-rust.md) | Accepted | — | Product runtime |
| [0028: Deferred CI Drivers Plus Artifacts Update Stance](0028-deferred-ci-drivers-update.md) | Accepted | — | Product runtime |
| [0029: Deploy/Release Shell Rust Delivery](0029-deploy-release-rust.md) | Accepted | — | Product runtime |
| [0030: Ruby Foundation Reconsideration Keeps Deferred Beyond V1](0030-ruby-foundation-reconsideration.md) | Superseded (admitted to v1 by 0032) | 2026-09-21 | V1 scope dispositions |
| [0031: PowerShell Foundation Reconsideration Keeps Deferred Beyond V1](0031-powershell-foundation-reconsideration.md) | Superseded (admitted to v1 by 0032) | 2026-09-21 | V1 scope dispositions |
| [0032: Admit Ruby and PowerShell Foundations, Re-select Bandit, Swift Spike Keeps Exclusion](0032-ruby-powershell-bandit-swift.md) | Accepted | 2026-09-22 | V1 scope dispositions |
