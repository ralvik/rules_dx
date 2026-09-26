"""Local-only execution requirements helper (single enablement point)."""

DX_REMOTE_QUALIFIED = False

DX_NO_REMOTE_EXEC = "no-remote-exec"

DX_FORBIDDEN_NO_REMOTE = "no-remote"

def dx_execution_requirements():
    """Returns the execution requirements for one Dx pipeline/evaluator action."""
    if DX_REMOTE_QUALIFIED:
        return {}
    return {DX_NO_REMOTE_EXEC: "1"}

def dx_is_local_only(requirements):
    """Reports whether requirements carry the local-only marker."""
    return requirements.get(DX_NO_REMOTE_EXEC, "") == "1"

def dx_has_forbidden_marker(requirements):
    """Reports whether requirements carry the cache-disabling marker."""
    return DX_FORBIDDEN_NO_REMOTE in requirements
