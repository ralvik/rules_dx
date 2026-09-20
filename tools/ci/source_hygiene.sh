#!/usr/bin/env bash
# Source comment hygiene guard (issue #428).
#
# `.bzl` module/function docstrings and Rust comments link to owning
# contracts instead of duplicating them, per `docs/AGENTS.md`
# (one-doc-per-fact, link don't copy applies to code comments too).
# `.bzl` headers carry one-line purpose plus owning-contract link;
# function docs keep only non-obvious invariants; Rust keeps only
# why-not-obvious notes with owning issue/ADR link.
#
# This harness machine-checks the trimmed state on a clean tree (8 checks):
# the clarified docs rule, short module headers, banned duplicated prose
# absent, function docs trimmed, Rust domain-split/battery provenance
# absent, Rust breadcrumb density, and the non-obvious invariants plus
# flagship contract links still present.
#
# Versioned here, run by CI via `bazel run //tools/ci:source_hygiene`,
# following //tools/ci:build_hygiene (issue #427).
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

# Clarified rule owns code comments too (issue #428).
if grep -q -F -e 'Link-don'"'"'t-copy applies to code comments' docs/AGENTS.md &&
  grep -q -F -e '.bzl` headers carry one-line purpose' docs/AGENTS.md; then
  ok
else
  bad "docs/AGENTS.md lost the code-comment rule (want link-don't-copy applies to code comments with .bzl header plus function plus Rust guidance, issue #428)"
fi

# Module headers stay short (issue #428): no `.bzl` header exceeds 8 lines
# (was 42 in generation/codegen.bzl and 97 in rust/rules/defs.bzl).
if python3 -c "
import pathlib
bad=[]
for f in sorted(pathlib.Path('.').rglob('*.bzl')):
    t=f.read_text(errors='ignore')
    if t.startswith('\"\"\"'):
        end=t.find('\"\"\"',3)
        if end==-1:
            continue
        n=len(t[:end+3].splitlines())
        if n>8:
            bad.append(f'{f}:{n}')
print(f'bzl headers over 8 lines: {len(bad)}')
for b in bad[:10]:
    print(b)
assert not bad, bad[:5]
"; then
  ok
else
  bad ".bzl module header exceeds 8 lines (want one-line purpose plus owning-contract link, issue #428)"
fi

# Banned duplicated prose stays out (issue #428): the ownership/history
# essays moved to owning docs, leaving one-line refs only.
if ! grep -rn -F -e 'frozen (slice' --include='*.bzl' . 2>/dev/null | grep -q . &&
  ! grep -rn -F -e 'Conflict rule' --include='*.bzl' . 2>/dev/null | grep -q . &&
  ! grep -rn -F -e 'Slice 4 adds' --include='*.bzl' . 2>/dev/null | grep -q . &&
  ! grep -rn -F -e 'Slice 2 (this file)' --include='*.bzl' . 2>/dev/null | grep -q . &&
  ! grep -rn -F -e 'Workspace \`Cargo.toml\`' --include='*.bzl' . 2>/dev/null | grep -q . &&
  ! grep -rn -F -e 'Rust \`pub\` surface' --include='*.bzl' . 2>/dev/null | grep -q . &&
  ! grep -rn -F -e 'Crate-level \`deny' --include='*.bzl' . 2>/dev/null | grep -q . &&
  ! grep -rn -F -e 'Physical/virtual ownership' --include='*.bzl' . 2>/dev/null | grep -q . &&
  ! grep -rn -F -e ' WP1 adds' --include='*.bzl' . 2>/dev/null | grep -q . &&
  ! grep -rn -F -e ' WP2 adds' --include='*.bzl' . 2>/dev/null | grep -q .; then
  ok
else
  bad "banned .bzl duplicated prose reappeared (want no issue #506-freeze/Conflict-rule/Slice-essays/Cargo-workspace/pub-surface/deny-warnings/physical-virtual/- history in *.bzl, issue #428)"
fi

# Function docs stay trimmed (issue #428): no Args/Returns restating
# types/contracts (was 202 blocks), and no function docstring exceeds
# 24 lines (was 39 in wrapper forwarders).
if python3 -c "
import pathlib
files=list(pathlib.Path('.').rglob('*.bzl'))
args=sum(f.read_text(errors='ignore').count('Args:') for f in files)
rets=sum(f.read_text(errors='ignore').count('Returns:') for f in files)
print(f'Args: {args}, Returns: {rets}')
assert args < 10, args
assert rets < 10, rets
" &&
  python3 -c "
import pathlib, re
bad=[]
for f in sorted(pathlib.Path('.').rglob('*.bzl')):
    t=f.read_text(errors='ignore')
    for m in re.finditer(r'def (\w+)\([^)]*\):\s*\n\s*\"\"\"(.*?)\"\"\"', t, re.DOTALL):
        n=len(m.group(2).strip().splitlines())
        if n>24:
            bad.append(f'{f}:{m.group(1)}:{n}')
print(f'func docs over 24 lines: {len(bad)}')
assert not bad, bad[:5]
"; then
  ok
else
  bad "function docs regressed (want no Args/Returns restating plus no func docstring over 24 lines, issue #428)"
fi

# Rust domain-split/battery provenance stays out (issue #428): the
# `(issue #236)` facade essays and `(issue #84)` battery tags were
# provenance for self-evident code; frozen/qualified notes stay.
if ! grep -rn -F -e '(issue #236' --include='*.rs' . 2>/dev/null | grep -q . &&
  ! grep -rn -F -e '(issue #84' --include='*.rs' . 2>/dev/null | grep -q . &&
  ! grep -rn -F -e 'issue #84' --include='*.rs' cli/atomic_fs/src/lib.rs 2>/dev/null | grep -q . &&
  ! grep -rn -e '^//!.*(issue #236' --include='*.rs' . 2>/dev/null | grep -q . &&
  ! grep -rn -F -e 'lives in the' --include='*.rs' cli/adopt/src/lib.rs 2>/dev/null | grep -q .; then
  ok
else
  bad "Rust self-evident breadcrumbs reappeared (want no (issue #236/(issue #84 in *.rs plus no lives-in-the facade essays in cli/adopt/src/lib.rs, issue #428)"
fi

# Rust breadcrumb density stays trimmed (issue #428): total issue refs
# under 150 (was 403 before the sweep; frozen/qualified notes stay).
if python3 -c "
import pathlib, re
n=sum(len(re.findall(r'issue #\d+', f.read_text(errors='ignore'))) for f in pathlib.Path('.').rglob('*.rs'))
print(f'Rust issue refs: {n}')
assert n < 150, n
"; then
  ok
else
  bad "Rust issue breadcrumbs >= 150 (want trimmed self-evident refs with only why-not-obvious notes kept, issue #428)"
fi

# Non-obvious invariants survive the trim (issue #428 is comment-only):
# the Bazel provided-twice workaround, the coverage lcov merger, the
# frozen legacy tokenizer, and the versioned schemas stay.
if grep -q -F -e 'provided twice' generation/codegen.bzl &&
  grep -q -F -e 'provided twice' env/plan.bzl &&
  grep -q -F -e '_lcov_merger' libs/starlark/wrapper.bzl &&
  grep -q -F -e 'frozen legacy contract' generation/codegen_shard/src/main.rs &&
  grep -q -F -e 'frozen legacy contract' cli/cli/src/args/tokenizer.rs &&
  grep -q -F -e 'CODEGEN_SCHEMA_VERSION' generation/codegen.bzl &&
  grep -q -F -e 'RUST_EDITION' rust/rules/edition.bzl; then
  ok
else
  bad "trim dropped non-obvious invariants (want provided-twice plus lcov-merger plus frozen-legacy plus schema plus edition, issue #428)"
fi

# Flagship contract links survive (issue #428): one-line purpose plus
# owning-contract link, not copied prose.
if grep -q -F -e 'docs/environments/codegen.md' generation/codegen.bzl &&
  grep -q -F -e 'docs/product/scope.md' generation/codegen.bzl &&
  grep -q -F -e 'docs/decisions/0013-rust-javascript-typescript-foundations.md' rust/rules/defs.bzl &&
  grep -q -F -e 'docs/quality/tool-integrations.md' quality/adapters.bzl &&
  grep -q -F -e 'docs/quality/quality-sources.md' quality/adapters.bzl &&
  grep -q -F -e 'docs/environments/environment.md' env/plan.bzl; then
  ok
else
  bad "flagship .bzl headers lost owning-contract links (want codegen plus rust-decisions plus adapters plus env links, issue #428)"
fi

dx_test_summary "source hygiene harness"
