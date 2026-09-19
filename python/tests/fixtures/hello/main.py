"""M14 seed binary: thin wrapper over the hello library."""

from hello import greet


def main():
    print(greet("world"))


if __name__ == "__main__":
    main()
