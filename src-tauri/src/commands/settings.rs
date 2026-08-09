//! 应用设置命令（tshark 路径 + 全局每页行数）。
//!
//! settings.json 结构：
//! `{ "tshark_path": "<path>" | null, "page_size": <u32> | null }`。
//! 读 `app_config_dir/settings.json`，缺失返回默认。
//! v1.1.2：新增 `page_size` 字段（全局每页行数），`#[serde(default)]`
//! 保证旧版 settings.json（无 page_size）反序列化时取 None → 前端回退 50。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;

use ruT0_data_kit_core::pcap;

/// settings.json 结构（v1.1.2 新增 page_size）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct AppSettings {
    pub(crate) tshark_path: Option<String>,
    #[serde(default)]
    pub(crate) page_size: Option<u32>,
}

/// 读取 app_config_dir 下的 settings.json，缺失返回默认。
pub(crate) fn read_settings(app: &AppHandle) -> AppSettings {
    use tauri::Manager;
    let dir = match app.path().app_config_dir() {
        Ok(p) => p,
        Err(_) => return AppSettings::default(),
    };
    let path = dir.join("settings.json");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return AppSettings::default(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

/// 写入 settings.json。
fn write_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    use tauri::Manager;
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("app_config_dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("create config dir: {e}"))?;
    let path = dir.join("settings.json");
    let json =
        serde_json::to_string_pretty(settings).map_err(|e| format!("serialize settings: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("write settings: {e}"))?;
    Ok(())
}

/// 自动探测本机 tshark。
///
/// 调 `core::pcap::detect_tshark()`，返回 `TsharkInfo` 或 `null`（探测失败）。
/// 全本地探测，不外发数据。
#[tauri::command]
pub async fn detect_tshark() -> Result<Value, String> {
    match pcap::detect_tshark() {
        Some(info) => serde_json::to_value(info).map_err(|e| e.to_string()),
        None => Ok(Value::Null),
    }
}

/// 启动时加载 tshark 路径并注入 core 运行时。
///
/// 读 `app_config_dir/settings.json` 的 `tshark_path`，非空时调
/// `core::pcap::set_tshark_path` 注入进程级覆盖。返回加载到的路径（或 null）。
#[tauri::command]
pub async fn load_tshark_path(app: AppHandle) -> Result<Value, String> {
    let settings = read_settings(&app);
    let path = settings.tshark_path;
    pcap::set_tshark_path(path.clone());
    serde_json::to_value(path).map_err(|e| e.to_string())
}

/// 保存 tshark 路径（写入 settings.json + 注入运行时）。
///
/// `path` 为 `Some(non_empty)` 时设置覆盖；`None` 或空串时清除覆盖，
/// 回退到 PATH 中的 `tshark`。
#[tauri::command]
pub async fn save_tshark_path(app: AppHandle, path: Option<String>) -> Result<(), String> {
    let mut settings = read_settings(&app);
    settings.tshark_path = path.clone();
    write_settings(&app, &settings)?;
    pcap::set_tshark_path(path);
    Ok(())
}

/// 加载全局每页行数（v1.1.2）。
///
/// 读 settings.json 的 `page_size`，未配置时返回 null（前端回退默认 50）。
#[tauri::command]
pub async fn load_page_size(app: AppHandle) -> Result<Option<u32>, String> {
    let settings = read_settings(&app);
    Ok(settings.page_size)
}

/// 保存全局每页行数（v1.1.2）。
///
/// `page_size = Some(n)` 时写入；`None` 时清除（前端回退默认 50）。
/// 仅持久化到 settings.json，不注入运行时——前端 state 自行 dispatch。
#[tauri::command]
pub async fn save_page_size(app: AppHandle, page_size: Option<u32>) -> Result<(), String> {
    let mut settings = read_settings(&app);
    settings.page_size = page_size;
    write_settings(&app, &settings)
}
