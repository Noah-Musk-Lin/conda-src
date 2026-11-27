# CondaSrc — Conda 镜像源切换器

基于 **Rust + Tauri v2** 构建的轻量级 Windows 桌面工具，用于安全、精准地切换 Conda 镜像源（支持 **PKU 北大镜像 / 官方源 / USTC 中科大镜像 / 自定义源**），零 Node.js 依赖，单文件 `.exe` 双击即用，无控制台黑窗。

---

## 目录

- [核心特性](#核心特性)
- [快速开始（直接下载使用）](#快速开始直接下载使用)
- [功能与界面说明](#功能与界面说明)
  - [1. 顶部快捷开关与工具栏](#1-顶部快捷开关与工具栏)
  - [2. 镜像源切换与自定义源](#2-镜像源切换与自定义源)
  - [3. 配置文件预览与生效验证](#3-配置文件预览与生效验证)
  - [4. 自动备份与一键还原](#4-自动备份与一键还原)
- [常见问题（FAQ）](#常见问题faq)
- [数据存储位置与换机迁移](#数据存储位置与换机迁移)
- [从源码编译与发布 Release](#从源码编译与发布-release)

---

## 核心特性

- **精准更新 `.condarc`**：仅更新 `default_channels` 与 `custom_channels`，完整保留 `channels`、`show_channel_urls`、`proxy_servers`、`envs_dirs` 及其他自定义频道配置；采用临时文件原子替换写入，防止配置文件损坏。
- **双 `.condarc` 同步更新**：自动检测 Conda 环境根目录下的 `.condarc`（如 `miniforge\.condarc`），防止 Conda 合并多份 `default_channels` 导致旧镜像残留。
- **虚拟环境历史索引规范化**：切换源时可自动扫描 `base` 与 `envs\*` 下的 `conda-meta\*.json`，将硬编码的旧镜像 URL 规范化为动态频道名（`defaults` / `conda-forge` / `pytorch` / `bioconda`），使旧环境始终跟随当前 `.condarc`。
- **内置可用镜像源**：
  - **北京大学镜像站 (PKU)**（国内主力推荐）
  - **Conda 官方源 (Official)**（包最新、无同步延迟）
  - **中科大镜像站 (USTC)**（国内备用）
  - *（已剔除已停止服务的 TUNA Anaconda 镜像）*
- **自定义镜像源管理**：支持新增、编辑、删除自定义或内网私有源，持久化保存在本地配置文件中。
- **并发延迟测速**：异步并发探测所有镜像源的 `repodata.json` 响应延迟（ms）与可用状态。
- **首次修改自动备份与快照还原**：首次修改前自动保留初始配置备份，随时可预览并一键回滚。
- **一键清理索引缓存**：自动探测本机 `conda` 路径，后台无黑窗执行 `conda clean -i -y`。

---

## 快速开始（直接下载使用）

1. 打开本仓库右侧的 **Releases** 页面，下载最新版 `conda-src-vX.Y.Z-windows-x64.exe`。
2. 放到任意目录，**双击直接运行**（Windows 10 / 11 自带 WebView2 运行时，无需安装 Rust 或其他依赖）。
3. 点击目标镜像卡片上的 **「切换到此源」** 即可完成切换。

> **权限提示**：如果你的 Conda 虚拟环境最初是在管理员终端创建的，`envs\*\conda-meta` 目录可能需要管理员权限才能写入。若希望同步规范化这些旧环境的索引，右键 `conda-src.exe` 选择 **「以管理员身份运行」** 即可；普通用户权限下 `.condarc` 切换仍可正常工作。

---

## 功能与界面说明

### 1. 顶部快捷开关与工具栏

| 控件 | 作用与建议 |
| :--- | :--- |
| **同步根目录**（默认开启） | 当检测到 Conda 根目录存在 `.condarc`（如 `miniforge\.condarc`）时显示。开启后会同步更新用户级与根目录两份 `.condarc`，避免多文件合并导致旧源残留。 |
| **同步环境索引**（默认开启） | 切换源时扫描并规范化各虚拟环境 `conda-meta\*.json` 中的硬编码镜像 URL，使其自动跟随当前镜像。 |
| **切换后清缓存**（默认关闭） | 开启后每次切换源会自动执行 `conda clean -i -y`。日常切换**建议保持关闭**（Conda 会按不同源 URL 分别缓存索引，频繁清空会重新下载几十 MB 的 `conda-forge` 索引）。 |
| **镜像测速** | 并发向所有镜像源的 `repodata.json` 发起 HEAD 探测，显示实时毫秒延迟（绿色 ≤200ms、黄色 ≤600ms、红色超时/失败）。 |
| **清理索引** | 手动执行一次 `conda clean -i -y`，适用于索引损坏或强制刷新的场景。 |

### 2. 镜像源切换与自定义源

- 点击卡片底部的 **「切换到此源」** 即可生效；若检测到用户级与根目录 `.condarc` 源不一致，顶部会弹出黄色提示并显示 **「同步统一到此源」**。
- 点击右上角 **「新增自定义源」** 可添加新的镜像站（如高校/公司内网镜像），支持自定义：
  - `default_channels`（每行一个 URL）
  - `custom_channels`（每行一条 `频道名=URL`，如 `conda-forge=https://...`）
  - 测速探测 URL（留空则自动根据通道推导）

### 3. 配置文件预览与生效验证

切换到 **`.condarc 预览`** 标签页可以：
- 实时查看用户级 `.condarc` 与根目录 `.condarc` 的完整 YAML 内容；
- 点击 **「验证实际生效配置」**，直接调用本机 `conda config --show channels default_channels custom_channels` 核对 Conda 合并后的最终生效结果。

### 4. 自动备份与一键还原

切换到 **「备份与还原」** 标签页可以：
- 查看首次修改前自动生成的 **初始自动备份**（`original.condarc.bak` / `original_root.condarc.bak`）；
- 点击 **「立即创建快照」** 保存当前配置的时间戳快照；
- 对任意备份进行 **预览** 或 **一键还原**（系统会自动识别该备份属于用户级还是根目录 `.condarc`）。

---

## 常见问题（FAQ）

### 1. 为什么运行 `conda config --show channels` 看到的内容没变？
`channels` 里显示的 `conda-forge`、`defaults` 只是**频道别名与优先级顺序**，不是实际下载地址。真正决定下载 URL 的是 `default_channels` 和 `custom_channels`。验证当前实际生效的下载地址请运行：
```powershell
conda info | findstr "https://"
# 或
conda config --show default_channels custom_channels
```

### 2. 为什么切完源之后，某个旧虚拟环境装包时还是显示以前的旧镜像？
Conda 求解器会自动读取当前环境 `envs\<环境名>\conda-meta\*.json` 中已安装包的历史来源 URL。保持顶部 **「同步环境索引」** 开启并（必要时以管理员身份）切换一次镜像即可一次性规范化所有旧环境；也可以临时在命令行加 `--override-channels -c conda-forge -c defaults`。

### 3. 换一台电脑怎么使用？
直接拷贝单个 `conda-src.exe` 到新的 Windows 10/11 电脑即可双击运行，程序会自动探测新电脑当前用户的 `.condarc` 以及 `CONDA_EXE` / `PATH` / 常见 Conda 安装路径。

---

## 数据存储位置与换机迁移

应用数据保存在当前用户的 `%APPDATA%\CondaSrc\` 目录下：
- `custom_mirrors.json`：用户自行添加的自定义镜像源（换机时拷贝此文件即可迁移自定义源）
- `backups\`：自动初始备份与手动快照备份

---

## 从源码编译与发布 Release

### 环境要求
- Windows 10 / 11（内置 WebView2）
- Rust 工具链（`rustc` + `cargo`），**无需 Node.js**

### 本地开发与编译
```powershell
# 运行单元测试
cargo test

# 开发模式启动
cargo run

# 编译优化压缩后的 Release 单文件（已开启 LTO 与 strip）
cargo build --release
```

### 使用 GitHub CLI (`gh`) 发布 Release
```powershell
# 1. 编译 Release 可执行文件
cargo build --release

# 2. 创建标签并上传 Release 附件（请将路径替换为实际生成的 exe 路径）
gh release create v0.1.0 .\target\release\conda-src.exe#conda-src-v0.1.0-windows-x64.exe `
  --title "CondaSrc v0.1.0" `
  --notes "首个正式版本：支持 PKU/官方/USTC/自定义源切换、双 .condarc 同步、环境索引规范化、并发测速与自动备份还原。"
```
