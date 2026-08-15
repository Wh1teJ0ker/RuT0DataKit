//! 脱敏命令（mask_column）+ 脱敏结果结构体。
//!
//! 全部 `#[tauri::command]` → `Result<T, String>` + `.map_err(|e| e.to_string())`。
//! 依赖 `DbManager`（列读写 + 规则持久化）。

use serde::Serialize;

use ruT0_data_kit_core::processor::validators::{is_valid_phone, validate_extracted_with_params};
use ruT0_data_kit_core::processor::rules::{ExtractParams, TemplateParams};
use ruT0_data_kit_core::processor::Masker;

use crate::db::Cell;

// ---------------------------------------------------------------------------
// 脱敏命令
// ---------------------------------------------------------------------------

/// 脱敏结果（camelCase）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskResult {
    pub affected: u32,
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
/// - `validate_rule_id` 可选（v1.1.4 T84）；`Some(rid)` 启用"先校验再脱敏"：
///   每行先按校验规则分发校验（phone-validate → `is_valid_phone`；params /
///   params_override → `validate_extracted_with_params`；pattern → 正则 `is_match`；
///   其他 → 通过）。通过 → 正常脱敏；不通过 → 输出 `invalid_text`（默认 `INVALID`）。
///   `None` → 跳过校验，走原脱敏流程。校验规则的查找与脱敏规则独立（两者可不同）。
/// - `invalid_text` 可选（T84）；校验失败时写入单元格的文本，默认 `INVALID`。
/// - `params_override` 可选（T84）；覆盖校验规则的 `params`（generic-validate 等）。
/// - `phone_prefixes` 可选（T84）；`phone-validate` 前缀白名单。
///
/// Masker 优先级：临时 replacement > rule.replacement > 默认 `*`。
/// Template 优先级：临时 template 参数 > rule.template。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn mask_column(
    sheet_id: i64,
    column: String,
    rule_id: Option<String>,
    replacement: Option<String>,
    template: Option<TemplateParams>,
    validate_rule_id: Option<String>,
    invalid_text: Option<String>,
    params_override: Option<ExtractParams>,
    phone_prefixes: Option<Vec<String>>,
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

    // v1.1.4 T84：先校验再脱敏。`validate_rule_id` 为 Some 时，每行先按校验规则
    //   分发校验；通过 → 正常脱敏，不通过 → 写入 `invalid_text`（默认 `INVALID`）。
    //   校验规则的查找与脱敏规则独立（两者可不同 id）。
    let validate_rule = if let Some(vrid) = &validate_rule_id {
        Some(
            db.get_rule(vrid)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("校验规则 `{vrid}` 不存在"))?,
        )
    } else {
        None
    };
    // 预编译校验规则的正则（rule.pattern 非空时），避免逐行重复编译。
    let validate_re = validate_rule
        .as_ref()
        .and_then(|r| r.pattern.as_ref().and_then(|p| regex::Regex::new(p).ok()));
    let phone_prefixes_ref: Vec<String> = phone_prefixes.clone().unwrap_or_default();
    let invalid_out = invalid_text
        .clone()
        .unwrap_or_else(|| "INVALID".to_string());

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
        // T84：先校验再脱敏。校验分发与 validate_multi_rules_to_two_sheets_inner
        //   的单行逻辑保持一致：
        //   - phone-validate → is_valid_phone（整串严格校验）
        //   - params / params_override → validate_extracted_with_params（override 优先）
        //   - pattern → 正则 is_match
        //   - 其他 → 默认通过
        let output = if let Some(vrule) = &validate_rule {
            let passed = if vrule.id == "phone-validate" {
                is_valid_phone(input, &phone_prefixes_ref)
            } else {
                let effective_params = params_override.as_ref().or(vrule.params.as_ref());
                if let Some(params) = effective_params {
                    validate_extracted_with_params(params, input).0
                } else if let Some(ref re) = validate_re {
                    re.is_match(input)
                } else {
                    true
                }
            };
            if passed {
                masker
                    .mask(input, rule_ref.as_ref())
                    .map_err(|e| e.to_string())?
                    .output
            } else {
                invalid_out.clone()
            }
        } else {
            masker
                .mask(input, rule_ref.as_ref())
                .map_err(|e| e.to_string())?
                .output
        };
        cells.push(Cell {
            sheet_id,
            row_idx: *row_idx,
            col_idx,
            value: Some(output),
        });
    }
    let affected = cells.len() as u32;
    if !cells.is_empty() {
        db.write_cells(sheet_id, &cells)
            .map_err(|e| e.to_string())?;
    }

    // after 快照：脱敏后的 cells（仅被修改列）。
    let after_json = serde_json::to_string(&cells).map_err(|e| e.to_string())?;
    // T84：validate_rule_id 为 Some 时追加校验参数字段，便于撤销 / 重做 / 审计还原。
    let params_json = if validate_rule_id.is_some() {
        serde_json::json!({
            "column": column,
            "ruleId": rule_id,
            "replacement": replacement,
            "template": template,
            "affected": affected,
            "validateRuleId": validate_rule_id,
            "invalidText": invalid_text,
            "paramsOverride": params_override,
            "phonePrefixes": phone_prefixes
        })
        .to_string()
    } else {
        serde_json::json!({
            "column": column,
            "ruleId": rule_id,
            "replacement": replacement,
            "template": template,
            "affected": affected
        })
        .to_string()
    };
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
