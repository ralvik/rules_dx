"""Python foundation pins plus uv projects."""

RULES_PYTHON_VERSION = "1.9.0"
ASPECT_RULES_PY_VERSION = "2.0.0-alpha.6"
PYTHON_VERSION = "3.12"

UV_HUB = "pypi"
UV_PROJECTS = [
    "//python/tests/fixtures/hello:pyproject.toml",
    "//quality/tools/python:pyproject.toml",
]
UV_LOCKS = [
    "//python/tests/fixtures/hello:uv.lock",
    "//quality/tools/python:uv.lock",
]

UV_REPIN_DIRS = [
    "python/tests/fixtures/hello",
    "quality/tools/python",
]
