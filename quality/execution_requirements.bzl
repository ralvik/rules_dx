
DX_REMOTE_QUALIFIED = False

DX_NO_REMOTE_EXEC = "no-remote-exec"

DX_FORBIDDEN_NO_REMOTE = "no-remote"

def dx_execution_requirements():
    if DX_REMOTE_QUALIFIED:
        return {}
    return {DX_NO_REMOTE_EXEC: "1"}

def dx_is_local_only(requirements):
    return requirements.get(DX_NO_REMOTE_EXEC, "") == "1"

def dx_has_forbidden_marker(requirements):
    return DX_FORBIDDEN_NO_REMOTE in requirements
