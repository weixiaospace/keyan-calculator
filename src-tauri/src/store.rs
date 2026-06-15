//! 本地 SQLite —— 唯一真相源
//!
//! 表：
//! - `folders`      常驻文件夹清单
//! - `attestations` append-only 存证日志（一条算码事件一行）
//! - `config`       键值配置

use crate::types::{AppConfig, Attestation, Folder};
use rusqlite::{params, Connection, Row};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;

/// Tauri 全局状态
pub struct AppState {
    pub db: Mutex<Connection>,
    /// 单调递增的算码时刻分配器，保证每条存证的 calc_ts 唯一（→ 派生码唯一）
    last_calc_ts: AtomicI64,
}

impl AppState {
    pub fn new(conn: Connection) -> Self {
        Self {
            db: Mutex::new(conn),
            last_calc_ts: AtomicI64::new(0),
        }
    }

    /// 分配一个严格递增、不重复的算码时刻（Unix 毫秒）
    pub fn next_calc_ts(&self) -> i64 {
        let now = now_ms();
        loop {
            let last = self.last_calc_ts.load(Ordering::Acquire);
            let next = if now > last { now } else { last + 1 };
            if self
                .last_calc_ts
                .compare_exchange(last, next, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return next;
            }
        }
    }
}

/// 当前 Unix 毫秒
pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// 某路径"最新存证"的元数据（用于缓存命中判定）
pub struct LatestMeta {
    pub file_size: i64,
    pub modified_time: i64,
    pub created_time: i64,
    pub uploaded_at: Option<i64>,
}

/// 待插入的存证行（含内部列 folder_id / rel_path）
pub struct AttRow {
    pub id: String,
    pub folder_id: String,
    pub path: String,
    pub rel_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub created_time: i64,
    pub modified_time: i64,
    pub sm3: String,
    pub calc_ts: i64,
    pub derived_code: String,
    pub time_source: String,
}

const ATT_COLS: &str =
    "id, path, file_name, file_size, created_time, modified_time, sm3, calc_ts, derived_code, time_source, uploaded_at";

fn map_attestation(row: &Row) -> rusqlite::Result<Attestation> {
    Ok(Attestation {
        id: row.get(0)?,
        path: row.get(1)?,
        file_name: row.get(2)?,
        file_size: row.get(3)?,
        created_time: row.get(4)?,
        modified_time: row.get(5)?,
        sm3: row.get(6)?,
        calc_ts: row.get(7)?,
        derived_code: row.get(8)?,
        time_source: row.get(9)?,
        uploaded_at: row.get(10)?,
    })
}

/// 初始化库：WAL + 表 + 索引
pub fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode=WAL;
        PRAGMA synchronous=NORMAL;

        CREATE TABLE IF NOT EXISTS folders (
            id           TEXT PRIMARY KEY,
            root_path    TEXT NOT NULL UNIQUE,
            name         TEXT NOT NULL,
            last_scan_at INTEGER
        );

        CREATE TABLE IF NOT EXISTS attestations (
            id            TEXT PRIMARY KEY,
            folder_id     TEXT NOT NULL,
            path          TEXT NOT NULL,
            rel_path      TEXT NOT NULL,
            file_name     TEXT NOT NULL,
            file_size     INTEGER NOT NULL,
            created_time  INTEGER NOT NULL,
            modified_time INTEGER NOT NULL,
            sm3           TEXT NOT NULL,
            calc_ts       INTEGER NOT NULL,
            derived_code  TEXT NOT NULL,
            time_source   TEXT NOT NULL,
            uploaded_at   INTEGER
        );

        CREATE INDEX IF NOT EXISTS idx_att_folder_rel ON attestations(folder_id, rel_path);
        CREATE INDEX IF NOT EXISTS idx_att_derived    ON attestations(derived_code);
        CREATE INDEX IF NOT EXISTS idx_att_uploaded   ON attestations(uploaded_at);

        CREATE TABLE IF NOT EXISTS config (
            key   TEXT PRIMARY KEY,
            value TEXT
        );
        ",
    )
}

// ---------- folders ----------

fn map_folder(row: &Row) -> rusqlite::Result<Folder> {
    Ok(Folder {
        id: row.get(0)?,
        root_path: row.get(1)?,
        name: row.get(2)?,
        last_scan_at: row.get(3)?,
    })
}

pub fn list_folders(conn: &Connection) -> rusqlite::Result<Vec<Folder>> {
    let mut stmt =
        conn.prepare("SELECT id, root_path, name, last_scan_at FROM folders ORDER BY name")?;
    let rows = stmt.query_map([], map_folder)?;
    rows.collect()
}

pub fn get_folder(conn: &Connection, id: &str) -> rusqlite::Result<Option<Folder>> {
    let mut stmt =
        conn.prepare("SELECT id, root_path, name, last_scan_at FROM folders WHERE id = ?1")?;
    let mut rows = stmt.query_map(params![id], map_folder)?;
    match rows.next() {
        Some(r) => Ok(Some(r?)),
        None => Ok(None),
    }
}

pub fn find_folder_by_path(conn: &Connection, path: &str) -> rusqlite::Result<Option<Folder>> {
    let mut stmt = conn
        .prepare("SELECT id, root_path, name, last_scan_at FROM folders WHERE root_path = ?1")?;
    let mut rows = stmt.query_map(params![path], map_folder)?;
    match rows.next() {
        Some(r) => Ok(Some(r?)),
        None => Ok(None),
    }
}

pub fn insert_folder(conn: &Connection, folder: &Folder) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO folders (id, root_path, name, last_scan_at) VALUES (?1, ?2, ?3, ?4)",
        params![folder.id, folder.root_path, folder.name, folder.last_scan_at],
    )?;
    Ok(())
}

pub fn delete_folder(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    // 仅删除文件夹清单条目；存证记录保留为孤儿数据（append-only）
    conn.execute("DELETE FROM folders WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn update_folder_scan(conn: &Connection, id: &str, ts: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE folders SET last_scan_at = ?1 WHERE id = ?2",
        params![ts, id],
    )?;
    Ok(())
}

// ---------- attestations ----------

/// 文件夹内每条路径的"最新存证"元数据
pub fn latest_meta_map(
    conn: &Connection,
    folder_id: &str,
) -> rusqlite::Result<HashMap<String, LatestMeta>> {
    let mut stmt = conn.prepare(
        "SELECT a.rel_path, a.file_size, a.modified_time, a.created_time, a.uploaded_at
         FROM attestations a
         WHERE a.folder_id = ?1
           AND a.calc_ts = (
               SELECT MAX(b.calc_ts) FROM attestations b
               WHERE b.folder_id = a.folder_id AND b.rel_path = a.rel_path
           )",
    )?;
    let rows = stmt.query_map(params![folder_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            LatestMeta {
                file_size: row.get(1)?,
                modified_time: row.get(2)?,
                created_time: row.get(3)?,
                uploaded_at: row.get(4)?,
            },
        ))
    })?;
    let mut map = HashMap::new();
    for r in rows {
        let (k, v) = r?;
        map.insert(k, v);
    }
    Ok(map)
}

/// 批量插入存证（单事务）
pub fn insert_attestations(conn: &mut Connection, rows: &[AttRow]) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO attestations
             (id, folder_id, path, rel_path, file_name, file_size, created_time, modified_time,
              sm3, calc_ts, derived_code, time_source, uploaded_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12, NULL)",
        )?;
        for r in rows {
            stmt.execute(params![
                r.id,
                r.folder_id,
                r.path,
                r.rel_path,
                r.file_name,
                r.file_size,
                r.created_time,
                r.modified_time,
                r.sm3,
                r.calc_ts,
                r.derived_code,
                r.time_source,
            ])?;
        }
    }
    tx.commit()
}

pub fn latest_for(
    conn: &Connection,
    folder_id: &str,
    rel_path: &str,
) -> rusqlite::Result<Option<Attestation>> {
    let sql = format!(
        "SELECT {ATT_COLS} FROM attestations
         WHERE folder_id = ?1 AND rel_path = ?2 ORDER BY calc_ts DESC LIMIT 1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map(params![folder_id, rel_path], map_attestation)?;
    match rows.next() {
        Some(r) => Ok(Some(r?)),
        None => Ok(None),
    }
}

pub fn history_for(
    conn: &Connection,
    folder_id: &str,
    rel_path: &str,
) -> rusqlite::Result<Vec<Attestation>> {
    let sql = format!(
        "SELECT {ATT_COLS} FROM attestations
         WHERE folder_id = ?1 AND rel_path = ?2 ORDER BY calc_ts DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![folder_id, rel_path], map_attestation)?;
    rows.collect()
}

pub fn pending_attestations(conn: &Connection) -> rusqlite::Result<Vec<Attestation>> {
    let sql = format!(
        "SELECT {ATT_COLS} FROM attestations WHERE uploaded_at IS NULL ORDER BY calc_ts ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_attestation)?;
    rows.collect()
}

pub fn mark_uploaded(conn: &mut Connection, ids: &[String], ts: i64) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare("UPDATE attestations SET uploaded_at = ?1 WHERE id = ?2")?;
        for id in ids {
            stmt.execute(params![ts, id])?;
        }
    }
    tx.commit()
}

// ---------- config ----------

pub fn get_config(conn: &Connection) -> rusqlite::Result<AppConfig> {
    let mut stmt = conn.prepare("SELECT value FROM config WHERE key = 'upload_endpoint'")?;
    let mut rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let upload_endpoint = match rows.next() {
        Some(r) => Some(r?),
        None => None,
    };
    Ok(AppConfig { upload_endpoint })
}

pub fn set_config(conn: &Connection, config: &AppConfig) -> rusqlite::Result<()> {
    match &config.upload_endpoint {
        Some(v) => {
            conn.execute(
                "INSERT INTO config (key, value) VALUES ('upload_endpoint', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = ?1",
                params![v],
            )?;
        }
        None => {
            conn.execute("DELETE FROM config WHERE key = 'upload_endpoint'", [])?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    fn row(folder_id: &str, rel: &str, size: i64, modified: i64, calc_ts: i64) -> AttRow {
        AttRow {
            id: format!("{rel}-{calc_ts}"),
            folder_id: folder_id.to_string(),
            path: format!("/root/{rel}"),
            rel_path: rel.to_string(),
            file_name: rel.to_string(),
            file_size: size,
            created_time: 100,
            modified_time: modified,
            sm3: "deadbeef".to_string(),
            calc_ts,
            derived_code: format!("code-{calc_ts}"),
            time_source: "local".to_string(),
        }
    }

    #[test]
    fn append_only_history_and_latest() {
        let mut conn = mem();
        // 同一路径两次算码（内容/时间变化 → 不同 calc_ts）
        insert_attestations(&mut conn, &[row("F", "a.txt", 10, 1000, 1)]).unwrap();
        insert_attestations(&mut conn, &[row("F", "a.txt", 20, 2000, 2)]).unwrap();

        // 历史保留两条，按 calc_ts 倒序
        let history = history_for(&conn, "F", "a.txt").unwrap();
        assert_eq!(history.len(), 2, "append-only：旧存证必须保留");
        assert_eq!(history[0].calc_ts, 2);
        assert_eq!(history[1].calc_ts, 1);

        // 最新存证元数据 = 较新那条
        let map = latest_meta_map(&conn, "F").unwrap();
        let latest = map.get("a.txt").unwrap();
        assert_eq!(latest.file_size, 20);
        assert_eq!(latest.modified_time, 2000);
    }

    #[test]
    fn pending_and_mark_uploaded() {
        let mut conn = mem();
        insert_attestations(
            &mut conn,
            &[row("F", "a.txt", 10, 1000, 1), row("F", "b.txt", 10, 1000, 2)],
        )
        .unwrap();

        // 全部待传
        let pending = pending_attestations(&conn).unwrap();
        assert_eq!(pending.len(), 2);

        // 标记其中一条已上传 → 不再入选待传
        mark_uploaded(&mut conn, &[pending[0].id.clone()], 9999).unwrap();
        let still_pending = pending_attestations(&conn).unwrap();
        assert_eq!(still_pending.len(), 1);
        assert_ne!(still_pending[0].id, pending[0].id);
    }

    #[test]
    fn config_roundtrip() {
        let conn = mem();
        assert!(get_config(&conn).unwrap().upload_endpoint.is_none());
        set_config(
            &conn,
            &AppConfig {
                upload_endpoint: Some("https://x/ingest".to_string()),
            },
        )
        .unwrap();
        assert_eq!(
            get_config(&conn).unwrap().upload_endpoint.as_deref(),
            Some("https://x/ingest")
        );
    }

    #[test]
    fn folder_crud_keeps_orphan_attestations() {
        let mut conn = mem();
        let folder = Folder {
            id: "F".to_string(),
            root_path: "/root".to_string(),
            name: "root".to_string(),
            last_scan_at: None,
        };
        insert_folder(&conn, &folder).unwrap();
        insert_attestations(&mut conn, &[row("F", "a.txt", 10, 1000, 1)]).unwrap();

        // 删除文件夹后，存证记录作为孤儿数据保留
        delete_folder(&conn, "F").unwrap();
        assert!(list_folders(&conn).unwrap().is_empty());
        assert_eq!(history_for(&conn, "F", "a.txt").unwrap().len(), 1);
    }
}
