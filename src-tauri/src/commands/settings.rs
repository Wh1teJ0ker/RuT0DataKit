//! v1.0.0 tshark 设置命令（多平台路径检测 + 持久化）。
//!
//! settings.json 结构：`{ "tshark_path": "<path>" | null }`。
//! 读 `app_config_dir/settings.json`，缺失返回默认。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;

use ruT0_data_kit_core::pcap;

/// settings.json 结构：`{ "tshark_path": "<path>" | null }`。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct TsharkSettings {
    pub(crate) tshark_path: Option<String>,
}

/// 读取 app_config_dir 下的 settings.json，缺失返回默认。
pub(crate) fn read_settings(app: &AppHandle) -> TsharkSettings {
    use tauri::Manager;
    let dir = match app.path().app_config_dir() {
        Ok(p) => p,
        Err(_) => return TsharkSettings::default(),
    };
    let path = dir.join("settings.json");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return TsharkSettings::default(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

/// 写入 settings.json。
fn write_settings(app: &AppHandle, settings: &TsharkSettings) -> Result<(), String> {
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
