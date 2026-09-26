"""Python foundation pins plus uv projects."""

# Pinned Python foundation (see MODULE.bazel; per-split pin_consistency in tools/ci/pin_consistency.sh).
RULES_PYTHON_VERSION = "1.9.0"
ASPECT_RULES_PY_VERSION = "2.0.0-alpha.6"
PYTHON_VERSION = "3.12"

# uv hub projects (single `pypi` hub; see MODULE.bazel uv.project).
UV_HUB = "pypi"
UV_PROJECTS = [
    "//python/tests/fixtures/hello:pyproject.toml",
    "//quality/tools/python:pyproject.toml",
]
UV_LOCKS = [
    "//python/tests/fixtures/hello:uv.lock",
    "//quality/tools/python:uv.lock",
]

# uv repin (resolver-owned; see docs/tools/tool-acquisition.md).
UV_REPIN_DIRS = [
    "python/tests/fixtures/hello",
    "quality/tools/python",
]
