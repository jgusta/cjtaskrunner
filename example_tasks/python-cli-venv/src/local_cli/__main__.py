import os

from local_cli import describe


def main() -> None:
    name = os.environ.get("CLI_NAME", "cli")
    mode = os.environ.get("CLI_MODE", "unset")
    print(describe(name, mode))


if __name__ == "__main__":
    main()
