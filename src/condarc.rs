use crate::models::{CondarcSnapshot, MirrorProfile};
use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn user_condarc_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "无法定位用户主目录".to_string())?;
    Ok(home.join(".condarc"))
}

/// 探测是否存在 conda 根目录下的 `.condarc` 路径。
pub fn detect_root_condarc_path(conda_exe: Option<&Path>) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(exe) = conda_exe {
        // 例如 D:\Application\miniforge\Scripts\conda.exe -> D:\Application\miniforge\.condarc
        if let Some(parent) = exe.parent().and_then(|p| p.parent()) {
            candidates.push(parent.join(".condarc"));
        }
    }
    candidates.push(PathBuf::from(r"D:\Application\miniforge\.condarc"));
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join("miniforge3").join(".condarc"));
        candidates.push(home.join("miniconda3").join(".condarc"));
        candidates.push(home.join("anaconda3").join(".condarc"));
    }

    for path in candidates {
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn normalize_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_lowercase()
}

fn extract_channels_from_yaml(root: &Mapping) -> (Vec<String>, BTreeMap<String, String>) {
    let mut default_channels = Vec::new();
    if let Some(Value::Sequence(seq)) = root.get(Value::String("default_channels".to_string())) {
        for item in seq {
            if let Some(s) = item.as_str() {
                default_channels.push(normalize_url(s));
            }
        }
    }

    let mut custom_channels = BTreeMap::new();
    if let Some(Value::Mapping(map)) = root.get(Value::String("custom_channels".to_string())) {
        for (k, v) in map {
            if let (Some(ks), Some(vs)) = (k.as_str(), v.as_str()) {
                custom_channels.insert(ks.trim().to_string(), normalize_url(vs));
            }
        }
    }

    (default_channels, custom_channels)
}

fn match_active_mirror(
    root: &Mapping,
    mirrors: &[MirrorProfile],
) -> (Option<String>, String) {
    let (cur_defaults, cur_customs) = extract_channels_from_yaml(root);

    for mirror in mirrors {
        let target_defaults: Vec<String> = mirror
            .default_channels
            .iter()
            .map(|u| normalize_url(u))
            .collect();

        // 判断 default_channels 是否完全匹配（或当镜像未定义 default_channels 时跳过）
        let defaults_match = if target_defaults.is_empty() {
            true
        } else {
            cur_defaults == target_defaults
        };

        // 判断 mirror 中定义的所有 custom_channels 是否都在当前配置中一致
        let customs_match = mirror.custom_channels.iter().all(|(k, v)| {
            cur_customs
                .get(k)
                .map(|cur_v| cur_v == &normalize_url(v))
                .unwrap_or(false)
        });

        if defaults_match && customs_match && (!target_defaults.is_empty() || !mirror.custom_channels.is_empty()) {
            return (Some(mirror.id.clone()), mirror.name.clone());
        }
    }

    if cur_defaults.is_empty() && cur_customs.is_empty() {
        (None, "未显式配置镜像（使用 Conda 默认行为）".to_string())
    } else {
        (None, "自定义 / 手动编辑配置".to_string())
    }
}

pub fn read_condarc_snapshot(
    mirrors: &[MirrorProfile],
    conda_exe: Option<&Path>,
) -> Result<CondarcSnapshot, String> {
    let path = user_condarc_path()?;
    let exists = path.exists();
    let raw_content = if exists {
        fs::read_to_string(&path).map_err(|e| format!("读取 .condarc 失败: {e}"))?
    } else {
        String::new()
    };

    let (active_mirror_id, active_mirror_name) = if raw_content.trim().is_empty() {
        (None, "未创建用户级 .condarc".to_string())
    } else {
        match serde_yaml::from_str::<Value>(&raw_content) {
            Ok(Value::Mapping(map)) => match_active_mirror(&map, mirrors),
            Ok(_) => (None, "YAML 根节点非映射结构".to_string()),
            Err(_) => (None, "YAML 解析异常（请检查语法）".to_string()),
        }
    };

    let root_path = detect_root_condarc_path(conda_exe);
    let (root_condarc_path, root_condarc_content, root_active_mirror_id, root_active_mirror_name) =
        match root_path {
            Some(rp) => {
                let raw = fs::read_to_string(&rp).unwrap_or_default();
                let (rid, rname) = if raw.trim().is_empty() {
                    (None, "空文件".to_string())
                } else {
                    match serde_yaml::from_str::<Value>(&raw) {
                        Ok(Value::Mapping(map)) => match_active_mirror(&map, mirrors),
                        _ => (None, "YAML 解析异常".to_string()),
                    }
                };
                (
                    Some(rp.to_string_lossy().to_string()),
                    Some(raw),
                    rid,
                    Some(rname),
                )
            }
            None => (None, None, None, None),
        };

    Ok(CondarcSnapshot {
        path: path.to_string_lossy().to_string(),
        exists,
        raw_content,
        active_mirror_id,
        active_mirror_name,
        root_condarc_path,
        root_condarc_content,
        root_active_mirror_id,
        root_active_mirror_name,
    })
}

/// 原子写入 `.condarc`，避免写入中断导致配置文件损坏。
pub fn write_condarc_atomic(path: &Path, content: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "无法确定 .condarc 所在目录".to_string())?;
    let tmp_path = parent.join(".condarc.condasrc.tmp");
    fs::write(&tmp_path, content).map_err(|e| format!("写入临时文件失败: {e}"))?;
    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("替换 .condarc 失败: {e}")
    })?;
    Ok(())
}

/// 精准更新 `.condarc` 中的 `default_channels` 与 `custom_channels`，
/// 完整保留 `channels`、`show_channel_urls`、`proxy_servers`、`envs_dirs` 等其他配置。
pub fn apply_mirror_to_condarc(path: &Path, mirror: &MirrorProfile) -> Result<(), String> {
    let mut root_map: Mapping = if path.exists() {
        let raw = fs::read_to_string(path).map_err(|e| format!("读取现有 .condarc 失败: {e}"))?;
        if raw.trim().is_empty() {
            Mapping::new()
        } else {
            match serde_yaml::from_str::<Value>(&raw)
                .map_err(|e| format!("现有 .condarc YAML 语法有误，已中止修改以防损坏: {e}"))?
            {
                Value::Mapping(m) => m,
                Value::Null => Mapping::new(),
                _ => return Err("现有 .condarc 根节点不是 YAML 对象，无法安全合并".to_string()),
            }
        }
    } else {
        // 文件不存在时初始化最小合理骨架
        let mut init = Mapping::new();
        init.insert(
            Value::String("channels".to_string()),
            Value::Sequence(vec![Value::String("conda-forge".to_string())]),
        );
        init.insert(
            Value::String("show_channel_urls".to_string()),
            Value::Bool(true),
        );
        init
    };

    // 1. 精准更新 default_channels
    if !mirror.default_channels.is_empty() {
        let seq = mirror
            .default_channels
            .iter()
            .map(|u| Value::String(u.clone()))
            .collect();
        root_map.insert(
            Value::String("default_channels".to_string()),
            Value::Sequence(seq),
        );
    }

    // 2. 精准更新 custom_channels（保留用户可能额外配置的其他自定义 channel 键）
    if !mirror.custom_channels.is_empty() {
        let key = Value::String("custom_channels".to_string());
        let mut existing_custom = match root_map.get(&key) {
            Some(Value::Mapping(m)) => m.clone(),
            _ => Mapping::new(),
        };
        for (ch_name, ch_url) in &mirror.custom_channels {
            existing_custom.insert(
                Value::String(ch_name.clone()),
                Value::String(ch_url.clone()),
            );
        }
        root_map.insert(key, Value::Mapping(existing_custom));
    }

    let serialized = serde_yaml::to_string(&Value::Mapping(root_map))
        .map_err(|e| format!("序列化 YAML 失败: {e}"))?;

    write_condarc_atomic(path, &serialized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::builtin_mirrors;

    #[test]
    fn surgical_update_preserves_other_fields() {
        let tmp_dir = std::env::temp_dir().join("condasrc_unit_test");
        let _ = fs::create_dir_all(&tmp_dir);
        let condarc_file = tmp_dir.join(".condarc");

        let initial = r#"
channels:
  - conda-forge
show_channel_urls: true
proxy_servers:
  http: http://127.0.0.1:7890
default_channels:
  - https://repo.anaconda.com/pkgs/main
  - https://repo.anaconda.com/pkgs/r
  - https://repo.anaconda.com/pkgs/msys2
custom_channels:
  conda-forge: https://conda.anaconda.org
  pytorch: https://conda.anaconda.org
  bioconda: https://conda.anaconda.org
  my-private: https://internal.example.com/conda
"#;
        fs::write(&condarc_file, initial).unwrap();

        let mirrors = builtin_mirrors();
        let pku = mirrors.iter().find(|m| m.id == "pku").unwrap();
        apply_mirror_to_condarc(&condarc_file, pku).unwrap();

        let updated_raw = fs::read_to_string(&condarc_file).unwrap();
        let parsed: Value = serde_yaml::from_str(&updated_raw).unwrap();
        let map = parsed.as_mapping().unwrap();

        // 保留其他字段
        assert_eq!(
            map.get(Value::String("show_channel_urls".into()))
                .and_then(|v| v.as_bool()),
            Some(true)
        );
        assert!(map.contains_key(Value::String("proxy_servers".into())));
        assert!(map.contains_key(Value::String("channels".into())));

        // 保留用户原有的额外 custom_channel
        let custom = map
            .get(Value::String("custom_channels".into()))
            .and_then(|v| v.as_mapping())
            .unwrap();
        assert_eq!(
            custom
                .get(Value::String("my-private".into()))
                .and_then(|v| v.as_str()),
            Some("https://internal.example.com/conda")
        );

        // 确认已切换为 PKU 且可正确识别
        let (matched_id, _) = match_active_mirror(map, &mirrors);
        assert_eq!(matched_id.as_deref(), Some("pku"));

        let _ = fs::remove_dir_all(&tmp_dir);
    }
}

