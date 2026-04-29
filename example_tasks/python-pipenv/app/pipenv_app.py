import os


def main() -> None:
    message = os.environ.get("PIPENV_EXAMPLE_MESSAGE", "unset")
    print(f"Pipenv example: {message}")


if __name__ == "__main__":
    main()
