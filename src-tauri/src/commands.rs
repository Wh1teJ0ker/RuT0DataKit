//! Tauri 命令层。
//!
//! v1.0.0: updater 相关命令（`check_update` / `install_update`），委托
//! `tauri-plugin-updater` 完成版本检测、下载、Ed25519 验签与安装。
//! - `commands/data.rs`    → import_file / get_sheet_data / list_sessions / open_session
//! - `commands/column.rs`  → reorder_columns / rename_column / set_column_visibility
//! - `commands/ai.rs`      → ai_suggest / invoke_ai_op
//! - `commands/settings.rs`→ get_setting / set_setting

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

/// updater 检测结果（序列化为 camelCase 给前端）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
}

impl UpdateStatus {
    /// 统一的「无可用更新」降级值：无新版本、无网络、查询异常时均用此值，
    /// 不向前端抛错（v1.0.0 静默降级策略）。
    fn none() -> Self {
        Self {
            available: false,
            version: None,
            notes: None,
        }
    }
}

/// 检查更新。返回 `UpdateStatus`，永远不抛错给前端：
/// - `Ok(Some(update))` → `available=true` + version + notes
/// - `Ok(None)`          → `available=false`
/// - `Err(_)`            → 静默降级 `available=false`（无网络/请求失败统一处理）
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<UpdateStatus, String> {
    match app.updater().map_err(|e| e.to_string())?.check().await {
        Ok(Some(update)) => Ok(UpdateStatus {
            available: true,
            version: Some(update.version.clone()),
            notes: update.body.clone(),
        }),
        Ok(None) => Ok(UpdateStatus::none()),
        // v1.0.0 不区分错误类型：无网络 / 无新版本 / 接口异常统一降级为不可用。
        Err(_) => Ok(UpdateStatus::none()),
    }
}

/// 安装更新。委托 `tauri-plugin-updater` 下载 + Ed25519 验签 + 安装。
/// 验签失败或安装失败时抛 `Err(e.to_string())` 给前端处理。
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no update available".to_string())?;
    update
        .download_and_install(|_chunk, _total| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
