//! v1.0.0 pcap 读取器：tshark 子进程 + HTTP 字段提取（移植自 v0.8.0）。
//!
//! - [`PcapReader::read`] 调 `tshark -q -r <path> -Y http.request -T fields ...`
//!   提取 8 个 HTTP 字段，每行一条 [`HttpRequest`]。
//! - **错误分类**（v1.2.2 修复，Windows 解析兼容）：
//!   - 命令无法 spawn（路径不存在 / 受限环境拦截）→ [`CoreError::DependencyMissing`]，
//!     文案提示用户「配置 tshark 路径或安装 Wireshark」，不再把「解析报错」与
//!     「依赖缺失」混为一谈。
//!   - tshark 正常启动但退出码非 0（含 tshark 自身报错 / 崩溃）→
//!     [`CoreError::Other`]，stderr 单行化注入。`-q` 已关闭 banner，
//!     因此 stderr 只可能是真正的报错文字，Windows 控制台编码（GBK/UTF-16）
//!     由 `String::from_utf8_lossy` 兜底，不会因为混入 banner 或编码差异导致
//!     误判解析失败。
//!   - tshark 明确提示「字段无效」（如当前版本不支持 HTTP 字段）→
//!     [`CoreError::NotImplemented`]，引导升级 Wireshark。
//!   - `-Y http.request` 无匹配帧 → 退出 0 + 空输出，返回空列表（正常语义，
//!     不是错误）。
//! - `body` 字段（`http.file_data`）tshark 以十六进制输出，由
//!   [`hex_to_bytes`] 解码成字节再 `String::from_utf8_lossy`。
//!
//! 全本地处理，tshark 仅在用户本机执行，不上传 pcap/规则/样本。

use std::path::Path;
use std::process::Command;

use crate::error::CoreError;
use crate::pcap::detect::resolve_tshark_cmd;

/// 一条 HTTP 请求记录（tshark 字段映射）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// tshark stderr 中出现「字段无效」的典型文案。
const FIELDS_INVALID_MARKERS: [&str; 3] = [
    "aren't valid",
    "not valid",
    "invalid field",
];

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
        let path_str = path.to_string_lossy().into_owned();
        let mut cmd = Command::new(&tshark);
        // 构造 tshark 字段提取命令：
        //   -q 关闭 banner / 捕获信息，使 stdout 纯净为字段、stderr 纯净为错误，
        //     避免 Windows 控制台编码把 banner 或报错混入解析。
        //   -E separator=\t + -E occurrence=f 与既有解析器约定一致。
        cmd.arg("-q")
            .arg("-r")
            .arg(&path_str)
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
            .arg("occurrence=f");

        // spawn/启动失败（路径不存在、无权限、受限环境拦截 .exe）——
        // 不是「解析失败」，是依赖不可用，归 DependencyMissing 并给可操作提示。
        let output = cmd.output().map_err(|e| {
            CoreError::DependencyMissing(format!(
                "无法启动 tshark（{}）：{e}。请在设置中配置正确的 tshark 路径，或安装 Wireshark 后重试。",
                tshark
            ))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(classify_tshark_failure(&tshark, &stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_tshark_output(&stdout))
    }
}

/// 把 tshark 非零退出分类为具体 [`CoreError`]。
///
/// - stderr 含「字段无效」→ [`CoreError::NotImplemented`]（引导升级 Wireshark）。
/// - 其他退出 → [`CoreError::Other`]，单行化 stderr，避免整段编码乱码刷屏。
fn classify_tshark_failure(tshark: &str, stderr: &str) -> CoreError {
    let stderr = stderr.trim();
    let body = if stderr.is_empty() {
        "(tshark 无错误输出)".to_string()
    } else {
        // 单行化：Windows 上 stderr 可能含 \r\n / ANSI；压成一行便于前端展示。
        stderr.lines().map(str::trim).collect::<Vec<_>>().join(" | ").into()
    };
    if FIELDS_INVALID_MARKERS.iter().any(|m| stderr.contains(m)) {
        return CoreError::NotImplemented(
            "当前 tshark 不支持所需的 HTTP 提取字段（http.request 等）。请升级 Wireshark/tshark 后重试。",
        );
    }
    CoreError::Other(format!("tshark（{tshark}）解析失败：{body}"))
}

/// 解析 tshark `-T fields` 的 stdout 为 [`HttpRequest`] 列表。
///
/// 每行一条记录，tab 分隔 8 字段；空行跳过。缺失字段以空串补齐
/// （tshark 对空字段可能输出连续分隔符）。`http.file_data`（索引 6）
/// tshark 以十六进制输出，由 [`hex_to_bytes`] 解码成字节再
/// `String::from_utf8_lossy`。
fn parse_tshark_output(stdout: &str) -> Vec<HttpRequest> {
    let mut result = Vec::new();
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        // 8 个字段；缺失的用空串补齐（tshark 对空字段可能输出连续分隔符）。
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
    result
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
    use crate::pcap::detect::set_tshark_path;

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

    #[test]
    fn parse_tshark_output_basic() {
        let stdout = "1\t192.168.1.1\t93.184.216.34\tGET\texample.com\t/\t\tRuT0DataKit-test/1.0";
        let reqs = parse_tshark_output(stdout);
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].frame_no, "1");
        assert_eq!(reqs[0].src_ip, "192.168.1.1");
        assert_eq!(reqs[0].dst_ip, "93.184.216.34");
        assert_eq!(reqs[0].method, "GET");
        assert_eq!(reqs[0].host, "example.com");
        assert_eq!(reqs[0].uri, "/");
        assert_eq!(reqs[0].body, "");
        assert_eq!(reqs[0].user_agent, "RuT0DataKit-test/1.0");
    }

    #[test]
    fn parse_tshark_output_skips_empty_lines() {
        let stdout = "\n\n1\t\t\tGET\thost\t/\t\t\n\n\n2\t\t\tPOST\thost2\t/path\t\t\n";
        let reqs = parse_tshark_output(stdout);
        assert_eq!(reqs.len(), 2);
        assert_eq!(reqs[0].frame_no, "1");
        assert_eq!(reqs[0].method, "GET");
        assert_eq!(reqs[0].host, "host");
        assert_eq!(reqs[1].frame_no, "2");
        assert_eq!(reqs[1].method, "POST");
        assert_eq!(reqs[1].host, "host2");
    }

    #[test]
    fn parse_tshark_output_hex_body_decoded() {
        // "Hello" = 48656c6c6f
        let stdout = "1\t10.0.0.1\t8.8.8.8\tPOST\thost\t/api\t48656c6c6f\tua";
        let reqs = parse_tshark_output(stdout);
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].body, "Hello");
        assert_eq!(reqs[0].method, "POST");
        assert_eq!(reqs[0].uri, "/api");
    }

    #[test]
    fn parse_tshark_output_missing_fields_padded() {
        // 只到 method（索引 3）→ 其余补空串
        let stdout = "1\t\t\tGET";
        let reqs = parse_tshark_output(stdout);
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].frame_no, "1");
        assert_eq!(reqs[0].src_ip, "");
        assert_eq!(reqs[0].dst_ip, "");
        assert_eq!(reqs[0].method, "GET");
        assert_eq!(reqs[0].host, "");
        assert_eq!(reqs[0].uri, "");
        assert_eq!(reqs[0].body, "");
        assert_eq!(reqs[0].user_agent, "");
    }

    #[test]
    fn parse_tshark_output_empty_input() {
        assert_eq!(parse_tshark_output("").len(), 0);
        assert_eq!(parse_tshark_output("\n\n\n").len(), 0);
        assert_eq!(parse_tshark_output("   \n\t\n").len(), 0);
    }

    // ---- v1.2.2：错误分类与 Windows 解析兼容 ----

    #[test]
    fn classify_tshark_failure_other_with_single_line_stderr() {
        let err = classify_tshark_failure("tshark", "line1\r\nline2\n");
        assert!(matches!(err, CoreError::Other(_)), "got: {err:?}");
        let msg = err.to_string();
        assert!(msg.contains("line1 | line2"), "单行化 stderr: {msg}");
        assert!(msg.contains("tshark"), "包含命令名: {msg}");
    }

    #[test]
    fn classify_tshark_failure_empty_stderr_not_blank() {
        let err = classify_tshark_failure("tshark", "  \n");
        assert!(matches!(err, CoreError::Other(_)), "got: {err:?}");
        assert!(err.to_string().contains("无错误输出"));
    }

    #[test]
    fn classify_tshark_failure_invalid_fields_is_not_implemented() {
        let err = classify_tshark_failure(
            "tshark",
            "tshark: Some fields aren't valid:\n\tnotarealfield",
        );
        assert!(matches!(err, CoreError::NotImplemented(_)), "got: {err:?}");
    }

    #[test]
    fn read_missing_binary_returns_dependency_missing() {
        // 覆盖到一个确定不存在的路径：Command::new 无法启动 → DependencyMissing，
        // 文案提示配置路径 / 安装 Wireshark（而不是报成「解析失败」）。
        let original = crate::pcap::detect::get_tshark_path();
        let missing = if cfg!(windows) {
            r"C:\nonexistent\RuT0_tshark_probe.exe"
        } else {
            "/nonexistent/RuT0_tshark_probe"
        };
        set_tshark_path(Some(missing.to_string()));
        let result = PcapReader::new().read(Path::new("/tmp/whatever.pcap"));
        set_tshark_path(original);
        let err = result.expect_err("不应成功");
        assert!(matches!(err, CoreError::DependencyMissing(_)), "got: {err:?}");
        assert!(
            err.to_string().contains("无法启动 tshark"),
            "提示可操作: {err}"
        );
    }
}