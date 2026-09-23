# Local Workflows

## Bootstrap

Fresh clone to green build, copy-paste. Linux x86_64 seed host plus Linux arm64 native (issue #410) plus Linux static-musl profiles (issue #411) plus macOS arm64 native (issue #412) plus Windows x86_64 MSVC-compatible native (issue #414; macOS x86_64 Not planned per #976).
Pinned versions: Bazel `9.2.0` (canonical `.bazelversion`), Bazelisk
`v1.29.0` (canonical `.github/actions/setup-bazelisk/action.yml`
defaults with per-OS sha256), pnpm `10.34.5` (via `packageManager`, use corepack).
Tracked copies in `.devcontainer/Dockerfile.prebuilt` (linux-amd64 pair) and below must equal
their canonical source; `//tools/ci:pin_consistency_test` fails on drift.

```sh
# 1. Pinned Bazelisk launcher (portable, retry, checksum, no sudo; issue #617):
# Canonical pins live in `.github/actions/setup-bazelisk/action.yml` (v1.29.0):
#   bazelisk-linux-amd64         5a408715e932c0250d28bd84555f12edbf70117de42f9181691c736eacc4a992
#   bazelisk-linux-arm64         e20e8b0f4f240091b7a55bf17b9398bd4f40ee70ae0208dff95dd4c445fb4010
#   bazelisk-darwin-amd64        16c3d7aa15323a9fb69f56c7ec5733ed18bedb786680d0ba13bb12a3c8083007
#   bazelisk-darwin-arm64        cee851f726789227d5561004e9904a52be45c3efb56f8b38b6993d6adbaa0409
#   bazelisk-windows-amd64.exe   092a8738d5b41aae7a85c42cc961b1034e3389aba43ffc20c0fabda7b43e095b
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) asset=bazelisk-linux-amd64; want=5a408715e932c0250d28bd84555f12edbf70117de42f9181691c736eacc4a992 ;;
  Linux-aarch64|Linux-arm64) asset=bazelisk-linux-arm64; want=e20e8b0f4f240091b7a55bf17b9398bd4f40ee70ae0208dff95dd4c445fb4010 ;;
  Darwin-x86_64) asset=bazelisk-darwin-amd64; want=16c3d7aa15323a9fb69f56c7ec5733ed18bedb786680d0ba13bb12a3c8083007 ;;
  Darwin-arm64) asset=bazelisk-darwin-arm64; want=cee851f726789227d5561004e9904a52be45c3efb56f8b38b6993d6adbaa0409 ;;
  MINGW*-x86_64|MSYS*-x86_64|CYGWIN*-x86_64) asset=bazelisk-windows-amd64.exe; want=092a8738d5b41aae7a85c42cc961b1034e3389aba43ffc20c0fabda7b43e095b ;;
  *) echo "unsupported host $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac
for i in 1 2 3; do curl -fsSL --retry 3 --retry-delay 2 -o /tmp/bazelisk \
  "https://github.com/bazelbuild/bazelisk/releases/download/v1.29.0/$asset" && break \
  || { echo "download attempt $i/3 failed" >&2; [ "$i" -lt 3 ] || exit 1; sleep 2; }; done
if command -v sha256sum >/dev/null 2>&1; then got=$(sha256sum /tmp/bazelisk | cut -d' ' -f1);
elif command -v shasum >/dev/null 2>&1; then got=$(shasum -a 256 /tmp/bazelisk | cut -d' ' -f1);
else got=$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' /tmp/bazelisk); fi
[ "$got" = "$want" ] || { echo "checksum mismatch: got $got want $want" >&2; exit 1; }
mkdir -p "$HOME/.local/bin" && cp -f /tmp/bazelisk "$HOME/.local/bin/bazel" \
  && chmod +x "$HOME/.local/bin/bazel" && rm -f /tmp/bazelisk
export PATH="$HOME/.local/bin:$PATH" && bazel version
# 2. Node/pnpm (if you touch the JS graph):
corepack enable && corepack prepare pnpm@10.34.5 --activate
# 3. Build and test (Bazelisk reads .bazelversion, no manual Bazel install):
bazel build //...
bazel test //...
# 4. Verify the vendored preset is fresh (fails when stale, never refreshes):
bazel run //cli/cli:dx -- update --check
```

The `curl` plus checksum bootstrap above stays manual by contract:
before Bazel runs there is no `dx` binary to bootstrap with, so no
`dx bootstrap` command exists. The snapshot harness fails without
`UPDATE_EXPECT` (CI sets no `UPDATE_EXPECT`); refreshes are explicit
local runs reviewed before committing.

Editor plus direnv plus hooks one-shot (after the green build):

```sh
bazel run //dx:env
bazel run //cli/cli:dx -- setup
bazel run //cli/cli:dx -- hooks install
direnv allow  # optional; otherwise export `.dx/bin` manually
```

First-hour evidence is one-shot per [ADR 0022](../decisions/0022-no-benchmarking.md):
see [First-Hour Timing](first-hour-timing.md) for the measured
clone-to-green record plus methodology. No CI timing budget is enforced.

Behind a proxy that returns 403 for `bcr.bazel.build`, create the
gitignored `user.bazelrc` overlay (already `try-import`ed from
`.bazelrc`; canonical lockfile URLs stay `bcr.bazel.build`).
The overlay stays manual per-person state by design and is never
committed; `try-import` plus `.gitignore` are the drift guard, and
`dx status` reports no proxy status:

```sh
printf '%s\n' \
  'common --registry=https://raw.githubusercontent.com/bazelbuild/bazel-central-registry/main/' \
  > user.bazelrc
```

## Offline Bootstrap

Airgapped hosts skip the `curl` bootstrap above and install from the
vendored bundle instead: see [Offline Bootstrap](../deploy/offline-bootstrap.md).
The bundle carries the same pinned launcher bytes plus the advisory
mirror with a checksum manifest, verified before anything installs;
first Bazel module and toolchain fetch still needs network once.

## Current Workflow

Build, test, and coverage run through Bazel on Linux x86_64 (plus Linux arm64 native, issue #410, plus static-musl profiles, issue #411, plus macOS arm64 native on macos-14, issue #412, plus Windows x86_64 MSVC-compatible native on windows-latest with shell bash, issue #414 (macOS x86_64 Not planned per #976)) with local-only
execution:

```sh
bazel build //...
bazel test //...
```

Coverage follows the mandatory project gate described in
[Testing](../testing/README.md#coverage):

```sh
bazel run //cli/cli:dx -- coverage --min-coverage <percent> //...
```

## Shell

Shell harness files stay `shellcheck` plus `shfmt` clean (`.shellcheckrc` bash plus all checks, `shfmt -i 2 -ci`):

```sh
shellcheck <files>
shfmt -i 2 -ci -d <files>  # check; -w rewrites in place
```

`.shellcheckrc` owns the bash-only policy plus the nine scoped disables
(see [Code style](style.md#shellcheck)); `.editorconfig` mirrors the shell
indent for editors (`-ci` stays CLI-only). `//tools/ci:shell_contract`
pins both configs.

## Corpus Dogfood

The repository corpus (`real_source_target(name = "corpus_*")` per content
type per package, issue #15, shared `tags = ["corpus"]`) is checked with the
real lint/format aspects; every produced result must pass the per-result
evaluator at `--fail_on warning`. The same invocations run in CI
(`.github/workflows/ci.yml`, `dogfood` self-call with test disabled
(coverage superset, issue #408 plus Phase 1 #607) over
verbatim `//...` per issue #408, plus `dogfood-freshness` for generate
freshness and audits), which installs no
quality tools: all tools execute as Bazel-resolved pinned actions.

Select the corpus targets, then build their `dx_results`:

```sh
bazel query "attr(tags, corpus, kind(real_source_target, //...))" \
  | LC_ALL=C sort -u > /tmp/corpus_targets.txt
bazel build $(tr '\n' ' ' < /tmp/corpus_targets.txt) \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect \
  --output_groups=dx_results
```

Pass the targets to `aquery` as a `+` union (a bare query expression over
`//...` widens the analyzed set): list the produced result protos, confirm
every corpus target owns results, and evaluate each one. Paths to
`--result` must be absolute: `bazel run` does not execute from the
workspace root.

```sh
union="$(paste -sd+ /tmp/corpus_targets.txt)"
bazel aquery "$union" \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect,//quality:real_aspects.bzl%real_format_aspect,//quality:real_aspects.bzl%real_js_lint_aspect,//quality:real_aspects.bzl%real_js_format_aspect,//quality:real_aspects.bzl%real_python_lint_aspect,//quality:real_aspects.bzl%real_rust_lint_aspect,//quality:real_aspects.bzl%real_rust_format_aspect \
  --output_groups=dx_results --output=text 2>/dev/null \
  | grep -o 'bazel-out/[^]]*\.pb' | sort -u \
  | sed 's|bazel-out/[^/]*/bin/|bazel-bin/|' > /tmp/corpus_results.txt
while read -r result; do
  bazel run //quality/evaluator:quality_evaluator -- \
    --result "$PWD/$result" --fail_on warning \
    --output "/tmp/corpus_eval_$(echo "$result" | tr '/' '_').marker"
done < /tmp/corpus_results.txt
```

Audit corpus ownership: every applicable file
(`BUILD.bazel`, `MODULE.bazel`, `*.bzl`, `*.toml`, `*.md`, excluding
generated locks and the `paket2bazel` hub output under
`third_party/dotnet/deps/`) must be owned by a corpus target. The reverse direction
is informational: siblings, native configs, and fixture-owned files are
corpus-referenced without being applicable sources.

```sh
head -1 third_party/dotnet/deps/paket.main.bzl | grep -q GENERATED
head -1 third_party/dotnet/deps/paket.main_extension.bzl | grep -qi GENERATED
git ls-files \
  | grep -E '(^|/)(BUILD\.bazel|MODULE\.bazel)$|\.(bzl|toml|md)$' \
  | grep -v -E '\.lock$' \
  | grep -v -E '^third_party/dotnet/deps/paket\.main(_extension)?\.bzl$' \
  | LC_ALL=C sort -u > /tmp/corpus_applicable.txt
bazel query "kind('source file', deps(kind(real_source_target, //...)))" 2>/dev/null \
  | grep -E '^(@@)?//' \
  | sed 's/^@@//; s|^//||; s|:|/|; s|^/||' \
  | LC_ALL=C sort -u > /tmp/corpus_closure.txt
comm -23 /tmp/corpus_applicable.txt /tmp/corpus_closure.txt
```

The last command prints nothing when every applicable file has a corpus
owner.

## Preset Update Loop

Shared Bazel execution flags live in the vendored preset (`tools/bazelrc`),
version-matched to `.bazelversion` and stamped with the per-release
`dx` version (`MODULE.bazel`). Review and change flags only through
the inventory in `tools/bazelrc/src/lib.rs` (mirrored in
`cli/adopt/src/preset_fragment.rs`; `//:preset_parity_test` proves
byte-identity):

```sh
bazel run //tools/bazelrc:preset_update -- --verify-only
bazel run //tools/bazelrc:preset_update
bazel run //cli/cli:dx -- update --check
bazel run //cli/cli:dx -- update go
```

The verify and `--check` commands reject stale generated files, print
the flag diff under review, and reject root `.bazelrc` lines that
duplicate preset flags (reconcile by removing the owned duplicates;
project overrides stay explicit and `user.bazelrc` stays last).
`preset.update_test` pins the Bazel pin, the dx stamp, and the inventory
in `bazel test //...`; `dx update` regenerates the consumer fragment
atomically and `--check` gates staleness (exit `0` clean / `1` stale).
Owned build profiles (`dx_debug`/`dx_dev`/`dx_release` plus provisional
`dx_dev_remote`/`dx_toolchain`) are reviewed the
same way; see [ADR 0021](../decisions/0021-build-profiles.md). Coverage
flags carry no ambient host path: `GENERATE_LLVM_LCOV=1` plus
toolchain-provided gcov/llvm-cov per host (see
[Build, Test, And Coverage](../cli/commands/build-test-coverage.md#dx-coverage)).

Version bumps flow through the native widen-one-requirement loop
(delivered, issue #260) as the sole updater (native-only, issue #461):
`dx bump <set:package> <version>` widens
one declared requirement (never batch), then `dx update <set>` applies the
resolver-owned lock refresh; bump PRs run the loop (discover stable-only,
widen one, update, regen, flag-diff review, test-pin updates,
full verification) per the [automation policy](automation.md). Each iteration
resets to a clean tree before the next dep. Auto-merge stays off by
default; when enabled it is update-only on green required checks (one dep per
PR, toggle only).
Bazel-surface PR shape: version-bump only with regen evidence, flag-diff review,
pin updates, and full verification (`bazel build //...`, `bazel test //...`,
coverage/dogfood).

## Linux-First Bring-Up

Bring-up starts Linux-first with local-only execution. Record unavailable
required hosts from
[ADR 0014](../decisions/0014-tested-platform-release-stack.md#required-platforms)
as gaps at entry; do not claim them. Linux-first is bring-up order, not a scope
reduction: v1 retains full required-host coverage.

## Local-Only Coverage

Coverage is local-only. The mandatory
[project coverage gate](../testing/README.md#coverage) must pass; the
adopted [first-party PR summary](../testing/strategy-details.md#github-coverage-reporting)
renders from the same Bazel-owned LCOV in CI with fork-PR step-summary-only
handling. Codecov stays opt-in only. Do not present local reports as service
evidence.

## Local Overrides

`dx init` absent-only scaffolding and `dx hooks install`/`status`/`run` dispatch are
implemented as specified in the [`dx init` and `dx hooks` contract](../cli/commands/hooks.md).
The gitignored root overlay `dx.local.toml` with its `[hooks]` table is created absent-only
by `dx hooks install`; the committed typed `hooks` workspace-policy section stays the team
baseline and CI never reads the personal overlay. This checkout does not consume the overlay
beyond hook-shim merging. Exact schema stays under the
[hooks contract](../cli/commands/hooks.md); this is not a current setup step beyond hooks.

Agent-local tool state (`.opencode/` whole-dir, machine-managed) plus the
`opencode.json` personal overlay stay untracked by design and are never
committed, so no committed copy exists. `.gitignore` owns both ignores and
`.bazelignore` mirrors the directory scope (file singletons have no form there).
