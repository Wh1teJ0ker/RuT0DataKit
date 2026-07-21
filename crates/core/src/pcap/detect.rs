//! v0.4.2 tshark 多平台自动探测 + 路径覆盖。
//!
//! 设计要点：
//! - 进程级全局覆盖路径（`Mutex<Option<String>>`）+ 三个 getter/setter：
//!   [`set_tshark_path`] / [`get_tshark_path`] / [`resolve_tshark_cmd`]。
//!   `PcapReader::read` 调 [`resolve_tshark_cmd`] 拿实际命令，覆盖为 None 时
//!   回退到 PATH 中的 `tshark`（v0.3.0 行为，零回归）。
//! - [`detect_tshark`]：按优先级探测候选路径，跑 `<path> --version` 退出 0 即
//!   视为可用，返回 [`TsharkInfo`]。候选列表覆盖 macOS / Linux / Windows 三平台
//!   常见安装位置（homebrew / apt / Wireshark.app / Program Files）。
//!
//! 全本地探测，不调用网络；不上传任何信息（满足 docs/00 §6「不外发数据」）。

use std::process::Command;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// 进程级 tshark 覆盖路径。`None` 表示回退到 PATH 中的 `tshark`。
///
/// 由 Tauri `load_tshark_path` / `save_tshark_path` 命令在启动时与用户配置时
/// 写入；`PcapReader::read` 通过 [`resolve_tshark_cmd`] 读取。
static TSHARK_OVERRIDE: Mutex<Option<String>> = Mutex::new(None);

/// 检测到的 tshark 信息（v0.4.2）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TsharkInfo {
    /// tshark 可执行文件绝对路径（或 `tshark` 字面量，表示走 PATH）。
    pub path: String,
    /// `tshark --version` 首行（如 `TShark (Wireshark) 4.2.6`）。
    pub version: String,
}

/// 设置 tshark 覆盖路径。`None` 清除覆盖，回退到 PATH 中的 `tshark`。
///
/// Tauri `save_tshark_path` / `load_tshark_path` 命令调用本函数注入运行时配置。
/// Poison 锁兜底为 panic（与 v0.4.2 之前行为对齐——单进程 GUI 不会 poison）。
pub fn set_tshark_path(path: Option<String>) {
    let mut guard = TSHARK_OVERRIDE.lock().expect("TSHARK_OVERRIDE poisoned");
    *guard = path.filter(|s| !s.trim().is_empty());
}

/// 读取当前覆盖路径（可能为 None）。
pub fn get_tshark_path() -> Option<String> {
    TSHARK_OVERRIDE
        .lock()
        .expect("TSHARK_OVERRIDE poisoned")
        .clone()
}

/// 解析实际要调用的 tshark 命令：覆盖路径优先，否则字面量 `tshark`。
///
/// `PcapReader::read` 用本函数的返回值作为 `Command::new(...)` 的第一参数。
pub fn resolve_tshark_cmd() -> String {
    get_tshark_path().unwrap_or_else(|| "tshark".to_string())
}

/// 多平台 tshark 候选路径（macOS / Linux / Windows）。
///
/// 顺序固定，[`detect_tshark`] 按列表顺序探测，先命中先返回。
/// Windows 路径含空格不引号——`Command::new` 接受完整路径字符串，OS 层负责解析。
pub fn candidate_paths() -> Vec<&'static str> {
    vec![
        // macOS homebrew (apple silicon + intel) + Wireshark.app bundle
        "/opt/homebrew/bin/tshark",
        "/usr/local/bin/tshark",
        "/Applications/Wireshark.app/Contents/MacOS/tshark",
        // Linux 包管理器常见路径
        "/usr/bin/tshark",
        "/usr/local/bin/tshark",
        // Windows Wireshark 默认安装路径
        r"C:\Program Files\Wireshark\tshark.exe",
        r"C:\Program Files (x86)\Wireshark\tshark.exe",
    ]
}

/// 自动探测本机可用的 tshark。
///
/// 探测顺序：
/// 1. 当前覆盖路径（[`get_tshark_path`]）—— 用户已显式配置的优先
/// 2. PATH 中的 `tshark`（字面量，OS 负责查 PATH）
/// 3. [`candidate_paths`] 各候选绝对路径
///
/// 每个候选跑 `<path> --version`，退出 0 即视为可用，取 stdout 首行作 version。
/// 全部失败返回 `None`，不 panic、不返回错误（调用方按 None 处理「未检测到」）。
pub fn detect_tshark() -> Option<TsharkInfo> {
    // 1. 已配置的覆盖路径优先
    let mut candidates: Vec<String> = Vec::new();
    if let Some(p) = get_tshark_path() {
        candidates.push(p);
    }
    // 2. PATH 中的 tshark
    candidates.push("tshark".to_string());
    // 3. 各平台绝对路径候选
    candidates.extend(candidate_paths().iter().map(|s| s.to_string()));

    for path in candidates {
        if let Some(info) = probe_tshark(&path) {
            return Some(info);
        }
    }
    None
}

/// 跑 `<path> --version` 探测单个候选，成功返回 [`TsharkInfo`]。
///
/// 退出码非 0 / spawn 失败 / stdout 为空均视为不可用，返回 `None`。
/// version 取 stdout 首行（tshark 首行形如 `TShark (Wireshark) 4.2.6 ...`）。
fn probe_tshark(path: &str) -> Option<TsharkInfo> {
    let out = Command::new(path).arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let version = stdout
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if version.is_empty() {
        return None;
    }
    Some(TsharkInfo {
        path: path.to_string(),
        version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_paths_covers_three_platforms() {
        let paths = candidate_paths();
        // macOS
        assert!(paths.iter().any(|p| p.contains("homebrew")));
        assert!(paths.iter().any(|p| p.contains("Wireshark.app")));
        // Linux
        assert!(paths.iter().any(|p| *p == "/usr/bin/tshark"));
        // Windows
        assert!(paths.iter().any(|p| p.contains(r"Program Files")));
        assert!(paths.iter().any(|p| p.contains(r"Program Files (x86)")));
    }

    #[test]
    fn set_and_get_tshark_path_roundtrip() {
        // 保存原值，测试结束后还原，避免污染其他测试。
        let original = get_tshark_path();
        set_tshark_path(Some("/tmp/fake_tshark".to_string()));
        assert_eq!(get_tshark_path(), Some("/tmp/fake_tshark".to_string()));
        set_tshark_path(None);
        assert_eq!(get_tshark_path(), None);
        // 还原
        set_tshark_path(original);
    }

    #[test]
    fn set_tshark_path_ignores_empty_or_whitespace() {
        let original = get_tshark_path();
        set_tshark_path(Some("   ".to_string()));
        assert_eq!(get_tshark_path(), None);
        set_tshark_path(Some("".to_string()));
        assert_eq!(get_tshark_path(), None);
        set_tshark_path(original);
    }

    #[test]
    fn resolve_tshark_cmd_falls_back_to_literal() {
        let original = get_tshark_path();
        set_tshark_path(None);
        assert_eq!(resolve_tshark_cmd(), "tshark");
        set_tshark_path(Some("/opt/homebrew/bin/tshark".to_string()));
        assert_eq!(resolve_tshark_cmd(), "/opt/homebrew/bin/tshark");
        set_tshark_path(original);
    }

    #[test]
    fn probe_tshark_nonexistent_returns_none() {
        assert!(probe_tshark("/nonexistent/path/tshark").is_none());
    }

    /// 本机有 tshark 时跑；CI 跳过。
    #[test]
    #[ignore = "本机 tshark 探测，CI 无 tshark 时跳过"]
    fn detect_tshark_local() {
        if let Some(info) = detect_tshark() {
            assert!(!info.path.is_empty());
            assert!(info.version.starts_with("TShark") || info.version.contains("TShark"));
        }
    }
}
