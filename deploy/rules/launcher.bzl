RUNFILES_BASH_INIT = """# --- begin runfiles.bash initialization v3 ---"""

def rlocation_path(ctx, f):
    sp = f.short_path
    if sp.startswith("../"):
        return sp[3:]
    return ctx.workspace_name + "/" + sp
