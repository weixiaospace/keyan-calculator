//! 与前端共享的数据类型（字段名即 TS 接口的字段名，snake_case）

use serde::{Deserialize, Serialize};

/// 算码状态
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    /// 未算过
    Uncomputed,
    /// 需重算（元数据变了）
    NeedsRecompute,
    /// 已算
    Computed,
}

/// 上传状态
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UploadStatus {
    Pending,
    Uploaded,
}

/// 常驻文件夹
#[derive(Debug, Clone, Serialize)]
pub struct Folder {
    pub id: String,
    pub root_path: String,
    pub name: String,
    pub last_scan_at: Option<i64>,
}

/// 文件树节点
#[derive(Debug, Clone, Serialize)]
pub struct TreeNode {
    pub name: String,
    pub rel_path: String,
    pub is_directory: bool,
    pub file_size: Option<i64>,
    pub compute_status: Option<FileStatus>,
    pub upload_status: Option<UploadStatus>,
    pub children: Vec<TreeNode>,
}

/// 一条存证记录
#[derive(Debug, Clone, Serialize)]
pub struct Attestation {
    pub id: String,
    pub path: String,
    pub file_name: String,
    pub file_size: i64,
    pub created_time: i64,
    pub modified_time: i64,
    pub sm3: String,
    pub calc_ts: i64,
    pub derived_code: String,
    pub time_source: String,
    pub uploaded_at: Option<i64>,
}

/// 选中文件的详情
#[derive(Debug, Clone, Serialize)]
pub struct FileDetail {
    pub rel_path: String,
    pub full_path: String,
    pub file_name: String,
    pub file_size: Option<i64>,
    pub created_time: Option<i64>,
    pub modified_time: Option<i64>,
    pub latest: Option<Attestation>,
    pub history: Vec<Attestation>,
    pub exists_on_disk: bool,
}

/// 扫描计数
#[derive(Debug, Clone, Serialize)]
pub struct ScanCounts {
    pub total_files: i64,
    pub uncomputed: i64,
    pub needs_recompute: i64,
    pub computed: i64,
    pub pending_upload: i64,
}

/// 扫描结果
#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    pub folder_id: String,
    pub tree: Vec<TreeNode>,
    pub counts: ScanCounts,
}

/// 算码结果
#[derive(Debug, Clone, Serialize)]
pub struct ComputeResult {
    pub computed: i64,
    pub skipped: i64,
}

/// 上传结果
#[derive(Debug, Clone, Serialize)]
pub struct UploadResult {
    pub uploaded: i64,
    pub failed: i64,
    pub errors: Vec<String>,
}

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub upload_endpoint: Option<String>,
}

/// 算码进度事件
#[derive(Debug, Clone, Serialize)]
pub struct ComputeProgress {
    pub folder_id: String,
    pub done: i64,
    pub total: i64,
    pub current: Option<String>,
}

/// 上传进度事件
#[derive(Debug, Clone, Serialize)]
pub struct UploadProgress {
    pub done: i64,
    pub total: i64,
}
