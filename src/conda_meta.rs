use crate::models::EnvMetaSyncReport;
use serde_json::Value;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// 将硬编码的官方源或镜像站频道 URL 规范化为动态频道名，
/// 使 Conda 求解器始终跟随当前 `.condarc` 生效的镜像源。
pub fn canonicalize_channel(ch: &str) -> Option<&'static str> {
    let lower = ch.trim().to_ascii_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return None;
    }

    // 匹配 defaults 系（pkgs/main, pkgs/r, pkgs/msys2, pkgs/free）
    let is_defaults = lower.contains("/pkgs/main")
        || lower.contains("/pkgs/r/")
        || lower.ends_with("/pkgs/r")
        || lower.contains("/pkgs/msys2")
        || lower.contains("/pkgs/free");
    if is_defaults {
        return Some("defaults");
    }

    // 匹配常见 cloud 频道（conda-forge, pytorch, bioconda）
    for known in ["conda-forge", "pytorch", "bioconda"] {
        let needle_cloud = format!("/anaconda/cloud/{known}");
        let needle_org = format!("conda.anaconda.org/{known}");
        if lower.contains(&needle_cloud) || lower.contains(&needle_org) {
            return Some(known);
        }
    }

    None
}

/// 根据 conda 可执行文件路径推导 Conda 安装根目录。
pub fn infer_conda_root(conda_exe: Option<&Path>) -> Option<PathBuf> {
    if let Some(exe) = conda_exe {
        // 如 D:\Application\miniforge\Scripts\conda.exe -> D:\Application\miniforge
        if let Some(root) = exe.parent().and_then(|p| p.parent()) {
            if root.join("conda-meta").is_dir() || root.join("envs").is_dir() {
                return Some(root.to_path_buf());
            }
        }
    }
    let fallback = PathBuf::from(r"D:\Application\miniforge");
    if fallback.join("conda-meta").is_dir() || fallback.join("envs").is_dir() {
        return Some(fallback);
    }
    None
}

fn sync_single_meta_dir(
    env_name: &str,
    meta_dir: &Path,
    report: &mut EnvMetaSyncReport,
) {
    let entries = match fs::read_dir(meta_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut env_updated = 0usize;
    let mut env_denied = false;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let raw = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // 快速预过滤，避免解析不需要改的 JSON
        if !raw.contains("http://") && !raw.contains("https://") {
            continue;
        }

        let mut data: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let canonical = match data.get("channel").and_then(|v| v.as_str()) {
            Some(ch) => match canonicalize_channel(ch) {
                Some(c) => c,
                None => continue,
            },
            None => continue,
        };

        let subdir = data
            .get("subdir")
            .and_then(|v| v.as_str())
            .unwrap_or("win-64")
            .to_string();
        let fn_name = data
            .get("fn")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if let Some(obj) = data.as_object_mut() {
            obj.insert("channel".to_string(), Value::String(canonical.to_string()));
            if !fn_name.is_empty() && obj.get("url").and_then(|v| v.as_str()).is_some() {
                obj.insert(
                    "url".to_string(),
                    Value::String(format!("{canonical}/{subdir}/{fn_name}")),
                );
            }
        }

        let serialized = match serde_json::to_string_pretty(&data) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let tmp_path = path.with_extension("json.condasrc.tmp");
        match fs::write(&tmp_path, &serialized) {
            Ok(_) => match fs::rename(&tmp_path, &path) {
                Ok(_) => {
                    env_updated += 1;
                }
                Err(e) => {
                    let _ = fs::remove_file(&tmp_path);
                    if e.kind() == ErrorKind::PermissionDenied {
                        env_denied = true;
                        break;
                    }
                }
            },
            Err(e) => {
                if e.kind() == ErrorKind::PermissionDenied {
                    env_denied = true;
                    break;
                }
            }
        }
    }

    if env_updated > 0 {
        report.updated_files += env_updated;
        report.touched_envs.push(env_name.to_string());
    }
    if env_denied {
        report.permission_denied_envs.push(env_name.to_string());
    }
}

/// 扫描 base 与所有虚拟环境的 `conda-meta/*.json`，将硬编码镜像 URL 规范化为动态频道名。
pub fn sync_all_envs_metadata(conda_root: &Path) -> EnvMetaSyncReport {
    let mut report = EnvMetaSyncReport::default();

    let base_meta = conda_root.join("conda-meta");
    if base_meta.is_dir() {
        sync_single_meta_dir("base", &base_meta, &mut report);
    }

    let envs_dir = conda_root.join("envs");
    if let Ok(entries) = fs::read_dir(&envs_dir) {
        let mut env_paths: Vec<(String, PathBuf)> = entries
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                if p.is_dir() {
                    let name = e.file_name().to_string_lossy().to_string();
                    let meta = p.join("conda-meta");
                    if meta.is_dir() {
                        return Some((name, meta));
                    }
                }
                None
            })
            .collect();
        env_paths.sort_by(|a, b| a.0.cmp(&b.0));

        for (name, meta) in env_paths {
            sync_single_meta_dir(&name, &meta, &mut report);
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_mirror_and_official_channels() {
        assert_eq!(
            canonicalize_channel("https://mirrors.pku.edu.cn/anaconda/pkgs/main/win-64"),
            Some("defaults")
        );
        assert_eq!(
            canonicalize_channel("https://repo.anaconda.com/pkgs/r/noarch"),
            Some("defaults")
        );
        assert_eq!(
            canonicalize_channel("https://mirrors.tuna.tsinghua.edu.cn/anaconda/pkgs/msys2/win-64"),
            Some("defaults")
        );
        assert_eq!(
            canonicalize_channel("https://mirrors.ustc.edu.cn/anaconda/cloud/conda-forge/noarch"),
            Some("conda-forge")
        );
        assert_eq!(
            canonicalize_channel("https://conda.anaconda.org/pytorch/win-64"),
            Some("pytorch")
        );
        assert_eq!(canonicalize_channel("defaults"), None);
        assert_eq!(canonicalize_channel("conda-forge"), None);
    }
}
