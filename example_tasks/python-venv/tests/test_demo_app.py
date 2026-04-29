import unittest

from demo_app import message


class DemoAppTests(unittest.TestCase):
    def test_message(self) -> None:
        self.assertEqual(message(), "hello from demo_app")
