# Loose passes, strict fails example (issue #615).
#
# Unsorted imports pass the loose selection (E4, E7, E9, F has no I001)
# and fail the strict preset (I). Illustrative only; not executed by the
# qualification harness as a tool run.
import sys
import os


def hello(name):
    return "hello " + name
