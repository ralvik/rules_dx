"""Shared deploy-launcher helpers (issue #317).

Single-sources the runfiles `rlocation` computation plus the standard
`runfiles.bash` initialization snippet (v3, copy-pasted from the Bazel
Bash runfiles library) so `archive.bzl`, `github.bzl`, `signing.bzl`,
and `bcr.bzl` no longer carry duplicated `*_rlocation` helpers and
custom `RF`/`MANIFEST`/`rloc()` probes.

Launchers resolve every input from their own runfiles forest via the
standard `rlocation` function and exec the real deploy program. The
per-instance `*_program` targets are `sh_binary` wrappers (srcs =
generated launcher script, data = pinned inputs plus the runfiles
library): `dx_deployment` symlinks the `sh_binary` entrypoint and
merges its runfiles, so `bazel run`, `dx deploy`, and direct
`bazel-bin` execution all keep working. Rlocation strings embedded in
the generated script are quoted with `@bazel_skylib//lib:shell.bzl`
`shell.quote` (single-quote), never manual double-quote interpolation.
"""

# Standard `runfiles.bash` initialization v3, copy-pasted from the Bazel
# Bash runfiles library
# (bazel_tools/tools/bash/runfiles/runfiles.bash). Handles
# `RUNFILES_DIR`, `RUNFILES_MANIFEST_FILE`, `$0.runfiles`, and
# `$0.runfiles_manifest` plus Bzlmod repo mapping. Replaces every
# custom `RF`/`MANIFEST`/`rloc()` probe.
RUNFILES_BASH_INIT = """# --- begin runfiles.bash initialization v3 ---
# Copy-pasted from the Bazel Bash runfiles library v3.
set -uo pipefail; set +e; f=bazel_tools/tools/bash/runfiles/runfiles.bash
# shellcheck disable=SC1090
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

    Args:
      ctx: rule context for the workspace name.
      f: the File to locate.

    Returns:
      The rlocation string for use with `rlocation`.
    """
    sp = f.short_path
    if sp.startswith("../"):
        return sp[3:]
    return ctx.workspace_name + "/" + sp
