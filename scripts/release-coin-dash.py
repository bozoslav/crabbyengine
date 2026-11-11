#!/usr/bin/env python3
"""Sign, archive and smoke-test the desktop package made by package-coin-dash.py."""

import argparse
import base64
import hashlib
import os
from pathlib import Path
import plistlib
import secrets
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile


ROOT = Path(__file__).resolve().parent.parent
APP = "Coin Dash.app"
WINDOWS = ("WINDOWS_SIGN_PFX", "WINDOWS_SIGN_PASSWORD")
MACOS = ("MACOS_SIGN_P12", "MACOS_SIGN_PASSWORD", "MACOS_SIGN_IDENTITY")
NOTARY = ("MACOS_NOTARY_APPLE_ID", "MACOS_NOTARY_PASSWORD", "MACOS_NOTARY_TEAM_ID")


def run(*cmd):
    # Some signing commands take passwords as arguments. Keep them out of errors.
    result = subprocess.run(cmd, check=False)
    if result.returncode:
        raise RuntimeError(f"{Path(cmd[0]).name} exited with status {result.returncode}")


def credentials(names, step):
    values = [os.environ.get(name) for name in names]
    if not any(values):
        print(f"Skipping {step}: credentials are not configured.", flush=True)
        return None
    missing = [name for name, value in zip(names, values) if not value]
    if missing:
        raise RuntimeError(f"{step} is partially configured; missing {', '.join(missing)}")
    return values


def signtool():
    if tool := shutil.which("signtool"):
        return tool
    sdk = Path(os.environ["ProgramFiles(x86)"]) / "Windows Kits" / "10" / "bin"
    tools = [p for p in sdk.glob("10.*/x64/signtool.exe") if p.parent.parent.name.replace(".", "").isdigit()]
    if not tools:
        raise RuntimeError("SignTool was not found in PATH or the Windows SDK")
    return max(tools, key=lambda p: tuple(map(int, p.parent.parent.name.split("."))))


def macos_app(package):
    # Keep the data in Resources. Relative links preserve the existing
    # executable-relative asset lookup and the package smoke test's layout.
    version = package.name.removeprefix("coin-dash-").rsplit("-macos-", 1)[0].split("-", 1)[0]
    entries = list(package.iterdir())
    contents = package / APP / "Contents"
    resources = contents / "Resources"
    executable = contents / "MacOS"
    resources.mkdir(parents=True)
    executable.mkdir()
    for entry in entries:
        if entry.name == "coin_dash":
            entry.rename(executable / entry.name)
        else:
            entry.rename(resources / entry.name)
            (executable / entry.name).symlink_to(Path("../Resources") / entry.name)
    with (contents / "Info.plist").open("wb") as file:
        plistlib.dump({
            "CFBundleExecutable": "coin_dash",
            "CFBundleIdentifier": "dev.ponko.coin-dash",
            "CFBundleName": "Coin Dash",
            "CFBundlePackageType": "APPL",
            "CFBundleShortVersionString": version,
            "CFBundleVersion": version,
        }, file)
    return contents.parent


def ditto_zip(path, archive):
    run("ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", path, archive)


def sign_macos(app):
    signing = credentials(MACOS, "macOS Developer ID signing")
    notarization = credentials(NOTARY, "macOS notarization")
    if not signing:
        if notarization:
            raise RuntimeError("macOS notarization requires Developer ID signing credentials")
        return False, False
    certificate, password, identity = signing
    if not identity.startswith("Developer ID Application:"):
        raise RuntimeError("MACOS_SIGN_IDENTITY must name a Developer ID Application certificate")
    with tempfile.TemporaryDirectory(prefix="coin-dash-sign-") as directory:
        temp = Path(directory)
        keychain = temp / "signing.keychain-db"
        keychain_password = secrets.token_hex(24)
        p12 = temp / "certificate.p12"
        p12.write_bytes(base64.b64decode(certificate, validate=True))
        run("security", "create-keychain", "-p", keychain_password, keychain)
        try:
            run("security", "unlock-keychain", "-p", keychain_password, keychain)
            run("security", "import", p12, "-k", keychain, "-P", password, "-T", "/usr/bin/codesign")
            run("security", "set-key-partition-list", "-S", "apple-tool:,apple:,codesign:", "-s",
                "-k", keychain_password, keychain)
            run("codesign", "--force", "--options", "runtime", "--timestamp",
                "--keychain", keychain, "--sign", identity, app)
            run("codesign", "--verify", "--deep", "--strict", app)
        finally:
            run("security", "delete-keychain", keychain)
        if notarization:
            apple_id, app_password, team = notarization
            submission = temp / "submission.zip"
            ditto_zip(app, submission)
            run("xcrun", "notarytool", "submit", submission, "--apple-id", apple_id,
                "--password", app_password, "--team-id", team, "--wait", "--timeout", "30m")
            run("xcrun", "stapler", "staple", app)
    return True, bool(notarization)


def sign(package):
    signed = notarized = False
    if sys.platform == "darwin":
        signed, notarized = sign_macos(macos_app(package))
    elif os.name == "nt":
        if signing := credentials(WINDOWS, "Windows Authenticode signing"):
            certificate, password = signing
            with tempfile.TemporaryDirectory(prefix="coin-dash-sign-") as directory:
                pfx = Path(directory) / "certificate.pfx"
                pfx.write_bytes(base64.b64decode(certificate, validate=True))
                run(signtool(), "sign", "/fd", "SHA256", "/tr", "http://timestamp.digicert.com",
                    "/td", "SHA256", "/f", pfx, "/p", password, package / "coin_dash.exe")
            signed = True
    else:
        print("Linux desktop archive does not use Authenticode or Apple notarization.")
    if output := os.environ.get("GITHUB_OUTPUT"):
        with Path(output).open("a", encoding="utf-8") as file:
            print(f"signed={str(signed).lower()}", file=file)
            print(f"notarized={str(notarized).lower()}", file=file)


def archive_and_smoke(package, out, signed, notarized):
    out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="coin-dash-release-") as directory:
        temp = Path(directory)
        extracted = temp / "extracted"
        extracted.mkdir()
        if sys.platform == "darwin":
            archive = temp / f"{package.name}.zip"
            ditto_zip(package, archive)
            run("ditto", "-x", "-k", archive, extracted)
        elif os.name == "nt":
            archive = Path(shutil.make_archive(
                temp / package.name, "zip", root_dir=package.parent, base_dir=package.name))
            with zipfile.ZipFile(archive) as file:
                file.extractall(extracted)
        else:
            archive = Path(shutil.make_archive(
                temp / package.name, "gztar", root_dir=package.parent, base_dir=package.name))
            with tarfile.open(archive) as file:
                file.extractall(extracted, filter="data")

        payload = extracted / package.name
        if sys.platform == "darwin":
            app = payload / APP
            if signed:
                run("codesign", "--verify", "--deep", "--strict", app)
            if notarized:
                run("xcrun", "stapler", "validate", app)
            payload = app / "Contents" / "MacOS"
        elif os.name == "nt" and signed:
            run(signtool(), "verify", "/pa", payload / "coin_dash.exe")
        run(sys.executable, ROOT / "scripts/package-coin-dash.py", "smoke", "--package", payload)
        with archive.open("rb") as file:
            digest = hashlib.file_digest(file, "sha256").hexdigest()
        shutil.copyfile(archive, out / archive.name)
        (out / f"{archive.name}.sha256").write_text(f"{digest}  {archive.name}\n", encoding="ascii")
        print(f"Verified archive and checksum: {archive.name}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("step", choices=("sign", "archive"))
    parser.add_argument("--package", type=Path, required=True, help="folder made by package --out")
    parser.add_argument("--out", type=Path, default=Path("desktop-assets"))
    args = parser.parse_args()
    packages = [p for p in args.package.glob("coin-dash-*") if p.is_dir()]
    if len(packages) != 1:
        parser.error(f"expected one Coin Dash package in {args.package}")
    package = packages[0].resolve()
    if args.step == "sign":
        sign(package)
    else:
        archive_and_smoke(package, args.out,
                          signed=os.environ.get("COIN_DASH_SIGNED") == "true",
                          notarized=os.environ.get("COIN_DASH_NOTARIZED") == "true")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError) as error:
        sys.exit(str(error))
