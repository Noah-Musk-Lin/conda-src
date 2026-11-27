"""将 Conda 虚拟环境 conda-meta/*.json 中硬编码的官方/清华源 URL 规范化为动态频道名。

原因：
  Conda 求解器会把 conda-meta/*.json 中形如 `https://repo.anaconda.com/pkgs/main/win-64`
  或 `https://mirrors.tuna.tsinghua.edu.cn/...` 的完整 URL 当成外部硬编码频道，
  导致即使切换了 .condarc 镜像源，安装时仍会去拉取这些固定 URL。
  将其规范化为 `defaults` / `conda-forge` / `pytorch` / `bioconda` 后，
  Conda 会自动、动态地跟随 .condarc 当前激活的镜像源（PKU / 官方 / USTC）。
"""

import glob
import json
import os
import re
import shutil

BACKUP_ROOT = os.path.join(os.environ.get("APPDATA", "."), "CondaSrc", "conda_meta_backups_v2")
ENVS_ROOT = r"D:\Application\miniforge\envs"

DEFAULTS_RE = re.compile(
    r"^https?://(?:repo\.anaconda\.com|mirrors\.tuna\.tsinghua\.edu\.cn/anaconda)/pkgs/(?:main|r|msys2|free)(?:/|$)",
    re.IGNORECASE,
)
CLOUD_RE = re.compile(
    r"^https?://(?:conda\.anaconda\.org|mirrors\.tuna\.tsinghua\.edu\.cn/anaconda/cloud)/(conda-forge|pytorch|bioconda)(?:/|$)",
    re.IGNORECASE,
)


def canonicalize_channel(ch: str) -> str | None:
    if DEFAULTS_RE.match(ch):
        return "defaults"
    m = CLOUD_RE.match(ch)
    if m:
        return m.group(1).lower()
    return None


def main() -> None:
    if not os.path.isdir(ENVS_ROOT):
        print(f"未找到环境目录: {ENVS_ROOT}")
        return

    total_updated = 0
    for env_name in sorted(os.listdir(ENVS_ROOT)):
        meta_dir = os.path.join(ENVS_ROOT, env_name, "conda-meta")
        if not os.path.isdir(meta_dir):
            continue

        env_count = 0
        for path in glob.glob(os.path.join(meta_dir, "*.json")):
            with open(path, "r", encoding="utf-8") as f:
                data = json.load(f)

            ch = data.get("channel")
            if not isinstance(ch, str):
                continue

            canonical = canonicalize_channel(ch)
            if not canonical:
                continue

            subdir = data.get("subdir") or "win-64"
            fn = data.get("fn") or ""

            data["channel"] = canonical
            if isinstance(data.get("url"), str) and fn:
                data["url"] = f"{canonical}/{subdir}/{fn}"

            env_backup_dir = os.path.join(BACKUP_ROOT, env_name)
            os.makedirs(env_backup_dir, exist_ok=True)
            shutil.copy2(path, os.path.join(env_backup_dir, os.path.basename(path)))

            tmp_path = path + ".tmp"
            with open(tmp_path, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=2, ensure_ascii=False)
            os.replace(tmp_path, path)
            env_count += 1
            total_updated += 1

        if env_count:
            print(f"[已规范化] {env_name}: {env_count} 个包 -> 动态跟随 .condarc")

    print(f"\n完成！共规范化 {total_updated} 个包元数据。")
    print(f"修改前备份保存在：{BACKUP_ROOT}")


if __name__ == "__main__":
    main()
