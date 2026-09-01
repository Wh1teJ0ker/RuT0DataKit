//! v1.0.0 tshark 多平台自动探测 + 路径覆盖（移植自 v0.8.0）。
//!
//! - 进程级全局覆盖路径（`Mutex<Option<String>>`）+ getter/setter。
//!   `PcapReader::read` 调 [`resolve_tshark_cmd`] 拿实际命令。
//! - v1.2.2：[`resolve_tshark_cmd`] 在覆盖为 None 时**自动探测**
//!   （[`detect_tshark`]），并缓存探测结果（进程生命周期内）。
//!   这解决了 Windows 上 tshark 不在 PATH 的问题——Wireshark 默认
//!   安装到 `C:\Program Files\Wireshark\` 但不加入 PATH，导致
//!   `Command::new("tshark")` spawn 失败、导入直接报 DependencyMissing。
//! - [`detect_tshark`]：按优先级探测候选路径，跑 `<path> --version`
//!   退出 0 即视为可用。候选列表覆盖 macOS / Linux / Windows 三平台。
//! - v1.2.2 修复：探测不再跑 `-r NUL` 健康检查——Windows 上 `tshark -r NUL`
//!   因 NUL 非合法 pcap 报错退出非 0，stderr 含「not valid」被误判为字段
//!   不兼容，导致有效 tshark 被探测拒绝。字段兼容性改由实际 pcap 解析时
//!   的 [`classify_tshark_failure`](crate::pcap::reader) 兜底。
//! - v1.2.2 修复：所有 tshark 子进程通过 [`build_tshark_command`] 构造，
//!   Windows 上设 `CREATE_NO_WINDOW` 标志，避免 GUI 应用 spawn CLI 子进程
//!   弹控制台窗口。
//!
//! 全本地探测，不调用网络；不上传任何信息（满足 docs/00 §6「不外发数据」）。

use std::process::Command;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Windows `CREATE_NO_WINDOW` 进程创建标志值。
///
/// GUI 应用（`windows_subsystem = "windows"`）spawn 子进程时，
/// 默认会弹一个控制台窗口（即使子进程是 CLI 工具如 tshark）。
/// 设此标志后子进程不分配控制台，避免窗口闪烁，也不干扰 stdout/stderr 管道。
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 构造 tshark 子进程 [`Command`]，带 Windows 平台的 `CREATE_NO_WINDOW` 标志。
///
/// 所有 tshark 调用（探测 + 实际 pcap 解析）都应通过此函数构造 Command，
/// 保证 Windows 上 GUI 应用 spawn CLI 子进程不弹控制台窗口。
/// Unix 上直接返回 `Command::new(path)`，无额外行为。
pub fn build_tshark_command(path: &str) -> Command {
    #[cfg(windows)]
    {
        let mut cmd = Command::new(path);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }
    #[cfg(not(windows))]
    {
        Command::new(path)
    }
}

/// 进程级 tshark 覆盖路径。`None` 表示回退到自动探测。
static TSHARK_OVERRIDE: Mutex<Option<String>> = Mutex::new(None);

/// 自动探测缓存。`None` = 尚未探测；`Some(path)` = 探测结果（可能是
/// 实际路径，也可能是 `"tshark"` 字面量表示探测失败、回退 PATH）。
/// `set_tshark_path` 改变覆盖时清空缓存，使下次 `resolve_tshark_cmd` 重新探测。
static TSHARK_CACHE: Mutex<Option<String>> = Mutex::new(None);

/// 检测到的 tshark 信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TsharkInfo {
    /// tshark 可执行文件绝对路径（或 `tshark` 字面量，表示走 PATH）。
    pub path: String,
    /// `tshark --version` 首行（如 `TShark (Wireshark) 4.2.6`）。
    pub version: String,
}

/// 设置 tshark 覆盖路径。`None` 清除覆盖，回退到自动探测。
///
/// 改变覆盖时清空探测缓存，使下次 [`resolve_tshark_cmd`] 重新探测。
pub fn set_tshark_path(path: Option<String>) {
    // Mutex 中毒仅在持锁线程 panic 且未恢复时发生，此时静默丢弃覆盖值
    // 优于让整个应用 panic。
    if let Ok(mut guard) = TSHARK_OVERRIDE.lock() {
        *guard = path.filter(|s| !s.trim().is_empty());
    }
    // 清空缓存：覆盖变了，旧探测结果不再有效。
    if let Ok(mut cache) = TSHARK_CACHE.lock() {
        *cache = None;
    }
}

/// 读取当前覆盖路径（可能为 None）。
pub fn get_tshark_path() -> Option<String> {
    TSHARK_OVERRIDE.lock().map(|g| g.clone()).unwrap_or(None)
}

/// 解析实际要调用的 tshark 命令。
///
/// 优先级：
/// 1. 用户显式覆盖路径（`set_tshark_path(Some(path))` / settings.json）
/// 2. 自动探测结果（`detect_tshark`），进程生命周期内缓存
/// 3. 探测失败时回退字面量 `"tshark"`（走 PATH，作为最后兜底）
///
/// v1.2.2 之前：覆盖为 None 时直接回退 `"tshark"` 字面量，在 Windows 上
/// Wireshark 不加入 PATH 导致 spawn 失败 → 导入报 DependencyMissing。
/// 现在：覆盖为 None 时调 [`detect_tshark`] 自动探测候选路径（含
/// `C:\Program Files\Wireshark\tshark.exe`），探测成功则缓存并使用。
pub fn resolve_tshark_cmd() -> String {
    // 1. 用户显式覆盖优先。
    if let Some(p) = get_tshark_path() {
        return p;
    }
    // 2. 查缓存，避免每次导入都重新探测。
    if let Ok(cache) = TSHARK_CACHE.lock() {
        if let Some(ref cached) = *cache {
            return cached.clone();
        }
    }
    // 3. 自动探测。
    let detected = detect_tshark().map(|info| info.path).unwrap_or_else(|| "tshark".to_string());
    if let Ok(mut cache) = TSHARK_CACHE.lock() {
        *cache = Some(detected.clone());
    }
    detected
}

/// 多平台 tshark 候选路径（macOS / Linux / Windows）。
///
/// 顺序固定，[`detect_tshark`] 按列表顺序探测，先命中先返回。
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
/// 1. 当前覆盖路径（用户已显式配置的优先）
/// 2. PATH 中的 `tshark`（字面量，OS 负责查 PATH）
/// 3. [`candidate_paths`] 各候选绝对路径
///
/// 全部失败返回 `None`，不 panic。
pub fn detect_tshark() -> Option<TsharkInfo> {
    let mut candidates: Vec<String> = Vec::new();
    if let Some(p) = get_tshark_path() {
        candidates.push(p);
    }
    candidates.push("tshark".to_string());
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
/// v1.2.2：仅检查 `--version` 退出 0 + 首行非空。此前额外跑一次
/// `-q -T fields -e frame.number -r <null>` 健康检查，但 Windows 上
/// `tshark -r NUL` 因 NUL 不是合法 pcap 会导致 tshark 报错退出非 0，
/// 且部分版本 stderr 含「not valid」字样被误判为「字段不兼容」，
/// 使探测对有效 tshark 返回 `None` → 设置页「自动检测」失败、
/// 用户手动选了路径但因 `resolve_tshark_cmd` 走探测缓存也受影响。
///
/// 字段兼容性由实际 pcap 解析时的 [`classify_tshark_failure`](crate::pcap::reader)
/// 兜底（stderr 含 `aren't valid` → `NotImplemented`），探测阶段不再拦截。
fn probe_tshark(path: &str) -> Option<TsharkInfo> {
    let version_out = build_tshark_command(path)
        .arg("--version")
        .output()
        .ok()?;
    if !version_out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&version_out.stdout);
    let version = stdout.lines().next().unwrap_or("").trim().to_string();
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
        assert!(paths.contains(&"/usr/bin/tshark"));
        // Windows
        assert!(paths.iter().any(|p| p.contains(r"Program Files")));
        assert!(paths.iter().any(|p| p.contains(r"Program Files (x86)")));
    }

    #[test]
    fn candidate_paths_windows_paths_are_absolute() {
        let paths = candidate_paths();
        let win_paths: Vec<_> = paths
            .iter()
            .filter(|p| p.contains(r"Program Files"))
            .copied()
            .collect();
        assert!(!win_paths.is_empty(), "至少有一条 Windows 路径");
        for p in &win_paths {
            assert!(p.starts_with(r"C:\"), "Windows 路径以盘符开头: {p}");
            assert!(
                p.ends_with("tshark.exe"),
                "Windows 路径以 tshark.exe 结尾: {p}"
            );
        }
    }

    #[test]
    fn set_and_get_tshark_path_roundtrip() {
        let original = get_tshark_path();
        set_tshark_path(Some("/tmp/fake_tshark".to_string()));
        assert_eq!(get_tshark_path(), Some("/tmp/fake_tshark".to_string()));
        set_tshark_path(None);
        assert_eq!(get_tshark_path(), None);
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
    fn resolve_tshark_cmd_falls_back_when_no_override() {
        // 覆盖为 None 时：自动探测 → 本机无 tshark → 回退 "tshark" 字面量。
        // （CI 环境无 tshark，detect_tshark() 返回 None，缓存为 "tshark"。）
        let original = get_tshark_path();
        set_tshark_path(None);
        let resolved = resolve_tshark_cmd();
        // 探测结果被缓存，应该是合法的非空字符串。
        assert!(!resolved.is_empty(), "resolve 不应返回空串: {resolved}");
        set_tshark_path(Some("/opt/homebrew/bin/tshark".to_string()));
        // 显式覆盖优先于缓存。
        assert_eq!(resolve_tshark_cmd(), "/opt/homebrew/bin/tshark");
        set_tshark_path(original);
    }

    #[test]
    fn set_tshark_path_clears_cache() {
        let original = get_tshark_path();
        // 第一次 resolve 填充缓存
        set_tshark_path(None);
        let _ = resolve_tshark_cmd();
        // 设置覆盖应清空缓存
        set_tshark_path(Some("/tmp/fake_tshark".to_string()));
        assert_eq!(resolve_tshark_cmd(), "/tmp/fake_tshark");
        // 清除覆盖也应清空缓存
        set_tshark_path(None);
        let _ = resolve_tshark_cmd();
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
