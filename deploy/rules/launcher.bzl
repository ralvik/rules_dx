"""Shared deploy-launcher helpers.
"""

# Standard `runfiles.bash` initialization v3, copy-pasted from the Bazel
# Bash runfiles library
# (bazel_tools/tools/bash/runfiles/runfiles.bash). Handles
# `RUNFILES_DIR`, `RUNFILES_MANIFEST_FILE`, `$0.runfiles`, and
# `$0.runfiles_manifest` plus Bzlmod repo mapping. Replaces every
# custom `RF`/`MANIFEST`/`rloc()` probe.
RUNFILES_BASH_INIT = """# --- begin runfiles.bash initialization v3 ---
# Copy-pasted from the Bazel Bash runfiles library v3.
# shellcheck disable=SC1090 (issue #914): single file-level pragma covers the
# five runfiles-layout probes below, which only exist under `bazel run` /
# `bazel test` (see `.shellcheckrc`, issue #319).
set -uo pipefail; set +e; f=bazel_tools/tools/bash/runfiles/runfiles.bash
source "${RUNFILES_DIR:-/dev/null}/$f" 2>/dev/null || \\
  source "$(grep -sm1 "^$f " "${RUNFILES_MANIFEST_FILE:-/dev/null}" | cut -f2- -d' ')" 2>/dev/null || \\
  source "$0.runfiles/$f" 2>/dev/null || \\
  source "$(grep -sm1 "^$f " "$0.runfiles_manifest" | cut -f2- -d' ')" 2>/dev/null || \\
  source "$(grep -sm1 "^$f " "$0.exe.runfiles_manifest" | cut -f2- -d' ')" 2>/dev/null || \\
  { echo>&2 "ERROR: cannot find $f"; exit 1; }; f=; set -e
# --- end runfiles.bash initialization v3 ---
"""

def rlocation_path(ctx, f):
    """Returns the runfiles rlocation for one file.

    Matches `$(rlocationpath)` and `rules_shell` `_to_rlocation_path`:
    main-repo files are `workspace/short_path`, external files strip
    the leading `../` (`../repo/path` -> `repo/path`).
    """
    sp = f.short_path
    if sp.startswith("../"):
        return sp[3:]
    return ctx.workspace_name + "/" + sp
