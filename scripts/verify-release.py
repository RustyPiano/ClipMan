import argparse
import base64
import json
from pathlib import Path
import re
import shutil
import subprocess
from urllib.parse import quote


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "src-tauri" / "target" / "release-verification"
VERSION = re.compile(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def gh_json(endpoint, *arguments):
    result = subprocess.run(
        ["gh", "api", "--hostname", "github.com", endpoint, *arguments],
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    return json.loads(result.stdout)


def decode_tauri(value):
    require(isinstance(value, str) and value.strip(), "签名或公钥不能为空")
    decoded = base64.b64decode(value.strip(), validate=True)
    require(bool(decoded), "签名或公钥解码后不能为空")
    return decoded


def previous_release(releases, tag):
    target = tuple(map(int, VERSION.fullmatch(tag).groups()))
    candidates = []
    for release in releases:
        match = VERSION.fullmatch(release["tag_name"])
        if release["draft"] or release["prerelease"] or not match:
            continue
        version = tuple(map(int, match.groups()))
        if version < target:
            candidates.append((version, release))
    return max(candidates, key=lambda item: item[0])[1] if candidates else None


def check_public_key(repo, tag, public_key):
    pages = gh_json(f"repos/{repo}/releases?per_page=100", "--paginate", "--slurp")
    releases = [release for page in pages for release in page]
    previous = previous_release(releases, tag)
    if previous is None:
        print("没有更早的公开正式版本，首次发布无需比较公钥。", flush=True)
        return releases
    previous_tag = previous["tag_name"]
    source = gh_json(
        f"repos/{repo}/contents/src-tauri/tauri.conf.json?ref={quote(previous_tag, safe='')}"
    )
    config = json.loads(base64.b64decode(source["content"]))
    require(
        decode_tauri(config["plugins"]["updater"]["pubkey"]) == decode_tauri(public_key),
        f"updater 公钥与 {previous_tag} 不同，已阻止常规发布；换钥需要独立迁移安排。",
    )
    print(f"updater 公钥与 {previous_tag} 一致。", flush=True)
    return releases


def find_release(releases, tag):
    matches = [release for release in releases if release["tag_name"] == tag]
    require(
        len(matches) == 1,
        f"未找到唯一的 {tag} Release；请检查仓库、标签和认证权限。"
        "读取草稿需要仓库 push 权限，Actions token 需要 contents:write。",
    )
    return matches[0]


def asset_index(assets):
    by_name = {}
    by_url = {}
    for asset in assets:
        name = asset["name"]
        require(asset["state"] == "uploaded" and asset["size"] > 0, f"附件未上传完整：{name}")
        by_name[name] = asset
        by_url[asset["url"]] = asset
        by_url[asset["browser_download_url"]] = asset
    return by_name, by_url


def resolve_asset(url, by_url):
    require(url in by_url, f"更新地址不属于当前 Release：{url}")
    return by_url[url]


def plan_assets(config, tag, manifest, assets):
    version = config["version"]
    require(tag == f"v{version}", "标签与本地 tauri.conf.json 版本不一致")
    require(manifest["version"].removeprefix("v") == version, "latest.json 版本与标签不一致")
    by_name, by_url = asset_index(assets)
    product = re.escape(config["productName"])
    prefix = rf"{product}_{re.escape(version)}_"
    patterns = {
        "darwin-aarch64-app": prefix + r"(?:aarch64|arm64)\.app\.tar\.gz",
        "darwin-x86_64-app": prefix + r"(?:x64|x86_64)\.app\.tar\.gz",
        "windows-x86_64-msi": prefix + r"(?:x64|x86_64)(?:_[A-Za-z0-9-]+)?\.msi",
        "windows-x86_64-nsis": prefix + r"(?:x64|x86_64)-setup\.exe",
        "linux-x86_64-appimage": prefix + r"(?:amd64|x86_64)\.AppImage",
        "linux-x86_64-deb": prefix + r"(?:amd64|x86_64)\.deb",
        "linux-x86_64-rpm": rf"{product}-{re.escape(version)}-[0-9]+\.x86_64\.rpm",
    }
    defaults = {
        "darwin-aarch64": ("darwin-aarch64-app",),
        "darwin-x86_64": ("darwin-x86_64-app",),
        "windows-x86_64": ("windows-x86_64-msi", "windows-x86_64-nsis"),
        "linux-x86_64": ("linux-x86_64-appimage",),
    }
    platforms = manifest["platforms"]
    require(isinstance(platforms, dict), "latest.json platforms 必须为对象")
    missing = (patterns.keys() | defaults.keys()) - platforms.keys()
    require(not missing, f"latest.json 缺少平台：{', '.join(sorted(missing))}")
    resolved = {}
    for platform, entry in platforms.items():
        asset = resolve_asset(entry["url"], by_url)
        signature = decode_tauri(entry["signature"])
        require(asset["name"] + ".sig" in by_name, f"缺少附件签名：{asset['name']}.sig")
        if platform in patterns:
            require(
                re.fullmatch(patterns[platform], asset["name"]) is not None,
                f"{platform} 指向了错误的版本、架构或安装格式：{asset['name']}",
            )
        resolved[platform] = (asset, signature)
    for platform, choices in defaults.items():
        require(
            any(resolved[platform] == resolved[choice] for choice in choices),
            f"{platform} 默认更新与对应安装格式不一致",
        )
    installers = []
    for architecture in (r"(?:aarch64|arm64)", r"(?:x64|x86_64)"):
        matches = [asset for name, asset in by_name.items()
                   if re.fullmatch(prefix + architecture + r"\.dmg", name)]
        require(len(matches) == 1, f"缺少或重复 macOS 安装包：{architecture}")
        installers.extend(matches)
    # 默认平台与安装格式条目指向同一附件，每个附件只验签一次。
    signatures = {asset["id"]: (asset, signature) for asset, signature in resolved.values()}
    return signatures.values(), installers


def download_asset(repo, asset, directory):
    destination = directory / f"{asset['id']}-{asset['name']}"
    # Release 附件替换后 ID 会改变；已下载内容仍需经过下面的验签。
    if not destination.is_file() or destination.stat().st_size != asset["size"]:
        with destination.open("wb") as output:
            subprocess.run(
                ["gh", "api", "--hostname", "github.com",
                 f"repos/{repo}/releases/assets/{asset['id']}",
                 "-H", "Accept: application/octet-stream"],
                stdout=output,
                check=True,
            )
    require(destination.stat().st_size == asset["size"], f"附件下载不完整：{asset['name']}")
    return destination


def verify_signature(minisign, artifact, signature_file, public_key_file):
    result = subprocess.run(
        [minisign, "-V", "-m", str(artifact), "-x", str(signature_file),
         "-p", str(public_key_file)],
        capture_output=True,
        text=True,
    )
    require(
        result.returncode == 0,
        f"签名验证失败：{artifact.name}；签名：{signature_file.name}；"
        f"公钥：{public_key_file.name}。{result.stderr.strip()}",
    )


def main():
    parser = argparse.ArgumentParser(description="校验 ClipMan Release 产物及 updater 公钥")
    parser.add_argument("--repo", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--check-key-only", action="store_true")
    parser.add_argument("--minisign", default="minisign")
    args = parser.parse_args()
    require(re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", args.repo), "仓库名称应为 owner/repo")
    require(all(part not in (".", "..") for part in args.repo.split("/")), "仓库名称不合法")
    require(VERSION.fullmatch(args.tag), "标签必须为 vX.Y.Z")
    config = json.loads((ROOT / "src-tauri" / "tauri.conf.json").read_text())
    require(args.tag == f"v{config['version']}", "标签与本地 tauri.conf.json 版本不一致")
    public_key = config["plugins"]["updater"]["pubkey"]
    decode_tauri(public_key)
    releases = check_public_key(args.repo, args.tag, public_key)
    if args.check_key_only:
        return
    minisign = shutil.which(args.minisign)
    require(minisign is not None, "未找到 minisign，请安装或通过 --minisign 指定路径")
    release = find_release(releases, args.tag)
    require(not release["prerelease"], "常规发布不能标记为预发布版本")
    assets = [asset for page in gh_json(
        f"repos/{args.repo}/releases/{release['id']}/assets?per_page=100",
        "--paginate", "--slurp",
    ) for asset in page]
    by_name, _ = asset_index(assets)
    require("latest.json" in by_name, "Release 缺少 latest.json")
    directory = OUTPUT / str(release["id"])
    directory.mkdir(parents=True, exist_ok=True)
    manifest = json.loads(download_asset(args.repo, by_name["latest.json"], directory).read_text())
    signed_assets, installers = plan_assets(config, args.tag, manifest, assets)
    public_key_file = directory / "updater.pub"
    public_key_file.write_bytes(decode_tauri(public_key))
    verified = 0
    for asset, expected_signature in signed_assets:
        artifact = download_asset(args.repo, asset, directory)
        signature_asset = by_name[asset["name"] + ".sig"]
        signature = download_asset(args.repo, signature_asset, directory)
        require(
            decode_tauri(signature.read_text()) == expected_signature,
            f"latest.json 与附件签名不一致：{asset['name']}",
        )
        decoded_signature = directory / f"{asset['id']}.minisig"
        decoded_signature.write_bytes(expected_signature)
        verify_signature(minisign, artifact, decoded_signature, public_key_file)
        print(f"签名通过：{asset['name']}", flush=True)
        verified += 1
    for installer in installers:
        download_asset(args.repo, installer, directory)
    print(f"{args.tag} 校验通过：四个平台，{verified} 份 updater 签名，macOS 双架构安装包。")


if __name__ == "__main__":
    main()
