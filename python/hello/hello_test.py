"""M14 seed pytest: proves wrapper test, imports, and coverage."""

from hello import greet


def test_greet():
    assert greet("world") == "Hello, world!"
