use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_builtin: bool,
    pub default_channels: Vec<String>,
    pub custom_channels: BTreeMap<String, String>,
    pub probe_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMirrorInput {
    pub id: Option<String>,
    pub name: String,
    pub description: String,
    pub default_channels: Vec<String>,
    pub custom_channels: BTreeMap<String, String>,
    pub probe_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    pub filename: String,
    pub label: String,
    pub target: String, // "user" | "root"
    pub is_original: bool,
    pub created_at: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CondarcSnapshot {
    pub path: String,
    pub exists: bool,
    pub raw_content: String,
    pub active_mirror_id: Option<String>,
    pub active_mirror_name: String,
    pub root_condarc_path: Option<String>,
    pub root_condarc_content: Option<String>,
    pub root_active_mirror_id: Option<String>,
    pub root_active_mirror_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub condarc: CondarcSnapshot,
    pub mirrors: Vec<MirrorProfile>,
    pub backups: Vec<BackupEntry>,
    pub conda_executable: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedTestResult {
    pub mirror_id: String,
    pub ok: bool,
    pub latency_ms: Option<u128>,
    pub status_code: Option<u16>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanCacheResult {
    pub ok: bool,
    pub command_used: String,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvMetaSyncReport {
    pub updated_files: usize,
    pub touched_envs: Vec<String>,
    pub permission_denied_envs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchResult {
    pub snapshot: AppSnapshot,
    pub backup_created: Option<String>,
    pub clean_result: Option<CleanCacheResult>,
    pub env_meta_sync: Option<EnvMetaSyncReport>,
}
