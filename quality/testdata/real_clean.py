"""Real-pipeline clean fixture (M15 WP2)."""


def add(first: int, second: int) -> int:
    """Add two integers.

    Parameters
    ----------
    first : int
        First operand.
    second : int
        Second operand.

    Returns
    -------
    int
        The sum.
    """
    return first + second


TOTAL = add(1, 2)
