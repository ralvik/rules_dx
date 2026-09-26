"""Shared deploy-launcher helpers."""

RUNFILES_BASH_INIT = """# --- begin runfiles.bash initialization v3 ---"""

def rlocation_path(ctx, f):
    """Returns the runfiles rlocation for one file."""
    sp = f.short_path
    if sp.startswith("../"):
        return sp[3:]
    return ctx.workspace_name + "/" + sp
