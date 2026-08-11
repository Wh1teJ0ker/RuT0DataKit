# TASK-T68-HANDOFF

```yaml
task_id: T68
goal: |
  新增后端 Tauri 命令 validate_multi_rules_to_two_sheets：接收多条「列名 + 规则 id」
  组合 + 身份证规则的跨字段配置（可选性别列/出生日期列 + 勾选比对），逐行逐规则
  校验（函数式走 validate_extracted，正则式走 RegexValidator），任一规则失败 →
  整行入 invalid，全通过 → 整行入 valid，结果分流到两个新 Tab（保留原列不新增列）。
  复用 TwoSheetResult / RowInvalidReason / ParseResult 结构 + 双 Tab 写出模式。

in_scope:
  - src-tauri/src/commands/processor.rs  # 新命令 + inner + 结构体 + 集成测试
  - src-tauri/src/lib.rs                 # generate_handler 注册新命令

out_of_scope:
  - 不改 crates/core/*（T67 已扩展规则系统）
  - 不改 frontend/*（T69 的工作）
  - 不改 src-tauri/src/db/*（复用现有 DB 方法）
  - 不改 validate_column 命令（保留）
  - 不改 validate_rows_to_two_sheets 命令（保留，不删除）
  - 不改 extract_validate_to_new_sheet 命令
  - 不改 schema.rs / migrate.rs

acceptance_criteria:
  - 新增 MultiRuleValidation 结构体（列名 + ruleId + 可选 crossField 配置）
  - 新增 validate_multi_rules_to_two_sheets_inner(db, sheet_id, session_id, rules, phone_prefixes) -> Result<TwoSheetResult, String>
  - 新增 #[tauri::command] validate_multi_rules_to_two_sheets 薄包装
  - lib.rs generate_handler 注册新命令
  - 逐行校验逻辑：对每条规则取该列 cell value → 按 rule.kind + rule.params 分发：
    - rule.kind=Validate + rule.params=Some → 调 validate_extracted(rule, value)
    - rule.kind=Validate + rule.params=None + rule.pattern=Some → 调 RegexValidator::validate(value, rule)
    - rule.kind=Extract + rule.params=Some → 调 validate_extracted(rule, value)（复用 extract 校验函数）
    - 其他 → 跳过（不校验）
  - 身份证跨字段（仅当 idcard 规则带 crossField 配置 + idcard 本身校验通过）：
    - 比对性别：normalize_gender(sex_col_val) vs idcard_gender(idcard_val)
    - 比对出生日期：idcard_val[6..14] == birth_col_val（且 birth 是 8 位数字）
  - 整行分流：任一规则失败 → 整行入 invalid + 收集 RowInvalidReason；全通过 → valid
  - 双 Tab 写出：create_sheet({name}_校验通过) + create_sheet({name}_校验失败)，写表头 + 数据行，保留原列
  - log_operation("validate_multi_rules_to_two_sheets", ...)
  - phone_prefixes 参数全局应用于 phone 类规则（phone-validate / phone-extract 的 PhonePrefix params）
  - 集成测试至少覆盖：_all_rules_pass / _mixed_pass_fail / _idcard_cross_field_sex_mismatch / _idcard_cross_field_birth_mismatch / _regex_rule_validation / _partial_mapping
  - cargo fmt --all 通过
  - cargo clippy --all-targets --all-features -- -D warnings 通过
  - cargo test --all 通过

verification_commands:
  - cargo fmt --all
  - cargo clippy --all-targets --all-features -- -D warnings
  - cargo test --all

files_likely_to_change:
  - src-tauri/src/commands/processor.rs
  - src-tauri/src/lib.rs

risks:
  - validate_extracted 返回 (bool, String)，IdCard 分支返回 (true, gender_string) — gender 字符串用于跨字段比对
  - phone 类规则的整串校验：validate_extracted 的 PhonePrefix 分支只查前缀不查 11 位/1 开头。T68 新命令对 phone-validate 规则应直接调 is_valid_phone(value, phone_prefixes) 而非走 validate_extracted 的 PhonePrefix 分支（或在新命令里先做 is_valid_phone 再做 check_phone_prefix，两者都查）
  - 跨字段配置：前端传 { sexColumn: Option<String>, birthColumn: Option<String>, checkSex: bool, checkBirth: bool } 附在身份证规则条目上
  - SQL 全部参数化绑定（复用现有 DB 方法，均已参数化）

depends_on: [T67]
status: planned
```

## 详细规格

### 1. 新增结构体（processor.rs）

```rust
/// 单条校验规则的跨字段配置（仅 idcard 规则用）。
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CrossFieldConfig {
    pub check_sex: bool,           // 是否比对性别一致性
    pub sex_column: Option<String>, // 性别列名
    pub check_birth: bool,         // 是否比对出生日期一致性
    pub birth_column: Option<String>, // 出生日期列名
}

/// 一条「列 + 规则」校验组合。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiRuleValidation {
    pub column: String,            // 要校验的列名
    pub rule_id: String,           // 校验规则 id
    #[serde(default)]
    pub cross_field: Option<CrossFieldConfig>, // 跨字段配置（仅 idcard 规则）
}
```

### 2. inner 函数签名

```rust
fn validate_multi_rules_to_two_sheets_inner(
    db: &DbManager,
    sheet_id: i64,
    session_id: i64,
    rules: &[MultiRuleValidation],
    phone_prefixes: &[String],
) -> Result<TwoSheetResult, String>
```

### 3. 校验分发逻辑（逐行逐规则）

对每行 row_vals（Vec<Option<String>>，按 col_idx 对齐到 col_count）：

```rust
let mut row_invalid_reasons: Vec<(String, String)> = Vec::new();
let mut idcard_valid = false;
let mut idcard_val = String::new();
let mut cross_field_config: Option<&CrossFieldConfig> = None;

for rv in rules {
    let col_idx = /* find_col_idx(rv.column) */;
    let value = row_vals.get(col_idx).cloned().flatten().unwrap_or_default();
    let rule = /* db.get_rule(&rv.rule_id) */;

    // 分发校验
    let (passed, msg) = if rule.id == "phone-validate" {
        // phone-validate 特殊处理：整串校验
        let ok = is_valid_phone(&value, phone_prefixes);
        (ok, if ok { String::new() } else { "手机号须为 11 位、1 开头".into() })
    } else if rule.params.is_some() {
        // 函数式校验（validate_extracted 分发）
        validate_extracted(&rule, &value)
    } else if let Some(ref pattern) = rule.pattern {
        // 正则式校验（RegexValidator）
        let re = Regex::new(pattern).map_err(...)?;
        let ok = re.is_match(&value);
        (ok, if ok { String::new() } else { format!("值不匹配规则 {}", rule.name) })
    } else {
        (true, String::new()) // 无校验条件，默认通过
    };

    if !passed {
        row_invalid_reasons.push((rv.column.clone(), msg));
    }

    // 记录 idcard 状态用于跨字段
    if rule.id == "idcard-validate" || rule.params == Some(ExtractParams::IdCard) {
        idcard_valid = passed;
        idcard_val = value.clone();
        cross_field_config = rv.cross_field.as_ref();
    }
}

// 跨字段联合校验（仅当 idcard 有效 + 配置了 cross_field）
if idcard_valid {
    if let Some(cf) = cross_field_config {
        if cf.check_sex {
            if let Some(ref sex_col) = cf.sex_column {
                let sex_val = /* 取 sex_col 列值 */;
                if let Some(norm) = normalize_gender(&sex_val) {
                    if let Some(inferred) = idcard_gender(&idcard_val) {
                        if norm != inferred {
                            row_invalid_reasons.push((sex_col.clone(), format!("性别不一致: 身份证推断{inferred}，性别列{sex_val}")));
                        }
                    }
                }
            }
        }
        if cf.check_birth {
            if let Some(ref birth_col) = cf.birth_column {
                let birth_val = /* 取 birth_col 列值 */;
                if is_valid_birth(&birth_val) {
                    let idcard_birth = &idcard_val[6..14];
                    if birth_val != idcard_birth {
                        row_invalid_reasons.push((birth_col.clone(), format!("出生日期与身份证号不一致: 身份证{idcard_birth}，出生日期{birth_val}")));
                    }
                }
            }
        }
    }
}
```

### 4. 双 Tab 写出（复用 validate_rows_to_two_sheets_inner L927-981 模式）

- `get_sheet_name(sheet_id)` → 源 sheet 名
- `create_sheet(session_id, "{name}_校验通过", 0)` + `create_sheet(session_id, "{name}_校验失败", 0)`
- `write_sheet` 闭包：表头 row_idx=0，数据行从 row_idx=1 起，列数 = headers.len()
- `log_operation("validate_multi_rules_to_two_sheets", ...)`
- 返回 `TwoSheetResult { valid_sheet, invalid_sheet, invalid_reasons }`

### 5. 集成测试（processor.rs tests mod）

- `_all_rules_pass`：3 条规则全映射，全有效行 → valid sheet 有 N 行，invalid 0 行
- `_mixed_pass_fail`：部分行有效部分无效 → 正确分流
- `_idcard_cross_field_sex_mismatch`：idcard 有效但 sex 列与 idcard 推断性别不一致 → invalid，reason 含"性别不一致"
- `_idcard_cross_field_birth_mismatch`：idcard 有效但 birth 与 idcard[6..14] 不一致 → invalid
- `_regex_rule_validation`：用 name-validate（正则）规则校验列 → 正确匹配/不匹配
- `_partial_mapping`：只映射 1 条规则 → 单规则校验分流

### 6. lib.rs 注册

在 `generate_handler!` 列表中 `validate_rows_to_two_sheets` 后追加 `commands::processor::validate_multi_rules_to_two_sheets,`
