"""Demo library with a local edge, a stdlib import, and a paired stub."""

import os

import helper


def greet(name):
    return "hello " + name + helper.suffix(os.name)
