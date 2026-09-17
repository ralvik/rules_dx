"""Tests for the polyglot geometry helpers."""

from shapes import area


def test_area():
    assert area(3, 4) == 12
