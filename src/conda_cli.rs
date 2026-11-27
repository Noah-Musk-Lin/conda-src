use crate::models::CleanCacheResult;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 探测本机可用的 conda 可执行文件路径。
pub fn discover_conda_executable() -> Option<PathBuf> {
    // 1. 优先检查 CONDA_EXE 环境变量
    if let Ok(conda_exe) = env::var("CONDA_EXE") {
        let p = PathBuf::from(conda_exe);
        if p.is_file() {
            return Some(p);
        }
    }

    // 2. 检查 PATH
    if let Ok(path_var) = env::var("PATH") {
        for dir in env::split_paths(&path_var) {
            for name in ["conda.exe", "conda.bat", "conda"] {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    // 3. 检查常见本地安装目录（包含本机 D:\Application\miniforge）
    let mut roots = vec![
        PathBuf::from(r"D:\Application\miniforge"),
        PathBuf::from(r"D:\Application\miniforge3"),
        PathBuf::from(r"D:\Application\miniconda3"),
        PathBuf::from(r"D:\Application\anaconda3"),
        PathBuf::from(r"C:\ProgramData\miniforge3"),
        PathBuf::from(r"C:\ProgramData\miniconda3"),
        PathBuf::from(r"C:\ProgramData\anaconda3"),
    ];
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join("miniforge3"));
        roots.push(home.join("miniconda3"));
        roots.push(home.join("anaconda3"));
        roots.push(home.join("mambaforge"));
    }

    for root in roots {
        for rel in [
            r"Scripts\conda.exe",
            r"condabin\conda.bat",
            r"bin\conda",
        ] {
            let candidate = root.join(rel);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

/// 执行 `conda clean -i -y` 清理索引缓存。
pub async fn clean_index_cache() -> Result<CleanCacheResult, String> {
    let conda_path = discover_conda_executable()
        .ok_or_else(|| "未检测到本机 conda 可执行文件，请检查安装路径或 PATH".to_string())?;

    tokio::task::spawn_blocking(move || run_clean_sync(&conda_path))
        .await
        .map_err(|e| format!("后台任务执行失败: {e}"))?
}

fn run_clean_sync(conda_path: &Path) -> Result<CleanCacheResult, String> {
    let display_cmd = format!("\"{}\" clean -i -y", conda_path.display());

    let is_bat = conda_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("bat") || e.eq_ignore_ascii_case("cmd"))
        .unwrap_or(false);

    let mut cmd = if is_bat {
        let mut c = Command::new("cmd");
        c.args(["/C", &conda_path.to_string_lossy(), "clean", "-i", "-y"]);
        c
    } else {
        let mut c = Command::new(conda_path);
        c.args(["clean", "-i", "-y"]);
        c
    };

    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("启动 conda clean 失败: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    Ok(CleanCacheResult {
        ok: output.status.success(),
        command_used: display_cmd,
        stdout,
        stderr,
    })
}

/// 调用 `conda config --show channels default_channels custom_channels` 获取 Conda 合并后的实际生效配置。
pub async fn get_effective_config() -> Result<String, String> {
    let conda_path = discover_conda_executable()
        .ok_or_else(|| "未检测到本机 conda 可执行文件".to_string())?;

    tokio::task::spawn_blocking(move || run_show_config_sync(&conda_path))
        .await
        .map_err(|e| format!("后台任务执行失败: {e}"))?
}

fn run_show_config_sync(conda_path: &Path) -> Result<String, String> {
    let is_bat = conda_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("bat") || e.eq_ignore_ascii_case("cmd"))
        .unwrap_or(false);

    let mut cmd = if is_bat {
        let mut c = Command::new("cmd");
        c.args([
            "/C",
            &conda_path.to_string_lossy(),
            "config",
            "--show",
            "channels",
            "default_channels",
            "custom_channels",
        ]);
        c
    } else {
        let mut c = Command::new(conda_path);
        c.args([
            "config",
            "--show",
            "channels",
            "default_channels",
            "custom_channels",
        ]);
        c
    };

    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("执行 conda config --show 失败: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("conda config --show 返回错误: {stderr}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
