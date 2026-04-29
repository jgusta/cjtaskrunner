import unittest

import pipenv_app


class PipenvAppTests(unittest.TestCase):
    def test_main_exists(self) -> None:
        self.assertTrue(callable(pipenv_app.main))
