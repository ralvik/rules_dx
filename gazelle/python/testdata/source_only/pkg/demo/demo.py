"""Demo library with a local edge, a stdlib import, and a paired stub."""

import helper
import os


def greet(name):
    return "hello " + name + helper.suffix(os.name)
