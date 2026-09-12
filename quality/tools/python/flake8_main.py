"""Flake8 entry point with exit-code propagation (M15 WP3).

Upstream `py_console_script_binary` expands to `{fn}()` without `sys.exit`,
so flake8's integer return (1 findings, 2 usage error) is dropped and every
run exits 0. Quality actions require the exit-code contract (0 clean,
1 findings), so this wrapper preserves the wheel-only graph and shared
managed runtime while propagating the return code.
"""

import sys

from flake8.main.cli import main

if __name__ == "__main__":
    sys.exit(main())
