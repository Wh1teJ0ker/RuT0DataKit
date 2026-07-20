//! T3-1：tshark 子进程 + HTTP 字段 TSV 提取。
//!
//! 设计要点（见 v0.3.0 计划）：
//! - 调 `tshark -r <path> -Y "http.request" -T fields -e <字段>... -E separator=\t
//!   -E occurrence=f`，stdout 按行 split → 每行按 `\t` split 8 字段 → [`HttpRequest`]。
//! - `http.file_data` 在 tshark `-T fields` 输出里是 hex 串（如 `7b22...7d`），
//!   手写 [`hex_to_bytes`] 解成字节，再用 `String::from_utf8_lossy` 转文本。
//!   不引入 `hex` crate（避免新依赖）。
//! - tshark 缺失（PATH 中找不到）：先以 `Command::new("tshark").arg("--version")`
//!   探测，失败立即返回 [`CoreError::DependencyMissing("tshark")`](crate::error::CoreError)。
//! - tshark 退出码非 0：把 stderr 文本塞进 `CoreError::Other`。
//! - 集成测试标 `#[ignore]`：CI 无 tshark 时跳过；本机手动
//!   `cargo test -- --ignored pcap` 验证 fixture。

use std::path::Path;
use std::process::Command;

use crate::error::CoreError;

/// 从 pcap 中提取出的单条 HTTP 请求（仅 method/host/uri/body 等可见字段）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct HttpRequest {
    /// `frame.number`：tshark 帧序号，作为 finding.location 回引线索。
    pub frame_no: u32,
    /// `ip.src`：源 IP（用于 top_src_ips 聚合）。
    pub src_ip: String,
    /// `ip.dst`：目的 IP。
    pub dst_ip: String,
    /// `http.request.method`：GET / POST / ...
    pub method: String,
    /// `http.host`：Host 头值。
    pub host: String,
    /// `http.request.uri`：含 query 的 URI（尚未双重 URL 解码，由 decoder 负责解码）。
    pub uri: String,
    /// `http.file_data`：请求体原始 hex → bytes → UTF-8 lossy。GET 通常为 None。
    pub body: Option<String>,
    /// `http.user_agent`：UA 头值。
    pub user_agent: String,
}

/// pcap 读取器：无状态，靠系统 tshark 完成解析。
pub struct PcapReader;

impl PcapReader {
    /// 读 pcap 文件，返回所有 `http.request` 帧对应的 [`HttpRequest`]。
    ///
    /// - tshark 缺失 → `CoreError::DependencyMissing("tshark")`。
    /// - tshark 退出非 0 → `CoreError::Other(stderr)`。
    /// - 单行字段数不足时跳过该行（容错，避免 tshark 偶发空字段导致整批失败）。
    pub fn read(path: &Path) -> Result<Vec<HttpRequest>, CoreError> {
        // 1. tshark 在场探测：`tshark --version` 退出 0 视为可用。
        //    不引入 which / which_cloud crate，零新增依赖。
        match Command::new("tshark").arg("--version").output() {
            Ok(out) if out.status.success() => {}
            Ok(_) | Err(_) => {
                return Err(CoreError::DependencyMissing(
                    "tshark".to_string(),
                ));
            }
        }

        // 2. 主提取调用。
        let output = Command::new("tshark")
            .args([
                "-r",
                path.to_string_lossy().as_ref(),
                "-Y",
                "http.request",
                "-T",
                "fields",
                "-e",
                "frame.number",
                "-e",
                "ip.src",
                "-e",
                "ip.dst",
                "-e",
                "http.request.method",
                "-e",
                "http.host",
                "-e",
                "http.request.uri",
                "-e",
                "http.file_data",
                "-e",
                "http.user_agent",
                "-E",
                "separator=\t",
                "-E",
                "occurrence=f",
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::Other(format!(
                "tshark exited {}: {stderr}",
                output.status.code().unwrap_or(-1)
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut requests = Vec::new();
        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            // 每行 8 字段；空字段保留为空串（http.file_data 可能为空）。
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() < 7 {
                continue;
            }
            let frame_no = fields[0].parse::<u32>().ok();
            let Some(frame_no) = frame_no else { continue };
            let body = if fields[6].is_empty() {
                None
            } else {
                // http.file_data 是 hex 串；手写 hex_to_bytes → UTF-8 lossy。
                hex_to_bytes(fields[6])
                    .map(|b| String::from_utf8_lossy(&b).into_owned())
            };
            requests.push(HttpRequest {
                frame_no,
                src_ip: fields[1].to_string(),
                dst_ip: fields[2].to_string(),
                method: fields[3].to_string(),
                host: fields[4].to_string(),
                uri: fields[5].to_string(),
                body,
                user_agent: fields.get(7).map(|s| s.to_string()).unwrap_or_default(),
            });
        }
        Ok(requests)
    }
}

/// 手写 hex 串 → bytes（不引入 `hex` crate）。
///
/// - 忽略首尾空白；空串返回空 Vec（视为已解码）。
/// - 奇数长度 / 非法字符返回 `None`。
/// - 兼容大小写（`7B` 与 `7b` 等价）。
fn hex_to_bytes(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.is_empty() {
        return Some(Vec::new());
    }
    if s.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let hi = hex_digit(bytes[i])?;
        let lo = hex_digit(bytes[i + 1])?;
        out.push(hi << 4 | lo);
        i += 2;
    }
    Some(out)
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_to_bytes_basic() {
        assert_eq!(hex_to_bytes("7B22").unwrap(), b"{\"");
        assert_eq!(hex_to_bytes("7b22").unwrap(), b"{\"");
        assert_eq!(hex_to_bytes("").unwrap(), Vec::<u8>::new());
        assert!(hex_to_bytes("7B2").is_none(), "odd length");
        assert!(hex_to_bytes("7B2G").is_none(), "invalid char");
    }

    #[test]
    fn hex_to_bytes_utf8_body() {
        // `{"username":"Y2hlbnlvbmc="}` 前缀 hex
        let hex = "7b22757365726e616d65223a225932686c6e6c76626d633d227d";
        let bytes = hex_to_bytes(hex).unwrap();
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("username"));
    }

    /// 本机有 tshark 时跑；CI 跳过。验证 fixture 至少 5000 条 POST。
    #[test]
    #[ignore]
    fn pcap_reader_reads_fixture() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests/fixtures/samples/pcap/data.pcapng");
        let reqs = PcapReader::read(&path).expect("tshark required for this test");
        assert!(
            reqs.len() >= 5000,
            "expected >=5000 HTTP requests, got {}",
            reqs.len()
        );
        let posts = reqs.iter().filter(|r| r.method == "POST").count();
        assert!(posts >= 5000, "expected >=5000 POST, got {posts}");
        // 第一条 POST body 应能解出 JSON
        let first_post = reqs.iter().find(|r| r.method == "POST").unwrap();
        let body = first_post.body.as_deref().expect("POST must have body");
        assert!(body.contains("username"), "body must contain username: {body}");
    }
}
