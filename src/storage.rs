use crate::models::{BackupEntry, CustomMirrorInput, MirrorProfile};
use crate::presets::builtin_mirrors;
use chrono::{DateTime, Local};
use std::fs;
use std::path::{Path, PathBuf};

const ORIGINAL_BACKUP_NAME: &str = "original.condarc.bak";
const ORIGINAL_ROOT_BACKUP_NAME: &str = "original_root.condarc.bak";

pub fn app_data_dir() -> Result<PathBuf, String> {
    let base = dirs::config_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| "无法定位用户配置目录".to_string())?;
    let dir = base.join("CondaSrc");
    fs::create_dir_all(&dir).map_err(|e| format!("创建应用配置目录失败: {e}"))?;
    Ok(dir)
}

pub fn backups_dir() -> Result<PathBuf, String> {
    let dir = app_data_dir()?.join("backups");
    fs::create_dir_all(&dir).map_err(|e| format!("创建备份目录失败: {e}"))?;
    Ok(dir)
}

fn custom_mirrors_file() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("custom_mirrors.json"))
}

pub fn load_custom_mirrors() -> Result<Vec<MirrorProfile>, String> {
    let path = custom_mirrors_file()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("读取自定义源配置失败: {e}"))?;
    let mut mirrors: Vec<MirrorProfile> =
        serde_json::from_str(&content).map_err(|e| format!("解析自定义源 JSON 失败: {e}"))?;
    for m in &mut mirrors {
        m.is_builtin = false;
    }
    Ok(mirrors)
}

pub fn save_custom_mirrors(mirrors: &[MirrorProfile]) -> Result<(), String> {
    let path = custom_mirrors_file()?;
    let json = serde_json::to_string_pretty(mirrors)
        .map_err(|e| format!("序列化自定义源配置失败: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("保存自定义源配置失败: {e}"))?;
    Ok(())
}

pub fn load_all_mirrors() -> Result<Vec<MirrorProfile>, String> {
    let mut all = builtin_mirrors();
    let custom = load_custom_mirrors()?;
    all.extend(custom);
    Ok(all)
}

pub fn upsert_custom_mirror(input: CustomMirrorInput) -> Result<MirrorProfile, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("镜像源名称不能为空".to_string());
    }
    let default_channels: Vec<String> = input
        .default_channels
        .into_iter()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if default_channels.is_empty() && input.custom_channels.is_empty() {
        return Err("至少需要填写一个 default_channels 或 custom_channels".to_string());
    }

    let custom_channels = input
        .custom_channels
        .into_iter()
        .map(|(k, v)| (k.trim().to_string(), v.trim().trim_end_matches('/').to_string()))
        .filter(|(k, v)| !k.is_empty() && !v.is_empty())
        .collect::<std::collections::BTreeMap<_, _>>();

    let probe_url = {
        let trimmed = input.probe_url.trim().to_string();
        if !trimmed.is_empty() {
            trimmed
        } else if let Some(cf) = custom_channels.get("conda-forge") {
            format!("{}/conda-forge/noarch/repodata.json", cf.trim_end_matches('/'))
        } else if let Some(first_def) = default_channels.first() {
            format!("{}/noarch/repodata.json", first_def.trim_end_matches('/'))
        } else {
            String::new()
        }
    };

    let mut customs = load_custom_mirrors()?;
    let id = match input.id {
        Some(existing_id) if !existing_id.trim().is_empty() => existing_id.trim().to_string(),
        _ => format!("custom-{}", Local::now().timestamp_millis()),
    };

    if builtin_mirrors().iter().any(|b| b.id == id) {
        return Err("不能覆盖内置镜像源".to_string());
    }

    let profile = MirrorProfile {
        id: id.clone(),
        name,
        description: input.description.trim().to_string(),
        is_builtin: false,
        default_channels,
        custom_channels,
        probe_url,
    };

    if let Some(pos) = customs.iter().position(|m| m.id == id) {
        customs[pos] = profile.clone();
    } else {
        customs.push(profile.clone());
    }

    save_custom_mirrors(&customs)?;
    Ok(profile)
}

pub fn delete_custom_mirror(id: &str) -> Result<(), String> {
    if builtin_mirrors().iter().any(|b| b.id == id) {
        return Err("内置镜像源不可删除".to_string());
    }
    let mut customs = load_custom_mirrors()?;
    let before = customs.len();
    customs.retain(|m| m.id != id);
    if customs.len() == before {
        return Err("未找到指定的自定义镜像源".to_string());
    }
    save_custom_mirrors(&customs)?;
    Ok(())
}

/// 首次修改前自动备份原始用户级 `.condarc`（如尚未存在 `original.condarc.bak`）。
pub fn ensure_original_backup(condarc_path: &Path) -> Result<Option<String>, String> {
    if !condarc_path.exists() {
        return Ok(None);
    }
    let bdir = backups_dir()?;
    let original_path = bdir.join(ORIGINAL_BACKUP_NAME);
    if original_path.exists() {
        return Ok(None);
    }
    fs::copy(condarc_path, &original_path)
        .map_err(|e| format!("创建初始备份失败: {e}"))?;
    Ok(Some(ORIGINAL_BACKUP_NAME.to_string()))
}

/// 首次修改根目录 `.condarc` 前自动备份（如尚未存在 `original_root.condarc.bak`）。
pub fn ensure_original_root_backup(root_condarc_path: &Path) -> Result<Option<String>, String> {
    if !root_condarc_path.exists() {
        return Ok(None);
    }
    let bdir = backups_dir()?;
    let original_path = bdir.join(ORIGINAL_ROOT_BACKUP_NAME);
    if original_path.exists() {
        return Ok(None);
    }
    fs::copy(root_condarc_path, &original_path)
        .map_err(|e| format!("创建根目录初始备份失败: {e}"))?;
    Ok(Some(ORIGINAL_ROOT_BACKUP_NAME.to_string()))
}

pub fn is_root_backup_filename(filename: &str) -> bool {
    filename == ORIGINAL_ROOT_BACKUP_NAME || filename.starts_with("root_condarc_")
}

/// 创建一份带时间戳的手动/快照备份。
pub fn create_timestamped_backup(condarc_path: &Path) -> Result<String, String> {
    if !condarc_path.exists() {
        return Err("当前 .condarc 文件不存在，无法创建备份".to_string());
    }
    let bdir = backups_dir()?;
    let name = format!("condarc_{}.bak", Local::now().format("%Y%m%d_%H%M%S"));
    let target = bdir.join(&name);
    fs::copy(condarc_path, &target).map_err(|e| format!("创建备份失败: {e}"))?;
    Ok(name)
}

pub fn list_backups() -> Result<Vec<BackupEntry>, String> {
    let bdir = backups_dir()?;
    let mut entries = Vec::new();

    for entry in fs::read_dir(&bdir).map_err(|e| format!("读取备份目录失败: {e}"))? {
        let entry = entry.map_err(|e| format!("读取备份项失败: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) if name.ends_with(".bak") => name.to_string(),
            _ => continue,
        };
        let meta = entry
            .metadata()
            .map_err(|e| format!("读取备份文件信息失败: {e}"))?;
        let created_at = meta
            .modified()
            .ok()
            .map(|t| {
                let dt: DateTime<Local> = t.into();
                dt.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .unwrap_or_else(|| "未知时间".to_string());

        let is_root = is_root_backup_filename(&filename);
        let is_original =
            filename == ORIGINAL_BACKUP_NAME || filename == ORIGINAL_ROOT_BACKUP_NAME;
        let label = if filename == ORIGINAL_BACKUP_NAME {
            "用户级初始自动备份（首次修改前）".to_string()
        } else if filename == ORIGINAL_ROOT_BACKUP_NAME {
            "根目录初始自动备份（首次修改前）".to_string()
        } else if is_root {
            format!("根目录快照 ({})", filename.trim_end_matches(".bak"))
        } else {
            format!("用户级快照 ({})", filename.trim_end_matches(".bak"))
        };

        entries.push(BackupEntry {
            filename,
            label,
            target: if is_root { "root" } else { "user" }.to_string(),
            is_original,
            created_at,
            size_bytes: meta.len(),
        });
    }

    // 初始备份排在最前，其余按文件名倒序（最新在前）
    entries.sort_by(|a, b| match (a.is_original, b.is_original) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => b.filename.cmp(&a.filename),
    });

    Ok(entries)
}

pub fn read_backup_content(filename: &str) -> Result<String, String> {
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err("非法的备份文件名".to_string());
    }
    let path = backups_dir()?.join(filename);
    if !path.exists() {
        return Err("指定的备份文件不存在".to_string());
    }
    fs::read_to_string(&path).map_err(|e| format!("读取备份文件失败: {e}"))
}

pub fn delete_backup(filename: &str) -> Result<(), String> {
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err("非法的备份文件名".to_string());
    }
    let path = backups_dir()?.join(filename);
    if !path.exists() {
        return Err("指定的备份文件不存在".to_string());
    }
    fs::remove_file(&path).map_err(|e| format!("删除备份失败: {e}"))
}
