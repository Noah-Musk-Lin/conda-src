#![cfg_attr(all(target_os = "windows", not(test)), windows_subsystem = "windows")]

mod conda_cli;
mod conda_meta;
mod condarc;
mod models;
mod presets;
mod speedtest;
mod storage;

use models::{AppSnapshot, CleanCacheResult, CustomMirrorInput, SpeedTestResult, SwitchResult};

fn build_snapshot() -> Result<AppSnapshot, String> {
    let mirrors = storage::load_all_mirrors()?;
    let conda_exe = conda_cli::discover_conda_executable();
    let condarc_snap = condarc::read_condarc_snapshot(&mirrors, conda_exe.as_deref())?;
    let backups = storage::list_backups()?;

    Ok(AppSnapshot {
        condarc: condarc_snap,
        mirrors,
        backups,
        conda_executable: conda_exe.map(|p| p.to_string_lossy().to_string()),
    })
}

#[tauri::command]
fn get_snapshot() -> Result<AppSnapshot, String> {
    build_snapshot()
}

#[tauri::command]
async fn switch_mirror(
    mirror_id: String,
    auto_clean: bool,
    sync_root: bool,
    sync_env_meta: bool,
) -> Result<SwitchResult, String> {
    let mirrors = storage::load_all_mirrors()?;
    let target = mirrors
        .iter()
        .find(|m| m.id == mirror_id)
        .ok_or_else(|| format!("未找到 ID 为 '{mirror_id}' 的镜像源"))?
        .clone();

    let condarc_path = condarc::user_condarc_path()?;

    // 首次修改前自动备份原始用户级 .condarc
    let mut created_backups: Vec<String> = Vec::new();
    if let Some(b) = storage::ensure_original_backup(&condarc_path)? {
        created_backups.push(b);
    }

    // 精准更新用户级 .condarc 的 default_channels 和 custom_channels
    condarc::apply_mirror_to_condarc(&condarc_path, &target)?;

    let conda_exe = conda_cli::discover_conda_executable();

    // 若开启同步且检测到环境根目录 .condarc（如 D:\Application\miniforge\.condarc），一并同步更新
    if sync_root {
        if let Some(root_path) = condarc::detect_root_condarc_path(conda_exe.as_deref()) {
            if root_path != condarc_path {
                if let Some(rb) = storage::ensure_original_root_backup(&root_path)? {
                    created_backups.push(rb);
                }
                condarc::apply_mirror_to_condarc(&root_path, &target)?;
            }
        }
    }

    // 若开启“同步环境索引”，自动将各虚拟环境 conda-meta/*.json 中硬编码的旧源 URL 规范化为动态频道名
    let env_meta_sync = if sync_env_meta {
        if let Some(conda_root) = conda_meta::infer_conda_root(conda_exe.as_deref()) {
            tokio::task::spawn_blocking(move || conda_meta::sync_all_envs_metadata(&conda_root))
                .await
                .ok()
        } else {
            None
        }
    } else {
        None
    };

    let backup_created = if created_backups.is_empty() {
        None
    } else {
        Some(created_backups.join(", "))
    };

    // 可选：触发 conda clean -i -y
    let clean_result = if auto_clean {
        match conda_cli::clean_index_cache().await {
            Ok(res) => Some(res),
            Err(err) => Some(CleanCacheResult {
                ok: false,
                command_used: "conda clean -i -y".to_string(),
                stdout: String::new(),
                stderr: err,
            }),
        }
    } else {
        None
    };

    let snapshot = build_snapshot()?;
    Ok(SwitchResult {
        snapshot,
        backup_created,
        clean_result,
        env_meta_sync,
    })
}

#[tauri::command]
async fn get_effective_conda_config() -> Result<String, String> {
    conda_cli::get_effective_config().await
}

#[tauri::command]
async fn run_speedtest() -> Result<Vec<SpeedTestResult>, String> {
    let mirrors = storage::load_all_mirrors()?;
    speedtest::run_all_speedtests(mirrors).await
}

#[tauri::command]
async fn clean_conda_index() -> Result<CleanCacheResult, String> {
    conda_cli::clean_index_cache().await
}

#[tauri::command]
fn save_custom_mirror(input: CustomMirrorInput) -> Result<AppSnapshot, String> {
    storage::upsert_custom_mirror(input)?;
    build_snapshot()
}

#[tauri::command]
fn remove_custom_mirror(mirror_id: String) -> Result<AppSnapshot, String> {
    storage::delete_custom_mirror(&mirror_id)?;
    build_snapshot()
}

#[tauri::command]
fn create_backup() -> Result<AppSnapshot, String> {
    let condarc_path = condarc::user_condarc_path()?;
    storage::create_timestamped_backup(&condarc_path)?;
    build_snapshot()
}

#[tauri::command]
fn restore_backup(filename: String) -> Result<AppSnapshot, String> {
    let content = storage::read_backup_content(&filename)?;
    if storage::is_root_backup_filename(&filename) {
        let conda_exe = conda_cli::discover_conda_executable();
        let root_path = condarc::detect_root_condarc_path(conda_exe.as_deref())
            .ok_or_else(|| "未检测到环境根目录 .condarc，无法还原根目录备份".to_string())?;
        let _ = storage::ensure_original_root_backup(&root_path);
        condarc::write_condarc_atomic(&root_path, &content)?;
    } else {
        let condarc_path = condarc::user_condarc_path()?;
        let _ = storage::ensure_original_backup(&condarc_path);
        condarc::write_condarc_atomic(&condarc_path, &content)?;
    }
    build_snapshot()
}

#[tauri::command]
fn remove_backup(filename: String) -> Result<AppSnapshot, String> {
    storage::delete_backup(&filename)?;
    build_snapshot()
}

#[tauri::command]
fn preview_backup(filename: String) -> Result<String, String> {
    storage::read_backup_content(&filename)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            switch_mirror,
            get_effective_conda_config,
            run_speedtest,
            clean_conda_index,
            save_custom_mirror,
            remove_custom_mirror,
            create_backup,
            restore_backup,
            remove_backup,
            preview_backup,
        ])
        .run(tauri::generate_context!())
        .expect("启动 CondaSrc 桌面应用失败");
}
