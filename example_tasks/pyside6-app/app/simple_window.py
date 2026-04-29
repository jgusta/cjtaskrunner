import os
import sys

from PySide6.QtWidgets import QApplication, QLabel


def window_title() -> str:
    return os.environ.get("PYSIDE_WINDOW_TITLE", "PySide6 Example")


def main() -> None:
    app = QApplication(sys.argv)
    label = QLabel("Hello from a PySide6 CJTasks example")
    label.setWindowTitle(window_title())
    label.resize(360, 120)
    label.show()
    raise SystemExit(app.exec())


if __name__ == "__main__":
    main()
