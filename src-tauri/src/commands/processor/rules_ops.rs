//! 规则管理命令（list_rules / toggle_rule / update_rule_params /
//! update_rule_template / update_rule_extract_config）。
//!
//! 全部 `#[tauri::command]` → `Result<T, String>` + `.map_err(|e| e.to_string())`。
//! 依赖 `DbManager`（规则持久化）。
//!
//! v1.1.0：规则不再经 `RuleState` 内存态，改从 DB 读取（`DbManager::get_rule`），
//! `toggle_rule` 写回 `rules.enabled`，新增 `update_rule_params` 写回
//! `rules.pattern` / `rules.replacement`。

use ruT0_data_kit_core::processor::rules::TemplateParams;

// ---------------------------------------------------------------------------
// 规则管理命令
// ---------------------------------------------------------------------------

/// 列出全部规则（camelCase 序列化）。从 DB 读取。
#[tauri::command]
pub fn list_rules(
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<Vec<ruT0_data_kit_core::processor::Rule>, String> {
    db.list_rules().map_err(|e| e.to_string())
}

/// 切换规则启用状态。写回 `rules.enabled`。
#[tauri::command]
pub fn toggle_rule(
    rule_id: String,
    enabled: bool,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<(), String> {
    db.set_rule_enabled(&rule_id, enabled)
        .map_err(|e| e.to_string())
        .and_then(|ok| {
            if ok {
                Ok(())
            } else {
                Err(format!("规则 `{rule_id}` 不存在"))
            }
        })
}

/// 更新规则可填参数（pattern / replacement）。`null` 字段表示不变。
/// v1.1.0：仅允许改参数，不可新增规则。
#[tauri::command]
pub fn update_rule_params(
    rule_id: String,
    pattern: Option<String>,
    replacement: Option<String>,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<(), String> {
    // Option<String> → Option<&str>：仅当非空字符串时更新，空串视为清空。
    let p = pattern.as_deref();
    let r = replacement.as_deref();
    db.update_rule_params(&rule_id, p, r)
        .map_err(|e| e.to_string())
        .and_then(|ok| {
            if ok {
                Ok(())
            } else {
                Err(format!("规则 `{rule_id}` 不存在"))
            }
        })
}

/// 更新规则的模板脱敏参数（`rules.template` 列）。v1.1.3 T49 新增。
///
/// `template` 为 `Some(tpl)` → 持久化到 DB；`None` → 清空模板（写 NULL）。
/// 前端选预设 → 填充 7 个可编辑参数框（含 T53 反向脱敏开关）→ 调本命令持久化
/// 到 `simple-mask`（整段脱敏）或 `segment-mask`（分段脱敏）规则。
/// SQL 全部用 `?N` + `params![]` 绑定，禁止字符串拼接。
#[tauri::command]
pub fn update_rule_template(
    rule_id: String,
    template: Option<TemplateParams>,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<(), String> {
    db.update_rule_template(&rule_id, template.as_ref())
        .map_err(|e| e.to_string())
        .and_then(|ok| {
            if ok {
                Ok(())
            } else {
                Err(format!("规则 `{rule_id}` 不存在"))
            }
        })
}

/// 更新规则的提取配置（`rules.pattern` + `rules.params` 列）。v1.1.3 T55 新增。
///
/// 前端 RulesPanel 编辑提取规则（phone-extract / bankcard-extract / ip4-extract
/// / ip6-extract）时调用本命令：
/// - `pattern` 为 `Some(s)` → 持久化提取正则；`None` → 不变；`Some("")` → 清空。
/// - `params` 为 `Some(p)` → 序列化为 JSON 写入 `rules.params` 列；`None` → 清空。
/// SQL 全部用 `?N` + `params![]` 绑定，禁止字符串拼接。
#[tauri::command]
pub fn update_rule_extract_config(
    rule_id: String,
    pattern: Option<String>,
    params: Option<ruT0_data_kit_core::processor::ExtractParams>,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<(), String> {
    let p = pattern.as_deref();
    db.update_rule_extract_config(&rule_id, p, params.as_ref())
        .map_err(|e| e.to_string())
        .and_then(|ok| {
            if ok {
                Ok(())
            } else {
                Err(format!("规则 `{rule_id}` 不存在"))
            }
        })
}

// 规则管理命令不直接依赖 Tauri runtime，但 `#[tauri::command]` 函数签名带
// `tauri::State<'_, DbManager>` 在单元测试中无法直接调用；规则管理的底层
// `DbManager` 方法（list_rules / set_rule_enabled / update_rule_params /
// update_rule_template / update_rule_extract_config）由 `db` 模块的测试与
// `extract_ops` / `undo_ops` 的集成测试间接覆盖，本模块不单独维护测试。
