import hashlib
import importlib.util
import io
import pathlib
import tarfile
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts" / "install_release_review_bundle.py"
SPEC = importlib.util.spec_from_file_location("install_release_review_bundle", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


def make_archive(path: pathlib.Path, members: dict[str, bytes], *, symlink: str | None = None):
    with tarfile.open(path, "w:gz") as archive:
        for name, body in members.items():
            info = tarfile.TarInfo(name)
            info.size = len(body)
            archive.addfile(info, io.BytesIO(body))
        if symlink is not None:
            info = tarfile.TarInfo(symlink)
            info.type = tarfile.SYMTYPE
            info.linkname = "/etc/passwd"
            archive.addfile(info)


class ReviewBundleInstallTests(unittest.TestCase):
    def test_installs_only_checksummed_reviews_tree(self):
        with tempfile.TemporaryDirectory() as temp:
            temp = pathlib.Path(temp)
            archive = temp / "reviews.tar.gz"
            make_archive(
                archive,
                {
                    "reviews/findings.json": b"{}\n",
                    "reviews/maintainer-signoff.toml": b"[approval]\n",
                },
            )
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            output = temp / "output"

            MODULE.install_review_bundle(archive, digest, output)

            self.assertEqual((output / "reviews/findings.json").read_bytes(), b"{}\n")
            self.assertEqual(
                (output / "reviews/maintainer-signoff.toml").read_bytes(),
                b"[approval]\n",
            )

    def test_rejects_bad_hash_traversal_links_and_non_review_payloads(self):
        fixtures = [
            ({"reviews/findings.json": b"{}\n"}, None, "does not match"),
            ({"../escape": b"owned"}, None, "unsafe archive path"),
            ({"payload.txt": b"wrong root"}, None, "outside reviews"),
            ({}, "reviews/link", "links are forbidden"),
        ]
        for members, symlink, expected in fixtures:
            with self.subTest(expected=expected), tempfile.TemporaryDirectory() as temp:
                temp = pathlib.Path(temp)
                archive = temp / "reviews.tar.gz"
                make_archive(archive, members, symlink=symlink)
                digest = hashlib.sha256(archive.read_bytes()).hexdigest()
                if expected == "does not match":
                    digest = "0" * 64
                with self.assertRaisesRegex(ValueError, expected):
                    MODULE.install_review_bundle(archive, digest, temp / "output")


if __name__ == "__main__":
    unittest.main()
