# ADR 0012: Language Toolchain Versions

## Status

Accepted.

## Context

Repositories may run applications and compatibility tests with more than one
runtime or compiler version. Restricting alternate versions to tests would prevent
legitimate binaries, tools, environments, native extensions, and generated outputs
from selecting them. Registering every possible version would waste setup and
resolution resources.

Version concepts differ across language ecosystems, so one implementation cannot
assume every language has an interpreter or follows identical transition rules.

## Decision

Every automatically available version-selectable foundation supplies the tested release-default
version without consumer configuration. Optional typed language policy may request additional
versions and, where the authoritative rules support it, select another workspace default. The
effective registered set contains the release default plus explicitly requested alternates; the
effective default must be a member and is inherited by compatible targets that do not override
it.

Executable, test, environment, code-generation, native-extension, and other
toolchain-dependent rules may select another registered version when the underlying
language rules support per-target selection. Such selection configures the relevant
dependency graph, not only the leaf target. Unknown or unregistered versions fail
during analysis.

Source-only libraries remain version-neutral when their providers and outputs do
not depend on a runtime/compiler version. Libraries with generated, compiled,
native, or otherwise version-specific outputs participate in toolchain selection.

The repository environment uses each represented language's effective default version unless
its typed environment configuration explicitly chooses another registered version. A
language's project metadata constraints are validated against selected versions but
do not choose or silently upgrade them.

Languages without a meaningful or supported version-selectable toolchain omit this
API rather than accepting ignored fields. Each language adapter maps the common
policy onto its authoritative upstream rules and documents deviations.

## Consequences

- Multiple versions are available for running, building, testing, and environment
  construction, not only compatibility tests.
- Only the release default and explicitly requested alternates may register toolchains or incur
  setup cost; their payloads remain lazy until required.
- One default keeps ordinary targets and IDE environments predictable.
- Per-target version matrices are ordinary Bazel targets with independent cache,
  logs, coverage, and failures.
- Cross-language consistency is semantic rather than forced into one identical
  implementation.

## Rejected Alternatives

- Registering alternate versions only for tests.
- Automatically registering every possible alternate language version.
- Inferring defaults from open-ended project metadata ranges.
- Forcing source-only libraries into unnecessary version-specific configurations.
- Accepting version fields for ecosystems whose Bazel rules cannot honor them.
