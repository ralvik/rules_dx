"""Canonical-repository label helpers.

Single-sources the canonical-repository prefix probe previously
copy-pasted across subject rules (each with its own
`buildifier: disable=canonical-repository`). Every subject rule loads
`strip_canonical`/`is_canonical` from here, so the buildifier
suppression lives in exactly one place.
"""

def is_canonical(text):
    """Returns whether Bazel rendered `text` with the canonical marker."""
    return text.startswith("@@")  # buildifier: disable=canonical-repository

def strip_canonical(text):
    """Strips one leading canonical-repository marker for readable observations.

    Pinned to the supported Bazel and requalified on version bumps.
    """
    if is_canonical(text):
        return text[2:]
    return text
