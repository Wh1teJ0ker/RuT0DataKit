//! 提取+校验→新 Tab / 行级多字段校验 / 多规则行级校验命令。
//!
//! 全部 `#[tauri::command]` → `Result<T, String>` + `.map_err(|e| e.to_string())`。
//! 依赖 `DbManager`（列读写 + 规则持久化 + sheet 创建）。
//!
//! v1.2.1 T8：公共类型（`ExtractValidateRow` / `RowInvalidReason` /
//! `TwoSheetResult` / `CrossFieldConfig` / `MultiRuleValidation`）与共享 helper
//! （`validate_single` / `group_cells_by_row` / `query_all_data_cells` /
//! `read_headers` / `write_sheet`）移入 [`super::helpers`]，本文件仅保留
//! 命令 + 核心逻辑。

use std::collections::HashMap;

use ruT0_data_kit_core::processor::rules::ExtractParams;
use ruT0_data_kit_core::processor::validators::{
    check_gender_consistency, clean_birth, is_valid_birth, validate_extracted,
    validate_extracted_with_params,
};

use crate::commands::columns::ParseResult;
use crate::commands::processor::helpers::{
    group_cells_by_row, query_all_data_cells, read_headers, validate_single, write_sheet,
    CrossFieldConfig, ExtractValidateRow, MultiRuleValidation, RowInvalidReason, TwoSheetResult,
};
use crate::db::{Cell, DbManager};

// ---------------------------------------------------------------------------
// 提取 + 函数式校验 → 新 Tab（v1.1.3 T55）
// ---------------------------------------------------------------------------

/// 提取 + 校验核心逻辑（接受 `&DbManager`，便于单测直接调用）。
///
/// 流程：
/// 1. `find_col_idx` + `query_column_cells` 读源列全部数据行
/// 2. 从 DB 取所有规则（`get_rule`），预编译各规则 `pattern` 正则（宽松召回）
/// 3. **拼接全列文本**（无分隔符）后统一跑 `find_iter`，确保跨块边界
///    （TXT 导入 4096 字节定长分块产生的相邻行）的模式不被截断，
///    且分块制造的人工 `\b` 不复存在——与官方题解做法一致
/// 4. 对每个候选调 `validate_extracted`
///    → 收集 `(源行号, 类型标签, 提取值, 有效, 说明)`
///    （T78：phone-extract 规则 + `phone_prefixes` 非空 → 用运行时白名单覆盖）
/// 5. （T55c）若任一规则 `params == IdCard` 且指定 `gender_col_idx`：
///    读性别列 → 构建行号→性别值映射 → 对有效 idcard 候选比对推断性别
///    与性别列值；矛盾则 `valid=false` + `note` 标注不一致
/// 6. `create_sheet(session_id, "{column}_提取", 0)` + `write_cells`
///    （表头 `[类型, 数据值]` + **仅有效**数据行）
/// 7. `log_operation("extract_to_sheet", before_snapshot=None)`（不进撤销栈）
///
/// 新 Tab 只含**有效候选**（`valid==true`），无效候选不写入但仍在返回的
/// `Vec<ExtractValidateRow>` 中保留供测试断言。
///
/// `idcard_allow_leading_zero`（v1.2.2）：仅对 `idcard-extract` 规则生效。
/// `true` → 临时用 `\b\d{17}[\dXx]\b` 覆盖 DB pattern（首位可为 0，宽松召回），
/// 不持久化到 DB。语义与 `phone_prefixes` 平行：运行时临时覆盖。
pub fn extract_validate_to_new_sheet_inner(
    db: &DbManager,
    sheet_id: i64,
    column: &str,
    rule_ids: &[String],
    session_id: i64,
    gender_col_idx: Option<u32>,
    phone_prefixes: &[String],
    idcard_allow_leading_zero: bool,
) -> Result<(ParseResult, Vec<ExtractValidateRow>), String> {
    if rule_ids.is_empty() {
        return Err("请至少选择一条提取规则".into());
    }

    // 去重：同一规则 ID 只编译/执行一次，避免重复扫描导致结果翻倍。
    let rule_ids: Vec<&String> = {
        let mut seen = std::collections::HashSet::new();
        rule_ids.iter().filter(|r| seen.insert(*r)).collect()
    };

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
    for rid in &rule_ids {
        let rule = db
            .get_rule(rid)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("规则 `{rid}` 不存在"))?;
        // v1.2.2：idcard_allow_leading_zero=true 时，临时用宽松正则覆盖 DB
        // pattern（首位可为 0），不持久化到 DB。与 phone_prefixes 语义平行。
        let is_idcard_rule =
            matches!(rule.params.as_ref(), Some(ExtractParams::IdCard));
        let pattern_str = if is_idcard_rule && idcard_allow_leading_zero {
            r"\b\d{17}[\dXx]\b"
        } else {
            rule.pattern
                .as_deref()
                .ok_or_else(|| format!("规则 `{rid}` 未配置提取正则（pattern 为空）"))?
        };
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

    // 拼接全列文本 + 记录每行起始偏移，用于映射匹配→源行号。
    //
    // 背景：TXT 导入器对无换行超大文件按 4096 字节定长分块，每块成为独立
    // DB 行。逐行跑正则会导致两类问题：
    // 1) 跨块边界模式被截断（如 IP "192.168.1.1" 被拆为 "...19"+"2.168.1.1..."）
    // 2) 逐行在块边界制造人工 \b，使原本连续的长数字串碎片通过校验
    //    （如 21 位 "850233386999983226607" 被分块后，碎片 "3386999983226607"
    //     长度 16 且通过 Luhn，作为假银行卡号入结果）
    //
    // 正确做法（与官方题解一致）：拼接全列后整体跑正则，\b 只在原文的
    // 自然边界生效，分块制造的人工 \b 不复存在。用偏移二分查找映射回源行号。
    //
    // 分隔符策略（按 source_type 区分）：
    // - txt：TXT 导入器对单行文件按 4096 字节分块，行间是同一连续字符串
    //   的片段 → 不插入分隔符，拼接后与原文逐字节一致，跨边界模式可恢复。
    //   插入 \n 会在拼接处制造 \b，使跨边界模式只匹配后半截。
    // - csv / xlsx / json 等：每行是独立记录 → 行间插入 \n 分隔符，为 \b
    //   提供自然边界，避免相邻纯数字行拼接后 \b 失效（如两个 11 位手机号
    //   拼成 22 位数字串，\b[1-9]\d{10}\b 无法匹配）。
    let is_chunked_txt = db
        .get_session(session_id)
        .map(|d| d.session.source_type == "txt")
        .unwrap_or(false);

    let mut full_string = String::new();
    let mut offsets: Vec<(u32, usize)> = Vec::with_capacity(rows.len()); // (row_idx, start_offset)
    for (row_idx, value) in &rows {
        offsets.push((*row_idx, full_string.len()));
        full_string.push_str(value.as_deref().unwrap_or(""));
        if !is_chunked_txt {
            full_string.push('\n');
        }
    }

    // 在拼接文本上整体扫描正则（与官方题解一致）：
    // - 跨块边界的完整模式可被正确匹配（无人工 \b 干扰）
    // - 原本连续的长数字串不会因分块而产生碎片假匹配
    let mut results: Vec<ExtractValidateRow> = Vec::new();
    let mut rows_with_hits: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for (rule, re, label) in &compiled {
        let is_idcard = matches!(rule.params.as_ref(), Some(ExtractParams::IdCard));
        let is_phone_extract = rule.id == "phone-extract";
        for m in re.find_iter(&full_string) {
            let candidate = m.as_str();
            // 二分查找匹配起始偏移对应的源行号。
            let row_idx = match offsets.binary_search_by(|(_, start)| start.cmp(&m.start())) {
                Ok(idx) => offsets[idx].0,
                Err(idx) => offsets[idx.saturating_sub(1)].0,
            };
            let (mut valid, mut note) = if is_phone_extract && !phone_prefixes.is_empty() {
                let params_override = ExtractParams::PhonePrefix {
                    allowed_prefixes: phone_prefixes.to_vec(),
                };
                validate_extracted_with_params(&params_override, candidate)
            } else {
                validate_extracted(rule, candidate)
            };
            if is_idcard && valid {
                if let Some(col_val) = gender_map.get(&row_idx) {
                    if let Some(msg) = check_gender_consistency(candidate, col_val) {
                        valid = false;
                        note = msg;
                    }
                }
            }
            results.push(ExtractValidateRow {
                source_row: row_idx,
                type_label: label.clone(),
                value: candidate.to_string(),
                valid,
                note,
            });
            rows_with_hits.insert(row_idx);
        }
    }

    let skipped = (rows.len() as u32) - (rows_with_hits.len() as u32);

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
///
/// `phone_prefixes`（T78）：手机号前缀白名单，仅当选中的规则含 `phone-extract`
/// 时使用。非空 → 覆盖 DB 规则 params 的 allowed_prefixes（运行时临时覆盖，不
/// 持久化）；空 → 回落 DB 规则 params。
///
/// `idcard_allow_leading_zero`（v1.2.2）：仅对 `idcard-extract` 规则生效。
/// `true` → 临时用 `\b\d{17}[\dXx]\b` 覆盖 DB pattern（首位可为 0，宽松召回），
/// 不持久化到 DB。`false` → 用 DB pattern（默认首位非零）。
#[tauri::command]
pub fn extract_validate_to_new_sheet(
    sheet_id: i64,
    column: String,
    rule_ids: Vec<String>,
    session_id: i64,
    gender_col: Option<String>,
    phone_prefixes: Vec<String>,
    idcard_allow_leading_zero: Option<bool>,
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
        &phone_prefixes,
        idcard_allow_leading_zero.unwrap_or(false),
    )?;
    Ok(parse_result)
}

// ---------------------------------------------------------------------------
// 多规则行级校验 → 双 Tab 输出（T68）
// ---------------------------------------------------------------------------

/// 多规则行级校验核心逻辑（接受 `&DbManager`，便于单测直接调用）。T68。
///
/// 接收任意「列名 + 规则 id」组合，逐行逐规则分发校验：
/// - `phone-validate` → 直接调 [`is_valid_phone`]（整串 11 位 + 纯数字 + 前缀白名单，空=不限）
/// - `rule.params` 非空 → [`validate_extracted`]（函数式：Username/Sex/Birth/
///   IdCard/Address/PhonePrefix/Luhn/Ipv4/Ipv6 分发）
/// - `rule.pattern` 非空 → 正则 `is_match`（如 name-validate）
/// - 其他 → 默认通过
///
/// 身份证跨字段联合校验（仅当 idcard-validate 规则带 `crossField` 配置 +
/// idcard 本身校验通过）：
/// - 比对性别：[`check_gender_consistency`](ruT0_data_kit_core::processor::validators::check_gender_consistency)
/// - 比对出生日期：`idcard_val[6..14] == birth_col_val`（且 birth 是 8 位数字）
///
/// 整行分流：任一规则失败 → 整行入 invalid + 收集 [`RowInvalidReason`]；
/// 全通过 → valid。双 Tab 写出（`{源sheet名}_校验通过` / `_校验失败`），
/// 保留原列不新增。`phone_prefixes` 全局应用于 phone-validate 规则。
pub fn validate_multi_rules_to_two_sheets_inner(
    db: &DbManager,
    sheet_id: i64,
    session_id: i64,
    rules: &[MultiRuleValidation],
    phone_prefixes: &[String],
) -> Result<TwoSheetResult, String> {
    if rules.is_empty() {
        return Err("请至少选择一条校验规则".into());
    }

    let sheet_name = db
        .get_sheet_name(sheet_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("sheet {sheet_id} 不存在"))?;

    // 表头（row_idx=0）。
    let headers = read_headers(db, sheet_id)?;
    let col_count = headers.len();

    // 分页读全部数据行 cells（query_cells 已排除 row_idx=0 表头）。
    let all_cells = query_all_data_cells(db, sheet_id)?;

    // 按 row_idx 分组，组内按 col_idx 排序。
    let grouped = group_cells_by_row(all_cells);

    // 预解析每条规则：col_idx + Rule（从 DB 取，找不到 → 报错）。
    // 同时预编译正则（rule.pattern 非空时），避免逐行重复编译。
    struct ResolvedRule {
        column: String,
        col_idx: u32,
        rule: ruT0_data_kit_core::processor::Rule,
        re: Option<regex::Regex>,
        cross_field: Option<CrossFieldConfig>,
        /// v1.1.4 续轮 T70：参数覆盖（None = 用 rule.params）。
        params_override: Option<ExtractParams>,
    }
    let mut resolved: Vec<ResolvedRule> = Vec::with_capacity(rules.len());
    for rv in rules {
        let col_idx = db
            .find_col_idx(sheet_id, &rv.column)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("列 `{}` 不存在", rv.column))?;
        let rule = db
            .get_rule(&rv.rule_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("规则 `{}` 不存在", rv.rule_id))?;
        let re = if let Some(ref pattern) = rule.pattern {
            Some(regex::Regex::new(pattern).map_err(|e| format!("正则编译失败: {e}"))?)
        } else {
            None
        };
        resolved.push(ResolvedRule {
            column: rv.column.clone(),
            col_idx,
            rule,
            re,
            cross_field: rv.cross_field.clone(),
            params_override: rv.params_override.clone(),
        });
    }

    // 逐行逐规则校验。
    let mut valid_rows: Vec<Vec<Option<String>>> = Vec::new();
    let mut invalid_rows: Vec<Vec<Option<String>>> = Vec::new();
    let mut invalid_reasons: Vec<RowInvalidReason> = Vec::new();

    for (&row_idx, cells_row) in grouped.iter() {
        debug_assert!(row_idx > 0, "T68: query_cells should exclude header");
        // 行内按 col_idx 排序对齐到 headers.len() 列（缺列补 None）。
        let mut row_vals: Vec<Option<String>> = vec![None; col_count];
        let mut sorted_cells = cells_row.clone();
        sorted_cells.sort_by_key(|(col, _)| *col);
        for (col, val) in sorted_cells {
            if (col as usize) < row_vals.len() {
                row_vals[col as usize] = val;
            }
        }

        let mut row_invalid_reasons: Vec<(String, String)> = Vec::new();
        let mut idcard_valid = false;
        let mut idcard_val = String::new();
        let mut cross_field_config: Option<&CrossFieldConfig> = None;

        for rr in &resolved {
            let value = row_vals
                .get(rr.col_idx as usize)
                .cloned()
                .flatten()
                .unwrap_or_default();

            // 单行校验分发（v1.2.1 T8：复用 helpers::validate_single）。
            let (passed, msg) = validate_single(
                &rr.rule,
                rr.re.as_ref(),
                &value,
                phone_prefixes,
                rr.params_override.as_ref(),
            );

            if !passed {
                row_invalid_reasons.push((rr.column.clone(), msg));
            }

            // 记录 idcard 状态用于跨字段联合校验。
            if rr.rule.id == "idcard-validate"
                || rr.rule.params.as_ref() == Some(&ExtractParams::IdCard)
            {
                idcard_valid = passed;
                idcard_val = value;
                cross_field_config = rr.cross_field.as_ref();
            }
        }

        // 跨字段联合校验（仅当 idcard 有效 + 配置了 cross_field）。
        if idcard_valid {
            if let Some(cf) = cross_field_config {
                if cf.check_sex {
                    if let Some(ref sex_col) = cf.sex_column {
                        let sex_idx = db
                            .find_col_idx(sheet_id, sex_col)
                            .map_err(|e| e.to_string())?
                            .ok_or_else(|| format!("性别列 `{sex_col}` 不存在"))?;
                        let sex_val = row_vals
                            .get(sex_idx as usize)
                            .cloned()
                            .flatten()
                            .unwrap_or_default();
                        if let Some(msg) = check_gender_consistency(&idcard_val, &sex_val) {
                            row_invalid_reasons.push((sex_col.clone(), msg));
                        }
                    }
                }
                if cf.check_birth {
                    if let Some(ref birth_col) = cf.birth_column {
                        let birth_idx = db
                            .find_col_idx(sheet_id, birth_col)
                            .map_err(|e| e.to_string())?
                            .ok_or_else(|| format!("出生日期列 `{birth_col}` 不存在"))?;
                        let birth_val = row_vals
                            .get(birth_idx as usize)
                            .cloned()
                            .flatten()
                            .unwrap_or_default();
                        if is_valid_birth(&birth_val) {
                            // idcard_val 一定 18 位且前 17 纯数字（is_valid_idcard 已保证）。
                            // T70：birth 可能含分隔符（如 "1949-12-31"），用 clean_birth 归一化后比对。
                            let cleaned_birth = clean_birth(&birth_val);
                            let idcard_birth = &idcard_val[6..14];
                            if cleaned_birth != idcard_birth {
                                row_invalid_reasons.push((
                                    birth_col.clone(),
                                    format!(
                                        "出生日期与身份证号不一致: 身份证{idcard_birth}，出生日期{birth_val}"
                                    ),
                                ));
                            }
                        }
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

    // 双 Tab 写出（v1.2.1 T8：复用 helpers::write_sheet）。
    let valid_sheet_name = format!("{sheet_name}_校验通过");
    let invalid_sheet_name = format!("{sheet_name}_校验失败");
    let valid_sheet_id = db
        .create_sheet(session_id, &valid_sheet_name, 0)
        .map_err(|e| e.to_string())?;
    let invalid_sheet_id = db
        .create_sheet(session_id, &invalid_sheet_name, 0)
        .map_err(|e| e.to_string())?;

    let valid_count = write_sheet(db, valid_sheet_id, &headers, &valid_rows)?;
    let invalid_count = write_sheet(db, invalid_sheet_id, &headers, &invalid_rows)?;

    db.log_operation(
        Some(sheet_id),
        "validate_multi_rules_to_two_sheets",
        &serde_json::json!({
            "sourceSheetId": sheet_id,
            "sourceSheetName": sheet_name,
            "validSheetId": valid_sheet_id,
            "invalidSheetId": invalid_sheet_id,
            "validCount": valid_count,
            "invalidCount": invalid_count,
            "rules": rules.iter().map(|r| {
                serde_json::json!({ "column": r.column, "ruleId": r.rule_id })
            }).collect::<Vec<_>>(),
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

/// `validate_multi_rules_to_two_sheets` 的 Tauri 命令包装。T68。
///
/// 接收任意「列名 + 规则 id」组合（`rules`），逐行逐规则校验 → 整行按
/// 通过/失败分流到两个新 Tab（`{源sheet名}_校验通过` / `_校验失败`），
/// 保留原列不新增。`phone_prefixes` 全局应用于 phone-validate 规则。
/// `rules[i].crossField` 仅在 `ruleId` 为 idcard-validate 时携带，用于触发
/// 性别/出生日期跨字段联合校验。
#[tauri::command]
pub fn validate_multi_rules_to_two_sheets(
    sheet_id: i64,
    session_id: i64,
    rules: Vec<MultiRuleValidation>,
    phone_prefixes: Vec<String>,
    db: tauri::State<'_, DbManager>,
) -> Result<TwoSheetResult, String> {
    validate_multi_rules_to_two_sheets_inner(&db, sheet_id, session_id, &rules, &phone_prefixes)
}

// ---------------------------------------------------------------------------
// commands 层测试
// ---------------------------------------------------------------------------

/// 提取/校验核心路径不依赖 Tauri runtime（`#[tauri::command]` 函数签名带
/// `tauri::State<'_, DbManager>` 在单元测试中无法直接调用）。测试策略：
/// 直接调用 `&DbManager` 接受的 `_inner` 函数，覆盖提取+校验核心路径。
#[cfg(test)]
mod tests {
    use super::{
        extract_validate_to_new_sheet_inner, validate_multi_rules_to_two_sheets_inner,
        CrossFieldConfig, MultiRuleValidation,
    };
    use crate::db::{Cell, DbManager};
    use ruT0_data_kit_core::processor::rules::ExtractParams;

    /// 构造一个 tempdir + 空 DbManager。返回 TempDir 以保活（TempDir drop 会
    /// 删除目录与 db 文件，必须跨测试函数持有）。
    fn setup_db() -> (tempfile::TempDir, DbManager) {
        let dir = tempfile::tempdir().unwrap();
        let mgr = DbManager::new(dir.path()).unwrap();
        (dir, mgr)
    }

    // ---- T55：提取 + 函数式校验 → 新 Tab ----

    /// 构造一个 sheet：1 列（col0=raw），表头 + 多行数据（含有效/无效候选）。
    /// 返回 (session_id, sheet_id)。source_type 默认 "csv"（独立记录，行间插 \n）。
    fn setup_extract_sheet(db: &DbManager, values: &[&str]) -> (i64, i64) {
        setup_extract_sheet_with_type(db, values, "csv")
    }

    /// 同 setup_extract_sheet，但可指定 source_type。
    /// source_type="txt" 模拟 TXT 4096 字节分块（行间无分隔符，连续字符串片段）。
    fn setup_extract_sheet_with_type(
        db: &DbManager,
        values: &[&str],
        source_type: &str,
    ) -> (i64, i64) {
        let session_id = db
            .create_session("extract-test", None, source_type, 0)
            .unwrap();
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
        // 手机号：13412345678 / 15987654321 有效（11 位纯数字，空名单不过滤前缀）；
        // 1201234567a 无效（非纯数字）；134123456 长度不足；134123456789 超长。
        // 正则 `\b[1-9]\d{10}\b` 召回 11 位首位非零纯数字串，故 1201234567a /
        // 134123456 / 134123456789 不命中。默认空前缀列表不过滤前缀。
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
            &[],
            false,
        )
        .unwrap();

        // 2 个有效候选（正则召回 11 位首位非零纯数字，空名单不过滤前缀）。
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.valid));
        assert_eq!(rows[0].value, "13412345678");
        assert_eq!(rows[1].value, "15987654321");
        assert_eq!(rows[0].type_label, "手机号");
        // skipped = 无候选的行数（2 行：1201234567a / 134123456）。
        assert_eq!(parse_result.skipped, 2);
        // 新 Tab 只写有效候选，row_count = 有效行数 = 2。
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
    fn extract_validate_phone_non_standard_prefix() {
        // 非标准前缀（如 7xx）：默认空前缀列表不过滤前缀，11 位纯数字均通过；
        // 须通过运行时前缀白名单限定特定前缀。
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let values = ["79996258889", "78638972987", "13412345678"];
        let (session_id, sheet_id) = setup_extract_sheet(&db, &values);

        // 1) 默认空前缀列表：7x 号码也通过（不过滤前缀），全部有效。
        let (parse_result, rows) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            &["phone-extract".to_string()],
            session_id,
            None,
            &[],
            false,
        )
        .unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows[0].valid);   // 79996258889
        assert!(rows[1].valid);   // 78638972987
        assert!(rows[2].valid);   // 13412345678
        assert_eq!(parse_result.row_count, 3);

        // 2) 运行时白名单 ["799","786"]：7x 号码放行，134 被拒。
        let (parse_result2, rows2) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            &["phone-extract".to_string()],
            session_id,
            None,
            &["799".to_string(), "786".to_string()],
            false,
        )
        .unwrap();
        assert_eq!(rows2.len(), 3);
        assert!(rows2[0].valid);   // 79996258889
        assert!(rows2[1].valid);   // 78638972987
        assert!(!rows2[2].valid);  // 13412345678
        assert_eq!(parse_result2.row_count, 2);
    }

    #[test]
    fn extract_validate_phone_with_prefix_filter() {
        // phone-extract 运行时前缀白名单覆盖。DB 规则 params.allowed_prefixes
        // 为空（不过滤前缀），运行时传 ["134"] → 仅 134 开头候选有效，其余判无效。
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let values = ["13412345678", "15987654321"];
        let (session_id, sheet_id) = setup_extract_sheet(&db, &values);

        let (parse_result, rows) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            &["phone-extract".to_string()],
            session_id,
            None,
            &["134".to_string()],
            false,
        )
        .unwrap();

        // 2 个候选都被正则召回，但只有 134 开头通过前缀白名单。
        assert_eq!(rows.len(), 2);
        assert!(rows[0].valid);
        assert_eq!(rows[0].value, "13412345678");
        assert!(!rows[1].valid);
        assert_eq!(rows[1].value, "15987654321");
        // 新 Tab 只写有效候选，row_count = 1。
        assert_eq!(parse_result.row_count, 1);
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
            &[],
            false,
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

    // ---- 跨块边界提取（拼接修复）----

    /// IPv4 地址跨行（块）边界：TXT 导入 4096 字节分块后，IP "192.168.1.1"
    /// 被拆为 "...prefix 19"（row1 末尾）+ "2.168.1.1 suffix"（row2 开头）。
    /// 逐行提取：row1 无 IP；row2 找到 "2.168.1.1"（值错误）。
    /// 拼接提取：找到完整 "192.168.1.1"，source_row = 1（匹配起始在 row1）。
    #[test]
    fn extract_validate_ip_cross_chunk_boundary() {
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let values = ["prefix 19", "2.168.1.1 suffix"];
        let (session_id, sheet_id) = setup_extract_sheet_with_type(&db, &values, "txt");

        let (parse_result, rows) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            &["ip4-extract".to_string()],
            session_id,
            None,
            &[],
            false,
        )
        .unwrap();

        // 拼接后找到完整 192.168.1.1（而非截断的 2.168.1.1）。
        assert_eq!(rows.len(), 1);
        assert!(rows[0].valid);
        assert_eq!(rows[0].value, "192.168.1.1");
        assert_eq!(rows[0].source_row, 1); // 匹配起始在 row1
        assert_eq!(parse_result.row_count, 1);
        assert_eq!(parse_result.skipped, 1); // row2 无独立候选
    }

    /// 银行卡号跨行（块）边界：19 位卡号被拆为 13 位 + 6 位。
    /// 逐行提取：row1 找到 13 位 "6222021234567"（Luhn 失败）；
    /// row2 无候选。拼接提取：找到完整 19 位 "6222021234567890128"
    ///（Luhn 通过）。
    #[test]
    fn extract_validate_bankcard_cross_chunk_boundary() {
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let values = ["data 6222021234567", "890128 end"];
        let (session_id, sheet_id) = setup_extract_sheet_with_type(&db, &values, "txt");

        let (parse_result, rows) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            &["bankcard-extract".to_string()],
            session_id,
            None,
            &[],
            false,
        )
        .unwrap();

        // 拼接后找到完整 19 位 Luhn-valid 卡号，而非截断的 13 位。
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, "6222021234567890128");
        assert_eq!(rows[0].source_row, 1); // 匹配起始在 row1
        assert_eq!(parse_result.skipped, 1); // row2 无独立候选
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
            &[],
            false,
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
        // 每行用空格分隔，避免拼接后跨行边界产生伪 IPv6 匹配。
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let values = ["::1", " 2001:db8::1", " 1:2:3"];
        let (session_id, sheet_id) = setup_extract_sheet(&db, &values);

        let (parse_result, rows) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            &["ip6-extract".to_string()],
            session_id,
            None,
            &[],
            false,
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
            &[],
            false,
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
            &[],
            false,
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
        let err =
            extract_validate_to_new_sheet_inner(&db, sheet_id, "raw", &[], session_id, None, &[], false)
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
            &[],
            false,
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
            &[],
            false,
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
    fn extract_validate_idcard_leading_zero_not_recalled() {
        // 正则 `\b[1-9]\d{16}[\dXx]\b` 首位非零：首位为 0 的身份证号不被召回。
        // 01010519491231002X 首位为 0 → 不应出现在结果中。
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let values = [
            "11010519491231002X", // 首位 1，有效
            "01010519491231002X", // 首位 0，不应被召回
        ];
        let (session_id, sheet_id) = setup_extract_sheet(&db, &values);

        let (_parse_result, rows) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            &["idcard-extract".to_string()],
            session_id,
            None,
            &[],
            false,
        )
        .unwrap();

        // 只有首位为 1 的身份证号被召回，首位为 0 的被正则排除。
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value, "11010519491231002X");
    }

    #[test]
    fn extract_validate_idcard_leading_zero_runtime_override() {
        // v1.2.2：idcard_allow_leading_zero=true 时，临时用宽松正则
        // \b\d{17}[\dXx]\b 覆盖 DB pattern，首位为 0 的身份证号也能被召回。
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let values = [
            "11010519491231002X", // 首位 1，有效
            "01010519491231002X", // 首位 0，宽松正则可召回；校验码仍需通过
        ];
        let (session_id, sheet_id) = setup_extract_sheet(&db, &values);

        let (_parse_result, rows) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            &["idcard-extract".to_string()],
            session_id,
            None,
            &[],
            true, // idcard_allow_leading_zero=true → 宽松正则
        )
        .unwrap();

        // 首位为 0 的身份证号也被召回（校验码 X 仍通过 is_valid_idcard）。
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].value, "11010519491231002X");
        assert_eq!(rows[1].value, "01010519491231002X");
    }

    #[test]
    fn extract_validate_duplicate_rule_ids_deduplicated() {
        // 同一规则 ID 重复传入不应导致结果翻倍。
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let values = [
            "11010519491231002X",
            "110105194912310038",
        ];
        let (session_id, sheet_id) = setup_extract_sheet(&db, &values);

        let (_parse_result, rows) = extract_validate_to_new_sheet_inner(
            &db,
            sheet_id,
            "raw",
            // 故意重复同一规则 ID
            &[
                "idcard-extract".to_string(),
                "idcard-extract".to_string(),
            ],
            session_id,
            None,
            &[],
            false,
        )
        .unwrap();

        // 去重后每条身份证号只出现一次，不会翻倍。
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].value, "11010519491231002X");
        assert_eq!(rows[1].value, "110105194912310038");
    }

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
            &[],
            false,
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

    // ---- T68：多规则行级校验 → 双 Tab 集成测试 ----

    /// 构造一个 7 列 sheet：username/name/sex/birth/idcard/phone/address。
    /// 表头 + 每行数据；`rows` 为 7 元组切片。返回 (session_id, sheet_id)。
    fn setup_multi_rules_sheet(
        db: &DbManager,
        rows: &[(&str, &str, &str, &str, &str, &str, &str)],
    ) -> (i64, i64) {
        let session_id = db
            .create_session("multi-rules-test", None, "csv", 0)
            .unwrap();
        let sheet_id = db.create_sheet(session_id, "raw", 0).unwrap();
        let headers = [
            "username", "name", "sex", "birth", "idcard", "phone", "address",
        ];
        let mut cells: Vec<Cell> = Vec::with_capacity((rows.len() + 1) * 7);
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

    /// 3 条规则全映射，全有效行 → valid sheet 有 N 行，invalid sheet 0 行。
    #[test]
    fn validate_multi_rules_to_two_sheets_all_rules_pass() {
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        // 110105194912310038 → 男，19491231，与 sex=男 / birth=19491231 一致
        let rows = [
            (
                "admin",
                "张三",
                "男",
                "19491231",
                "110105194912310038",
                "13412345678",
                "北京市朝阳区1号101室",
            ),
            (
                "lufe1jian",
                "李四",
                "女",
                "19491231",
                "11010519491231002X",
                "15987654321",
                "北京市朝阳区1号101室",
            ),
        ];
        let (session_id, sheet_id) = setup_multi_rules_sheet(&db, &rows);
        let rules = vec![
            MultiRuleValidation {
                column: "username".into(),
                rule_id: "username-validate".into(),
                cross_field: None,
                params_override: None,
            },
            MultiRuleValidation {
                column: "idcard".into(),
                rule_id: "idcard-validate".into(),
                cross_field: Some(CrossFieldConfig {
                    check_sex: true,
                    sex_column: Some("sex".into()),
                    check_birth: true,
                    birth_column: Some("birth".into()),
                }),
                params_override: None,
            },
            MultiRuleValidation {
                column: "phone".into(),
                rule_id: "phone-validate".into(),
                cross_field: None,
                params_override: None,
            },
        ];
        let res = validate_multi_rules_to_two_sheets_inner(&db, sheet_id, session_id, &rules, &[])
            .unwrap();
        assert_eq!(res.valid_sheet.row_count, 2);
        assert_eq!(res.invalid_sheet.row_count, 0);
        assert!(res.invalid_reasons.is_empty());
        // 列数 = 7（保留原列）
        let valid_cells = db
            .query_cells(res.valid_sheet.new_sheet_id, 0, 100)
            .unwrap();
        let max_col = valid_cells.iter().map(|c| c.col_idx).max().unwrap();
        assert_eq!(max_col, 6);
    }

    /// 部分行有效部分无效 → 正确分流。
    #[test]
    fn validate_multi_rules_to_two_sheets_mixed_pass_fail() {
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let rows = [
            // 有效行
            (
                "admin",
                "张三",
                "男",
                "19491231",
                "110105194912310038",
                "13412345678",
                "北京市朝阳区1号101室",
            ),
            // 无效行：username 含非法字符（ab.cd）
            (
                "ab.cd",
                "李四",
                "女",
                "19491231",
                "11010519491231002X",
                "15987654321",
                "北京市朝阳区1号101室",
            ),
        ];
        let (session_id, sheet_id) = setup_multi_rules_sheet(&db, &rows);
        let rules = vec![
            MultiRuleValidation {
                column: "username".into(),
                rule_id: "username-validate".into(),
                cross_field: None,
                params_override: None,
            },
            MultiRuleValidation {
                column: "idcard".into(),
                rule_id: "idcard-validate".into(),
                cross_field: None,
                params_override: None,
            },
        ];
        let res = validate_multi_rules_to_two_sheets_inner(&db, sheet_id, session_id, &rules, &[])
            .unwrap();
        assert_eq!(res.valid_sheet.row_count, 1);
        assert_eq!(res.invalid_sheet.row_count, 1);
        assert_eq!(res.invalid_reasons.len(), 1);
        assert_eq!(res.invalid_reasons[0].field, "username");
        assert!(res.invalid_reasons[0].reason.contains("用户名"));
    }

    /// 跨字段：idcard 有效但 sex 列与 idcard 推断性别不一致 → invalid。
    #[test]
    fn validate_multi_rules_to_two_sheets_idcard_cross_field_sex_mismatch() {
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        // idcard 110105194912310038 → 第 17 位 3 奇 → 男；sex 列写「女」
        let rows = [(
            "admin",
            "张三",
            "女",
            "19491231",
            "110105194912310038",
            "13412345678",
            "北京市朝阳区1号101室",
        )];
        let (session_id, sheet_id) = setup_multi_rules_sheet(&db, &rows);
        let rules = vec![MultiRuleValidation {
            column: "idcard".into(),
            rule_id: "idcard-validate".into(),
            cross_field: Some(CrossFieldConfig {
                check_sex: true,
                sex_column: Some("sex".into()),
                check_birth: false,
                birth_column: None,
            }),
            params_override: None,
        }];
        let res = validate_multi_rules_to_two_sheets_inner(&db, sheet_id, session_id, &rules, &[])
            .unwrap();
        assert_eq!(res.invalid_sheet.row_count, 1);
        assert!(res
            .invalid_reasons
            .iter()
            .any(|r| r.field == "sex" && r.reason.contains("性别不一致")));
    }

    /// 跨字段：idcard 有效但 birth 与 idcard[6..14] 不一致 → invalid。
    #[test]
    fn validate_multi_rules_to_two_sheets_idcard_cross_field_birth_mismatch() {
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        // idcard 出生日期 19491231；birth 列写 20000101
        let rows = [(
            "admin",
            "张三",
            "男",
            "20000101",
            "110105194912310038",
            "13412345678",
            "北京市朝阳区1号101室",
        )];
        let (session_id, sheet_id) = setup_multi_rules_sheet(&db, &rows);
        let rules = vec![MultiRuleValidation {
            column: "idcard".into(),
            rule_id: "idcard-validate".into(),
            cross_field: Some(CrossFieldConfig {
                check_sex: false,
                sex_column: None,
                check_birth: true,
                birth_column: Some("birth".into()),
            }),
            params_override: None,
        }];
        let res = validate_multi_rules_to_two_sheets_inner(&db, sheet_id, session_id, &rules, &[])
            .unwrap();
        assert_eq!(res.invalid_sheet.row_count, 1);
        assert!(res
            .invalid_reasons
            .iter()
            .any(|r| r.field == "birth" && r.reason.contains("出生日期与身份证号不一致")));
    }

    /// 正则规则校验：用 name-validate（正则 `^[\u4e00-\u9fa5]{2,4}$`）校验列。
    #[test]
    fn validate_multi_rules_to_two_sheets_regex_rule_validation() {
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        // 张三（2 字，通过）/ Zhang（非中文，不通过）/ 诸葛亮（3 字，通过）
        let rows = [
            (
                "admin",
                "张三",
                "男",
                "19491231",
                "110105194912310038",
                "13412345678",
                "北京市朝阳区1号101室",
            ),
            (
                "admin2",
                "Zhang",
                "男",
                "19491231",
                "110105194912310038",
                "13412345678",
                "北京市朝阳区1号101室",
            ),
            (
                "admin3",
                "诸葛亮",
                "男",
                "19491231",
                "110105194912310038",
                "13412345678",
                "北京市朝阳区1号101室",
            ),
        ];
        let (session_id, sheet_id) = setup_multi_rules_sheet(&db, &rows);
        let rules = vec![MultiRuleValidation {
            column: "name".into(),
            rule_id: "name-validate".into(),
            cross_field: None,
            params_override: None,
        }];
        let res = validate_multi_rules_to_two_sheets_inner(&db, sheet_id, session_id, &rules, &[])
            .unwrap();
        // 张三 / 诸葛亮 通过，Zhang 不通过
        assert_eq!(res.valid_sheet.row_count, 2);
        assert_eq!(res.invalid_sheet.row_count, 1);
        assert_eq!(res.invalid_reasons.len(), 1);
        assert_eq!(res.invalid_reasons[0].field, "name");
        assert!(res.invalid_reasons[0].reason.contains("值不匹配规则"));
    }

    /// 只映射 1 条规则 → 单规则校验分流。
    #[test]
    fn validate_multi_rules_to_two_sheets_partial_mapping() {
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        let rows = [
            (
                "admin",
                "张三",
                "男",
                "19491231",
                "110105194912310038",
                "13412345678",
                "北京市朝阳区1号101室",
            ),
            (
                "ab.cd",
                "李四",
                "女",
                "19491231",
                "11010519491231002X",
                "15987654321",
                "北京市朝阳区1号101室",
            ),
        ];
        let (session_id, sheet_id) = setup_multi_rules_sheet(&db, &rows);
        let rules = vec![MultiRuleValidation {
            column: "username".into(),
            rule_id: "username-validate".into(),
            cross_field: None,
            params_override: None,
        }];
        let res = validate_multi_rules_to_two_sheets_inner(&db, sheet_id, session_id, &rules, &[])
            .unwrap();
        // admin 有效，ab.cd 无效
        assert_eq!(res.valid_sheet.row_count, 1);
        assert_eq!(res.invalid_sheet.row_count, 1);
        assert_eq!(res.invalid_reasons[0].field, "username");
        // 无 idcard 映射 → 无跨字段原因
        assert!(res
            .invalid_reasons
            .iter()
            .all(|r| !r.reason.contains("身份证")));
    }

    // ---- v1.1.4 续轮 T70：params_override + clean_birth 跨字段 ----

    /// generic-validate 规则行带 `paramsOverride`（前端编辑字符类 + 长度范围后下发）。
    /// override 优先于 DB rule.params，覆盖 DB 默认的 `{allow_digits:true,
    /// allow_letters:true, allow_special_chars:"", min_len:None, max_len:None}`。
    ///
    /// 构造 3 行 code 列：
    /// - "abc123" → 数字+字母，长度 6≥3 → 通过
    /// - "abc@123" → 含特殊字符 @（白名单为空）→ 不通过
    /// - "ab" → 长度 2<3 → 不通过
    #[test]
    fn validate_multi_rules_to_two_sheets_generic_with_params_override() {
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        // 1 列 code + 3 行数据。
        let session_id = db
            .create_session("generic-override-test", None, "csv", 0)
            .unwrap();
        let sheet_id = db.create_sheet(session_id, "raw", 0).unwrap();
        let headers = ["code"];
        let rows = ["abc123", "abc@123", "ab"];
        let mut cells: Vec<Cell> = Vec::with_capacity(rows.len() + 1);
        for (c, h) in headers.iter().enumerate() {
            cells.push(Cell {
                sheet_id,
                row_idx: 0,
                col_idx: c as u32,
                value: Some((*h).to_string()),
            });
        }
        for (i, v) in rows.iter().enumerate() {
            cells.push(Cell {
                sheet_id,
                row_idx: (i + 1) as u32,
                col_idx: 0,
                value: Some((*v).to_string()),
            });
        }
        db.write_cells(sheet_id, &cells).unwrap();

        let rules = vec![MultiRuleValidation {
            column: "code".into(),
            rule_id: "generic-validate".into(),
            cross_field: None,
            params_override: Some(ExtractParams::Generic {
                allow_digits: true,
                allow_letters: true,
                allow_special_chars: String::new(),
                min_len: Some(3),
                max_len: None,
            }),
        }];
        let res = validate_multi_rules_to_two_sheets_inner(&db, sheet_id, session_id, &rules, &[])
            .unwrap();
        // abc123 → valid
        assert_eq!(res.valid_sheet.row_count, 1);
        // abc@123 / ab → invalid
        assert_eq!(res.invalid_sheet.row_count, 2);
        assert_eq!(res.invalid_reasons.len(), 2);
        // 每条失败原因都指向 code 列 + generic 失败消息。
        assert!(res
            .invalid_reasons
            .iter()
            .all(|r| r.field == "code" && r.reason.contains("通用校验未通过")));
    }

    /// 跨字段 birth 比对支持分隔符格式（T70 E89）。
    /// birth="1949-12-31" 经 `clean_birth` 归一化为 "19491231"，
    /// 与 idcard[6..14]="19491231" 一致 → 整行 valid。
    #[test]
    fn validate_multi_rules_to_two_sheets_birth_with_separators() {
        let (_dir, db) = setup_db();
        db.seed_builtin_rules().unwrap();
        // birth 写带分隔符的 "1949-12-31"；idcard 110105194912310038 第 7-14 位 = 19491231。
        let rows = [(
            "admin",
            "张三",
            "男",
            "1949-12-31",
            "110105194912310038",
            "13412345678",
            "北京市朝阳区1号101室",
        )];
        let (session_id, sheet_id) = setup_multi_rules_sheet(&db, &rows);
        let rules = vec![MultiRuleValidation {
            column: "idcard".into(),
            rule_id: "idcard-validate".into(),
            cross_field: Some(CrossFieldConfig {
                check_sex: false,
                sex_column: None,
                check_birth: true,
                birth_column: Some("birth".into()),
            }),
            params_override: None,
        }];
        let res = validate_multi_rules_to_two_sheets_inner(&db, sheet_id, session_id, &rules, &[])
            .unwrap();
        // clean_birth("1949-12-31") = "19491231" == idcard[6..14] → 一致 → valid。
        assert_eq!(res.valid_sheet.row_count, 1);
        assert_eq!(res.invalid_sheet.row_count, 0);
        assert!(res.invalid_reasons.is_empty());
    }
}
