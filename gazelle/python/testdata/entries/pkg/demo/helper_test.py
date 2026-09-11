"""Test for the helper library; resolves only the local edge."""

import helper


def test_suffix():
    assert helper.suffix("x") == "<x>"
