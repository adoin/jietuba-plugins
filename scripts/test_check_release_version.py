import unittest

from check_release_version import validate


class VersionTests(unittest.TestCase):
    def test_stable_and_prerelease_match(self):
        for tag in ("v0.4.0", "v0.4.0-beta.1"):
            validate(tag, "0.4.0", {"first": "0.4.0", "second": "0.4.0"})

    def test_rejects_wrong_tags_and_versions(self):
        for tag in ("v0.4", "release-0.4.0", "v0.4.0-", "v0.4.0-../x"):
            with self.assertRaises(ValueError):
                validate(tag, "0.4.0", {"plugin": "0.4.0"})
        with self.assertRaises(ValueError):
            validate("v0.4.1", "0.4.0", {"plugin": "0.4.0"})
        with self.assertRaises(ValueError):
            validate("v0.4.0", "0.4.0", {"plugin": "0.3.0"})
        with self.assertRaises(ValueError):
            validate("v0.4.0", "0.4.0", {})
