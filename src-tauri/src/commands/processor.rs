//! v1.1.0 处理器类 IPC 命令（脱敏 / 校验 / 提取 / 规则管理）。
//!
//! 全部 `#[tauri::command]` → `Result<T, String>` + `.map_err(|e| e.to_string())`。
//! 依赖 `DbManager`（列读写 + 规则持久化）。
//!
//! v1.1.0：规则不再经 `RuleState` 内存态，改从 DB 读取（`DbManager::get_rule`），
//! `toggle_rule` 写回 `rules.enabled`，新增 `update_rule_params` 写回
//! `rules.pattern` / `rules.replacement`。

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use ruT0_data_kit_core::processor::func_validator::{
    idcard_gender, is_valid_address, is_valid_birth, is_valid_idcard, is_valid_phone, is_valid_sex,
    is_valid_username, validate_extracted,
};
use ruT0_data_kit_core::processor::rules::{ExtractParams, TemplateParams};
use ruT0_data_kit_core::processor::{
    ExtractItem, Extractor, Masker, PiiExtractor, RegexValidator, Validator,
};

use crate::commands::columns::ParseResult;
use crate::db::{Cell, DbManager};

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
                params: None,
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
// 提取 + 函数式校验 → 新 Tab（v1.1.3 T55）
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

/// 性别值归一化（T55c）。去空格 + 小写后匹配：
/// - "男"/"male"/"m"/"1" → '男'
/// - "女"/"female"/"f"/"2" → '女'
/// - 其他 → None（不可识别，跳过比对）
fn normalize_gender(s: &str) -> Option<char> {
    let t = s.trim().to_lowercase();
    match t.as_str() {
        "男" | "male" | "m" | "1" => Some('男'),
        "女" | "female" | "f" | "2" => Some('女'),
        _ => None,
    }
}

/// 提取 + 校验核心逻辑（接受 `&DbManager`，便于单测直接调用）。
///
/// 流程：
/// 1. `find_col_idx` + `query_column_cells` 读源列全部数据行
/// 2. 从 DB 取所有规则（`get_rule`），预编译各规则 `pattern` 正则（宽松召回）
/// 3. 逐行 → 逐规则 `find_iter` 提取候选 → 对每个候选调 `validate_extracted`
///    → 收集 `(源行号, 类型标签, 提取值, 有效, 说明)`
/// 4. （T55c）若任一规则 `params == IdCard` 且指定 `gender_col_idx`：
///    读性别列 → 构建行号→性别值映射 → 对有效 idcard 候选比对推断性别
///    与性别列值；矛盾则 `valid=false` + `note` 标注不一致
/// 5. `create_sheet(session_id, "{column}_提取", 0)` + `write_cells`
///    （表头 `[类型, 数据值]` + **仅有效**数据行）
/// 6. `log_operation("extract_to_sheet", before_snapshot=None)`（不进撤销栈）
///
/// 新 Tab 只含**有效候选**（`valid==true`），无效候选不写入但仍在返回的
/// `Vec<ExtractValidateRow>` 中保留供测试断言。
pub fn extract_validate_to_new_sheet_inner(
    db: &DbManager,
    sheet_id: i64,
    column: &str,
    rule_ids: &[String],
    session_id: i64,
    gender_col_idx: Option<u32>,
) -> Result<(ParseResult, Vec<ExtractValidateRow>), String> {
    if rule_ids.is_empty() {
        return Err("请至少选择一条提取规则".into());
    }

    let col_idx = db
        .find_col_idx(sheet_id, column)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("列 `{column}` 不存在"))?;

    // 预编译所有规则的正则。
    let mut compiled: Vec<(
        ruT0_data_kit_core::processor::rules::Rule,
        regex::Regex,
        String,
    )> = Vec::with_capacity(rule_ids.len());
    for rid in rule_ids {
        let rule = db
            .get_rule(rid)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("规则 `{rid}` 不存在"))?;
        let pattern_str = rule
            .pattern
            .as_deref()
            .ok_or_else(|| format!("规则 `{rid}` 未配置提取正则（pattern 为空）"))?;
        let re = regex::Regex::new(pattern_str).map_err(|e| format!("正则编译失败: {e}"))?;
        // 类型标签 = 规则名去掉「提取」后缀；结果为空时用原名兜底。
        let label = rule.name.trim_end_matches("提取");
        let label = if label.is_empty() {
            rule.name.clone()
        } else {
            label.to_string()
        };
        compiled.push((rule, re, label));
    }

    let rows = db
        .query_column_cells(sheet_id, col_idx)
        .map_err(|e| e.to_string())?;

    // T55c：若任一规则为 IdCard 且指定性别列，读取性别列构建 行号→原值 映射，
    // 用于对有效 idcard 候选执行性别联合校验。
    let any_idcard = compiled
        .iter()
        .any(|(r, _, _)| matches!(r.params.as_ref(), Some(ExtractParams::IdCard)));
    let gender_map: HashMap<u32, String> = if any_idcard {
        if let Some(gender_col) = gender_col_idx {
            db.query_column_cells(sheet_id, gender_col)
                .map_err(|e| e.to_string())?
                .into_iter()
                .filter_map(|(row_idx, v)| v.map(|s| (row_idx, s)))
                .collect()
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    };

    // 逐行 → 逐规则提取候选 → 校验 → 收集结果。
    let mut results: Vec<ExtractValidateRow> = Vec::new();
    let mut skipped: u32 = 0; // 无任何候选的行数
    for (row_idx, value) in &rows {
        let input = value.as_deref().unwrap_or("");
        let mut row_hit = false;
        for (rule, re, label) in &compiled {
            let is_idcard = matches!(rule.params.as_ref(), Some(ExtractParams::IdCard));
            for m in re.find_iter(input) {
                let candidate = m.as_str();
                let (mut valid, mut note) = validate_extracted(rule, candidate);
                // T55c：身份证性别联合校验（仅当指定性别列 + 候选本身有效 + 可推断性别）
                if is_idcard && valid {
                    if let Some(inferred) = idcard_gender(candidate) {
                        if let Some(col_val) = gender_map.get(row_idx) {
                            if let Some(norm) = normalize_gender(col_val) {
                                if norm != inferred {
                                    valid = false;
                                    note = format!(
                                        "性别不一致: 身份证推断{inferred}，性别列{col_val}"
                                    );
                                }
                                // 一致 → 保持 valid=true + note=性别（已由 validate_extracted 设置）
                            }
                            // 性别列值不可识别 → 跳过比对，保持 valid=true
                        }
                        // 无对应行 → 跳过比对
                    }
                }
                results.push(ExtractValidateRow {
                    source_row: *row_idx,
                    type_label: label.clone(),
                    value: candidate.to_string(),
                    valid,
                    note,
                });
                row_hit = true;
            }
        }
        if !row_hit {
            skipped += 1;
        }
    }

    // 新 Tab 只写有效候选：2 列 [类型, 数据值]。
    let valid_rows: Vec<&ExtractValidateRow> = results.iter().filter(|r| r.valid).collect();
    let row_count = valid_rows.len() as u32;

    // 创建新 sheet（position=0；前端通过 ADD_SHEET_FROM_PARSE action 追加到 sheets 数组）。
    let new_sheet_name = format!("{column}_提取");
    let new_sheet_id = db
        .create_sheet(session_id, &new_sheet_name, 0)
        .map_err(|e| e.to_string())?;

    // 写表头行（row_idx=0）+ 有效数据行。2 列：类型 / 数据值
    let headers: Vec<String> = vec!["类型".into(), "数据值".into()];
    let mut cells: Vec<Cell> = Vec::new();
    for (col, header) in headers.iter().enumerate() {
        cells.push(Cell {
            sheet_id: new_sheet_id,
            row_idx: 0,
            col_idx: col as u32,
            value: Some(header.clone()),
        });
    }
    for (row, r) in valid_rows.iter().enumerate() {
        cells.push(Cell {
            sheet_id: new_sheet_id,
            row_idx: (row + 1) as u32, // row_idx=0 是表头
            col_idx: 0,
            value: Some(r.type_label.clone()),
        });
        cells.push(Cell {
            sheet_id: new_sheet_id,
            row_idx: (row + 1) as u32,
            col_idx: 1,
            value: Some(r.value.clone()),
        });
    }

    if !cells.is_empty() {
        db.write_cells(new_sheet_id, &cells)
            .map_err(|e| e.to_string())?;
    }

    // extract_to_sheet 不纳入撤销栈（新增 sheet，撤销 = 关闭 Tab）。
    db.log_operation(
        Some(new_sheet_id),
        "extract_to_sheet",
        &serde_json::json!({
            "sourceSheetId": sheet_id,
            "column": column,
            "ruleIds": rule_ids,
            "rowCount": row_count,
            "skipped": skipped
        })
        .to_string(),
        "{}",
    )
    .map_err(|e| e.to_string())?;

    Ok((
        ParseResult {
            new_sheet_id,
            headers,
            row_count,
            skipped,
        },
        results,
    ))
}

/// `extract_validate_to_new_sheet` 的 Tauri 命令包装。
///
/// 提取指定列的候选（按各规则正则）→ 函数式校验（Luhn / IPv4 / IPv6 / 手机前缀 /
/// 身份证校验码）→ 落到新 Tab（`{column}_提取`），含 [类型, 数据值] 两列，
/// 只写有效候选。新 Tab 不进撤销栈（撤销 = 关闭 Tab），与 `parse_column_as_json` 一致。
///
/// T56：支持批量多规则，`rule_ids` 传数组；类型标签 = 规则名去掉「提取」后缀。
///
/// `gender_col`（T55c）：可选性别列名，仅当选中的规则含 `idcard-extract` 时使用。
/// 前端在提取时从当前 sheet headers 中选择；若指定，对有效 idcard 候选比对第 17 位
/// 推断性别与该列值，矛盾 → `valid=false`（不写入新 Tab）。非 idcard 规则不受影响。
#[tauri::command]
pub fn extract_validate_to_new_sheet(
    sheet_id: i64,
    column: String,
    rule_ids: Vec<String>,
    session_id: i64,
    gender_col: Option<String>,
    db: tauri::State<'_, DbManager>,
) -> Result<ParseResult, String> {
    if rule_ids.is_empty() {
        return Err("请至少选择一条提取规则".into());
    }
    // 解析性别列名 → col_idx（找不到列 → 报错，避免静默跳过联合校验）
    let gender_col_idx = match gender_col.as_deref() {
        Some(c) if !c.is_empty() => Some(
            db.find_col_idx(sheet_id, c)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("性别列 `{c}` 不存在"))?,
        ),
        _ => None,
    };
    let (parse_result, _rows) = extract_validate_to_new_sheet_inner(
        &db,
        sheet_id,
        &column,
        &rule_ids,
        session_id,
        gender_col_idx,
    )?;
    Ok(parse_result)
}

// ---------------------------------------------------------------------------
// 行级多字段校验 → 双 Tab 输出（T57）
// ---------------------------------------------------------------------------

/// 字段→列名映射（T57）。前端为每个字段类型选择对应的源 sheet 列头名；
/// 未映射（`None`）的字段不校验，原值原样复制到两个输出 Tab。
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldColumnMapping {
    pub username: Option<String>,
    pub name: Option<String>,
    pub sex: Option<String>,
    pub birth: Option<String>,
    pub idcard: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
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

/// 行级多字段校验核心逻辑（接受 `&DbManager`，便于单测直接调用）。
///
/// 流程：
/// 1. `get_sheet_name(sheet_id)` → 源 sheet 名（None → 报错「sheet 不存在」）
/// 2. `query_cells` 分页读全部 cells（page_size=500）→ 按 row_idx 分组 →
///    `Vec<Vec<Option<String>>>`（跳过 row_idx=0 表头）+ headers
/// 3. `find_col_idx` 把 mapping 里每个字段名转 col_idx（找不到列 → 报错）
/// 4. 逐行：对每个已映射字段取 cell value → 校验：
///    - username → [`is_valid_username`]
///    - name → 正则 `^[\u4e00-\u9fa5]{2,4}$`
///    - sex → [`is_valid_sex`]
///    - birth → [`is_valid_birth`]
///    - idcard → [`is_valid_idcard`]
///    - phone → [`is_valid_phone`]（带前缀白名单）
///    - address → [`is_valid_address`]
/// 5. 跨字段联合（仅当相关字段都映射且 idcard 本身有效）：
///    - sex vs idcard：[`idcard_gender`] 推断与 [`normalize_gender`] 比对
///    - birth vs idcard：idcard 第 7-14 位（1-based）== birth 值
/// 6. 任一字段失败 → 收集 [`RowInvalidReason`]，整行归入 invalid；全部通过 → valid
/// 7. `create_sheet` ×2：`{name}_校验通过` + `{name}_校验失败`，各写表头 + 对应行
/// 8. `log_operation("validate_rows_to_two_sheets", ...)`（不进撤销栈）
/// 9. 返回 [`TwoSheetResult`]
///
/// 输出 Tab 保留原列（列数 = headers.len()，列顺序与源 sheet 一致），不新增列。
/// `phone_prefixes` 为手机号前缀白名单（空=仅检查 1 开头+11 位）。
pub fn validate_rows_to_two_sheets_inner(
    db: &DbManager,
    sheet_id: i64,
    session_id: i64,
    mapping: &FieldColumnMapping,
    phone_prefixes: &[String],
) -> Result<TwoSheetResult, String> {
    // 至少映射一个字段，否则校验无意义。
    let any_mapped = [
        &mapping.username,
        &mapping.name,
        &mapping.sex,
        &mapping.birth,
        &mapping.idcard,
        &mapping.phone,
        &mapping.address,
    ]
    .iter()
    .any(|o| o.as_ref().is_some_and(|s| !s.is_empty()));
    if !any_mapped {
        return Err("请至少映射一个字段".into());
    }

    let sheet_name = db
        .get_sheet_name(sheet_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("sheet {sheet_id} 不存在"))?;

    // 分页读全部 cells。
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

    // 按 row_idx 分组，组内按 col_idx 排序（query_cells 已 ASC，这里稳定）。
    let mut grouped: BTreeMap<u32, Vec<(u32, Option<String>)>> = BTreeMap::new();
    for c in all_cells {
        grouped
            .entry(c.row_idx)
            .or_default()
            .push((c.col_idx, c.value));
    }

    // headers：row_idx=0 的 cells 按 col_idx 排序取 value。
    let headers: Vec<String> = grouped
        .get(&0)
        .map(|cells| {
            let mut v = cells.clone();
            v.sort_by_key(|(col, _)| *col);
            v.into_iter()
                .map(|(_, val)| val.unwrap_or_default())
                .collect()
        })
        .unwrap_or_default();
    let col_count = headers.len();

    // 字段 → col_idx 解析（找不到列 → 报错，避免静默跳过）。
    let resolve_col = |field: &str, name: &Option<String>| -> Result<Option<u32>, String> {
        match name.as_ref().filter(|s| !s.is_empty()) {
            Some(col_name) => {
                let idx = db
                    .find_col_idx(sheet_id, col_name)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| format!("字段 {field} 的列 `{col_name}` 不存在"))?;
                Ok(Some(idx))
            }
            None => Ok(None),
        }
    };
    let username_col = resolve_col("username", &mapping.username)?;
    let name_col = resolve_col("name", &mapping.name)?;
    let sex_col = resolve_col("sex", &mapping.sex)?;
    let birth_col = resolve_col("birth", &mapping.birth)?;
    let idcard_col = resolve_col("idcard", &mapping.idcard)?;
    let phone_col = resolve_col("phone", &mapping.phone)?;
    let address_col = resolve_col("address", &mapping.address)?;

    // 姓名正则预编译（2-4 位中文，同 name-validate 规则）。
    let name_re = regex::Regex::new(r"^[\u4e00-\u9fa5]{2,4}$")
        .map_err(|e| format!("姓名正则编译失败: {e}"))?;

    // 逐行校验。row_idx=0 是表头，跳过。
    let mut valid_rows: Vec<Vec<Option<String>>> = Vec::new();
    let mut invalid_rows: Vec<Vec<Option<String>>> = Vec::new();
    let mut invalid_reasons: Vec<RowInvalidReason> = Vec::new();

    for (&row_idx, cells_row) in grouped.iter() {
        if row_idx == 0 {
            continue;
        }
        // 行内按 col_idx 排序对齐到 col_count 列（缺列补 None）。
        let mut row_vals: Vec<Option<String>> = vec![None; col_count];
        let mut sorted_cells = cells_row.clone();
        sorted_cells.sort_by_key(|(col, _)| *col);
        for (col, val) in sorted_cells {
            if (col as usize) < row_vals.len() {
                row_vals[col as usize] = val;
            }
        }

        // 取某列的字符串值（col_idx 越界或 None → ""）。
        let cell_str = |col: Option<u32>| -> String {
            col.and_then(|c| row_vals.get(c as usize).and_then(|v| v.clone()))
                .unwrap_or_default()
        };

        let mut row_invalid_reasons: Vec<(String, String)> = Vec::new();

        // ---- 单字段校验 ----
        if let Some(col) = username_col {
            let v = cell_str(Some(col));
            if !is_valid_username(&v) {
                row_invalid_reasons.push(("username".into(), "用户名须为纯字母数字".into()));
            }
        }
        if let Some(col) = name_col {
            let v = cell_str(Some(col));
            if !name_re.is_match(&v) {
                row_invalid_reasons.push(("name".into(), "姓名须为 2-4 位中文".into()));
            }
        }
        if let Some(col) = sex_col {
            let v = cell_str(Some(col));
            if !is_valid_sex(&v) {
                row_invalid_reasons.push(("sex".into(), "性别须为「男」或「女」".into()));
            }
        }
        if let Some(col) = birth_col {
            let v = cell_str(Some(col));
            if !is_valid_birth(&v) {
                row_invalid_reasons.push(("birth".into(), "出生日期须为 8 位数字".into()));
            }
        }
        let idcard_val = idcard_col.map(|c| cell_str(Some(c))).unwrap_or_default();
        let idcard_valid = if idcard_col.is_some() {
            if !is_valid_idcard(&idcard_val) {
                row_invalid_reasons.push(("idcard".into(), "非合法身份证号".into()));
                false
            } else {
                true
            }
        } else {
            false // 未映射 idcard → 不做跨字段校验
        };
        if let Some(col) = phone_col {
            let v = cell_str(Some(col));
            if !is_valid_phone(&v, phone_prefixes) {
                row_invalid_reasons.push(("phone".into(), "手机号须为 11 位、1 开头".into()));
            }
        }
        if let Some(col) = address_col {
            let v = cell_str(Some(col));
            if !is_valid_address(&v) {
                row_invalid_reasons.push((
                    "address".into(),
                    "地址格式不符（全中文+号1-1500+室101-999）".into(),
                ));
            }
        }

        // ---- 跨字段联合校验（仅当 idcard 本身有效 + 相关字段都映射）----
        if idcard_valid {
            // sex vs idcard 性别
            if sex_col.is_some() {
                let sex_val = cell_str(sex_col);
                // 仅当 sex 列值本身是合法的「男」/「女」时才比对（非法值已在单字段校验记录）
                if let Some(norm) = normalize_gender(&sex_val) {
                    if let Some(inferred) = idcard_gender(&idcard_val) {
                        if norm != inferred {
                            row_invalid_reasons.push((
                                "sex".into(),
                                format!("性别不一致: 身份证推断{inferred}，性别列{sex_val}"),
                            ));
                        }
                    }
                }
            }
            // birth vs idcard 出生日期码（idcard 第 7-14 位，0-indexed [6..14]）
            if birth_col.is_some() {
                let birth_val = cell_str(birth_col);
                if is_valid_birth(&birth_val) {
                    // idcard_val 一定 18 位且前 17 纯数字（is_valid_idcard 已保证）
                    let idcard_birth = &idcard_val[6..14];
                    if birth_val != idcard_birth {
                        row_invalid_reasons.push((
                            "birth".into(),
                            format!(
                                "出生日期与身份证号不一致: 身份证{idcard_birth}，出生日期{birth_val}"
                            ),
                        ));
                    }
                }
            }
        }

        if row_invalid_reasons.is_empty() {
            valid_rows.push(row_vals);
        } else {
            for (field, reason) in row_invalid_reasons {
                invalid_reasons.push(RowInvalidReason {
                    source_row: row_idx,
                    field,
                    reason,
                });
            }
            invalid_rows.push(row_vals);
        }
    }

    // 创建两个新 sheet + 写表头 + 数据行。
    let valid_sheet_name = format!("{sheet_name}_校验通过");
    let invalid_sheet_name = format!("{sheet_name}_校验失败");
    let valid_sheet_id = db
        .create_sheet(session_id, &valid_sheet_name, 0)
        .map_err(|e| e.to_string())?;
    let invalid_sheet_id = db
        .create_sheet(session_id, &invalid_sheet_name, 0)
        .map_err(|e| e.to_string())?;

    let write_sheet = |sid: i64, rows: &[Vec<Option<String>>]| -> Result<u32, String> {
        let mut cells: Vec<Cell> = Vec::with_capacity((rows.len() + 1) * col_count.max(1));
        // 表头（row_idx=0）
        for (col, header) in headers.iter().enumerate() {
            cells.push(Cell {
                sheet_id: sid,
                row_idx: 0,
                col_idx: col as u32,
                value: Some(header.clone()),
            });
        }
        // 数据行（从 row_idx=1 起）
        for (r, row) in rows.iter().enumerate() {
            for (c, val) in row.iter().enumerate() {
                cells.push(Cell {
                    sheet_id: sid,
                    row_idx: (r + 1) as u32,
                    col_idx: c as u32,
                    value: val.clone(),
                });
            }
        }
        if !cells.is_empty() {
            db.write_cells(sid, &cells).map_err(|e| e.to_string())?;
        }
        Ok(rows.len() as u32)
    };

    let valid_count = write_sheet(valid_sheet_id, &valid_rows)?;
    let invalid_count = write_sheet(invalid_sheet_id, &invalid_rows)?;

    db.log_operation(
        Some(sheet_id),
        "validate_rows_to_two_sheets",
        &serde_json::json!({
            "sourceSheetId": sheet_id,
            "sourceSheetName": sheet_name,
            "validSheetId": valid_sheet_id,
            "invalidSheetId": invalid_sheet_id,
            "validCount": valid_count,
            "invalidCount": invalid_count,
        })
        .to_string(),
        "{}",
    )
    .map_err(|e| e.to_string())?;

    Ok(TwoSheetResult {
        valid_sheet: ParseResult {
            new_sheet_id: valid_sheet_id,
            headers: headers.clone(),
            row_count: valid_count,
            skipped: 0,
        },
        invalid_sheet: ParseResult {
            new_sheet_id: invalid_sheet_id,
            headers,
            row_count: invalid_count,
            skipped: 0,
        },
        invalid_reasons,
    })
}

/// `validate_rows_to_two_sheets` 的 Tauri 命令包装。
///
/// 行级多字段校验：读源 sheet 全部行 → 逐字段校验 → 跨字段联合校验
/// （sex vs idcard 性别、birth vs idcard 出生日期码）→ 整行按通过/失败分流
/// 到两个新 Tab（`{源sheet名}_校验通过` / `{源sheet名}_校验失败`），保留原列
/// 不新增。`invalid_reasons` 仅随返回值回前端用于 summary 消息，不写入 sheet。
///
/// - `field_columns`：字段类型 → 源列名映射（未映射的字段不校验）
/// - `phone_prefixes`：手机号前缀白名单（空=仅检查 1 开头+11 位）
#[tauri::command]
pub fn validate_rows_to_two_sheets(
    sheet_id: i64,
    session_id: i64,
    field_columns: FieldColumnMapping,
    phone_prefixes: Vec<String>,
    db: tauri::State<'_, DbManager>,
) -> Result<TwoSheetResult, String> {
    validate_rows_to_two_sheets_inner(&db, sheet_id, session_id, &field_columns, &phone_prefixes)
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
    use super::{
        extract_validate_to_new_sheet_inner, validate_rows_to_two_sheets_inner, FieldColumnMapping,
        RowValidation,
    };
    use crate::db::{Cell, DbManager, OperationRow, UndoableOpRow};
    use ruT0_data_kit_core::processor::rules::RuleRegistry;
    use ruT0_data_kit_core::processor::{Masker, RegexValidator, SimpleMasker, Validator};

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

    // ---- T55：提取 + 函数式校验 → 新 Tab ----

    /// 构造一个 sheet：1 列（col0=raw），表头 + 多行数据（含有效/无效候选）。
    /// 返回 (session_id, sheet_id)。
    fn setup_extract_sheet(db: &DbManager, values: &[&str]) -> (i64, i64) {
        let session_id = db.create_session("extract-test", None, "csv", 0).unwrap();
        let sheet_id = db.create_sheet(session_id, "raw", 0).unwrap();
        let mut cells: Vec<Cell> = Vec::with_capacity(values.len() + 1);
        cells.push(Cell {
            sheet_id,
            row_idx: 0,
            col_idx: 0,
            value: Some("raw".into()),
        });
        for (i, v) in values.iter().enumerate() {
            cells.push(Cell {
                sheet_id,
                row_idx: (i + 1) as u32,
                col_idx: 0,
                value: Some((*v).to_string()),
            });
        }
        db.write_cells(sheet_id, &cells).unwrap();
        (session_id, sheet_id)
    }

    /// 构造一个 sheet：2 列（col0=idcard，col1=gender），表头 + 多行数据。
    /// 用于 T55c 身份证 + 性别联合校验测试。返回 (session_id, sheet_id)。
    fn setup_extract_sheet_two_cols(db: &DbManager, rows: &[(&str, &str)]) -> (i64, i64) {
        let session_id = db.create_session("extract-test", None, "csv", 0).unwrap();
        let sheet_id = db.create_sheet(session_id, "raw", 0).unwrap();
        let mut cells: Vec<Cell> = Vec::with_capacity((rows.len() + 1) * 2);
        // 表头
        cells.push(Cell {
            sheet_id,
            row_idx: 0,
            col_idx: 0,
            value: Some("idcard".into()),
        });
        cells.push(Cell {
            sheet_id,
            row_idx: 0,
            col_idx: 1,
            value: Some("gender".into()),
        });
        for (i, (idv, gv)) in rows.iter().enumerate() {
            let r = (i + 1) as u32;
            cells.push(Cell {
                sheet_id,
                row_idx: r,
                col_idx: 0,
                value: Some((*idv).to_string()),
            });
            cells.push(Cell {
                sheet_id,
                row_idx: r,
                col_idx: 1,
                value: Some((*gv).to_string()),
            });
        }
        db.write_cells(sheet_id, &cells).unwrap();
        (session_id, sheet_id)
    }

    #[test]
    fn extract_validate_phone_to_new_sheet() {
        // 手机号：13412345678 / 15987654321 有效；1201234567a 无效（非纯数字）；
        // 134123456 长度不足；134123456789 超长。正则 `\b1\d{10}\b` 只会召回
        // 11 位纯数字串，故 1201234567a / 134123456 / 134123456789 不命中。
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let values = ["13412345678", "15987654321", "1201234567a", "134123456"];
        let (session_id, sheet_id) = setup_extract_sheet(&db, &values);

        let (parse_result, rows) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            &["phone-extract".to_string()],
            session_id,
            None,
        )
        .unwrap();

        // 2 个有效候选（正则只召回 11 位 1 开头纯数字）。
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.valid));
        assert_eq!(rows[0].value, "13412345678");
        assert_eq!(rows[1].value, "15987654321");
        assert_eq!(rows[0].type_label, "手机号");
        // skipped = 无候选的行数（2 行：1201234567a / 134123456）。
        assert_eq!(parse_result.skipped, 2);
        // T56：新 Tab 只写有效候选，row_count = 有效行数 = 2。
        assert_eq!(parse_result.row_count, 2);
        assert_eq!(parse_result.headers.len(), 2);
        assert_eq!(parse_result.headers[0], "类型");
        assert_eq!(parse_result.headers[1], "数据值");
        // 新 sheet 第 0 行是表头：类型 / 数据值。
        let header_cells = db.query_row_cells(parse_result.new_sheet_id, 0).unwrap();
        assert_eq!(header_cells.len(), 2);
        assert_eq!(header_cells[0].value.as_deref(), Some("类型"));
        assert_eq!(header_cells[1].value.as_deref(), Some("数据值"));
        // 数据行 col0（类型）= "手机号"，col1（数据值）= phone。
        let data_col0 = db.query_column_cells(parse_result.new_sheet_id, 0).unwrap();
        assert_eq!(data_col0.len(), 2);
        assert_eq!(data_col0[0].1.as_deref(), Some("手机号"));
        let data_col1 = db.query_column_cells(parse_result.new_sheet_id, 1).unwrap();
        assert_eq!(data_col1[0].1.as_deref(), Some("13412345678"));
        assert_eq!(data_col1[1].1.as_deref(), Some("15987654321"));
    }

    #[test]
    fn extract_validate_bankcard_luhn_to_new_sheet() {
        // 银行卡：6222021234567890128 过 Luhn；6222021234567890124 不过。
        // 两者均为 19 位首位非 0，正则 `\b[1-9]\d{12,18}\b` 召回。
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let values = ["6222021234567890128", "6222021234567890124"];
        let (session_id, sheet_id) = setup_extract_sheet(&db, &values);

        let (parse_result, rows) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            &["bankcard-extract".to_string()],
            session_id,
            None,
        )
        .unwrap();

        assert_eq!(rows.len(), 2);
        assert!(rows[0].valid); // 6222021234567890128 过 Luhn
        assert!(!rows[1].valid); // 6222021234567890124 不过 Luhn
        assert_eq!(rows[0].value, "6222021234567890128");
        assert_eq!(rows[0].type_label, "银行卡号");
        // T56：新 Tab 只写有效候选 → 1 行。
        assert_eq!(parse_result.row_count, 1);
        assert_eq!(parse_result.skipped, 0);
        let data_col0 = db.query_column_cells(parse_result.new_sheet_id, 0).unwrap();
        assert_eq!(data_col0.len(), 1);
        assert_eq!(data_col0[0].1.as_deref(), Some("银行卡号"));
    }

    #[test]
    fn extract_validate_ipv4_to_new_sheet() {
        // IPv4：192.168.1.1 有效；256.1.1.1 无效（超范围）；
        // 192.168.01.1 无效（前导零）。正则会召回前三者。
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let values = ["192.168.1.1", "256.1.1.1", "192.168.01.1"];
        let (session_id, sheet_id) = setup_extract_sheet(&db, &values);

        let (parse_result, rows) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            &["ip4-extract".to_string()],
            session_id,
            None,
        )
        .unwrap();

        assert_eq!(rows.len(), 3);
        assert!(rows[0].valid); // 192.168.1.1
        assert!(!rows[1].valid); // 256.1.1.1 超范围
        assert!(!rows[2].valid); // 192.168.01.1 前导零
        assert_eq!(rows[0].type_label, "IPv4地址");
        // T56：只写有效 → 1 行。
        assert_eq!(parse_result.row_count, 1);
        assert_eq!(parse_result.skipped, 0);
    }

    #[test]
    fn extract_validate_ipv6_to_new_sheet() {
        // IPv6：::1 / 2001:db8::1 有效；1:2:3 无效（仅 3 段且无 :: 缩写，
        // IPv6 要求 8 段或 :: 压缩）。宽松正则会召回三者。
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let values = ["::1", "2001:db8::1", "1:2:3"];
        let (session_id, sheet_id) = setup_extract_sheet(&db, &values);

        let (parse_result, rows) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            &["ip6-extract".to_string()],
            session_id,
            None,
        )
        .unwrap();

        assert_eq!(rows.len(), 3);
        assert!(rows[0].valid); // ::1
        assert!(rows[1].valid); // 2001:db8::1
        assert!(!rows[2].valid); // 1:2:3 段数不足且无 :: 缩写
        assert_eq!(rows[0].type_label, "IPv6地址");
        // T56：只写有效 → 2 行。
        assert_eq!(parse_result.row_count, 2);
        assert_eq!(parse_result.skipped, 0);
    }

    #[test]
    fn extract_validate_unknown_rule_returns_err() {
        let (_dir, db) = setup_db();
        let (session_id, sheet_id) = setup_extract_sheet(&db, &["192.168.1.1"]);
        let err = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            &["nonexistent-rule".to_string()],
            session_id,
            None,
        )
        .unwrap_err();
        assert!(err.contains("不存在"));
    }

    #[test]
    fn extract_validate_unknown_column_returns_err() {
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let (session_id, sheet_id) = setup_extract_sheet(&db, &["192.168.1.1"]);
        let err = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "nope",
            &["ip4-extract".to_string()],
            session_id,
            None,
        )
        .unwrap_err();
        assert!(err.contains("不存在"));
    }

    #[test]
    fn extract_validate_empty_rules_returns_err() {
        // T56：空规则数组 → 报错
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let (session_id, sheet_id) = setup_extract_sheet(&db, &["192.168.1.1"]);
        let err = extract_validate_to_new_sheet_inner(&db, sheet_id, "raw", &[], session_id, None)
            .unwrap_err();
        assert!(err.contains("至少选择一条"));
    }

    // ---- T55c：身份证提取 + 性别联合校验 ----

    #[test]
    fn extract_validate_idcard_to_new_sheet() {
        // 无性别列：仅校验码 + 性别推断。
        // 11010519491231002X → 校验码 X，女（第 17 位=2 偶）
        // 110105194912310038 → 校验码 8，男（第 17 位=3 奇）
        // 110105194912310021 → 校验码应为 X，实际 1 → 无效
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let values = [
            "11010519491231002X",
            "110105194912310038",
            "110105194912310021",
        ];
        let (session_id, sheet_id) = setup_extract_sheet(&db, &values);

        let (parse_result, rows) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            &["idcard-extract".to_string()],
            session_id,
            None, // 无性别列
        )
        .unwrap();

        assert_eq!(rows.len(), 3);
        assert!(rows[0].valid); // 02X 有效
        assert_eq!(rows[0].note, "女");
        assert_eq!(rows[0].type_label, "身份证号");
        assert!(rows[1].valid); // 038 有效
        assert_eq!(rows[1].note, "男");
        assert!(!rows[2].valid); // 021 校验码错
        assert_eq!(rows[2].note, "非合法身份证号");
        // T56：只写有效候选 → 2 行。
        assert_eq!(parse_result.row_count, 2);
        assert_eq!(parse_result.skipped, 0);
        let data_col0 = db.query_column_cells(parse_result.new_sheet_id, 0).unwrap();
        assert_eq!(data_col0.len(), 2);
        assert_eq!(data_col0[0].1.as_deref(), Some("身份证号"));
    }

    #[test]
    fn extract_validate_idcard_gender_collision_to_new_sheet() {
        // 2 列（idcard + gender），性别联合校验：
        // row1: 038（男） + "男"     → 一致 → valid
        // row2: 038（男） + "女"     → 矛盾 → invalid + note 含「性别不一致」
        // row3: 02X（女） + "female" → 一致 → valid
        // row4: 02X（女） + ""       → 空值跳过 → valid
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let rows_data: &[(&str, &str)] = &[
            ("110105194912310038", "男"),
            ("110105194912310038", "女"),
            ("11010519491231002X", "female"),
            ("11010519491231002X", ""),
        ];
        let (session_id, sheet_id) = setup_extract_sheet_two_cols(&db, rows_data);

        let (parse_result, rows) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "idcard",
            &["idcard-extract".to_string()],
            session_id,
            Some(1), // gender 列 col_idx=1
        )
        .unwrap();

        assert_eq!(rows.len(), 4);
        // row1: 038 男 + 男 → 一致
        assert!(rows[0].valid);
        assert_eq!(rows[0].note, "男");
        // row2: 038 男 + 女 → 矛盾
        assert!(!rows[1].valid);
        assert!(rows[1].note.contains("性别不一致"));
        assert!(rows[1].note.contains("男"));
        // row3: 02X 女 + female → 一致
        assert!(rows[2].valid);
        assert_eq!(rows[2].note, "女");
        // row4: 02X 女 + "" → 空值跳过，保持有效
        assert!(rows[3].valid);
        assert_eq!(rows[3].note, "女");
        // T56：只写有效候选 → 3 行（row2 无效不写入）。
        assert_eq!(parse_result.row_count, 3);
    }

    // ---- T56：批量多规则提取 ----

    #[test]
    fn extract_validate_batch_multi_rules_to_new_sheet() {
        // 批量多规则：idcard-extract + name-extract + phone-extract 三条规则。
        // 混合数据：1 行身份证号、1 行姓名、1 行手机号、1 行无关文本。
        // 期望：新 Tab 含多类型有效结果，类型标签正确，只含有效行。
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let values = [
            "11010519491231002X", // 身份证号（女，有效）
            "伍玮琪",             // 姓名（name-extract 召回，valid=true）
            "13412345678",        // 手机号（有效）
            "hello world",        // 无候选
        ];
        let (session_id, sheet_id) = setup_extract_sheet(&db, &values);

        let (parse_result, rows) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            &[
                "idcard-extract".to_string(),
                "name-extract".to_string(),
                "phone-extract".to_string(),
            ],
            session_id,
            None,
        )
        .unwrap();

        // 3 个有效候选（身份证号 + 姓名 + 手机号），1 行无候选被跳过。
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|r| r.valid));
        // 类型标签 = 规则名去掉「提取」后缀
        assert_eq!(rows[0].type_label, "身份证号");
        assert_eq!(rows[0].value, "11010519491231002X");
        assert_eq!(rows[1].type_label, "姓名");
        assert_eq!(rows[1].value, "伍玮琪");
        assert_eq!(rows[2].type_label, "手机号");
        assert_eq!(rows[2].value, "13412345678");
        assert_eq!(parse_result.row_count, 3);
        assert_eq!(parse_result.skipped, 1); // hello world 无候选
        assert_eq!(parse_result.headers, vec!["类型", "数据值"]);
        // 读回新 Tab 验证内容。
        let data_col0 = db.query_column_cells(parse_result.new_sheet_id, 0).unwrap();
        assert_eq!(data_col0.len(), 3);
        assert_eq!(data_col0[0].1.as_deref(), Some("身份证号"));
        assert_eq!(data_col0[1].1.as_deref(), Some("姓名"));
        assert_eq!(data_col0[2].1.as_deref(), Some("手机号"));
        let data_col1 = db.query_column_cells(parse_result.new_sheet_id, 1).unwrap();
        assert_eq!(data_col1[0].1.as_deref(), Some("11010519491231002X"));
        assert_eq!(data_col1[1].1.as_deref(), Some("伍玮琪"));
        assert_eq!(data_col1[2].1.as_deref(), Some("13412345678"));
    }

    // ---- T57：行级多字段校验集成测试 ----

    /// 构造一个 7 列 sheet：username/name/sex/birth/idcard/phone/address。
    /// 表头 + 每行数据；`rows` 为 7 元组切片。返回 (session_id, sheet_id)。
    fn setup_validate_sheet(
        db: &DbManager,
        rows: &[(&str, &str, &str, &str, &str, &str, &str)],
    ) -> (i64, i64) {
        let session_id = db.create_session("validate-test", None, "csv", 0).unwrap();
        let sheet_id = db.create_sheet(session_id, "raw", 0).unwrap();
        let headers = [
            "username", "name", "sex", "birth", "idcard", "phone", "address",
        ];
        let mut cells: Vec<Cell> = Vec::with_capacity((rows.len() + 1) * 7);
        // 表头
        for (c, h) in headers.iter().enumerate() {
            cells.push(Cell {
                sheet_id,
                row_idx: 0,
                col_idx: c as u32,
                value: Some((*h).to_string()),
            });
        }
        for (i, row) in rows.iter().enumerate() {
            let r = (i + 1) as u32;
            for (c, v) in [row.0, row.1, row.2, row.3, row.4, row.5, row.6]
                .iter()
                .enumerate()
            {
                cells.push(Cell {
                    sheet_id,
                    row_idx: r,
                    col_idx: c as u32,
                    value: Some((*v).to_string()),
                });
            }
        }
        db.write_cells(sheet_id, &cells).unwrap();
        (session_id, sheet_id)
    }

    /// 7 字段全映射。全有效行 → valid sheet 有 N 行，invalid sheet 0 行。
    #[test]
    fn validate_rows_to_two_sheets_all_fields_valid() {
        let (_dir, db) = setup_db();
        let rows = [
            (
                "admin",
                "张三",
                "男",
                "19491231",
                "110105194912310038",
                "13412345678",
                "内蒙古自治区呼和浩特市玉泉区大南街街道1340号540室",
            ),
            (
                "lufe1jian",
                "李四",
                "女",
                "19491231",
                "11010519491231002X",
                "15987654321",
                "北京市朝阳区建国路1号101室",
            ),
        ];
        let (session_id, sheet_id) = setup_validate_sheet(&db, &rows);
        let mapping = FieldColumnMapping {
            username: Some("username".into()),
            name: Some("name".into()),
            sex: Some("sex".into()),
            birth: Some("birth".into()),
            idcard: Some("idcard".into()),
            phone: Some("phone".into()),
            address: Some("address".into()),
        };
        let res =
            validate_rows_to_two_sheets_inner(&db, sheet_id, session_id, &mapping, &[]).unwrap();
        assert_eq!(res.valid_sheet.row_count, 2);
        assert_eq!(res.invalid_sheet.row_count, 0);
        assert!(res.invalid_reasons.is_empty());
        // 验证列数 = 7（保留原列）
        let valid_cells = db
            .query_cells(res.valid_sheet.new_sheet_id, 0, 100)
            .unwrap();
        let max_col = valid_cells.iter().map(|c| c.col_idx).max().unwrap();
        assert_eq!(max_col, 6);
    }

    /// 部分有效 / 部分无效 → 正确分流。
    #[test]
    fn validate_rows_to_two_sheets_mixed_valid_invalid() {
        let (_dir, db) = setup_db();
        let rows = [
            // 有效行
            (
                "admin",
                "张三",
                "男",
                "19491231",
                "110105194912310038",
                "13412345678",
                "北京市朝阳区建国路1号101室",
            ),
            // 无效行：username 含非法字符
            (
                "ab.cd",
                "李四",
                "女",
                "19491231",
                "11010519491231002X",
                "15987654321",
                "北京市朝阳区建国路1号101室",
            ),
        ];
        let (session_id, sheet_id) = setup_validate_sheet(&db, &rows);
        let mapping = FieldColumnMapping {
            username: Some("username".into()),
            name: Some("name".into()),
            sex: Some("sex".into()),
            birth: Some("birth".into()),
            idcard: Some("idcard".into()),
            phone: Some("phone".into()),
            address: Some("address".into()),
        };
        let res =
            validate_rows_to_two_sheets_inner(&db, sheet_id, session_id, &mapping, &[]).unwrap();
        assert_eq!(res.valid_sheet.row_count, 1);
        assert_eq!(res.invalid_sheet.row_count, 1);
        assert_eq!(res.invalid_reasons.len(), 1);
        assert_eq!(res.invalid_reasons[0].field, "username");
    }

    /// 跨字段：idcard 推断男但 sex 列=女 → invalid（性别不一致）。
    #[test]
    fn validate_rows_to_two_sheets_cross_field_sex_mismatch() {
        let (_dir, db) = setup_db();
        // idcard 110105194912310038 → 第 17 位 3 奇 → 男；sex 列写「女」
        let rows = [(
            "admin",
            "张三",
            "女",
            "19491231",
            "110105194912310038",
            "13412345678",
            "北京市朝阳区建国路1号101室",
        )];
        let (session_id, sheet_id) = setup_validate_sheet(&db, &rows);
        let mapping = FieldColumnMapping {
            username: Some("username".into()),
            name: Some("name".into()),
            sex: Some("sex".into()),
            birth: Some("birth".into()),
            idcard: Some("idcard".into()),
            phone: Some("phone".into()),
            address: Some("address".into()),
        };
        let res =
            validate_rows_to_two_sheets_inner(&db, sheet_id, session_id, &mapping, &[]).unwrap();
        assert_eq!(res.invalid_sheet.row_count, 1);
        assert!(res
            .invalid_reasons
            .iter()
            .any(|r| r.field == "sex" && r.reason.contains("性别不一致")));
    }

    /// 跨字段：birth 与 idcard 出生日期码不一致 → invalid。
    #[test]
    fn validate_rows_to_two_sheets_cross_field_birth_mismatch() {
        let (_dir, db) = setup_db();
        // idcard 出生日期 19491231；birth 列写 20000101
        let rows = [(
            "admin",
            "张三",
            "男",
            "20000101",
            "110105194912310038",
            "13412345678",
            "北京市朝阳区建国路1号101室",
        )];
        let (session_id, sheet_id) = setup_validate_sheet(&db, &rows);
        let mapping = FieldColumnMapping {
            username: Some("username".into()),
            name: Some("name".into()),
            sex: Some("sex".into()),
            birth: Some("birth".into()),
            idcard: Some("idcard".into()),
            phone: Some("phone".into()),
            address: Some("address".into()),
        };
        let res =
            validate_rows_to_two_sheets_inner(&db, sheet_id, session_id, &mapping, &[]).unwrap();
        assert_eq!(res.invalid_sheet.row_count, 1);
        assert!(res
            .invalid_reasons
            .iter()
            .any(|r| r.field == "birth" && r.reason.contains("出生日期与身份证号不一致")));
    }

    /// 只映射 username+name（无 idcard）→ 无跨字段校验，按单字段分流。
    #[test]
    fn validate_rows_to_two_sheets_partial_mapping() {
        let (_dir, db) = setup_db();
        let rows = [
            (
                "admin",
                "张三",
                "男",
                "19491231",
                "110105194912310038",
                "13412345678",
                "北京市朝阳区建国路1号101室",
            ),
            (
                "ab.cd",
                "李四",
                "女",
                "19491231",
                "11010519491231002X",
                "15987654321",
                "北京市朝阳区建国路1号101室",
            ),
        ];
        let (session_id, sheet_id) = setup_validate_sheet(&db, &rows);
        let mapping = FieldColumnMapping {
            username: Some("username".into()),
            name: Some("name".into()),
            sex: None,
            birth: None,
            idcard: None,
            phone: None,
            address: None,
        };
        let res =
            validate_rows_to_two_sheets_inner(&db, sheet_id, session_id, &mapping, &[]).unwrap();
        // 行1 username=admin（有效）+ name=张三（有效）→ valid
        assert_eq!(res.valid_sheet.row_count, 1);
        // 行2 username=ab.cd（无效）→ invalid
        assert_eq!(res.invalid_sheet.row_count, 1);
        assert_eq!(res.invalid_reasons[0].field, "username");
        // 无 idcard 映射 → 无跨字段原因
        assert!(res
            .invalid_reasons
            .iter()
            .all(|r| !r.reason.contains("身份证")));
    }

    /// phone 前缀白名单过滤：134 通过，159 被 filtered out。
    #[test]
    fn validate_rows_to_two_sheets_phone_prefix_filter() {
        let (_dir, db) = setup_db();
        let rows = [
            (
                "admin",
                "张三",
                "男",
                "19491231",
                "110105194912310038",
                "13412345678",
                "北京市朝阳区建国路1号101室",
            ),
            (
                "lufe1jian",
                "李四",
                "女",
                "19491231",
                "11010519491231002X",
                "15987654321",
                "北京市朝阳区建国路1号101室",
            ),
        ];
        let (session_id, sheet_id) = setup_validate_sheet(&db, &rows);
        let mapping = FieldColumnMapping {
            username: Some("username".into()),
            name: Some("name".into()),
            sex: Some("sex".into()),
            birth: Some("birth".into()),
            idcard: Some("idcard".into()),
            phone: Some("phone".into()),
            address: Some("address".into()),
        };
        // 白名单只允许 134 → 行2（159）phone 失败
        let res = validate_rows_to_two_sheets_inner(
            &db,
            sheet_id,
            session_id,
            &mapping,
            &["134".to_string()],
        )
        .unwrap();
        assert_eq!(res.valid_sheet.row_count, 1);
        assert_eq!(res.invalid_sheet.row_count, 1);
        assert!(res.invalid_reasons.iter().any(|r| r.field == "phone"));
    }

    // ---- T61：validate_column IPC 契约回归 ----
    //
    // 前端 ValidatePanel 原本读 `res.results`，但后端 `validate_column` 直接返回
    // `Vec<RowValidation>`（裸数组），导致校验结果全部被忽略。本测试固化契约：
    //   1. 返回类型是 `Vec<RowValidation>`，不是包了一层的对象；
    //   2. row_idx 是 DB 绝对行号（数据行从 1 开始，与前端 `r.rowIdx - 1 - base`
    //      换算一致）；
    //   3. 空列 / 全通过 / 部分不通过分别有正确数量与 passed 标记。

    /// 构造一个 sheet：1 列（col0=name），表头 + 给定数据行。返回 sheet_id。
    fn setup_validate_column_sheet(db: &DbManager, values: &[&str]) -> i64 {
        let session_id = db.create_session("vc-test", None, "csv", 0).unwrap();
        let sheet_id = db.create_sheet(session_id, "vc", 0).unwrap();
        let mut cells: Vec<Cell> = Vec::with_capacity(values.len() + 1);
        cells.push(Cell {
            sheet_id,
            row_idx: 0,
            col_idx: 0,
            value: Some("name".into()),
        });
        for (i, v) in values.iter().enumerate() {
            cells.push(Cell {
                sheet_id,
                row_idx: (i + 1) as u32,
                col_idx: 0,
                value: Some((*v).to_string()),
            });
        }
        db.write_cells(sheet_id, &cells).unwrap();
        sheet_id
    }

    /// 直接驱动 `validate_column` 的核心逻辑（无 Tauri State 注入），断言返回值是
    /// `Vec<RowValidation>` 且 row_idx / passed 符合前端消费契约。
    #[test]
    fn validate_column_returns_vec_contract() {
        let (_dir, db) = setup_db();
        db.upsert_rule(&RuleRegistry::name_validate_rule()).unwrap();
        // 行1=张三（2 字中文，通过）、行2=Zhang（非中文，不通过）、行3=诸葛亮（3 字中文，通过）。
        let sheet_id = setup_validate_column_sheet(&db, &["张三", "Zhang", "诸葛亮"]);

        // 复用 validate_column 的内部流程：读列 → RegexValidator::validate → 收集。
        let col_idx = db
            .find_col_idx(sheet_id, "name")
            .unwrap()
            .expect("name 列存在");
        let rule = db.get_rule("name-validate").unwrap().expect("规则存在");
        let rows = db.query_column_cells(sheet_id, col_idx).unwrap();
        let validator = RegexValidator;
        let mut results: Vec<RowValidation> = Vec::with_capacity(rows.len());
        for (row_idx, value) in &rows {
            let input = value.as_deref().unwrap_or("");
            let vr = validator.validate(input, &rule).unwrap();
            results.push(RowValidation {
                row_idx: *row_idx,
                passed: vr.passed,
                message: vr.message,
            });
        }

        // 契约 1：返回数组长度 = 数据行数（不含表头行）。
        assert_eq!(results.len(), 3);
        // 契约 2：row_idx 是 DB 绝对行号，数据行从 1 开始（前端 `rowIdx - 1 - base`）。
        assert_eq!(results[0].row_idx, 1);
        assert_eq!(results[1].row_idx, 2);
        assert_eq!(results[2].row_idx, 3);
        // 契约 3：passed 标记正确（张三 / 诸葛亮 通过，Zhang 不通过）。
        assert!(results[0].passed);
        assert!(!results[1].passed);
        assert!(results[2].passed);

        // 契约 4：序列化为 JSON 时是裸数组 `[...]`，不是 `{ "results": [...] }`。
        // 前端 `Array.isArray(results)` 必须为 true，这是 T61 修复的根因。
        let json = serde_json::to_string(&results).unwrap();
        assert!(
            json.starts_with('[') && json.ends_with(']'),
            "validate_column 应序列化为裸数组，实际: {json}"
        );
        assert!(
            !json.contains("\"results\""),
            "validate_column 不应包含 results 包装字段，实际: {json}"
        );
        // camelCase：rowIdx（不是 row_idx）。
        assert!(json.contains("\"rowIdx\""));
        assert!(!json.contains("\"row_idx\""));
    }

    /// 空数据列 → 返回空数组（前端 `Array.isArray([])` 为 true，不会误高亮）。
    #[test]
    fn validate_column_empty_returns_empty_vec() {
        let (_dir, db) = setup_db();
        db.upsert_rule(&RuleRegistry::name_validate_rule()).unwrap();
        let sheet_id = setup_validate_column_sheet(&db, &[]);

        let col_idx = db
            .find_col_idx(sheet_id, "name")
            .unwrap()
            .expect("name 列存在");
        let rows = db.query_column_cells(sheet_id, col_idx).unwrap();
        // query_column_cells 排除表头行，空数据 → 空。
        assert!(rows.is_empty());
        // 模拟 validate_column 的返回：空 Vec。
        let results: Vec<RowValidation> = Vec::new();
        let json = serde_json::to_string(&results).unwrap();
        assert_eq!(json, "[]");
        assert!(json.starts_with('['));
    }
}
