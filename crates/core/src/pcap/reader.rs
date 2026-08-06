//! v1.0.0 pcap 读取器：tshark 子进程 + HTTP 字段提取（移植自 v0.8.0）。
//!
//! - [`PcapReader::read`] 调 `tshark -r <path> -Y http.request -T fields ...`
//!   提取 8 个 HTTP 字段，每行一条 [`HttpRequest`]。
//! - tshark 缺失返回 [`CoreError::DependencyMissing`]，GUI 层据此弹提示
//!   并禁用按钮。tshark 子进程退出码非 0 返回 [`CoreError::Other`]。
//! - `body` 字段（`http.file_data`）tshark 以十六进制输出，由
//!   [`hex_to_bytes`] 解码成字节再 `String::from_utf8_lossy`。
//!
//! 全本地处理，tshark 仅在用户本机执行，不上传 pcap/规则/样本。

use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::pcap::detect::resolve_tshark_cmd;

/// 一条 HTTP 请求记录（tshark 字段映射）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpRequest {
    pub frame_no: String,
    pub src_ip: String,
    pub dst_ip: String,
    pub method: String,
    pub host: String,
    pub uri: String,
    pub body: String,
    pub user_agent: String,
}

/// pcap 读取器：调 tshark 子进程提取 HTTP 请求字段。
#[derive(Debug, Default, Clone, Copy)]
pub struct PcapReader;

impl PcapReader {
    pub fn new() -> Self {
        Self
    }

    /// 读取 pcap 文件，返回 HTTP 请求记录列表。
    pub fn read(&self, path: &Path) -> Result<Vec<HttpRequest>, CoreError> {
        let tshark = resolve_tshark_cmd();
        let output = Command::new(&tshark)
            .arg("-r")
            .arg(path)
            .arg("-Y")
            .arg("http.request")
            .arg("-T")
            .arg("fields")
            .arg("-e")
            .arg("frame.number")
            .arg("-e")
            .arg("ip.src")
            .arg("-e")
            .arg("ip.dst")
            .arg("-e")
            .arg("http.request.method")
            .arg("-e")
            .arg("http.host")
            .arg("-e")
            .arg("http.request.uri")
            .arg("-e")
            .arg("http.file_data")
            .arg("-e")
            .arg("http.user_agent")
            .arg("-E")
            .arg("separator=\t")
            .arg("-E")
            .arg("occurrence=f")
            .output()
            .map_err(|e| CoreError::DependencyMissing(format!("tshark: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::Other(format!(
                "tshark exited {}: {stderr}",
                output.status.code().unwrap_or(-1)
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut result = Vec::new();
        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            // 7 个字段；缺失的用空串补齐（tshark 对空字段可能输出连续分隔符）。
            let get = |idx: usize| -> String { parts.get(idx).unwrap_or(&"").to_string() };
            let body_raw = get(6);
            let body = if body_raw.is_empty() {
                String::new()
            } else {
                // tshark 以十六进制输出 http.file_data；解码成字节再转字符串。
                let bytes = hex_to_bytes(&body_raw);
                String::from_utf8_lossy(&bytes).to_string()
            };
            result.push(HttpRequest {
                frame_no: get(0),
                src_ip: get(1),
                dst_ip: get(2),
                method: get(3),
                host: get(4),
                uri: get(5),
                body,
                user_agent: get(7),
            });
        }
        Ok(result)
    }
}

/// 十六进制串 → 字节（tshark `http.file_data` 字段解码用）。
fn hex_to_bytes(s: &str) -> Vec<u8> {
    let s = s.trim();
    let mut bytes = Vec::with_capacity(s.len() / 2);
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i + 1 < chars.len() {
        let hi = hex_digit(chars[i]);
        let lo = hex_digit(chars[i + 1]);
        if let (Some(h), Some(l)) = (hi, lo) {
            bytes.push((h << 4) | l);
        }
        i += 2;
    }
    bytes
}

fn hex_digit(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some(c as u8 - b'0'),
        'a'..='f' => Some(c as u8 - b'a' + 10),
        'A'..='F' => Some(c as u8 - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_to_bytes_basic() {
        assert_eq!(hex_to_bytes("48656c6c6f"), b"Hello");
        assert_eq!(hex_to_bytes(""), Vec::<u8>::new());
    }

    #[test]
    fn hex_to_bytes_ignores_odd_trailing() {
        // 奇数长度末位丢弃
        assert_eq!(hex_to_bytes("4"), Vec::<u8>::new());
        assert_eq!(hex_to_bytes("41b"), b"A");
    }

    #[test]
    fn hex_to_bytes_case_insensitive() {
        assert_eq!(hex_to_bytes("6F6E"), b"on");
    }

    /// 本机有 tshark 时跑；CI 跳过。
    #[test]
    #[ignore = "本机 tshark 读取，CI 无 tshark/无样本时跳过"]
    fn read_fixture_pcap() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("pcap")
            .join("base.pcap");
        if !path.exists() {
            return;
        }
        let reqs = PcapReader::new().read(&path).expect("read pcap");
        assert!(!reqs.is_empty());
        assert!(reqs[0].method.contains("GET") || reqs[0].method.contains("POST"));
    }
}
