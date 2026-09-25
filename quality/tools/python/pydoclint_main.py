"""Run the pinned pydoclint entry point with exit-code propagation."""

import sys

from pydoclint.main import main

if __name__ == "__main__":
    sys.exit(main())
