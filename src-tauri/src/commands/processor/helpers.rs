//! 处理器命令共享类型。
//!
//! v1.2.1 T8：从 `mask_ops.rs` + `extract_ops.rs` 收敛公共结果/配置结构体，
//! 消除跨文件重复定义。

use ruT0_data_kit_core::processor::rules::ExtractParams;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::commands::columns::ParseResult;
use crate::db::{Cell, DbManager};

// ---------------------------------------------------------------------------
// 脱敏结果
// ---------------------------------------------------------------------------

/// 脱敏结果（camelCase）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskResult {
    pub affected: u32,
}

// ---------------------------------------------------------------------------
// 提取 + 校验结果
// ---------------------------------------------------------------------------

/// 单条提取候选的校验结果（camelCase）。新 Tab 一行对应一条有效候选。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractValidateRow {
    /// 源 sheet 中的行号（1-based 数据行；表头不算）。
    pub source_row: u32,
    /// 类型标签（规则名去掉「提取」后缀，如「身份证号」「手机号」）。
    pub type_label: String,
    /// 正则提取出的候选原文。
    pub value: String,
    /// 是否通过 `validate_extracted` 校验。
    pub valid: bool,
    /// 校验说明（通过为空串；失败给原因，如「Luhn 校验未通过」）。
    pub note: String,
}

/// 单行单字段的校验失败原因（camelCase）。不写入 sheet（用户要求「保留原有字段，
/// 不新增列」），仅随 IPC 返回供前端 summary 消息展示。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowInvalidReason {
    /// 源 sheet 数据行号（1-based；表头 row_idx=0 不校验）。
    pub source_row: u32,
    /// 失败字段名（username/name/sex/birth/idcard/phone/address；跨字段失败用
    /// `sex`/`birth`，理由注明「与身份证号不一致」）。
    pub field: String,
    pub reason: String,
}

/// 双 Tab 校验结果（camelCase）。`valid_sheet` / `invalid_sheet` 各为一份
/// `ParseResult`，前端分别 `ADD_SHEET_FROM_PARSE` + `SET_SHEET_DATA` 落地。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TwoSheetResult {
    pub valid_sheet: ParseResult,
    pub invalid_sheet: ParseResult,
    pub invalid_reasons: Vec<RowInvalidReason>,
}

/// 单条校验规则的跨字段配置（仅 idcard 规则用，camelCase）。
///
/// 前端为 idcard-validate 规则条目附上此配置：
/// - `checkSex=true` + `sexColumn="sex"` → 比对性别列值与 idcard 第 17 位推断性别
/// - `checkBirth=true` + `birthColumn="birth"` → 比对出生日期列值与 idcard[6..14]
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CrossFieldConfig {
    /// 是否比对性别一致性。
    pub check_sex: bool,
    /// 性别列名（`checkSex=true` 时必填）。
    pub sex_column: Option<String>,
    /// 是否比对出生日期一致性。
    pub check_birth: bool,
    /// 出生日期列名（`checkBirth=true` 时必填）。
    pub birth_column: Option<String>,
}

/// 一条「列 + 规则」校验组合（camelCase）。T68。
///
/// 前端为源 sheet 的每个待校验列各发一条：`column` = 源列名，
/// `ruleId` 指向 DB `rules` 表（如 `username-validate` / `name-validate` /
/// `idcard-validate` 等）。`crossField` 仅在 `ruleId` 为 idcard-validate 时
/// 携带，用于触发跨字段联合校验。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiRuleValidation {
    /// 要校验的源列名。
    pub column: String,
    /// 校验规则 id（指向 DB `rules` 表）。
    pub rule_id: String,
    /// 跨字段配置（仅 idcard-validate 规则用）。
    #[serde(default)]
    pub cross_field: Option<CrossFieldConfig>,
    /// 参数覆盖（v1.1.4 续轮 T70）：允许前端为每行发送参数覆盖 DB 默认值。
    /// 目前主要服务于 `generic-validate` 规则（前端 UI 编辑字符类开关 +
    /// 长度范围后下发）。`None` = 用 DB `rule.params`。
    #[serde(default)]
    pub params_override: Option<ExtractParams>,
}

// ---------------------------------------------------------------------------
// 单行校验分发（extract_ops + mask_ops 共用）
// ---------------------------------------------------------------------------

/// 单值校验分发（v1.2.1 T8 提取共享 helper）。
///
/// 统一 `extract_ops::validate_multi_rules_to_two_sheets_inner` 与
/// `mask_ops::mask_column` 的单行校验逻辑：
/// - `phone-validate` → `is_valid_phone`（整串 11 位 + 纯数字 + 前缀白名单）
/// - `rule.params` 或 `params_override` 非空 → `validate_extracted_with_params`
///   （override 优先于 DB rule.params）
/// - `rule.pattern` 非空 → 正则 `is_match`
/// - 其他 → 默认通过
///
/// 返回 `(passed, message)`：通过时 message 为空串；失败时 message 含原因。
pub fn validate_single(
    rule: &ruT0_data_kit_core::processor::Rule,
    re: Option<&regex::Regex>,
    value: &str,
    phone_prefixes: &[String],
    params_override: Option<&ExtractParams>,
) -> (bool, String) {
    if rule.id == "phone-validate" {
        let ok = ruT0_data_kit_core::processor::validators::is_valid_phone(value, phone_prefixes);
        (
            ok,
            if ok {
                String::new()
            } else {
                "手机号须为 11 位纯数字".into()
            },
        )
    } else {
        let effective_params = params_override.or(rule.params.as_ref());
        if let Some(params) = effective_params {
            ruT0_data_kit_core::processor::validators::validate_extracted_with_params(
                params, value,
            )
        } else if let Some(re) = re {
            let ok = re.is_match(value);
            (
                ok,
                if ok {
                    String::new()
                } else {
                    format!("值不匹配规则 {}", rule.name)
                },
            )
        } else {
            (true, String::new())
        }
    }
}

/// 按 row_idx 分组 cells，组内按 col_idx 排序（v1.2.1 T8 提取共享 helper）。
///
/// 用于 `validate_multi_rules_to_two_sheets_inner` 的全量数据加载后分组。
/// 返回 `BTreeMap<row_idx, Vec<(col_idx, value)>>`，行号有序便于逐行遍历。
pub fn group_cells_by_row(
    cells: Vec<Cell>,
) -> std::collections::BTreeMap<u32, Vec<(u32, Option<String>)>> {
    let mut grouped: std::collections::BTreeMap<u32, Vec<(u32, Option<String>)>> =
        std::collections::BTreeMap::new();
    for c in cells {
        grouped
            .entry(c.row_idx)
            .or_default()
            .push((c.col_idx, c.value));
    }
    grouped
}

/// 分页加载 sheet 全部数据 cells（v1.2.1 T8 提取共享 helper）。
///
/// `query_cells` 已排除 `row_idx=0` 表头行。按 500 行/页分页加载全部数据行。
pub fn query_all_data_cells(db: &crate::db::DbManager, sheet_id: i64) -> Result<Vec<Cell>, String> {
    let total_rows = db.count_rows(sheet_id).map_err(|e| e.to_string())?;
    let page_size: u32 = 500;
    let pages = total_rows.div_ceil(page_size).max(1);
    let mut all_cells: Vec<Cell> = Vec::new();
    for p in 1..=pages {
        let cells = db
            .query_cells(sheet_id, p, page_size)
            .map_err(|e| e.to_string())?;
        all_cells.extend(cells);
    }
    Ok(all_cells)
}

/// 读取 sheet 表头（row_idx=0），按 col_idx 排序（v1.2.1 T8 提取共享 helper）。
pub fn read_headers(db: &crate::db::DbManager, sheet_id: i64) -> Result<Vec<String>, String> {
    let header_cells = db.query_row_cells(sheet_id, 0).map_err(|e| e.to_string())?;
    let mut header_pairs: Vec<(u32, String)> = header_cells
        .iter()
        .map(|c| (c.col_idx, c.value.clone().unwrap_or_default()))
        .collect();
    header_pairs.sort_by_key(|(col, _)| *col);
    Ok(header_pairs.into_iter().map(|(_, v)| v).collect())
}

/// 双 sheet 写出结果（v1.2.1 T8 提取共享 helper）。
///
/// 向 `sheet_id` 写入表头 + 数据行，返回写入的数据行数。
/// `headers` 为表头列表，`rows` 为每行的列值（`None` = 空单元格）。
pub fn write_sheet(
    db: &crate::db::DbManager,
    sheet_id: i64,
    headers: &[String],
    rows: &[Vec<Option<String>>],
) -> Result<u32, String> {
    let col_count = headers.len().max(1);
    let mut cells: Vec<Cell> = Vec::with_capacity((rows.len() + 1) * col_count);
    // 表头（row_idx=0）
    for (col, header) in headers.iter().enumerate() {
        cells.push(Cell {
            sheet_id,
            row_idx: 0,
            col_idx: col as u32,
            value: Some(header.clone()),
        });
    }
    // 数据行（从 row_idx=1 起）
    for (r, row) in rows.iter().enumerate() {
        for (c, val) in row.iter().enumerate() {
            cells.push(Cell {
                sheet_id,
                row_idx: (r + 1) as u32,
                col_idx: c as u32,
                value: val.clone(),
            });
        }
    }
    if !cells.is_empty() {
        db.write_cells(sheet_id, &cells).map_err(|e| e.to_string())?;
    }
    Ok(rows.len() as u32)
}

/// 记录列级操作日志（统一 `log_operation_with_snapshot` ceremony）。
///
/// `kind` 为操作类型（`replace_in_column` / `base64_column` / `hash_column` /
/// `transform_column` / `mask` / `replace_all`），`column` 为列名/列号，
/// `payload` 为操作参数 JSON（`column`/`from`/`to`/`mode` 等），`before`/`after`
/// 为撤销快照。返回日志 id。
pub fn log_column_op(
    db: &DbManager,
    sheet_id: i64,
    kind: &str,
    column: &str,
    payload: serde_json::Value,
    before: Option<&str>,
    after: &str,
) -> Result<i64, String> {
    let mut payload = payload;
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("column".into(), json!(column));
    }
    db.log_operation_with_snapshot(Some(sheet_id), kind, &payload.to_string(), before, after)
        .map_err(|e| e.to_string())
}
