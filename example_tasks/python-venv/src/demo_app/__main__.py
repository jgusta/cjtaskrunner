import os

from demo_app import message


def main() -> None:
    app_env = os.environ.get("APP_ENV", "unset")
    app_message = os.environ.get("APP_MESSAGE", "unset")
    print(f"{message()} [{app_env}] {app_message}")


if __name__ == "__main__":
    main()
