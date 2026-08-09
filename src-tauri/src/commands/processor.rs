//! v1.1.0 处理器类 IPC 命令（脱敏 / 校验 / 提取 / 规则管理）。
//!
//! 全部 `#[tauri::command]` → `Result<T, String>` + `.map_err(|e| e.to_string())`。
//! 依赖 `DbManager`（列读写 + 规则持久化）。
//!
//! v1.1.0：规则不再经 `RuleState` 内存态，改从 DB 读取（`DbManager::get_rule`），
//! `toggle_rule` 写回 `rules.enabled`，新增 `update_rule_params` 写回
//! `rules.pattern` / `rules.replacement`。

use serde::Serialize;

use ruT0_data_kit_core::processor::rules::TemplateParams;
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

/// 更新规则的通用模板脱敏参数（`rules.template` 列）。v1.1.3 T49 新增。
///
/// `template` 为 `Some(tpl)` → 持久化到 DB；`None` → 清空模板（写 NULL）。
/// 前端选预设 → 填充 6 个可编辑参数框 → 调本命令持久化到 `general-mask` 规则。
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
/// - `template` 可选（v1.1.3 T49）；`Some(tpl)` 临时覆盖规则的 `template` 字段
///   （仅本次调用，不写回 DB）。前端选预设 → 填充 6 参数 → 透传给本参数执行脱敏。
///   `None` → 用规则自身的 `template`。空模板（所有字段 `None`）→ 不脱敏（透传）。
///
/// Masker 优先级：临时 replacement > rule.replacement > 默认 `*`。
/// Template 优先级：临时 template 参数 > rule.template。
#[tauri::command]
pub fn mask_column(
    sheet_id: i64,
    column: String,
    rule_id: Option<String>,
    replacement: Option<String>,
    template: Option<TemplateParams>,
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
    // v1.1.3 T49：临时 template 参数非空 → 覆盖规则 template（不写回 DB）。
    //   前端选预设 → 填充 6 参数 → 透传给本参数执行脱敏。`None` → 用规则自身 template。
    //   空模板（TemplateParams::default()）→ SimpleMasker 视为不脱敏（透传）。
    let template_override = template.as_ref().filter(|t| !t.is_empty()).cloned();
    let rule_ref = if rule_override.is_some() || template_override.is_some() {
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
                template: None,
            });
        if let Some(repl) = &rule_override {
            r.replacement = Some(repl.clone());
        }
        if let Some(tpl) = &template_override {
            r.template = Some(tpl.clone());
        }
        Some(r)
    } else {
        rule_owned.clone()
    };

    // before 快照：列全部数据行的原始值（排除表头，与 mask 只改数据行的语义一致）。
    let before_cells: Vec<Cell> = rows
        .iter()
        .map(|(row_idx, value)| Cell {
            sheet_id,
            row_idx: *row_idx,
            col_idx,
            value: value.clone(),
        })
        .collect();
    let before_json = serde_json::to_string(&before_cells).map_err(|e| e.to_string())?;

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

    // after 快照：脱敏后的 cells（仅被修改列）。
    let after_json = serde_json::to_string(&cells).map_err(|e| e.to_string())?;
    let params_json = serde_json::json!({
        "column": column,
        "ruleId": rule_id,
        "replacement": replacement,
        "template": template,
        "affected": affected
    })
    .to_string();
    db.log_operation_with_snapshot(
        Some(sheet_id),
        "mask",
        &params_json,
        Some(&before_json),
        &after_json,
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

// ---------------------------------------------------------------------------
// 撤销 / 重做 / 可撤销列表命令（v1.1.1）
// ---------------------------------------------------------------------------

/// 撤销结果（camelCase）。`restored` = 回写的 cell 数。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoResult {
    pub restored: u32,
}

/// 重做结果（camelCase）。`restored` = 回写的 cell 数。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedoResult {
    pub restored: u32,
}

/// 可撤销操作摘要（camelCase）。撤销工具栏列表用。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoableOp {
    pub id: i64,
    pub kind: String,
    pub created_at: String,
}

/// 撤销某次就地变更操作：读 `before_snapshot_json` → 回写 cells
/// → `log_operation("undo")`（undo 自身不可再撤销）。返回回写 cell 数。
///
/// - 操作不存在 → `Err("操作 {op_id} 不存在")`
/// - `before_snapshot_json` 为 `None` → `Err("操作 {op_id} 不可撤销（无 before 快照）")`
#[tauri::command]
pub fn undo_operation(
    op_id: i64,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<UndoResult, String> {
    let op = db
        .query_operation_by_id(op_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("操作 {op_id} 不存在"))?;
    let before_json = op
        .before_snapshot_json
        .as_ref()
        .ok_or_else(|| format!("操作 {op_id} 不可撤销（无 before 快照）"))?;
    let before_cells: Vec<Cell> =
        serde_json::from_str(before_json).map_err(|e| format!("解析 before 快照失败: {e}"))?;
    let sheet_id = op
        .sheet_id
        .ok_or_else(|| format!("操作 {op_id} 无 sheet_id"))?;
    let restored = before_cells.len() as u32;
    if !before_cells.is_empty() {
        db.write_cells(sheet_id, &before_cells)
            .map_err(|e| e.to_string())?;
    }
    // undo 自身 log_operation(kind="undo")，before_snapshot=None → 不可再撤销。
    db.log_operation(
        Some(sheet_id),
        "undo",
        &serde_json::json!({ "opId": op_id }).to_string(),
        "{}",
    )
    .map_err(|e| e.to_string())?;
    Ok(UndoResult { restored })
}

/// 重做某次就地变更操作：读 `result_snapshot_json`（after）→ 回写 cells
/// → `log_operation("redo")`（redo 自身不可再撤销）。返回回写 cell 数。
///
/// - 操作不存在 → `Err("操作 {op_id} 不存在")`
/// - `result_snapshot_json` 为 `None` 或 `"{}"` → `Err("操作 {op_id} 不可重做")`
#[tauri::command]
pub fn redo_operation(
    op_id: i64,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<RedoResult, String> {
    let op = db
        .query_operation_by_id(op_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("操作 {op_id} 不存在"))?;
    let after_json = op
        .result_snapshot_json
        .as_deref()
        .filter(|s| !s.is_empty() && *s != "{}")
        .ok_or_else(|| format!("操作 {op_id} 不可重做（无 result 快照）"))?;
    let after_cells: Vec<Cell> =
        serde_json::from_str(after_json).map_err(|e| format!("解析 result 快照失败: {e}"))?;
    let sheet_id = op
        .sheet_id
        .ok_or_else(|| format!("操作 {op_id} 无 sheet_id"))?;
    let restored = after_cells.len() as u32;
    if !after_cells.is_empty() {
        db.write_cells(sheet_id, &after_cells)
            .map_err(|e| e.to_string())?;
    }
    db.log_operation(
        Some(sheet_id),
        "redo",
        &serde_json::json!({ "opId": op_id }).to_string(),
        "{}",
    )
    .map_err(|e| e.to_string())?;
    Ok(RedoResult { restored })
}

/// 列出某 sheet 最近 50 条可撤销操作（kind ∈ {mask, replace_in_column,
/// replace_all}，按 `created_at` DESC）。供前端撤销工具栏展示。
#[tauri::command]
pub fn list_undoable_operations(
    sheet_id: i64,
    db: tauri::State<'_, crate::db::DbManager>,
) -> Result<Vec<UndoableOp>, String> {
    let rows = db
        .list_undoable_operations(sheet_id, 50)
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|r| UndoableOp {
            id: r.id,
            kind: r.kind,
            created_at: r.created_at,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// commands 层测试（v1.1.1 新增）
// ---------------------------------------------------------------------------

/// 撤销/重做核心路径不依赖 Tauri runtime（`#[tauri::command]` 函数签名带
/// `tauri::State<'_, DbManager>` 在单元测试中无法直接调用）。测试策略：
/// 直接调用 `DbManager` 方法模拟 mask 流程
/// （`query_column_cells` → `SimpleMasker::mask` → `write_cells`
/// → `log_operation_with_snapshot`），再测 undo / redo / list_undoable_operations
/// 的底层逻辑，覆盖撤销核心路径。
#[cfg(test)]
mod tests {
    use crate::db::{Cell, DbManager, OperationRow, UndoableOpRow};
    use ruT0_data_kit_core::processor::{Masker, SimpleMasker};

    /// 构造一个 tempdir + 空 DbManager。返回 TempDir 以保活（TempDir drop 会
    /// 删除目录与 db 文件，必须跨测试函数持有）。
    fn setup_db() -> (tempfile::TempDir, DbManager) {
        let dir = tempfile::tempdir().unwrap();
        let mgr = DbManager::new(dir.path()).unwrap();
        (dir, mgr)
    }

    /// 构造一个 sheet：1 列（col0=name），表头 + 2 行数据（张三 / 李四）。
    fn setup_sheet_with_data(db: &DbManager) -> i64 {
        let session_id = db.create_session("test", None, "csv", 3).unwrap();
        let sheet_id = db.create_sheet(session_id, "test", 0).unwrap();
        let cells: Vec<Cell> = vec![
            Cell {
                sheet_id,
                row_idx: 0,
                col_idx: 0,
                value: Some("name".into()),
            },
            Cell {
                sheet_id,
                row_idx: 1,
                col_idx: 0,
                value: Some("张三".into()),
            },
            Cell {
                sheet_id,
                row_idx: 2,
                col_idx: 0,
                value: Some("李四".into()),
            },
        ];
        db.write_cells(sheet_id, &cells).unwrap();
        sheet_id
    }

    /// 模拟 `mask_column` 的底层流程（无 Tauri State）：
    /// 读列 cells → SimpleMasker::mask → write_cells → log_operation_with_snapshot。
    /// 返回 mask operation 的 id。
    fn simulate_mask(db: &DbManager, sheet_id: i64, col_idx: u32) -> i64 {
        let rows = db.query_column_cells(sheet_id, col_idx).unwrap();
        // before 快照：列原始值。
        let before_cells: Vec<Cell> = rows
            .iter()
            .map(|(row_idx, value)| Cell {
                sheet_id,
                row_idx: *row_idx,
                col_idx,
                value: value.clone(),
            })
            .collect();
        let before_json = serde_json::to_string(&before_cells).unwrap();
        // mask：用通用脱敏（无规则），保留首尾 + 中间 `*`。
        let masker = SimpleMasker;
        let mut cells: Vec<Cell> = Vec::with_capacity(rows.len());
        for (row_idx, value) in &rows {
            let input = value.as_deref().unwrap_or("");
            let result = masker.mask(input, None).unwrap();
            cells.push(Cell {
                sheet_id,
                row_idx: *row_idx,
                col_idx,
                value: Some(result.output),
            });
        }
        if !cells.is_empty() {
            db.write_cells(sheet_id, &cells).unwrap();
        }
        let after_json = serde_json::to_string(&cells).unwrap();
        db.log_operation_with_snapshot(
            Some(sheet_id),
            "mask",
            r#"{"column":"name","affected":2}"#,
            Some(&before_json),
            &after_json,
        )
        .unwrap()
    }

    /// 模拟 `undo_operation` 的底层流程：读 op → 解析 before → write_cells
    /// → log_operation("undo")。返回回写 cell 数。
    fn simulate_undo(db: &DbManager, op_id: i64) -> Result<u32, String> {
        let op: OperationRow = db
            .query_operation_by_id(op_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("操作 {op_id} 不存在"))?;
        let before_json = op
            .before_snapshot_json
            .as_ref()
            .filter(|s| !s.is_empty() && *s != "{}")
            .ok_or_else(|| format!("操作 {op_id} 不可撤销（无 before 快照）"))?;
        let before_cells: Vec<Cell> =
            serde_json::from_str(before_json).map_err(|e| format!("解析 before 快照失败: {e}"))?;
        let sheet_id = op
            .sheet_id
            .ok_or_else(|| format!("操作 {op_id} 无 sheet_id"))?;
        let restored = before_cells.len() as u32;
        if !before_cells.is_empty() {
            db.write_cells(sheet_id, &before_cells)
                .map_err(|e| e.to_string())?;
        }
        db.log_operation(
            Some(sheet_id),
            "undo",
            &serde_json::json!({ "opId": op_id }).to_string(),
            "{}",
        )
        .map_err(|e| e.to_string())?;
        Ok(restored)
    }

    /// 模拟 `redo_operation` 的底层流程：读 op → 解析 result_snapshot →
    /// write_cells → log_operation("redo")。返回回写 cell 数。
    fn simulate_redo(db: &DbManager, op_id: i64) -> Result<u32, String> {
        let op = db
            .query_operation_by_id(op_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("操作 {op_id} 不存在"))?;
        let after_json = op
            .result_snapshot_json
            .as_deref()
            .filter(|s| !s.is_empty() && *s != "{}")
            .ok_or_else(|| format!("操作 {op_id} 不可重做（无 result 快照）"))?;
        let after_cells: Vec<Cell> =
            serde_json::from_str(after_json).map_err(|e| format!("解析 result 快照失败: {e}"))?;
        let sheet_id = op
            .sheet_id
            .ok_or_else(|| format!("操作 {op_id} 无 sheet_id"))?;
        let restored = after_cells.len() as u32;
        if !after_cells.is_empty() {
            db.write_cells(sheet_id, &after_cells)
                .map_err(|e| e.to_string())?;
        }
        db.log_operation(
            Some(sheet_id),
            "redo",
            &serde_json::json!({ "opId": op_id }).to_string(),
            "{}",
        )
        .map_err(|e| e.to_string())?;
        Ok(restored)
    }

    /// 读列全部数据行的值（按 row_idx 升序），断言用。
    fn column_values(db: &DbManager, sheet_id: i64, col_idx: u32) -> Vec<String> {
        db.query_column_cells(sheet_id, col_idx)
            .unwrap()
            .into_iter()
            .map(|(_, v)| v.unwrap_or_default())
            .collect()
    }

    #[test]
    fn mask_then_undo_restores_cells() {
        // 用例 1：mask 后 undo，cells 恢复为 mask 前的原始值。
        let (_dir, db) = setup_db();
        let sheet_id = setup_sheet_with_data(&db);
        let original = column_values(&db, sheet_id, 0);
        assert_eq!(original, vec!["张三".to_string(), "李四".to_string()]);

        let mask_op_id = simulate_mask(&db, sheet_id, 0);
        // mask 后值已脱敏（"张三"→"张*", "李四"→"李*"，通用脱敏保留首尾+中间*）
        let masked = column_values(&db, sheet_id, 0);
        assert_ne!(masked, original);
        assert!(masked.iter().all(|v| v.contains('*')));

        // undo：before 快照回写 → 恢复原始值。
        let restored = simulate_undo(&db, mask_op_id).unwrap();
        assert_eq!(restored, 2);
        let after_undo = column_values(&db, sheet_id, 0);
        assert_eq!(after_undo, original);
    }

    #[test]
    fn mask_then_redo_reapplies_mask() {
        // 用例 2：mask 后 undo 后 redo，cells 重新脱敏。
        let (_dir, db) = setup_db();
        let sheet_id = setup_sheet_with_data(&db);
        let original = column_values(&db, sheet_id, 0);

        let mask_op_id = simulate_mask(&db, sheet_id, 0);
        let masked = column_values(&db, sheet_id, 0);
        assert_ne!(masked, original);

        // undo 后恢复原始。
        simulate_undo(&db, mask_op_id).unwrap();
        assert_eq!(column_values(&db, sheet_id, 0), original);

        // redo：result 快照回写 → 重新脱敏。
        let restored = simulate_redo(&db, mask_op_id).unwrap();
        assert_eq!(restored, 2);
        assert_eq!(column_values(&db, sheet_id, 0), masked);
    }

    #[test]
    fn undo_non_undoable_op_returns_err() {
        // 用例 3：对 before_snapshot=None 的操作 undo 返回错误。
        let (_dir, db) = setup_db();
        let sheet_id = setup_sheet_with_data(&db);
        // 插入一个不可撤销操作（before_snapshot=None，如 import）。
        let non_undoable_id = db
            .log_operation(Some(sheet_id), "import", "{}", "{}")
            .unwrap();
        let err = simulate_undo(&db, non_undoable_id).unwrap_err();
        assert!(err.contains("不可撤销"));
    }

    #[test]
    fn list_undoable_operations_filters_kind() {
        // 用例 4：只返回 mask/replace_in_column/replace_all，不返回 undo/redo/import。
        let (_dir, db) = setup_db();
        let sheet_id = setup_sheet_with_data(&db);

        let mask_id = simulate_mask(&db, sheet_id, 0);
        // undo mask → 产生一条 undo 操作（不应出现在 list 中）。
        simulate_undo(&db, mask_id).unwrap();
        // 再插一条 import（不应出现）。
        db.log_operation(Some(sheet_id), "import", "{}", "{}")
            .unwrap();

        let rows: Vec<UndoableOpRow> = db.list_undoable_operations(sheet_id, 50).unwrap();
        assert!(rows.iter().all(|r| {
            matches!(
                r.kind.as_str(),
                "mask" | "replace_in_column" | "replace_all"
            )
        }));
        // 仅 mask 一条可撤销操作（undo/import 被过滤）。
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "mask");
    }

    #[test]
    fn list_undoable_operations_order_desc() {
        // 用例 5：按 created_at DESC 排序。
        let (_dir, db) = setup_db();
        let sheet_id = setup_sheet_with_data(&db);

        // 连续 3 次 mask（每次都覆盖同列值，但各自记录独立 op）。
        let id1 = simulate_mask(&db, sheet_id, 0);
        let id2 = simulate_mask(&db, sheet_id, 0);
        let id3 = simulate_mask(&db, sheet_id, 0);

        let rows = db.list_undoable_operations(sheet_id, 50).unwrap();
        assert_eq!(rows.len(), 3);
        // DESC：最新（id3）在最前。
        assert_eq!(rows[0].id, id3);
        assert_eq!(rows[1].id, id2);
        assert_eq!(rows[2].id, id1);
    }
}
