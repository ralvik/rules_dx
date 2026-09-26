"""Shared deploy-launcher helpers.
"""

RUNFILES_BASH_INIT = """# --- begin runfiles.bash initialization v3 ---
set -uo pipefail; set +e; f=bazel_tools/tools/bash/runfiles/runfiles.bash
source "${RUNFILES_DIR:-/dev/null}/$f" 2>/dev/null || \\
  source "$(grep -sm1 "^$f " "${RUNFILES_MANIFEST_FILE:-/dev/null}" | cut -f2- -d' ')" 2>/dev/null || \\
  source "$0.runfiles/$f" 2>/dev/null || \\
  source "$(grep -sm1 "^$f " "$0.runfiles_manifest" | cut -f2- -d' ')" 2>/dev/null || \\
  source "$(grep -sm1 "^$f " "$0.exe.runfiles_manifest" | cut -f2- -d' ')" 2>/dev/null || \\
  { echo>&2 "ERROR: cannot find $f"; exit 1; }; f=; set -e
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
