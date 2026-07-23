//! pcap 扫描 + tshark 路径配置命令（4 命令）。

use std::path::Path;

use ruT0_data_kit_core::pcap::{self, PcapReader};
use ruT0_data_kit_core::pipeline::scan_pcap;
use ruT0_data_kit_core::rules::{FieldRule, RuleSet};
use ruT0_data_kit_core::scan::DefaultSensitiveScan;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

/// 读取 .pcap/.pcapng 文件并跑流量扫描 pipeline（tshark 子进程 + 双重 URL 解码
/// + base64 字段解码 + 敏感扫描），一次返回 `{ entries, report }`。
///
/// - `entries`：core `pcap::PcapReader::read` 解析出的 `Vec<HttpRequest>`，
///   前端用于原始 HTTP 请求表渲染。
/// - `report`：core `pipeline::scan_pcap` 产出的 `Report`，含 findings +
///   summary（total_requests / sensitive_hits / decoded_fragments /
///   top_src_ips）。
///
/// tshark 缺失时返回 `CoreError::DependencyMissing("tshark")`，前端 PcapView
/// 据此 `message.error` 弹提示并禁用按钮。fixture 是 5000 POST × JSON body
/// 单字段 base64，敏感扫描规则集与日志扫描一致（idcard/phone/name 三类）。
#[tauri::command]
pub fn scan_pcap_file(path: String) -> Result<Value, String> {
    let requests = PcapReader::read(Path::new(&path)).map_err(|e| e.to_string())?;
    let scan = DefaultSensitiveScan::new();
    let rules = pcap_sensitive_ruleset();
    let report = scan_pcap(Path::new(&path), &scan, &rules).map_err(|e| e.to_string())?;
    let report_value = serde_json::to_value(&report).map_err(|e| e.to_string())?;
    let entries_value = serde_json::to_value(&requests).map_err(|e| e.to_string())?;
    Ok(json!({
        "entries": entries_value,
        "report": report_value,
    }))
}

/// v0.3.0 pcap 默认敏感扫描规则集：idcard / phone / name 三类内置 validator。
///
/// 与日志扫描口径对齐：fixture 仅含 PII（身份证 / 中文姓名 / 手机号），不跑
/// SQLi 签名（v0.3.0 scope）。返回独立 RuleSet（硬编码，v0.4.4 规则池初始
/// 为空但 pcap 扫描仍需这三条 PII validator 才能产出 finding）。
fn pcap_sensitive_ruleset() -> RuleSet {
    RuleSet {
        validators: vec![
            FieldRule {
                field: "id_card".into(),
                scope: "idcard".into(),
                tag: "extract".into(),
                params: None,
                message: None,
                description: None,
            },
            FieldRule {
                field: "phone".into(),
                scope: "phone".into(),
                tag: "extract".into(),
                params: None,
                message: None,
                description: None,
            },
            FieldRule {
                field: "name".into(),
                scope: "name".into(),
                tag: "extract".into(),
                params: None,
                message: None,
                description: None,
            },
        ],
        maskers: vec![],
    }
}

// ─────────────────────────────────────────────────────────────────────
// v0.4.2 T7-2 设置模块：tshark 多平台自动探测 + 路径配置持久化
// ─────────────────────────────────────────────────────────────────────

/// settings.json 在 `app_config_dir` 下的相对文件名。
const SETTINGS_FILE_NAME: &str = "settings.json";

/// 解析 `app_config_dir/settings.json` 的绝对路径。
///
/// 失败返回字符串错误（前端按 message.error 弹出）。目录不存在不在此处创建——
/// 读取时返回 None 即可，写入时由 `save_tshark_path` 调 `create_dir_all`。
fn settings_file_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("无法定位配置目录: {e}"))?;
    Ok(config_dir.join(SETTINGS_FILE_NAME))
}

/// 自动探测本机可用的 tshark（v0.4.2 T7-2）。
///
/// 调 core `pcap::detect_tshark`：按优先级探测覆盖路径 → PATH `tshark` →
/// 各平台候选绝对路径，跑 `<path> --version` 退出 0 即视为可用。返回：
/// - 命中：`{ "path": "...", "version": "TShark (Wireshark) 4.x.x" }`
/// - 未命中：`{ "path": null, "version": null }`
///
/// 仅本机子进程探测，不调用网络（满足 docs/00 §6「不外发数据」）。
#[tauri::command]
pub fn detect_tshark() -> Result<Value, String> {
    match pcap::detect_tshark() {
        Some(info) => Ok(json!({
            "path": info.path,
            "version": info.version,
        })),
        None => Ok(json!({ "path": null, "version": null })),
    }
}

/// 从 `app_config_dir/settings.json` 读取用户保存的 tshark 覆盖路径。
///
/// 文件不存在 / 字段缺失返回 `Ok(None)`，不视为错误。读到非空路径时同步调
/// `pcap::set_tshark_path` 注入运行时，让后续 `PcapReader::read` 立即生效。
///
/// 启动时由 SettingsView `useEffect` 调用一次，把持久化配置灌入内存。
#[tauri::command]
pub fn load_tshark_path(app: AppHandle) -> Result<Option<String>, String> {
    let path = settings_file_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("读取 settings.json 失败: {e}"))?;
    let v: Value = serde_json::from_str(&content)
        .map_err(|e| format!("settings.json 解析失败: {e}"))?;
    let saved = v
        .get("tshark_path")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty());
    // 注入运行时（即使为 None 也调一次，确保与文件语义对齐）。
    pcap::set_tshark_path(saved.clone());
    Ok(saved)
}

/// 把 tshark 覆盖路径保存到 `app_config_dir/settings.json`（v0.4.2 T7-2）。
///
/// `path` 为 `Some(s)` 时写入 `{ "tshark_path": s }`；为 `None` 时写入
/// `{ "tshark_path": null }`（语义：清除自定义路径，回退到 PATH）。同时调
/// `pcap::set_tshark_path` 注入运行时，立即生效。
///
/// 目录不存在时 `create_dir_all` 兜底创建。文件 IO 失败返回字符串错误。
#[tauri::command]
pub fn save_tshark_path(app: AppHandle, path: Option<String>) -> Result<(), String> {
    let file_path = settings_file_path(&app)?;
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {e}"))?;
    }
    let v = json!({ "tshark_path": path });
    let s = serde_json::to_string_pretty(&v)
        .map_err(|e| format!("settings.json 序列化失败: {e}"))?;
    std::fs::write(&file_path, s).map_err(|e| format!("写入 settings.json 失败: {e}"))?;
    // 注入运行时，立即生效。
    pcap::set_tshark_path(path);
    Ok(())
}
