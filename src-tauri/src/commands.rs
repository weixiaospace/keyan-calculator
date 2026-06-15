//! Tauri 命令 —— 串起 扫描 / 算码 / 存证 / 上传
//!
//! 全部为同步命令：哈希用 rayon 并行，上传用 reqwest::blocking。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;
use serde_json::json;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::hasher::{derived_code, sm3_file_streaming};
use crate::scanner::{scan_dir, ScannedFile};
use crate::store::{self, AppState, AttRow};
use crate::types::*;

type CmdResult<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/// 该路径的元数据是否与最新存证一致（命中缓存）
fn metadata_matches(m: &store::LatestMeta, f: &ScannedFile) -> bool {
    m.file_size == f.file_size && m.modified_time == f.modified_time && m.created_time == f.created_time
}

// ---------- 文件夹管理 ----------

#[tauri::command]
pub fn list_folders(state: State<AppState>) -> CmdResult<Vec<Folder>> {
    let conn = state.db.lock().unwrap();
    store::list_folders(&conn).map_err(err)
}

#[tauri::command]
pub fn add_folder(state: State<AppState>, path: String) -> CmdResult<Folder> {
    let conn = state.db.lock().unwrap();
    if let Some(existing) = store::find_folder_by_path(&conn, &path).map_err(err)? {
        return Ok(existing);
    }
    let name = Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| path.clone());
    let folder = Folder {
        id: Uuid::new_v4().to_string(),
        root_path: path,
        name,
        last_scan_at: None,
    };
    store::insert_folder(&conn, &folder).map_err(err)?;
    Ok(folder)
}

#[tauri::command]
pub fn remove_folder(state: State<AppState>, folder_id: String) -> CmdResult<()> {
    let conn = state.db.lock().unwrap();
    store::delete_folder(&conn, &folder_id).map_err(err)
}

// ---------- 扫描（便宜，判状态、剔除已删） ----------

#[tauri::command]
pub async fn scan_folder(state: State<'_, AppState>, folder_id: String) -> CmdResult<ScanResult> {
    // 取根路径后释放锁，再做文件系统扫描
    let root = {
        let conn = state.db.lock().unwrap();
        store::get_folder(&conn, &folder_id)
            .map_err(err)?
            .ok_or_else(|| "文件夹不存在".to_string())?
            .root_path
    };

    let files = scan_dir(Path::new(&root));

    let latest = {
        let conn = state.db.lock().unwrap();
        let map = store::latest_meta_map(&conn, &folder_id).map_err(err)?;
        store::update_folder_scan(&conn, &folder_id, store::now_ms()).map_err(err)?;
        map
    };

    Ok(build_scan_result(folder_id, &files, &latest))
}

/// 根据扫描到的文件与"最新存证"元数据，分类状态并组织成文件树
fn build_scan_result(
    folder_id: String,
    files: &[ScannedFile],
    latest: &HashMap<String, store::LatestMeta>,
) -> ScanResult {
    let mut status: HashMap<String, (FileStatus, Option<UploadStatus>)> = HashMap::new();
    let mut counts = ScanCounts {
        total_files: 0,
        uncomputed: 0,
        needs_recompute: 0,
        computed: 0,
        pending_upload: 0,
    };

    for f in files {
        if f.is_directory {
            continue;
        }
        counts.total_files += 1;
        let (cs, us) = match latest.get(&f.rel_path) {
            None => {
                counts.uncomputed += 1;
                (FileStatus::Uncomputed, None)
            }
            Some(m) => {
                if metadata_matches(m, f) {
                    counts.computed += 1;
                    let u = if m.uploaded_at.is_some() {
                        UploadStatus::Uploaded
                    } else {
                        counts.pending_upload += 1;
                        UploadStatus::Pending
                    };
                    (FileStatus::Computed, Some(u))
                } else {
                    counts.needs_recompute += 1;
                    (FileStatus::NeedsRecompute, None)
                }
            }
        };
        status.insert(f.rel_path.clone(), (cs, us));
    }

    let tree = build_tree(files, &status);
    ScanResult {
        folder_id,
        tree,
        counts,
    }
}

/// 把扁平扫描结果组织成文件树
fn build_tree(
    files: &[ScannedFile],
    status: &HashMap<String, (FileStatus, Option<UploadStatus>)>,
) -> Vec<TreeNode> {
    let mut children_of: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, f) in files.iter().enumerate() {
        let parent = match f.rel_path.rfind('/') {
            Some(p) => f.rel_path[..p].to_string(),
            None => String::new(),
        };
        children_of.entry(parent).or_default().push(i);
    }
    build_nodes("", files, &children_of, status)
}

fn build_nodes(
    parent: &str,
    files: &[ScannedFile],
    children_of: &HashMap<String, Vec<usize>>,
    status: &HashMap<String, (FileStatus, Option<UploadStatus>)>,
) -> Vec<TreeNode> {
    let mut nodes = Vec::new();
    if let Some(idxs) = children_of.get(parent) {
        for &i in idxs {
            let f = &files[i];
            let (compute_status, upload_status) = if f.is_directory {
                (None, None)
            } else {
                match status.get(&f.rel_path) {
                    Some((c, u)) => (Some(*c), *u),
                    None => (Some(FileStatus::Uncomputed), None),
                }
            };
            let children = if f.is_directory {
                build_nodes(&f.rel_path, files, children_of, status)
            } else {
                Vec::new()
            };
            nodes.push(TreeNode {
                name: f.file_name.clone(),
                rel_path: f.rel_path.clone(),
                is_directory: f.is_directory,
                file_size: if f.is_directory {
                    None
                } else {
                    Some(f.file_size)
                },
                compute_status,
                upload_status,
                children,
            });
        }
    }
    // 目录在前，再按名称排序
    nodes.sort_by(|a, b| b.is_directory.cmp(&a.is_directory).then(a.name.cmp(&b.name)));
    nodes
}

// ---------- 算码（贵，流式读全文件，rayon 并行） ----------

#[tauri::command]
pub async fn compute_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    folder_id: String,
    force: bool,
) -> CmdResult<ScanResult> {
    let root = {
        let conn = state.db.lock().unwrap();
        store::get_folder(&conn, &folder_id)
            .map_err(err)?
            .ok_or_else(|| "文件夹不存在".to_string())?
            .root_path
    };

    let files = scan_dir(Path::new(&root));
    let latest = {
        let conn = state.db.lock().unwrap();
        store::latest_meta_map(&conn, &folder_id).map_err(err)?
    };

    // 仅对 未算 / 需重算（或强制）的文件算码
    let to_compute: Vec<&ScannedFile> = files
        .iter()
        .filter(|f| !f.is_directory)
        .filter(|f| match latest.get(&f.rel_path) {
            None => true,
            Some(m) => force || !metadata_matches(m, f),
        })
        .collect();

    let total = to_compute.len();
    let done = AtomicUsize::new(0);
    let st: &AppState = &state;
    // 进度事件节流：整批最多约 100 次，避免 IPC 风暴拖慢 UI
    let step = (total / 100).max(1);

    // 限制哈希并发为 CPU 核数-1，给 UI/系统留一核，避免大批量算码时整机卡顿
    let threads = std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1).max(1))
        .unwrap_or(1);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .map_err(err)?;

    let rows: Vec<AttRow> = pool.install(|| {
        to_compute
            .par_iter()
        .filter_map(|f| {
            let sm3 = sm3_file_streaming(Path::new(&f.abs_path)).ok()?;
            let calc_ts = st.next_calc_ts();
            let code = derived_code(&sm3, f.created_time, f.modified_time, calc_ts);
            let d = done.fetch_add(1, Ordering::SeqCst) + 1;
            if d % step == 0 || d == total {
                let _ = app.emit(
                    "compute-progress",
                    ComputeProgress {
                        folder_id: folder_id.clone(),
                        done: d as i64,
                        total: total as i64,
                        current: Some(f.rel_path.clone()),
                    },
                );
            }
            Some(AttRow {
                id: Uuid::new_v4().to_string(),
                folder_id: folder_id.clone(),
                path: f.abs_path.clone(),
                rel_path: f.rel_path.clone(),
                file_name: f.file_name.clone(),
                file_size: f.file_size,
                created_time: f.created_time,
                modified_time: f.modified_time,
                sm3,
                calc_ts,
                derived_code: code,
                time_source: "local".to_string(),
            })
        })
            .collect()
    });

    {
        let mut conn = state.db.lock().unwrap();
        store::insert_attestations(&mut conn, &rows).map_err(err)?;
    }

    // 复用本次扫描的 files，重新分类后直接返回最新树，省去前端再扫一次磁盘
    let latest_after = {
        let conn = state.db.lock().unwrap();
        store::latest_meta_map(&conn, &folder_id).map_err(err)?
    };
    Ok(build_scan_result(folder_id, &files, &latest_after))
}

#[tauri::command]
pub async fn compute_file(
    state: State<'_, AppState>,
    folder_id: String,
    rel_path: String,
    force: bool,
) -> CmdResult<ComputeResult> {
    let root = {
        let conn = state.db.lock().unwrap();
        store::get_folder(&conn, &folder_id)
            .map_err(err)?
            .ok_or_else(|| "文件夹不存在".to_string())?
            .root_path
    };

    let abs = Path::new(&root).join(&rel_path);
    let meta = std::fs::metadata(&abs).map_err(|_| "文件不存在或无法访问".to_string())?;
    if !meta.is_file() {
        return Err("目标不是文件".to_string());
    }

    let file_size = meta.len() as i64;
    let modified_time = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let created_time = meta
        .created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(modified_time);

    // 非强制时，若元数据与最新存证一致则跳过
    if !force {
        let conn = state.db.lock().unwrap();
        if let Some(latest) = store::latest_for(&conn, &folder_id, &rel_path).map_err(err)? {
            if latest.file_size == file_size
                && latest.modified_time == modified_time
                && latest.created_time == created_time
            {
                return Ok(ComputeResult {
                    computed: 0,
                    skipped: 1,
                });
            }
        }
    }

    let sm3 = sm3_file_streaming(&abs).map_err(err)?;
    let calc_ts = state.next_calc_ts();
    let code = derived_code(&sm3, created_time, modified_time, calc_ts);

    let file_name = abs
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| rel_path.clone());

    let row = AttRow {
        id: Uuid::new_v4().to_string(),
        folder_id,
        path: abs.to_string_lossy().to_string(),
        rel_path,
        file_name,
        file_size,
        created_time,
        modified_time,
        sm3,
        calc_ts,
        derived_code: code,
        time_source: "local".to_string(),
    };

    let mut conn = state.db.lock().unwrap();
    store::insert_attestations(&mut conn, &[row]).map_err(err)?;
    Ok(ComputeResult {
        computed: 1,
        skipped: 0,
    })
}

// ---------- 详情 ----------

#[tauri::command]
pub fn get_file_detail(
    state: State<AppState>,
    folder_id: String,
    rel_path: String,
) -> CmdResult<FileDetail> {
    let conn = state.db.lock().unwrap();
    let folder = store::get_folder(&conn, &folder_id)
        .map_err(err)?
        .ok_or_else(|| "文件夹不存在".to_string())?;

    let abs: PathBuf = Path::new(&folder.root_path).join(&rel_path);
    let meta = std::fs::metadata(&abs).ok();
    let exists_on_disk = meta.is_some();

    let file_size = meta.as_ref().filter(|m| m.is_file()).map(|m| m.len() as i64);
    let modified_time = meta.as_ref().and_then(|m| {
        m.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
    });
    let created_time = meta.as_ref().and_then(|m| {
        m.created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
    });

    let file_name = Path::new(&rel_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| rel_path.clone());

    let latest = store::latest_for(&conn, &folder_id, &rel_path).map_err(err)?;
    let history = store::history_for(&conn, &folder_id, &rel_path).map_err(err)?;

    Ok(FileDetail {
        rel_path,
        full_path: abs.to_string_lossy().to_string(),
        file_name,
        file_size,
        created_time,
        modified_time,
        latest,
        history,
        exists_on_disk,
    })
}

// ---------- 上传 ----------

#[tauri::command]
pub async fn upload_pending(app: AppHandle, state: State<'_, AppState>) -> CmdResult<UploadResult> {
    let endpoint = {
        let conn = state.db.lock().unwrap();
        store::get_config(&conn).map_err(err)?.upload_endpoint
    };
    let endpoint = match endpoint {
        Some(e) if !e.trim().is_empty() => e,
        _ => return Err("请先在设置中配置上传地址".to_string()),
    };

    let pending = {
        let conn = state.db.lock().unwrap();
        store::pending_attestations(&conn).map_err(err)?
    };

    if pending.is_empty() {
        return Ok(UploadResult {
            uploaded: 0,
            failed: 0,
            errors: vec![],
        });
    }

    let client = reqwest::blocking::Client::new();
    let total = pending.len() as i64;
    let mut done: i64 = 0;
    let mut uploaded: i64 = 0;
    let mut failed: i64 = 0;
    let mut errors: Vec<String> = Vec::new();

    for chunk in pending.chunks(200) {
        let body: Vec<_> = chunk
            .iter()
            .map(|a| {
                json!({
                    "derived_code": a.derived_code,
                    "sm3": a.sm3,
                    "file_name": a.file_name,
                    "path": a.path,
                    "file_size": a.file_size,
                    "created_time": a.created_time,
                    "modified_time": a.modified_time,
                    "calc_ts": a.calc_ts,
                    "time_source": a.time_source,
                })
            })
            .collect();

        match client.post(&endpoint).json(&body).send() {
            Ok(resp) if resp.status().is_success() => {
                let ids: Vec<String> = chunk.iter().map(|a| a.id.clone()).collect();
                let mut conn = state.db.lock().unwrap();
                store::mark_uploaded(&mut conn, &ids, store::now_ms()).map_err(err)?;
                uploaded += chunk.len() as i64;
            }
            Ok(resp) => {
                failed += chunk.len() as i64;
                errors.push(format!("HTTP {}", resp.status()));
            }
            Err(e) => {
                failed += chunk.len() as i64;
                errors.push(e.to_string());
            }
        }

        done += chunk.len() as i64;
        let _ = app.emit("upload-progress", UploadProgress { done, total });
    }

    Ok(UploadResult {
        uploaded,
        failed,
        errors,
    })
}

// ---------- 配置 ----------

#[tauri::command]
pub fn get_config(state: State<AppState>) -> CmdResult<AppConfig> {
    let conn = state.db.lock().unwrap();
    store::get_config(&conn).map_err(err)
}

#[tauri::command]
pub fn set_config(state: State<AppState>, config: AppConfig) -> CmdResult<()> {
    let conn = state.db.lock().unwrap();
    store::set_config(&conn, &config).map_err(err)
}
