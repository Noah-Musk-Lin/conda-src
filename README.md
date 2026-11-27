# CondaSrc — Conda 镜像源切换器

基于 **Rust + Tauri v2** 构建的轻量级 Windows 桌面工具，用于安全、快速地切换用户级 `.condarc` 镜像源（无需 Node.js，纯 Cargo 编译）。

## 核心特性

- **精准更新 `.condarc`**：仅修改 `default_channels` 与 `custom_channels`，完整保留 `channels`、`show_channel_urls`、`proxy_servers`、`envs_dirs` 等其他自定义配置，采用临时文件原子替换防止配置损坏。
- **内置可用镜像源**：
  - **北京大学镜像站 (PKU)**
  - **Conda 官方源 (Official)**
  - **中科大镜像站 (USTC)**
  - *（已剔除停止服务的 TUNA Anaconda 镜像）*
- **自定义源管理**：支持新增、编辑、删除自定义镜像源，持久化保存在 `%APPDATA%\CondaSrc\custom_mirrors.json`。
- **并发延迟测速**：异步并发探测所有镜像源 `repodata.json` 响应延迟（ms）与可用状态。
- **自动初始备份与快照还原**：首次修改 `.condarc` 前自动保存 `original.condarc.bak` 至 `%APPDATA%\CondaSrc\backups\`，同时支持随时创建时间戳快照、预览与一键还原。
- **一键清理索引缓存**：自动探测本机 `conda` 路径（支持 `miniforge` / `miniconda` / `anaconda`），支持手动或切换后自动执行 `conda clean -i -y`。

## 构建与运行

```powershell
# 开发模式运行
cargo run

# 编译 Release 单文件可执行程序（输出在 target\release\conda-src.exe）
cargo build --release

# 运行单元测试
cargo test
```
