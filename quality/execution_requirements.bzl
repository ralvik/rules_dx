"""Local-only execution requirements helper (single enablement point).

Contract: `docs/quality/action-model.md#outputs-remote-cache-and-execution`.
"""

# Remote qualification switch: False keeps every Dx pipeline plus evaluator
# action local-only; qualifying remote flips this one place (plus toolchain
# and platform inputs) instead of editing every call site.
DX_REMOTE_QUALIFIED = False

# Local-only marker keeping actions off remote executors while locally cacheable.
DX_NO_REMOTE_EXEC = "no-remote-exec"

# Cache-disabling marker that must never appear on Dx actions.
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
