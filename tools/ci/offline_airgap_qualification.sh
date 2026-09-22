#!/usr/bin/env bash
# Offline/airgap bootstrap plus vendored advisory mirror qualification.
#
# Owns the offline bootstrap with fixture evidence and owned gaps,
# without claiming platform, consumer, or release support:
# - delivered as-built: vendored launcher plus advisory mirror bundle
#   with SHA256SUMS manifests, verified before install with no network
#   (`deploy/offline/bootstrap-offline.sh`: no curl, no wget, no URL
#   fetch; checksum mismatch fails before mutation);
# - vendored advisory mirror for `dx audit` (declared `.dx/advisory/`
#   inputs, `file://` provenance validating like upstream `https://`
#   with the same sha256 plus same-day freshness gates, fail-closed
#   mapping preserved; live CLI performs no fetch on either path);
# - offline `dx setup`/`env`/`codegen` from the bundle with unchanged
#   selection plus atomic-commit semantics; first Bazel module and
#   toolchain fetch still needs network once, steady-state offline
#   after (proven by `dx setup --dry-run` planning with network tools
#   shadowed);
# - fixture evidence: miniature bundle plus expected pins in
#   `tools/ci/tests/fixtures/offline_airgap/`, hermetic bundle
#   manifests via `//deploy/offline:offline_demo`, and a
#   network-disabled run (curl/wget shadowed by failing stubs, so any
#   network attempt fails) proving bootstrap plus advisory population
#   plus tamper fail-closed plus setup planning.
# Seed Linux x86_64 only; no Supported claim. No live mirror service
# is operated; dry-run/local-mirror fixtures only.
#
# Versioned here, run by CI via `bazel run //tools/ci:offline_airgap_qualification`,
# following //tools/ci:sbom_upload_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

contract="docs/deploy/offline-bootstrap.md"
script="deploy/offline/bootstrap-offline.sh"
advisory="cli/audit/src/advisory.rs"
exec_audit="cli/cli/src/exec/audit.rs"
bundle_rule="deploy/offline/offline.bzl"
bundle_build="deploy/offline/BUILD.bazel"
audit_doc="docs/cli/commands/audit-update-bazel.md"
setup_doc="docs/cli/commands/environment-codegen-setup.md"
workflows_doc="docs/contributing/local-workflows.md"
pins="tools/ci/tests/fixtures/offline_airgap/pins.bzl"
expected="tools/ci/tests/fixtures/offline_airgap/offline_airgap.expected"
fixture_build="tools/ci/tests/fixtures/offline_airgap/BUILD.bazel"
fixture_bundle="tools/ci/tests/fixtures/offline_airgap/bundle"
targets="tools/ci/ci_targets_c.bzl"

# Contract owns the vendored bundle with checksum-before-install.
if grep -q -F -e '## Vendored bundle' "$contract" &&
  grep -q -F -e 'SHA256SUMS' "$contract" &&
  grep -q -F -e 'before installing anything' "$contract" &&
  grep -q -F -e 'no `curl`' "$contract"; then
  ok
else
  bad "offline-bootstrap.md lost its vendored-bundle plus checksum-before-install record (#774)"
fi

# Contract splits first fetch from steady-state offline.
if grep -q -F -e '## First fetch versus steady-state offline' "$contract" &&
  grep -q -F -e 'needs network once' "$contract" &&
  grep -q -F -e 'no new fetch when inputs are' "$contract" &&
  grep -q -F -e 'fails' "$contract"; then
  ok
else
  bad "offline-bootstrap.md lost its first-fetch versus steady-state split (#774)"
fi

# Contract owns the mirror staleness plus fail-closed semantics.
if grep -q -F -e '## Vendored advisory mirror' "$contract" &&
  grep -q -F -e 'file://' "$contract" &&
  grep -q -F -e 'same-day freshness' "$contract" &&
  grep -q -F -e 'advisory_refresh_failed' "$contract" &&
  grep -q -F -e 'never clean and' "$contract"; then
  ok
else
  bad "offline-bootstrap.md lost its mirror staleness plus fail-closed record (#774)"
fi

# Contract owns offline setup/env/codegen plus the dry-run proof.
if grep -q -F -e '## Offline setup, env, and codegen' "$contract" &&
  grep -q -F -e 'dx setup --dry-run' "$contract" &&
  grep -q -F -e 'unchanged' "$contract"; then
  ok
else
  bad "offline-bootstrap.md lost its offline setup/env/codegen record (#774)"
fi

# Bootstrap performs no download: no curl, wget, or URL fetch in
# executable lines (comments may name the banned forms).
if ! grep -E -e '^[^#]*\bcurl\b' "$script" | grep -q . &&
  ! grep -E -e '^[^#]*\bwget\b' "$script" | grep -q . &&
  ! grep -E -e '^[^#]*urllib' "$script" | grep -q . &&
  ! grep -E -e '^[^#]*urlopen' "$script" | grep -q .; then
  ok
else
  bad "bootstrap-offline.sh gained a network call (want no curl/wget/URL fetch, #774)"
fi

# Bootstrap verifies before mutating: both manifests verify in phase 1,
# installation plus population happen in phase 2.
if grep -q -F -e 'verify_manifest "$bundle/bazelisk"' "$script" &&
  grep -q -F -e 'verify_manifest "$bundle/advisory"' "$script" &&
  grep -q -F -e 'Phase 1: verify everything before mutating anything' "$script" &&
  grep -q -F -e 'Phase 2: install the launcher plus populate' "$script"; then
  ok
else
  bad "bootstrap-offline.sh lost its verify-before-mutate phasing (#774)"
fi

# Advisory accepts the vendored mirror without weakening the gate.
if grep -q -F -e 'pub fn is_local_mirror' "$advisory" &&
  grep -q -F -e 'pub fn is_accepted_url' "$advisory" &&
  grep -q -F -e 'file://' "$advisory" &&
  grep -q -F -e 'vendored_file_mirror_validates_like_upstream' "$advisory" &&
  grep -q -F -e 'docs/deploy/offline-bootstrap.md' "$advisory"; then
  ok
else
  bad "advisory.rs lost its file:// mirror provenance plus parity pins (#774)"
fi

# CLI names the vendored mirror in missing plus stale diagnostics.
if grep -q -F -e 'vendored advisory mirror' "$exec_audit" &&
  grep -q -F -e 'offline-bootstrap.md#vendored-advisory-mirror' "$exec_audit"; then
  ok
else
  bad "exec/audit.rs lost its vendored-mirror diagnostic hint (#774)"
fi

# Bundle rule plus demo stay wired.
if grep -q -F -e 'def offline_bundle' "$bundle_rule" &&
  grep -q -F -e 'hermetic `//deploy/rules:hasher`' "$bundle_rule" &&
  grep -q -F -e 'offline_bundle(' "$bundle_build" &&
  grep -q -F -e 'name = "offline_demo"' "$bundle_build"; then
  ok
else
  bad "deploy/offline lost its offline_bundle rule plus offline_demo wiring (#774)"
fi

# Live proof: the demo bundle builds green on the seed host.
if bazel build //deploy/offline:offline_demo --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "offline_demo failed to build (want green on the seed host, #774)"
fi

# Live proof: the manifest binds exact bytes in script format.
if python3 -c "
import hashlib
for name in ['demo-bazelisk-linux-amd64', 'demo-cargo.json']:
    want = hashlib.sha256(open('deploy/offline/demo/' + name, 'rb').read()).hexdigest()
    lines = open('bazel-bin/deploy/offline/offline_demo.SHA256SUMS').read().splitlines()
    hit = [line for line in lines if line == want + '  ' + name]
    assert hit, name
print('manifest binds exact bytes')
" >/dev/null 2>&1; then
  ok
else
  bad "offline_demo manifest does not bind exact demo bytes in SHA256SUMS format (#774)"
fi

# Docs point at the contract instead of copying it.
if grep -q -F -e 'deploy/offline-bootstrap.md' "$audit_doc" &&
  grep -q -F -e 'file://' "$audit_doc" &&
  grep -q -F -e 'deploy/offline-bootstrap.md' "$setup_doc" &&
  grep -q -F -e 'deploy/offline-bootstrap.md' "$workflows_doc"; then
  ok
else
  bad "audit/setup/workflows docs lost their offline-bootstrap contract links (#774)"
fi

# Network-disabled run: shadow curl/wget with failing stubs so any
# network attempt fails, then bootstrap from the fixture bundle.
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/stubs" "$scratch/bin" "$scratch/ws"
printf '#!/usr/bin/env bash\necho "network disabled: curl stub" >&2\nexit 1\n' >"$scratch/stubs/curl"
printf '#!/usr/bin/env bash\necho "network disabled: wget stub" >&2\nexit 1\n' >"$scratch/stubs/wget"
chmod +x "$scratch/stubs/curl" "$scratch/stubs/wget"
if PATH="$scratch/stubs:$PATH" bash "$script" --bundle "$PWD/$fixture_bundle" --install-dir "$scratch/bin" --workspace "$scratch/ws" >/dev/null 2>&1 &&
  cmp -s "$scratch/bin/bazel" "$fixture_bundle/bazelisk/bazelisk-linux-amd64"; then
  ok
else
  bad "network-disabled bootstrap failed (want install from the fixture bundle with curl/wget stubbed, #774)"
fi

# Populated mirror binds bytes plus freshness plus file:// provenance.
if python3 -c "
import hashlib, json, datetime
meta = json.load(open('$scratch/ws/.dx/advisory/cargo.meta.json'))
raw = open('$scratch/ws/.dx/advisory/cargo.json', 'rb').read()
assert meta['set'] == 'cargo', meta
assert meta['url'].startswith('file://'), meta
assert meta['sha256'] == hashlib.sha256(raw).hexdigest(), meta
assert meta['retrieved_at'] == datetime.datetime.now(datetime.timezone.utc).strftime('%Y-%m-%d'), meta
assert meta['path'] == '.dx/advisory/cargo.json', meta
print('mirror population binds bytes plus freshness')
" >/dev/null 2>&1; then
  ok
else
  bad "populated advisory mirror does not bind bytes plus freshness plus file:// provenance (#774)"
fi

# Tampering fails closed before install or population.
tampered="$scratch/tampered"
cp -r "$fixture_bundle" "$tampered"
printf 'tampered' >"$tampered/advisory/cargo.json"
if PATH="$scratch/stubs:$PATH" bash "$script" --bundle "$tampered" --install-dir "$scratch/tampered-bin" --workspace "$scratch/tampered-ws" >/dev/null 2>&1; then
  bad "tampered bundle bootstrapped (want fail closed before mutation, #774)"
else
  ok
fi

# Live proof: advisory mirror unit tests stay green.
if bazel test //cli/audit:dx_audit_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "dx_audit_test failed (want green advisory mirror units, #774)"
fi

# Live proof: setup planning needs no network.
if PATH="$scratch/stubs:$PATH" bazel run //cli/cli:dx -- setup --dry-run >/dev/null 2>&1; then
  ok
else
  bad "dx setup --dry-run failed under stubbed network (want exit 0 with no launch, #774)"
fi

# Fixture files stay present with corpus coverage.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  [[ -f "$fixture_bundle/bazelisk/bazelisk-linux-amd64" ]] &&
  [[ -f "$fixture_bundle/bazelisk/SHA256SUMS" ]] &&
  [[ -f "$fixture_bundle/advisory/cargo.json" ]] &&
  [[ -f "$fixture_bundle/advisory/SHA256SUMS" ]] &&
  grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'offline_airgap.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "offline-airgap fixture missing (want pins plus expected plus bundle plus corpus BUILD, #774)"
fi

# Pins record bundle plus mirror plus setup plus rejected plus honesty.
if grep -q -F -e 'BUNDLE_LAUNCHER' "$pins" &&
  grep -q -F -e 'MIRROR_URL' "$pins" &&
  grep -q -F -e 'MIRROR_FAIL_CLOSED' "$pins" &&
  grep -q -F -e 'SETUP_OFFLINE' "$pins" &&
  grep -q -F -e 'REJECTED_CURL_BOOTSTRAP' "$pins" &&
  grep -q -F -e 'REJECTED_LIVE_MIRROR' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #774' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "pins.bzl lost its bundle plus mirror plus setup plus rejected plus honesty pins under #774"
fi

# Expected fixture pins the offline plus rejected plus honesty lines.
if grep -q -F -e 'issue #774' "$expected" &&
  grep -q -F -e 'no network' "$expected" &&
  grep -q -F -e 'fail closed' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected" &&
  grep -q -F -e 'owned gap' "$expected"; then
  ok
else
  bad "offline_airgap.expected lost its offline plus rejected plus honesty lines under #774"
fi

# Split targets file owns the harness target (see tools/ci/BUILD.bazel add_c).
if grep -q -F -e 'name = "offline_airgap_qualification"' "$targets" &&
  grep -q -F -e 'offline_airgap_qualification.sh' "$targets"; then
  ok
else
  bad "tools/ci/ci_targets_c.bzl lost the offline_airgap_qualification wiring (want target, #774)"
fi

dx_test_summary "offline airgap bootstrap harness"
