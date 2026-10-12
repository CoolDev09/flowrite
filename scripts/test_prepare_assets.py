import hashlib
import io
from pathlib import Path
import tempfile
import unittest
import zipfile
from prepare_assets import checked_files, install


def fixture(entries):
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w") as archive:
        for name, content in entries:
            archive.writestr(name, content)
    data = output.getvalue()
    return data, {
        "sha256": hashlib.sha256(data).hexdigest(),
        "files": {name: hashlib.sha256(content).hexdigest() for name, content in entries},
    }


class AssetSetupTests(unittest.TestCase):
    def test_valid_archive_and_isolated_install(self):
        data, manifest = fixture([("fonts/LICENSE", b"license"), ("nlprule/en_rules.bin", b"rules")])
        verified = checked_files(data, manifest)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            install(verified, root)
            self.assertEqual((root / "assets/nlprule/en_rules.bin").read_bytes(), b"rules")

    def test_archive_checksum_failure(self):
        data, manifest = fixture([("fonts/LICENSE", b"license")])
        with self.assertRaises(ValueError):
            checked_files(data + b"altered", manifest)

    def test_file_checksum_failure(self):
        data, manifest = fixture([("fonts/LICENSE", b"license")])
        manifest["files"]["fonts/LICENSE"] = "0" * 64
        with self.assertRaises(ValueError):
            checked_files(data, manifest)

    def test_unexpected_file(self):
        data, manifest = fixture([("fonts/LICENSE", b"license")])
        manifest["files"] = {}
        with self.assertRaises(ValueError):
            checked_files(data, manifest)

    def test_path_traversal(self):
        data, manifest = fixture([("fonts/../../outside", b"bad")])
        with self.assertRaises(ValueError):
            checked_files(data, manifest)


if __name__ == "__main__":
    unittest.main()
