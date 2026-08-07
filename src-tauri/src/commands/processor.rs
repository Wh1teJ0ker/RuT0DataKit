//! v1.1.0 处理器类 IPC 命令（脱敏 / 校验 / 提取 / 规则管理）。
//!
//! 全部 `#[tauri::command]` → `Result<T, String>` + `.map_err(|e| e.to_string())`。
//! 依赖 `DbManager`（列读写 + 规则持久化）。
//!
//! v1.1.0：规则不再经 `RuleState` 内存态，改从 DB 读取（`DbManager::get_rule`），
//! `toggle_rule` 写回 `rules.enabled`，新增 `update_rule_params` 写回
//! `rules.pattern` / `rules.replacement`。

use serde::Serialize;

use ruT0_data_kit_core::processor::{
    ExtractItem, Extractor, Masker, PiiExtractor, RegexValidator, Validator,
};

use crate::db::Cell;

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

// ---------------------------------------------------------------------------
// 脱敏 / 校验 / 提取命令
// ---------------------------------------------------------------------------

/// 脱敏结果（camelCase）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskResult {
    pub affected: u32,
}

/// 校验单行结果（camelCase）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowValidation {
    pub row_idx: u32,
    pub passed: bool,
    pub message: String,
}

/// 提取单行命中（camelCase）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowExtract {
    pub row_idx: u32,
    pub hits: Vec<ExtractItem>,
}

/// 脱敏指定列：读取列全部数据行 → `SimpleMasker::mask` → 回写 cells（upsert）
/// → `log_operation("mask")`。返回受影响行数。
///
/// - `rule_id` 可选；指向 DB `rules` 表的脱敏规则。`None` 且 `replacement` 也为空
///   时走通用脱敏（保留首尾）。`None` + `replacement` 非空 → 等价于临时 mask 规则
///   （保留首字符 + 其余用掩码字符替换）。
/// - `replacement` 可选；非空字符串的首个字符会**临时覆盖**规则/默认的掩码字符
///   （仅本次调用，不写回 DB）。空串/`None` → 默认 `*`。
///
/// Masker 优先级：临时 replacement > rule.replacement > 默认 `*`。
#[tauri::command]
pub fn mask_column(
    sheet_id: i64,
    column: String,
    rule_id: Option<String>,
    replacement: Option<String>,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<MaskResult, String> {
    let col_idx = db
        .find_col_idx(sheet_id, &column)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("列 `{column}` 不存在"))?;

    let rows = db
        .query_column_cells(sheet_id, col_idx)
        .map_err(|e| e.to_string())?;

    // 可选规则：从 DB 查找。
    let rule_owned = if let Some(rid) = &rule_id {
        db.get_rule(rid).map_err(|e| e.to_string())?
    } else {
        None
    };
    // 临时 replacement 非空 → 覆盖规则 replacement（不写回 DB）。
    // rule_id=None + replacement 非空 → 构造临时 mask 规则。
    let rule_override = replacement.as_deref().and_then(|s| {
        if s.is_empty() {
            None
        } else {
            Some(s.to_string())
        }
    });
    let rule_ref = if let Some(repl) = &rule_override {
        // 临时 mask 规则：若 rule_id 存在则克隆覆盖，否则构造临时规则。
        let mut r = rule_owned
            .clone()
            .unwrap_or_else(|| ruT0_data_kit_core::processor::Rule {
                id: "temp-mask".into(),
                name: "临时脱敏".into(),
                kind: ruT0_data_kit_core::processor::RuleKind::Mask,
                field: None,
                pattern: None,
                replacement: None,
                enabled: true,
                description: String::new(),
            });
        r.replacement = Some(repl.clone());
        Some(r)
    } else {
        rule_owned.clone()
    };

    let masker = ruT0_data_kit_core::processor::SimpleMasker;
    let mut cells: Vec<Cell> = Vec::with_capacity(rows.len());
    for (row_idx, value) in &rows {
        let input = value.as_deref().unwrap_or("");
        let result = masker
            .mask(input, rule_ref.as_ref())
            .map_err(|e| e.to_string())?;
        cells.push(Cell {
            sheet_id,
            row_idx: *row_idx,
            col_idx,
            value: Some(result.output),
        });
    }
    let affected = cells.len() as u32;
    if !cells.is_empty() {
        db.write_cells(sheet_id, &cells)
            .map_err(|e| e.to_string())?;
    }

    db.log_operation(
        Some(sheet_id),
        "mask",
        &serde_json::json!({ "column": column, "ruleId": rule_id, "replacement": replacement, "affected": affected })
            .to_string(),
        "{}",
    )
    .map_err(|e| e.to_string())?;

    Ok(MaskResult { affected })
}

/// 校验指定列：读取列全部数据行 → `RegexValidator::validate` → 返回逐行结果
/// → `log_operation("validate")`。不通过的行由前端高亮 `invalid`。
///
/// 规则从 DB `rules` 表读取（`rule_id` 必填）。
#[tauri::command]
pub fn validate_column(
    sheet_id: i64,
    column: String,
    rule_id: String,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<Vec<RowValidation>, String> {
    let col_idx = db
        .find_col_idx(sheet_id, &column)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("列 `{column}` 不存在"))?;

    let rule = db
        .get_rule(&rule_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("规则 `{rule_id}` 不存在"))?;

    let rows = db
        .query_column_cells(sheet_id, col_idx)
        .map_err(|e| e.to_string())?;

    let validator = RegexValidator;
    let mut results = Vec::with_capacity(rows.len());
    for (row_idx, value) in &rows {
        let input = value.as_deref().unwrap_or("");
        let vr = validator
            .validate(input, &rule)
            .map_err(|e| e.to_string())?;
        results.push(RowValidation {
            row_idx: *row_idx,
            passed: vr.passed,
            message: vr.message,
        });
    }

    db.log_operation(
        Some(sheet_id),
        "validate",
        &serde_json::json!({ "column": column, "ruleId": rule_id, "rows": results.len() })
            .to_string(),
        "{}",
    )
    .map_err(|e| e.to_string())?;

    Ok(results)
}

/// 提取指定列 PII：读取列全部数据行 → `PiiExtractor::extract` → 返回逐行命中
/// → `log_operation("extract")`。命中的行由前端高亮 `hit`。
#[tauri::command]
pub fn extract_column(
    sheet_id: i64,
    column: String,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<Vec<RowExtract>, String> {
    let col_idx = db
        .find_col_idx(sheet_id, &column)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("列 `{column}` 不存在"))?;

    let rows = db
        .query_column_cells(sheet_id, col_idx)
        .map_err(|e| e.to_string())?;

    let extractor = PiiExtractor::new().map_err(|e| e.to_string())?;
    let mut results = Vec::with_capacity(rows.len());
    for (row_idx, value) in &rows {
        let input = value.as_deref().unwrap_or("");
        let hits = extractor.extract(input).map_err(|e| e.to_string())?;
        results.push(RowExtract {
            row_idx: *row_idx,
            hits,
        });
    }

    let total_hits: usize = results.iter().map(|r| r.hits.len()).sum();
    db.log_operation(
        Some(sheet_id),
        "extract",
        &serde_json::json!({ "column": column, "totalHits": total_hits, "rows": results.len() })
            .to_string(),
        "{}",
    )
    .map_err(|e| e.to_string())?;

    Ok(results)
}
