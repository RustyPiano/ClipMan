import base64
import copy
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "verify_release", Path(__file__).with_name("verify-release.py")
)
VERIFY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VERIFY)


def release_fixture():
    config = {"version": "2.3.0", "productName": "ClipMan"}
    formats = {
        "darwin-aarch64-app": "ClipMan_2.3.0_aarch64.app.tar.gz",
        "darwin-x86_64-app": "ClipMan_2.3.0_x64.app.tar.gz",
        "windows-x86_64-msi": "ClipMan_2.3.0_x64_en-US.msi",
        "windows-x86_64-nsis": "ClipMan_2.3.0_x64-setup.exe",
        "linux-x86_64-appimage": "ClipMan_2.3.0_amd64.AppImage",
        "linux-x86_64-deb": "ClipMan_2.3.0_amd64.deb",
        "linux-x86_64-rpm": "ClipMan-2.3.0-1.x86_64.rpm",
    }
    names = list(formats.values())
    names += [name + ".sig" for name in names]
    names += ["ClipMan_2.3.0_aarch64.dmg", "ClipMan_2.3.0_x64.dmg", "latest.json"]
    assets = [{
        "id": index,
        "name": name,
        "size": 1,
        "state": "uploaded",
        "url": f"https://api.github.com/repos/owner/repo/releases/assets/{index}",
        "browser_download_url": f"https://github.com/owner/repo/releases/download/v2.3.0/{name}",
    } for index, name in enumerate(names, 1)]
    by_name = {asset["name"]: asset for asset in assets}
    signature = base64.b64encode(b"unit-test-signature").decode()
    platforms = {platform: {"url": by_name[name]["url"], "signature": signature}
                 for platform, name in formats.items()}
    for default, installer in {
        "darwin-aarch64": "darwin-aarch64-app",
        "darwin-x86_64": "darwin-x86_64-app",
        "windows-x86_64": "windows-x86_64-msi",
        "linux-x86_64": "linux-x86_64-appimage",
    }.items():
        platforms[default] = platforms[installer].copy()
    return config, {"version": "2.3.0", "platforms": platforms}, assets


class ReleaseVerificationTests(unittest.TestCase):
    def test_complete_release_and_missing_platform(self):
        config, manifest, assets = release_fixture()
        signed, installers = VERIFY.plan_assets(config, "v2.3.0", manifest, assets)
        self.assertEqual(len(list(signed)), 7)
        self.assertEqual(len(installers), 2)
        del manifest["platforms"]["linux-x86_64-rpm"]
        with self.assertRaisesRegex(ValueError, "缺少平台"):
            VERIFY.plan_assets(config, "v2.3.0", manifest, assets)

    def test_rejects_external_and_other_release_urls(self):
        config, manifest, assets = release_fixture()
        for url in (
            "https://example.com/update.exe",
            "https://api.github.com/repos/owner/repo/releases/assets/999999",
            "https://github.com/owner/repo/releases/download/v2.2.1/ClipMan.exe",
            assets[0]["url"] + "?token=untrusted",
        ):
            with self.subTest(url=url):
                changed = copy.deepcopy(manifest)
                changed["platforms"]["windows-x86_64-nsis"]["url"] = url
                with self.assertRaisesRegex(ValueError, "不属于当前 Release"):
                    VERIFY.plan_assets(config, "v2.3.0", changed, assets)

    def test_rejects_wrong_format_default_version_and_missing_signature(self):
        config, manifest, assets = release_fixture()
        changed = copy.deepcopy(manifest)
        changed["platforms"]["windows-x86_64-nsis"] = changed["platforms"]["darwin-aarch64-app"]
        with self.assertRaisesRegex(ValueError, "安装格式"):
            VERIFY.plan_assets(config, "v2.3.0", changed, assets)
        changed = copy.deepcopy(manifest)
        changed["platforms"]["linux-x86_64"] = changed["platforms"]["linux-x86_64-deb"]
        with self.assertRaisesRegex(ValueError, "默认更新"):
            VERIFY.plan_assets(config, "v2.3.0", changed, assets)
        changed = copy.deepcopy(manifest)
        changed["version"] = "2.2.1"
        with self.assertRaisesRegex(ValueError, "版本"):
            VERIFY.plan_assets(config, "v2.3.0", changed, assets)
        with self.assertRaisesRegex(ValueError, "缺少附件签名"):
            VERIFY.plan_assets(config, "v2.3.0", manifest, [asset for asset in assets
                               if asset["name"] != "ClipMan_2.3.0_x64-setup.exe.sig"])
        for value in ("", "not-base64!"):
            changed = copy.deepcopy(manifest)
            changed["platforms"]["windows-x86_64-nsis"]["signature"] = value
            with self.assertRaises(ValueError):
                VERIFY.plan_assets(config, "v2.3.0", changed, assets)

    def test_previous_release_and_public_key_change(self):
        releases = [
            {"tag_name": tag, "draft": draft, "prerelease": prerelease}
            for tag, draft, prerelease in (
                ("v2.3.0", False, False), ("v2.4.0", False, False),
                ("v2.2.2", True, False), ("v2.2.1", False, False),
                ("v2.2.0", False, True), ("v2.1.0", False, False),
            )
        ]
        self.assertEqual(VERIFY.previous_release(releases, "v2.3.0")["tag_name"], "v2.2.1")
        self.assertIsNone(VERIFY.previous_release(releases, "v1.0.0"))
        old_key = base64.b64encode(b"old public key").decode()
        old_config = '{"plugins":{"updater":{"pubkey":"' + old_key + '"}}}'
        source = {"content": base64.b64encode(old_config.encode()).decode()}
        with patch.object(VERIFY, "gh_json", side_effect=[[releases], source]):
            self.assertEqual(VERIFY.check_public_key("owner/repo", "v2.3.0", old_key), releases)
        with patch.object(VERIFY, "gh_json", side_effect=[[releases], source]):
            with self.assertRaisesRegex(ValueError, "公钥与 v2.2.1 不同"):
                VERIFY.check_public_key("owner/repo", "v2.3.0", base64.b64encode(b"new public key").decode())

    def test_finds_draft_release_and_rejects_missing_access(self):
        draft = {"tag_name": "v2.3.0", "draft": True, "prerelease": False, "id": 123}
        published = {"tag_name": "v2.2.1", "draft": False, "prerelease": False, "id": 122}
        self.assertEqual(VERIFY.find_release([published, draft], "v2.3.0"), draft)
        self.assertEqual(VERIFY.find_release([published, draft], "v2.2.1"), published)
        with self.assertRaisesRegex(ValueError, "草稿.*contents:write"):
            VERIFY.find_release([published], "v2.3.0")
        with patch.object(VERIFY, "gh_json", return_value=[[draft]]):
            self.assertEqual(VERIFY.check_public_key("owner/repo", "v2.3.0", "unused"), [draft])

    def test_minisign_rejects_tampered_artifact(self):
        minisign = shutil.which(os.environ.get("MINISIGN", "minisign"))
        self.assertIsNotNone(minisign, "测试需要 minisign，可通过 MINISIGN 指定可执行文件")
        VERIFY.OUTPUT.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="test-", dir=VERIFY.OUTPUT) as temporary:
            directory = Path(temporary)
            public_key = directory / "test.pub"
            secret_key = directory / "test.key"
            artifact = directory / "artifact.bin"
            signature = directory / "artifact.minisig"
            subprocess.run([minisign, "-G", "-W", "-p", str(public_key), "-s", str(secret_key)],
                           check=True, capture_output=True)
            artifact.write_bytes(b"release verification regression test")
            subprocess.run([minisign, "-S", "-s", str(secret_key), "-m", str(artifact),
                            "-x", str(signature)], check=True, capture_output=True)
            public_key.write_bytes(VERIFY.decode_tauri(base64.b64encode(public_key.read_bytes()).decode()))
            signature.write_bytes(VERIFY.decode_tauri(base64.b64encode(signature.read_bytes()).decode()))
            VERIFY.verify_signature(minisign, artifact, signature, public_key)
            artifact.write_bytes(b"modified release artifact")
            with self.assertRaisesRegex(ValueError, "签名验证失败.*公钥"):
                VERIFY.verify_signature(minisign, artifact, signature, public_key)


if __name__ == "__main__":
    unittest.main()
