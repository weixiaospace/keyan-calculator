//! 国密 SM3 算码 + 时间存证派生码
//!
//! - 文件内容 SM3：流式分块读取，内存恒定，支持任意大文件
//! - 派生码：SM3( sm3 | created(秒) | modified(秒) | calc_ts(毫秒) )，不可变

use sm3::{Digest, Sm3};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// 对一段字节算 SM3，返回 64 位十六进制字符串
pub fn sm3_bytes(data: &[u8]) -> String {
    let mut hasher = Sm3::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// 流式计算文件内容的 SM3（每次读 1MB，内存恒定）
pub fn sm3_file_streaming(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sm3::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// 时间存证派生码 = SM3( "{sm3}|{created}|{modified}|{calc_ts}" )
///
/// - `created` / `modified`：文件创建/修改时间，Unix 秒
/// - `calc_ts_ms`：算码时刻，Unix 毫秒
pub fn derived_code(sm3: &str, created: i64, modified: i64, calc_ts_ms: i64) -> String {
    let payload = format!("{}|{}|{}|{}", sm3, created, modified, calc_ts_ms);
    sm3_bytes(payload.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_sm3_known_vector() {
        // 国密 SM3 标准测试向量： SM3("abc")
        assert_eq!(
            sm3_bytes(b"abc"),
            "66c7f0f462eeedd9d1f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba8e0"
        );
    }

    #[test]
    fn test_sm3_file_streaming_matches_oneshot() {
        let mut f = NamedTempFile::new().unwrap();
        let content = b"hello sm3 streaming";
        f.write_all(content).unwrap();
        f.flush().unwrap();

        let streamed = sm3_file_streaming(f.path()).unwrap();
        assert_eq!(streamed, sm3_bytes(content));
        assert_eq!(streamed.len(), 64);
    }

    #[test]
    fn test_derived_code_is_deterministic_and_input_sensitive() {
        let sm3 = "66c7f0f462eeedd9d1f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba8e0";
        let a = derived_code(sm3, 1000, 2000, 3000);
        let b = derived_code(sm3, 1000, 2000, 3000);
        assert_eq!(a, b, "相同输入必须得到相同派生码");
        assert_eq!(a.len(), 64);

        // 任一输入变化都应改变派生码
        assert_ne!(a, derived_code(sm3, 1001, 2000, 3000));
        assert_ne!(a, derived_code(sm3, 1000, 2001, 3000));
        assert_ne!(a, derived_code(sm3, 1000, 2000, 3001));
    }
}
