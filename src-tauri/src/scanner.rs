//! 文件夹扫描（便宜，只读元数据，不读文件内容）
//!
//! - jwalk 并行遍历
//! - 跳过点文件/隐藏目录
//! - 跳过符号链接

use jwalk::WalkDir;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// 扫描得到的一个条目（文件或目录）
#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub file_name: String,
    /// 相对文件夹根的路径，分隔符统一为 '/'
    pub rel_path: String,
    pub abs_path: String,
    pub is_directory: bool,
    /// 文件字节数；目录为 0
    pub file_size: i64,
    /// 创建时间，Unix 秒；取不到时回退为修改时间
    pub created_time: i64,
    /// 修改时间，Unix 秒
    pub modified_time: i64,
}

/// 遍历 `root`，返回其中所有非隐藏、非符号链接的条目（不含根本身）
pub fn scan_dir(root: &Path) -> Vec<ScannedFile> {
    let entries: Vec<_> = WalkDir::new(root)
        .skip_hidden(true)
        .process_read_dir(|_depth, _path, _state, children| {
            // 遍历阶段就剔除点文件/隐藏目录，避免递归进入 .git 等
            children.retain(|entry| {
                if let Ok(e) = entry {
                    !e.file_name().to_string_lossy().starts_with('.')
                } else {
                    false
                }
            });
        })
        .into_iter()
        .filter_map(|e| e.ok())
        .collect();

    let mut out = Vec::with_capacity(entries.len());
    for entry in entries {
        let path = entry.path();

        // 跳过符号链接，避免循环引用与重复计数
        if entry.path_is_symlink() {
            continue;
        }

        let rel_path = match path.strip_prefix(root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if rel_path.is_empty() {
            continue; // 根目录本身
        }
        // 路径中任一段以 . 开头则跳过
        if rel_path.split('/').any(|p| p.starts_with('.')) {
            continue;
        }

        let file_name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        let meta = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue, // 断链/无权限
        };

        let is_directory = meta.is_dir();
        let file_size = if meta.is_file() { meta.len() as i64 } else { 0 };

        let modified_time = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let created_time = meta
            .created()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(modified_time);

        out.push(ScannedFile {
            file_name,
            rel_path,
            abs_path: path.to_string_lossy().to_string(),
            is_directory,
            file_size,
            created_time,
            modified_time,
        });
    }
    out
}
