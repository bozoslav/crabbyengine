"""Release packaging regressions; no credentials, compiler or network required."""
import hashlib
import contextlib
import io
import importlib.util
import os
from pathlib import Path
import plistlib
import sys
import tempfile
import tarfile
import subprocess
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("release", Path(__file__).with_name("release-coin-dash.py"))
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseTests(unittest.TestCase):
    def test_linux_archive_normalizes_ownership_times_and_round_trips(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "coin-dash-test-linux-x86_64"
            package.mkdir()
            exe = package / "coin_dash"
            exe.write_bytes(b"binary fixture")
            exe.chmod(0o755)
            asset = package / "assets" / "coin.png"
            asset.parent.mkdir()
            asset.write_bytes(b"asset fixture")
            first, second = root / "first.tar.gz", root / "second.tar.gz"
            release.linux_archive(package, first)
            for path in (exe, asset, asset.parent, package):
                os.utime(path, (1234567890, 1234567890))
            release.linux_archive(package, second)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            self.assertEqual(first.read_bytes()[4:8], b"\0\0\0\0")
            with tarfile.open(first) as tar:
                for member in tar.getmembers():
                    self.assertEqual((member.uid, member.gid, member.uname, member.gname, member.mtime),
                                     (0, 0, "root", "root", 0))
                tar.extractall(root / "extracted", filter="data")
            restored = root / "extracted" / package.name
            self.assertEqual((restored / "coin_dash").read_bytes(), b"binary fixture")
            self.assertEqual((restored / "assets/coin.png").read_bytes(), b"asset fixture")
            if os.name != "nt":
                self.assertEqual((restored / "coin_dash").stat().st_mode & 0o777, 0o755)

    def test_signtool_missing_sdk_has_clean_error_without_environment_key(self):
        with patch.dict(os.environ, {}, clear=True), patch.object(release.shutil, "which", return_value=None), \
                patch.object(Path, "glob", return_value=[]):
            with self.assertRaisesRegex(RuntimeError, "SignTool was not found"):
                release.signtool()

    def signing_environment(self):
        return dict(zip(release.MACOS, ("Y2VydA==", "test-password", "Developer ID Application: Test")))

    def test_keychain_cleanup_does_not_hide_signing_failure(self):
        def command(*args):
            if args[0] == "codesign":
                raise RuntimeError("original signing failure")
            if args[:2] == ("security", "delete-keychain"):
                raise RuntimeError("cleanup failure")
        with patch.dict(os.environ, self.signing_environment(), clear=True), \
                patch.object(release, "run", side_effect=command), contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaisesRegex(RuntimeError, "original signing failure"):
                release.sign_macos(Path("Coin Dash.app"))

    def test_notary_rejection_fetches_log_and_never_staples(self):
        environment = self.signing_environment() | dict.fromkeys(release.NOTARY, "test-value")
        def command(*args):
            if args[:3] == ("xcrun", "notarytool", "log"):
                Path(args[-1]).write_text('{"issues": ["rejection detail"]}', encoding="utf-8")
        result = subprocess.CompletedProcess([], 0, '{"id":"test-id","status":"Invalid"}', "")
        output = io.StringIO()
        with patch.dict(os.environ, environment, clear=True), \
                patch.object(release, "run", side_effect=command) as run, \
                patch.object(release.subprocess, "run", return_value=result), contextlib.redirect_stdout(output):
            with self.assertRaisesRegex(RuntimeError, "Invalid"):
                release.sign_macos(Path("Coin Dash.app"))
            self.assertTrue(any(c.args[:4] == ("xcrun", "notarytool", "log", "test-id") for c in run.call_args_list))
            self.assertFalse(any(c.args[:2] == ("xcrun", "stapler") for c in run.call_args_list))
        self.assertIn("rejection detail", output.getvalue())

    def test_notary_acceptance_staples_bundle(self):
        environment = self.signing_environment() | dict.fromkeys(release.NOTARY, "test-value")
        result = subprocess.CompletedProcess([], 0, '{"id":"test-id","status":"Accepted"}', "")
        with patch.dict(os.environ, environment, clear=True), patch.object(release, "run") as run, \
                patch.object(release.subprocess, "run", return_value=result):
            app = Path("Coin Dash.app")
            self.assertEqual(release.sign_macos(app), (True, True))
            run.assert_any_call("xcrun", "stapler", "staple", app)

    def test_credentials_require_all_or_none_without_disclosing_values(self):
        for names in (release.WINDOWS, release.MACOS, release.NOTARY):
            with patch.dict(os.environ, {}, clear=True):
                self.assertIsNone(release.credentials(names, "test"))
            for missing in names:
                values = {name: "private-value" for name in names if name != missing}
                with patch.dict(os.environ, values, clear=True):
                    with self.assertRaises(RuntimeError) as raised:
                        release.credentials(names, "test")
                    self.assertIn(missing, str(raised.exception))
                    self.assertNotIn("private-value", str(raised.exception))
            with patch.dict(os.environ, dict.fromkeys(names, "value"), clear=True):
                self.assertEqual(release.credentials(names, "test"), ["value"] * len(names))

    def test_checksum_uses_lf_and_names_the_hashed_archive(self):
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / "coin-dash-test.zip"
            archive.write_bytes(b"archive fixture")
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            checksum = archive.with_suffix(".zip.sha256")
            release.write_checksum(checksum, digest, archive.name)
            self.assertEqual(checksum.read_bytes(), f"{digest}  {archive.name}\n".encode("ascii"))
            self.assertNotIn(b"\r", checksum.read_bytes())

    def test_no_credentials_still_seals_and_verifies_macos_bundle(self):
        with patch.dict(os.environ, {}, clear=True), patch.object(release, "run") as run:
            self.assertEqual(release.sign_macos(Path("Coin Dash.app")), (False, False))
            self.assertEqual(run.call_args_list[0].args,
                             ("codesign", "--force", "--sign", "-", Path("Coin Dash.app")))
            self.assertEqual(run.call_args_list[1].args,
                             ("codesign", "--verify", "--deep", "--strict", Path("Coin Dash.app")))

    def test_notarization_without_signing_fails(self):
        with patch.dict(os.environ, dict.fromkeys(release.NOTARY, "value"), clear=True):
            with self.assertRaisesRegex(RuntimeError, "requires Developer ID"):
                release.sign_macos(Path("Coin Dash.app"))

    @unittest.skipIf(os.name == "nt", "Real macOS bundle symlinks are tested on Unix runners")
    def test_bundle_preserves_assets_links_and_exposes_documentation(self):
        with tempfile.TemporaryDirectory() as directory:
            package = Path(directory) / "coin-dash-1.2.3-macos-aarch64"
            package.mkdir()
            (package / "coin_dash").write_bytes(b"executable")
            asset = package / "assets/coin_dash/coin.png"
            asset.parent.mkdir(parents=True)
            asset.write_bytes(b"image")
            names = ("README.txt", "LICENSE-MIT", "LICENSE-APACHE", "NOTICE",
                     "CONTENT-NOTICES.txt", "THIRD-PARTY-NOTICES.txt")
            for name in names:
                (package / name).write_text(name, encoding="utf-8")
            app = release.macos_app(package)
            executable = app / "Contents/MacOS"
            self.assertEqual((executable / "coin_dash").read_bytes(), b"executable")
            self.assertTrue((executable / "assets").is_symlink())
            self.assertEqual((executable / "assets/coin_dash/coin.png").read_bytes(), b"image")
            for name in names:
                self.assertEqual((package / name).read_bytes(), (executable / name).read_bytes())
            with (app / "Contents/Info.plist").open("rb") as file:
                info = plistlib.load(file)
            self.assertEqual(info["CFBundleShortVersionString"], "1.2.3")
            self.assertEqual(info["CFBundleInfoDictionaryVersion"], "6.0")
            self.assertTrue(info["NSHighResolutionCapable"])
            self.assertEqual(info["LSMinimumSystemVersion"], "11.0")
            self.assertEqual(info["LSApplicationCategoryType"], "public.app-category.games")

    def test_release_tag_must_match_cargo_version(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text('[package]\nversion = "1.2.3"\n', encoding="utf-8")
            with patch.object(release, "ROOT", root):
                release.check_version("v1.2.3")
                release.check_version("v0.0.0-test-desktop-dry-run")
                with self.assertRaisesRegex(RuntimeError, "does not match"):
                    release.check_version("v1.2.0")


if __name__ == "__main__":
    unittest.main()
