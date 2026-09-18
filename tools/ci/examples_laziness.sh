#!/usr/bin/env bash
# Examples laziness audit (issue #85, slice 2): managed-acquisition and
# unused-foundation zero-work, static half verifiable on a clean tree.
#
# The consumer contract (docs/tools/tool-acquisition.md) forbids ecosystem
# installers in the private tool graph (`pip install`, `npm install`,
# `cargo install`, `dotnet tool install`, ...), and the laziness matrix
# (docs/testing/tools.md) requires unused foundations to contribute zero
# targets/actions. Runtime attribution (process logs, aquery/exec-log
# proof over external consumers with network denied) stays open per #85
# and is recorded as a gap, not claimed here.
#
# This harness machine-checks the static half:
#  - no prohibited installer command appears in tool-implementation code
#    (quality/, tools/, language foundations, dx/, cli/);
#  - each single-foundation adopt-* example loads only its own
#    foundation wrapper (plus fixtures/ecosystem locks), never another
#    foundation's wrapper, so unused foundations contribute no targets.
#
# Versioned here, run by CI via `bazel run //tools/ci:examples_laziness`,
# following //tools/ci:examples_readme.
set -euo pipefail

if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
  workspace="$BUILD_WORKSPACE_DIRECTORY"
else
  workspace="$(git rev-parse --show-toplevel)"
fi
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

# No-install attribution, static half: prohibited installer commands must
# not appear in tool-implementation code. Docs legitimately discuss them
# (contract + matrix), this harness names them as patterns, and the GHCR
# hygiene audit (tools/ci/ghcr_hygiene.sh) greps the Dockerfile for them
# as a negative gate (audit pattern, not an invocation), so all three
# are excluded from the search scope. Equivalent installers (`uv pip
# install`, `go install`, `dotnet add`, `bundle install`, `nuget install`,
# `Install-Module`, `mvn install`, `uv add`, `cargo add`, `gem install`,
# `dotnet restore`, `nuget restore`, `npm ci`, `yarn install`, `yarn add`,
# `pipx install`, `go get`, `dotnet tool restore`, `poetry install`,
# `poetry add`, `pipenv install`, `bun install`, `Install-Script`,
# `pip3 install`, `uv tool install`, `npm add`, `bun add`, `bundle add`,
# `Install-Package`, `pipenv sync`, `poetry sync`, `uv sync`,
# `uv pip sync`, `dotnet tool update`, `Install-PSResource`, `pnpm dlx`,
# `npx`, `cargo update`, `npm update`, `uv lock`, `poetry lock`,
# `pipenv lock`, `poetry update`, `uv pip compile`, `pip compile`,
# `bundle update`, `gem update`, `pipenv update`, `poetry export`,
# `yarn upgrade`, `pnpm upgrade`, `bun update`, `uv export`, `yarn dlx`,
# `bunx`, `uvx`, `go mod download`, `cargo fetch`, `Update-Module`,
# `pip download`, `npm exec`, `yarn exec`, `pnpm exec`, `bun x`,
# `cargo binstall`) are
# covered as the contract's "or equivalent installer" clause per issue
# #85 and the laziness matrix (pip, uv, npm, pnpm, Cargo, Maven, NuGet,
# Bundler, PowerShell Gallery).
hits="$(grep -rn -F -e 'pip install' -e 'npm install' -e 'cargo install' -e 'dotnet tool install' -e 'pnpm install' -e 'pnpm add' -e 'uv pip install' -e 'go install' -e 'dotnet add' -e 'bundle install' -e 'nuget install' -e 'Install-Module' -e 'mvn install' -e 'uv add' -e 'cargo add' -e 'gem install' -e 'dotnet restore' -e 'nuget restore' -e 'npm ci' -e 'yarn install' -e 'yarn add' -e 'pipx install' -e 'go get' -e 'dotnet tool restore' -e 'poetry install' -e 'poetry add' -e 'pipenv install' -e 'bun install' -e 'Install-Script' -e 'pip3 install' -e 'uv tool install' -e 'npm add' -e 'bun add' -e 'bundle add' -e 'Install-Package' -e 'pipenv sync' -e 'poetry sync' -e 'uv sync' -e 'uv pip sync' -e 'dotnet tool update' -e 'Install-PSResource' -e 'pnpm dlx' -e 'npx' -e 'cargo update' -e 'npm update' -e 'uv lock' -e 'poetry lock' -e 'pipenv lock' -e 'poetry update' -e 'uv pip compile' -e 'pip compile' -e 'bundle update' -e 'gem update' -e 'pipenv update' -e 'poetry export' -e 'yarn upgrade' -e 'pnpm upgrade' -e 'bun update' -e 'uv export' -e 'yarn dlx' -e 'bunx' -e 'uvx' -e 'go mod download' -e 'cargo fetch' -e 'Update-Module' -e 'pip download' -e 'npm exec' -e 'yarn exec' -e 'pnpm exec' -e 'bun x' -e 'cargo binstall' --include='*.bzl' --include='*.py' --include='*.rs' --include='*.sh' --include='*.js' --include='*.ts' quality/ tools/ rust/ python/ javascript/ typescript/ go/ java/ kotlin/ scala/ csharp/ fsharp/ cc/ dx/ cli/ 2>/dev/null | grep -v -F -e 'tools/ci/examples_laziness.sh' | grep -v -F -e 'tools/ci/ghcr_hygiene.sh' || true)"
if [[ -z "$hits" ]]; then
  ok
else
  bad "prohibited installer invocation in tool code: $(echo "$hits" | head -n 5)"
fi

# Unused-foundation zero-work, static half: each single-foundation
# example must load exactly its own foundation wrapper(s).
check_isolation() { # dir, want-foundation-list...
  local dir="$1"; shift
  local got
  got="$(grep -rh '^load' "$dir" --include='BUILD.bazel' 2>/dev/null | grep -o '@rules_dx//[a-z_]*/' | sed 's|@rules_dx//||; s|/||' | LC_ALL=C sort -u | tr '\n' ' ')"
  local want="$* "
  if [[ "$got" == "$want" ]]; then
    ok
  else
    bad "$dir loads [$got], want [$want]"
  fi
}

check_isolation examples/adopt-rust rust
check_isolation examples/adopt-python python
check_isolation examples/adopt-js-ts javascript typescript
check_isolation examples/adopt-go go
check_isolation examples/adopt-cpp cc
check_isolation examples/adopt-java java
check_isolation examples/adopt-kotlin kotlin
check_isolation examples/adopt-scala scala
check_isolation examples/adopt-csharp csharp
check_isolation examples/adopt-fsharp fsharp
check_isolation examples/adopt-polyglot javascript python rust typescript

echo "examples laziness audit: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
