//! E2E 集成测试：覆盖 `docs/00-需求文档.md` §4 全部验收项。
//!
//! v0.1.0 重构后：预置别名全部删除（用户要求「只做规则模版」），所有规则
//! 均通过 4 个通用脱敏算子（`template` / `split_template` / `regex_replace` /
//! `const_replace`）+ 3 个通用校验算子（`regex` / `algorithm` /
//! `regex_with_guard`）+ 显式 params 配置。本文件不再引用任何预置别名。
//!
//! 测试函数清单（重构后）：
//! 1. `csv_mask_fields`：sample_mask.csv + default_mask → 6 列两行字段断言。
//! 2. `xlsx_mask_same_as_csv`：sample_mask.xlsx + default_mask → 与 csv 一致。
//! 3. `detect_type_csv_xlsx`：detect_type 命中 Csv / Xlsx。
//! 4. `headers_not_masked`：mask_pipeline 后表头不变。
//! 5. `short_input_passthrough`：短于阈值输入原样返回、不 panic。
//! 6. `custom_example_loadable_and_applied`：custom_example.yaml 可加载并应用。
//! 7. `rich_rule_maskers_contract`：3 种通用脱敏算子契约（regex_replace /
//!    const_replace 等）。
//! 8. `mask_pipeline_selected_rows_only`：mask_pipeline_selected 只对选中行脱敏。
//! 9. `mask_pipeline_selected_ignores_out_of_range`：越界索引静默忽略。
//! 10. `rich_rules_on_sample_csv`：富规则综合（regex_replace + const_replace）
//!    应用到 sample_mask.csv。
//! 11. `selected_rows_on_sample_csv`：对 sample_mask.csv 选第 1 行应用 phone
//!    模板。
//! 12. `mask_pipeline_columns_only_selected`：列勾选只对 selected_columns 脱敏。
//! 13. `mask_pipeline_columns_ignores_unknown`：列勾选忽略未知列名。
//! 14. `mask_pipeline_columns_empty_set`：列勾选空集原样输出。
//! 15. `validate_pipeline_basic`：单字段 validator 逐 cell 校验。
//! 16. `validate_pipeline_no_validators`：空 validators 全 true。
//! 17. `mask_pipeline_columns_and_selected_coexist`：列勾选 + 行勾选共存。
//! 18. `validate_pipeline_edge_cases`：validator 声明不存在 field 静默跳过。
//! 19. `mask_op_template_preset_equivalence`：template 算子各场景等价性。
//! 20. `mask_op_regex_replace_dispatch`：regex_replace 算子契约 + 算子清单。
//! 21. `validate_op_regex_preset_equivalence`：regex 算子契约 + 算子清单。
//! 22. `mask_op_split_template_and_const_replace`：split_template / const_replace
//!    通用算子契约。
//! 23. `validate_op_algorithm_and_guard_equivalence`：algorithm + regex_with_guard
//!    算子契约 + apply/build 等价。
//!
//! v0.2.0 日志切片新增（T2-6）：
//! 24. `log_parse_full`：LogReader 读 access.log，断言 1860 行全解析 + line_no 连续。
//! 25. `log_scan_full`：SignatureEngine + DefaultSensitiveScan + 默认 RuleSet 跑
//!    scan_log，断言 Report.kind=="log_scan"、SQLi > 1000、弱口令 ≥ 4、
//!    4 类新签名（union/extractvalue/sleep/tautology）各 ≥ 1 命中。
//! 26. `log_signatures_6_categories`：6 条 LogEntry 各触发 1 类签名，各配反例。
//!
//! v0.2.1 语义增强新增（T2-11）：
//! 27. `log_payload_parser_e2e`：构造盲注 entry → scan_log_entry → 断言
//!    hit.parsed_payload.read_target == Some("database()")。
//! 28. `log_decoded_query_plus_decode`：构造 entry query 含 `+` → parse_line →
//!    断言 decoded_query 含空格而非 `+`。

mod common;

use std::collections::HashSet;

use ruT0_data_kit_core::log::LogReader;
use ruT0_data_kit_core::logsign::SignatureEngine;
use ruT0_data_kit_core::pipeline::detect_type;
use ruT0_data_kit_core::pipeline::columns::mask_pipeline_columns;
use ruT0_data_kit_core::pipeline::log_scan::scan_log;
use ruT0_data_kit_core::pipeline::mask::mask_pipeline;
use ruT0_data_kit_core::pipeline::mask::mask_pipeline_selected;
use ruT0_data_kit_core::pipeline::validate::validate_pipeline;
use ruT0_data_kit_core::pipeline::SourceType;
use ruT0_data_kit_core::readers::{CsvReader, Records, SourceReader, XlsxReader};
use ruT0_data_kit_core::rules::{
    load_default_mask_ruleset, load_ruleset, FieldRule, MaskRule, RuleSet,
};
use ruT0_data_kit_core::scan::DefaultSensitiveScan;
use serde_yml::Value;
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────
// 辅助：各通用算子的 params 构造（等价旧预置别名行为，便于复用）
// ─────────────────────────────────────────────────────────────────────

/// 构造 phone 模板 params（等价旧 `phone_mask` 预置）：keep 3+4，11 位 guard。
fn phone_template_params() -> HashMap<String, Value> {
    let mut p = HashMap::new();
    p.insert("keep_prefix".into(), Value::Number(3.into()));
    p.insert("keep_suffix".into(), Value::Number(4.into()));
    p.insert("mask_char".into(), Value::String("*".into()));
    p.insert("mask_min_len".into(), Value::Number(4.into()));
    p.insert("min_len".into(), Value::Number(11.into()));
    p.insert("max_len".into(), Value::Number(11.into()));
    p.insert("cjk".into(), Value::Bool(false));
    p
}

/// 构造中文姓名模板 params（等价旧 `name_mask` 预置）：cjk=true，1 字原样、
/// 2 字 `首*`、>=3 字 `首+*(n-2)+末`。
fn name_template_params() -> HashMap<String, Value> {
    let mut p = HashMap::new();
    p.insert("keep_prefix".into(), Value::Number(1.into()));
    p.insert("keep_suffix".into(), Value::Number(1.into()));
    p.insert("mask_char".into(), Value::String("*".into()));
    p.insert("mask_min_len".into(), Value::Number(1.into()));
    p.insert("cjk".into(), Value::Bool(true));
    p
}

/// 构造身份证模板 params（等价旧 `idcard_mask` 预置）：keep 6+4，18 位 guard。
fn idcard_template_params() -> HashMap<String, Value> {
    let mut p = HashMap::new();
    p.insert("keep_prefix".into(), Value::Number(6.into()));
    p.insert("keep_suffix".into(), Value::Number(4.into()));
    p.insert("mask_char".into(), Value::String("*".into()));
    p.insert("mask_min_len".into(), Value::Number(8.into()));
    p.insert("min_len".into(), Value::Number(18.into()));
    p.insert("max_len".into(), Value::Number(18.into()));
    p.insert("cjk".into(), Value::Bool(false));
    p
}

/// 构造银行卡模板 params（等价旧 `bankcard_mask` 预置）：keep 6+4，
/// 长度 16~19 guard。
fn bankcard_template_params() -> HashMap<String, Value> {
    let mut p = HashMap::new();
    p.insert("keep_prefix".into(), Value::Number(6.into()));
    p.insert("keep_suffix".into(), Value::Number(4.into()));
    p.insert("mask_char".into(), Value::String("*".into()));
    p.insert("mask_min_len".into(), Value::Number(4.into()));
    p.insert("min_len".into(), Value::Number(16.into()));
    p.insert("max_len".into(), Value::Number(19.into()));
    p.insert("cjk".into(), Value::Bool(false));
    p
}

/// 构造客户号模板 params（等价旧 `customer_id_mask` 预置）：keep 1+0，
/// 至少 2 字 guard。
fn customer_id_template_params() -> HashMap<String, Value> {
    let mut p = HashMap::new();
    p.insert("keep_prefix".into(), Value::Number(1.into()));
    p.insert("keep_suffix".into(), Value::Number(0.into()));
    p.insert("mask_char".into(), Value::String("*".into()));
    p.insert("mask_min_len".into(), Value::Number(4.into()));
    p.insert("min_len".into(), Value::Number(2.into()));
    p.insert("cjk".into(), Value::Bool(false));
    p
}

/// 构造邮箱切分模板 params（等价旧 `email_mask` 预置）：
/// separator=@、segment_index=0、内层 keep 1+1 cjk=true。
fn email_split_template_params() -> HashMap<String, Value> {
    let mut p = HashMap::new();
    p.insert("separator".into(), Value::String("@".into()));
    p.insert("segment_index".into(), Value::Number(0.into()));
    p.insert("keep_prefix".into(), Value::Number(1.into()));
    p.insert("keep_suffix".into(), Value::Number(1.into()));
    p.insert("mask_char".into(), Value::String("*".into()));
    p.insert("mask_min_len".into(), Value::Number(1.into()));
    p.insert("cjk".into(), Value::Bool(true));
    p
}

/// 构造 regex 校验算子 params：邮箱正则。
fn email_regex_params() -> HashMap<String, Value> {
    let mut p = HashMap::new();
    p.insert(
        "pattern".into(),
        Value::String(r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$".into()),
    );
    p.insert("message".into(), Value::String("email format invalid".into()));
    p
}

/// 构造 algorithm 校验算子 params：algo=idcard。
fn idcard_algorithm_params() -> HashMap<String, Value> {
    let mut p = HashMap::new();
    p.insert("algo".into(), Value::String("idcard".into()));
    p
}

/// 构造 regex_with_guard 校验算子 params：phone 号段白名单 + 11 位正则。
fn phone_guard_params() -> HashMap<String, Value> {
    let mut p = HashMap::new();
    p.insert("guard".into(), Value::String("phone".into()));
    p.insert("pattern".into(), Value::String(r"^\d{11}$".into()));
    p.insert("prefix_set".into(), Value::String("real".into()));
    p
}

// ─────────────────────────────────────────────────────────────────────
// 测试用例
// ─────────────────────────────────────────────────────────────────────

/// 读 sample_mask.csv + default_mask 后跑 mask_pipeline，返回脱敏结果。
fn mask_csv_with_default() -> Vec<Vec<String>> {
    let records = CsvReader::new()
        .read(&common::csv_path())
        .expect("read sample_mask.csv");
    let rules = load_default_mask_ruleset().expect("load default_mask.yaml");
    let result = mask_pipeline(&records, &rules).expect("mask_pipeline");
    result.masked.rows
}

/// 验收项 1：sample_mask.csv + default_mask 字段值断言。
#[test]
fn csv_mask_fields() {
    let rows = mask_csv_with_default();
    assert_eq!(rows.len(), 2, "expected 2 data rows");

    // 第 0 行：12345678 / 张三 / 110101199001011234 / 13812345678 /
    //         zhangsan@example.com / 6225887654321098
    assert_eq!(rows[0][0], "1*******");
    assert_eq!(rows[0][1], "张*");
    assert_eq!(rows[0][2], "110101********1234");
    assert_eq!(rows[0][3], "138****5678");
    assert_eq!(rows[0][4], "z******n@example.com");
    assert_eq!(rows[0][5], "622588******1098");

    // 第 1 行：87654321 / 李四海 / 110101198505051234 / 13987654321 /
    //         lisi@x.cn / 6225881111222233
    assert_eq!(rows[1][0], "8*******");
    assert_eq!(rows[1][1], "李*海");
    assert_eq!(rows[1][2], "110101********1234");
    assert_eq!(rows[1][3], "139****4321");
    assert_eq!(rows[1][4], "l**i@x.cn");
    assert_eq!(rows[1][5], "622588******2233");
}

/// 验收项 2：sample_mask.xlsx + default_mask 与 csv 结果一致。
#[test]
fn xlsx_mask_same_as_csv() {
    let xlsx_records = XlsxReader::new()
        .read(&common::xlsx_path())
        .expect("read sample_mask.xlsx");
    let rules = load_default_mask_ruleset().expect("load default_mask.yaml");
    let xlsx_result = mask_pipeline(&xlsx_records, &rules).expect("mask_pipeline xlsx");

    let csv_rows = mask_csv_with_default();

    assert_eq!(xlsx_result.masked.rows.len(), csv_rows.len());
    for (i, (xlsx_row, csv_row)) in xlsx_result
        .masked
        .rows
        .iter()
        .zip(csv_rows.iter())
        .enumerate()
    {
        assert_eq!(xlsx_row, csv_row, "xlsx row {i} differs from csv row {i}");
    }
}

/// 验收项 3：detect_type 对 .csv / .xlsx 命中对应模块。
#[test]
fn detect_type_csv_xlsx() {
    assert_eq!(
        detect_type(&common::csv_path()).expect("detect csv"),
        SourceType::Csv
    );
    assert_eq!(
        detect_type(&common::xlsx_path()).expect("detect xlsx"),
        SourceType::Xlsx
    );
}

/// 验收项 4：表头不脱敏。
#[test]
fn headers_not_masked() {
    let records = CsvReader::new()
        .read(&common::csv_path())
        .expect("read sample_mask.csv");
    let original_headers = records.headers.clone();
    let rules = load_default_mask_ruleset().expect("load default_mask.yaml");
    let result = mask_pipeline(&records, &rules).expect("mask_pipeline");
    assert_eq!(result.masked.headers, original_headers);
    assert_eq!(
        result.masked.headers,
        vec![
            "customer_id".to_string(),
            "name".to_string(),
            "id_card".to_string(),
            "phone".to_string(),
            "email".to_string(),
            "bank_card".to_string(),
        ]
    );
}

/// 验收项 5：短于阈值输入原样返回、不 panic。
#[test]
fn short_input_passthrough() {
    use ruT0_data_kit_core::readers::Records;
    use ruT0_data_kit_core::rules::MaskRule;

    // phone="138"（3 字，短于 11 位阈值）+ email="" 空串。
    let records = Records {
        headers: vec!["phone".to_string()],
        rows: vec![
            vec!["138".to_string()],
            vec!["".to_string()],
        ],
    };
    // v0.1.0 重构后无预置别名：用 template 算子 + phone 模板 params 复现
    // 旧 phone_mask 行为（min_len=max_len=11 guard）。
    let rules = RuleSet {
        validators: vec![],
        maskers: vec![MaskRule {
            field: "phone".to_string(),
            masker: "template".to_string(),
            params: Some(phone_template_params()),
            description: None,
        }],
    };
    let result = mask_pipeline(&records, &rules).expect("mask_pipeline");
    assert_eq!(result.masked.rows[0][0], "138", "短于阈值原样返回");
    assert_eq!(result.masked.rows[1][0], "", "空串原样返回");
    assert_eq!(result.masked.headers, vec!["phone".to_string()]);
}

/// 验收项 6：custom_example.yaml 可加载并应用到 sample_mask.csv。
#[test]
fn custom_example_loadable_and_applied() {
    let rules = load_ruleset(&common::custom_example_path())
        .expect("load custom_example.yaml");
    let records = CsvReader::new()
        .read(&common::csv_path())
        .expect("read sample_mask.csv");
    let result = mask_pipeline(&records, &rules).expect("mask_pipeline");

    // customer_id `12345678`（8 字）：keep 2+2，mask 4 个 #
    // → "12####78"
    // customer_id `87654321`（8 字）→ "87####21"
    // phone `13812345678`（11 字）：keep 3+4，mask 4 个 *
    // → "138****5678"
    // phone `13987654321`（11 字）→ "139****4321"
    assert_eq!(result.masked.rows[0][0], "12####78");
    assert_eq!(result.masked.rows[1][0], "87####21");
    assert_eq!(result.masked.rows[0][3], "138****5678");
    assert_eq!(result.masked.rows[1][3], "139****4321");

    // custom_example.yaml 只声明 customer_id 与 phone 两个字段，
    // 其余列应原样输出。
    assert_eq!(result.masked.rows[0][1], "张三");
    assert_eq!(result.masked.rows[0][2], "110101199001011234");
    assert_eq!(result.masked.rows[0][4], "zhangsan@example.com");
    assert_eq!(result.masked.rows[0][5], "6225887654321098");
}

/// 验收项 7：3 种通用脱敏算子各自的参数契约与输出。
///
/// v0.1.0 重构后：`regex_extract` / `delete` / `replace` 旧名不再支持，
/// 全部归并为通用算子：
/// - `regex_replace` + `match_mode=first` + `replacement="$0"` → 等价 regex_extract
/// - `const_replace` + `with=""` → 等价 delete
/// - `const_replace` + `with="REDACTED"` → 等价 replace
#[test]
fn rich_rule_maskers_contract() {
    use ruT0_data_kit_core::rules::build_masker;

    fn params(p: &[(&str, &str)]) -> HashMap<String, Value> {
        p.iter()
            .map(|(k, v)| (k.to_string(), Value::String((*v).to_string())))
            .collect()
    }

    // regex_replace: pattern="(\d{3})\d{4}(\d{4})" replacement="$1****$2"
    // 对 "13812345678" → "138****5678"
    let r = MaskRule {
        field: "x".into(),
        masker: "regex_replace".into(),
        params: Some(params(&[
            ("pattern", r"(\d{3})\d{4}(\d{4})"),
            ("replacement", "$1****$2"),
        ])),
        description: None,
    };
    let m = build_masker(&r).expect("regex_replace masker");
    assert_eq!(m.mask("13812345678"), "138****5678");

    // regex_replace + match_mode=first + replacement="$0" → 提取首个匹配
    // （等价旧 regex_extract）：对 "tel:13812345678" → "13812345678"
    let r = MaskRule {
        field: "x".into(),
        masker: "regex_replace".into(),
        params: Some(params(&[
            ("pattern", r"\d{11}"),
            ("replacement", "$0"),
            ("match_mode", "first"),
        ])),
        description: None,
    };
    let m = build_masker(&r).expect("regex_replace (first) masker");
    assert_eq!(m.mask("tel:13812345678"), "13812345678");
    // 无匹配返回原值
    assert_eq!(m.mask("no digits"), "no digits");

    // const_replace + with="" → 等价 delete：对任意输入返回 ""
    let r = MaskRule {
        field: "x".into(),
        masker: "const_replace".into(),
        params: Some(params(&[("with", "")])),
        description: None,
    };
    let m = build_masker(&r).expect("const_replace (empty) masker");
    assert_eq!(m.mask("anything"), "");
    assert_eq!(m.mask(""), "");

    // const_replace + with="REDACTED" → 等价 replace
    let r = MaskRule {
        field: "x".into(),
        masker: "const_replace".into(),
        params: Some(params(&[("with", "REDACTED")])),
        description: None,
    };
    let m = build_masker(&r).expect("const_replace (REDACTED) masker");
    assert_eq!(m.mask("anything"), "REDACTED");
    assert_eq!(m.mask(""), "REDACTED");
}

/// 验收项 8：mask_pipeline_selected 只对选中行脱敏，未选中行原样。
#[test]
fn mask_pipeline_selected_rows_only() {
    // 3 行数据，选中第 0、2 行应用 phone 模板。
    let records = Records {
        headers: vec!["phone".to_string()],
        rows: vec![
            vec!["13812345678".to_string()],
            vec!["13912345678".to_string()],
            vec!["13712345678".to_string()],
        ],
    };
    let rules = RuleSet {
        validators: vec![],
        maskers: vec![MaskRule {
            field: "phone".to_string(),
            masker: "template".to_string(),
            params: Some(phone_template_params()),
            description: None,
        }],
    };
    let mut selected: HashSet<usize> = HashSet::new();
    selected.insert(0);
    selected.insert(2);

    let r = mask_pipeline_selected(&records, &rules, &selected).expect("mask_pipeline_selected");
    assert_eq!(r.summary.total_rows, 3);
    assert_eq!(r.summary.masked_rows, 2);
    // 选中行被脱敏
    assert_eq!(r.masked.rows[0][0], "138****5678");
    // 未选中行原样
    assert_eq!(r.masked.rows[1][0], "13912345678");
    // 选中行被脱敏
    assert_eq!(r.masked.rows[2][0], "137****5678");
    // 表头不变
    assert_eq!(r.masked.headers, vec!["phone".to_string()]);
    // by_field 只统计选中行
    assert_eq!(*r.summary.by_field.get("phone").unwrap_or(&0), 2);
}

/// 验收项 9：mask_pipeline_selected 对越界索引静默忽略，不 panic。
#[test]
fn mask_pipeline_selected_ignores_out_of_range() {
    let records = Records {
        headers: vec!["phone".to_string()],
        rows: vec![vec!["13812345678".to_string()]],
    };
    let rules = RuleSet {
        validators: vec![],
        maskers: vec![MaskRule {
            field: "phone".to_string(),
            masker: "template".to_string(),
            params: Some(phone_template_params()),
            description: None,
        }],
    };
    let mut selected: HashSet<usize> = HashSet::new();
    selected.insert(99);

    let r = mask_pipeline_selected(&records, &rules, &selected).expect("mask_pipeline_selected");
    assert_eq!(r.summary.masked_rows, 0);
    assert_eq!(r.masked.rows[0][0], "13812345678");
}

/// 验收项 10：富规则综合应用到 sample_mask.csv。
///
/// 同时挂两条通用脱敏规则到同一份 sample_mask.csv：
/// - `regex_replace` 挂 phone 列：`(\d{3})\d{4}(\d{4})` → `$1****$2`
///   （11 位手机号 → `138****5678`）
/// - `const_replace` 挂 email 列：with="" 清空
/// - `const_replace` 挂 name 列：with="***" 替换为固定字符串
///
/// 验证 mask_pipeline 端到端能把多条通用算子同时应用到真实样本，
/// 并保证未挂规则的列原样输出。
#[test]
fn rich_rules_on_sample_csv() {
    use ruT0_data_kit_core::rules::build_masker;

    fn params(p: &[(&str, &str)]) -> HashMap<String, Value> {
        p.iter()
            .map(|(k, v)| (k.to_string(), Value::String((*v).to_string())))
            .collect()
    }

    let records = CsvReader::new()
        .read(&common::csv_path())
        .expect("read sample_mask.csv");
    let rules = RuleSet {
        validators: vec![],
        maskers: vec![
            MaskRule {
                field: "phone".into(),
                masker: "regex_replace".into(),
                params: Some(params(&[
                    ("pattern", r"(\d{3})\d{4}(\d{4})"),
                    ("replacement", "$1****$2"),
                ])),
                description: None,
            },
            MaskRule {
                field: "email".into(),
                masker: "const_replace".into(),
                params: Some(params(&[("with", "")])),
                description: None,
            },
            MaskRule {
                field: "name".into(),
                masker: "const_replace".into(),
                params: Some(params(&[("with", "***")])),
                description: None,
            },
        ],
    };
    let r = mask_pipeline(&records, &rules).expect("mask_pipeline");

    // 第 0 行：phone 13812345678 → 138****5678
    //         email → ""（const_replace with=""）
    //         name 张三 → "***"（const_replace with="***"）
    assert_eq!(r.masked.rows[0][3], "138****5678", "phone regex_replace");
    assert_eq!(r.masked.rows[0][4], "", "email const_replace empty");
    assert_eq!(r.masked.rows[0][1], "***", "name const_replace ***");

    // 第 1 行：phone 13987654321 → 139****4321
    assert_eq!(r.masked.rows[1][3], "139****4321");
    assert_eq!(r.masked.rows[1][4], "");
    assert_eq!(r.masked.rows[1][1], "***");

    // 未挂规则的列原样输出：customer_id / id_card / bank_card
    assert_eq!(r.masked.rows[0][0], "12345678");
    assert_eq!(r.masked.rows[0][2], "110101199001011234");
    assert_eq!(r.masked.rows[0][5], "6225887654321098");

    // 确认三个字段都被计入 by_field（2 行 × 3 字段 = 6 次调用，每字段 2 次）
    assert_eq!(*r.summary.by_field.get("phone").unwrap(), 2);
    assert_eq!(*r.summary.by_field.get("email").unwrap(), 2);
    assert_eq!(*r.summary.by_field.get("name").unwrap(), 2);
    assert_eq!(r.summary.masked_rows, 2);

    // 顺手验证 build_masker 对这三条规则都能构造成功（与 mask_pipeline 一致）。
    for rule in &rules.maskers {
        assert!(build_masker(rule).is_some(), "build_masker ok for {}", rule.masker);
    }
}

/// 验收项 11：mask_pipeline_selected 对 sample_mask.csv 只对选中行脱敏。
///
/// 选第 1 行（0-based index 1），应用 phone 模板：第 0 行 phone 原样、
/// 第 1 行 phone 脱敏，masked_rows=1。
#[test]
fn selected_rows_on_sample_csv() {
    let records = CsvReader::new()
        .read(&common::csv_path())
        .expect("read sample_mask.csv");
    let rules = RuleSet {
        validators: vec![],
        maskers: vec![MaskRule {
            field: "phone".to_string(),
            masker: "template".to_string(),
            params: Some(phone_template_params()),
            description: None,
        }],
    };
    let selected: HashSet<usize> = [1].iter().copied().collect();

    let r = mask_pipeline_selected(&records, &rules, &selected).expect("mask_pipeline_selected");

    // 未选中行原样
    assert_eq!(r.masked.rows[0][3], "13812345678", "row 0 phone untouched");
    // 选中行被脱敏
    assert_eq!(r.masked.rows[1][3], "139****4321", "row 1 phone masked");
    // 汇总
    assert_eq!(r.summary.total_rows, 2);
    assert_eq!(r.summary.masked_rows, 1);
    assert_eq!(*r.summary.by_field.get("phone").unwrap(), 1);
    // 表头不变
    assert_eq!(
        r.masked.headers,
        vec![
            "customer_id".to_string(),
            "name".to_string(),
            "id_card".to_string(),
            "phone".to_string(),
            "email".to_string(),
            "bank_card".to_string(),
        ]
    );
}

/// 验收项 12：mask_pipeline_columns 只对 selected_columns 中的规则列脱敏。
///
/// 规则声明 phone + name 两个字段；selected_columns 只选 phone。验证 phone
/// 被脱敏、name 原样、masked_rows=2、by_field 只含 phone。
#[test]
fn mask_pipeline_columns_only_selected() {
    let records = Records {
        headers: vec!["phone".to_string(), "name".to_string()],
        rows: vec![
            vec!["13812345678".to_string(), "张三".to_string()],
            vec!["13987654321".to_string(), "李四海".to_string()],
        ],
    };
    let rules = RuleSet {
        validators: vec![],
        maskers: vec![
            MaskRule {
                field: "phone".into(),
                masker: "template".into(),
                params: Some(phone_template_params()),
                description: None,
            },
            MaskRule {
                field: "name".into(),
                masker: "template".into(),
                params: Some(name_template_params()),
                description: None,
            },
        ],
    };
    let mut selected: HashSet<String> = HashSet::new();
    selected.insert("phone".to_string());

    let r = mask_pipeline_columns(&records, &rules, &selected).expect("mask_pipeline_columns");
    // phone 被脱敏
    assert_eq!(r.masked.rows[0][0], "138****5678");
    assert_eq!(r.masked.rows[1][0], "139****4321");
    // name 未勾选 → 原样
    assert_eq!(r.masked.rows[0][1], "张三");
    assert_eq!(r.masked.rows[1][1], "李四海");
    assert_eq!(r.summary.total_rows, 2);
    assert_eq!(r.summary.masked_rows, 2);
    assert!(r.summary.by_field.contains_key("phone"));
    assert!(!r.summary.by_field.contains_key("name"));
    assert_eq!(*r.summary.by_field.get("phone").unwrap(), 2);
}

/// 验收项 13：mask_pipeline_columns 对 selected_columns 中不存在的列名静默
/// 忽略，只对存在的规则列脱敏。
#[test]
fn mask_pipeline_columns_ignores_unknown() {
    let records = Records {
        headers: vec!["phone".to_string()],
        rows: vec![vec!["13812345678".to_string()]],
    };
    let rules = RuleSet {
        validators: vec![],
        maskers: vec![MaskRule {
            field: "phone".into(),
            masker: "template".into(),
            params: Some(phone_template_params()),
            description: None,
        }],
    };
    let mut selected: HashSet<String> = HashSet::new();
    selected.insert("ghost_column".to_string());
    selected.insert("phone".to_string());

    let r = mask_pipeline_columns(&records, &rules, &selected).expect("mask_pipeline_columns");
    // phone 被脱敏，ghost_column 被静默忽略
    assert_eq!(r.masked.rows[0][0], "138****5678");
    assert_eq!(r.summary.masked_rows, 1);
    assert!(r.summary.by_field.contains_key("phone"));
    // 表头不变
    assert_eq!(r.masked.headers, vec!["phone".to_string()]);
}

/// 验收项 14：mask_pipeline_columns 对空集 selected_columns 全部原样输出，
/// masked_rows=0、by_field 为空。
#[test]
fn mask_pipeline_columns_empty_set() {
    let records = Records {
        headers: vec!["phone".to_string(), "name".to_string()],
        rows: vec![
            vec!["13812345678".to_string(), "张三".to_string()],
            vec!["13987654321".to_string(), "李四海".to_string()],
        ],
    };
    let rules = RuleSet {
        validators: vec![],
        maskers: vec![MaskRule {
            field: "phone".into(),
            masker: "template".into(),
            params: Some(phone_template_params()),
            description: None,
        }],
    };
    let selected: HashSet<String> = HashSet::new();

    let r = mask_pipeline_columns(&records, &rules, &selected).expect("mask_pipeline_columns");
    assert_eq!(r.summary.total_rows, 2);
    assert_eq!(r.summary.masked_rows, 0);
    assert!(r.summary.by_field.is_empty());
    // 全部原样
    assert_eq!(r.masked.rows[0][0], "13812345678");
    assert_eq!(r.masked.rows[0][1], "张三");
    assert_eq!(r.masked.rows[1][0], "13987654321");
    assert_eq!(r.masked.rows[1][1], "李四海");
}

/// 验收项 15：validate_pipeline 对单字段 validator 逐 cell 校验，
/// valid_matrix + summary 正确。
///
/// v0.1.0 重构后：用 `regex_with_guard` 算子 + phone 守卫 params 复现
/// 旧 phone validator 行为。
#[test]
fn validate_pipeline_basic() {
    let records = Records {
        headers: vec!["phone".to_string()],
        rows: vec![
            vec!["13812345678".to_string()],
            vec!["13987654321".to_string()],
            vec!["abc".to_string()],
        ],
    };
    let rules = RuleSet {
        validators: vec![FieldRule {
            field: "phone".into(),
            validator: "regex_with_guard".into(),
            params: Some(phone_guard_params()),
            regex: None,
            message: None,
            description: None,
        }],
        maskers: vec![],
    };
    let r = validate_pipeline(&records, &rules).expect("validate_pipeline");
    assert_eq!(r.headers, vec!["phone".to_string()]);
    assert_eq!(r.rows.len(), 3);
    assert_eq!(r.valid_matrix.len(), 3);
    assert!(r.valid_matrix[0][0]);
    assert!(r.valid_matrix[1][0]);
    assert!(!r.valid_matrix[2][0]);
    assert_eq!(r.summary.total_rows, 3);
    assert_eq!(r.summary.invalid_rows, 1);
    assert_eq!(*r.summary.by_field.get("phone").unwrap_or(&0), 1);
}

/// 验收项 16：validate_pipeline 对 rules.validators=[] 时 valid_matrix 全 true、
/// invalid_rows=0、by_field 为空。
#[test]
fn validate_pipeline_no_validators() {
    let records = Records {
        headers: vec!["a".to_string(), "b".to_string()],
        rows: vec![
            vec!["x".to_string(), "y".to_string()],
            vec!["z".to_string(), "w".to_string()],
        ],
    };
    let rules = RuleSet {
        validators: vec![],
        maskers: vec![],
    };
    let r = validate_pipeline(&records, &rules).expect("validate_pipeline");
    assert_eq!(r.summary.total_rows, 2);
    assert_eq!(r.summary.invalid_rows, 0);
    assert!(r.summary.by_field.is_empty());
    assert_eq!(r.valid_matrix, vec![vec![true, true], vec![true, true]]);
    assert_eq!(r.rows, records.rows);
}

/// 验收项 17：`mask_pipeline_columns`（列勾选）与 `mask_pipeline_selected`（行勾选）
/// 共存于同一规则集，互不影响，旧函数仍正常（证明向后兼容）。
///
/// 规则集声明 phone 字段；分别：
/// - 列勾选（选 phone 列、所有行）→ 两行 phone 均脱敏；
/// - 行勾选（选第 0 行、空 selected_columns 等价不传列集，但旧函数签名无列参数）
///   → 仅第 0 行 phone 脱敏，第 1 行原样。
#[test]
fn mask_pipeline_columns_and_selected_coexist() {
    let records = Records {
        headers: vec!["phone".to_string()],
        rows: vec![
            vec!["13812345678".to_string()],
            vec!["13987654321".to_string()],
        ],
    };
    let rules = RuleSet {
        validators: vec![],
        maskers: vec![MaskRule {
            field: "phone".into(),
            masker: "template".into(),
            params: Some(phone_template_params()),
            description: None,
        }],
    };

    // 列勾选：选 phone 列 → 两行均脱敏
    let mut cols: HashSet<String> = HashSet::new();
    cols.insert("phone".to_string());
    let by_cols = mask_pipeline_columns(&records, &rules, &cols).expect("mask_pipeline_columns");
    assert_eq!(by_cols.masked.rows[0][0], "138****5678");
    assert_eq!(by_cols.masked.rows[1][0], "139****4321");
    assert_eq!(by_cols.summary.masked_rows, 2);
    assert_eq!(*by_cols.summary.by_field.get("phone").unwrap(), 2);

    // 行勾选：选第 0 行 → 仅第 0 行脱敏，第 1 行原样
    let mut rows: HashSet<usize> = HashSet::new();
    rows.insert(0);
    let by_rows =
        mask_pipeline_selected(&records, &rules, &rows).expect("mask_pipeline_selected");
    assert_eq!(by_rows.masked.rows[0][0], "138****5678");
    assert_eq!(by_rows.masked.rows[1][0], "13987654321", "row 1 untouched by row-select");
    assert_eq!(by_rows.summary.masked_rows, 1);
    assert_eq!(*by_rows.summary.by_field.get("phone").unwrap(), 1);

    // 列勾选空集 → 全部原样（向后兼容：空集不脱敏任何列）
    let empty_cols: HashSet<String> = HashSet::new();
    let by_empty =
        mask_pipeline_columns(&records, &rules, &empty_cols).expect("mask_pipeline_columns empty");
    assert_eq!(by_empty.summary.masked_rows, 0);
    assert!(by_empty.summary.by_field.is_empty());
    assert_eq!(by_empty.masked.rows[0][0], "13812345678");
    assert_eq!(by_empty.masked.rows[1][0], "13987654321");
}

/// 验收项 18：`validate_pipeline` 边界——validator 声明不存在的 field 时该规则
/// 静默跳过不报错，`by_field` 不含该 field，对应列保持全 true（旧行为不变）。
#[test]
fn validate_pipeline_edge_cases() {
    let records = Records {
        headers: vec!["phone".to_string()],
        rows: vec![
            vec!["13812345678".to_string()],
            vec!["abc".to_string()],
        ],
    };
    // validator 声明一个 records 中不存在的 field "ghost_field"
    let rules = RuleSet {
        validators: vec![FieldRule {
            field: "ghost_field".into(),
            validator: "regex_with_guard".into(),
            params: Some(phone_guard_params()),
            regex: None,
            message: None,
            description: None,
        }],
        maskers: vec![],
    };
    let r = validate_pipeline(&records, &rules).expect("validate_pipeline");
    // ghost_field 不在 headers 中 → 规则跳过
    assert!(r.summary.by_field.is_empty(), "by_field should not contain ghost_field");
    assert_eq!(r.summary.invalid_rows, 0);
    // phone 列无 validator 声明 → 全 true
    assert!(r.valid_matrix[0][0]);
    assert!(r.valid_matrix[1][0]);
    assert_eq!(r.rows, records.rows);
}

/// 验收项 19（v0.1.0 重构）：`template` 通用算子各场景等价性。
///
/// v0.1.0 重构后无预置别名：直接构造 `MaskOp::Template(TemplateOp{...})`
/// 验证 phone / idcard / bankcard / customer_id / name 各场景输出与
/// 旧预置实现一致：
/// - 18 位身份证脱敏为 `110101********1234`，非 18 位原样返回（guard）。
/// - 11 位 phone 脱敏为 `138****5678`，非 11 位原样返回。
/// - 16~19 位 bankcard 脱敏，9~10 位原样返回。
/// - customer_id 1 字原样、空串原样、≥2 字首字保留 + 中间 * 填充。
/// - name 走 cjk 分支：1 字原样、2 字 `首*`、≥3 字 `首+*(n-2)+末`。
#[test]
fn mask_op_template_preset_equivalence() {
    use ruT0_data_kit_core::rules::{apply_mask_op, MaskOp, TemplateOp};

    // idcard
    {
        let op = MaskOp::Template(TemplateOp {
            keep_prefix: 6,
            keep_suffix: 4,
            mask_char: '*',
            mask_min_len: 8,
            min_len: Some(18),
            max_len: Some(18),
            cjk: false,
        });
        assert_eq!(apply_mask_op(&op, "110101199001011234"), "110101********1234");
        assert_eq!(apply_mask_op(&op, "11010119900101123X"), "110101********123X");
        assert_eq!(apply_mask_op(&op, "11010119900101"), "11010119900101"); // 17 位原样
        assert_eq!(apply_mask_op(&op, "1101011990010112345"), "1101011990010112345"); // 19 位原样
        assert_eq!(apply_mask_op(&op, ""), "");
    }
    // phone
    {
        let op = MaskOp::Template(TemplateOp {
            keep_prefix: 3,
            keep_suffix: 4,
            mask_char: '*',
            mask_min_len: 4,
            min_len: Some(11),
            max_len: Some(11),
            cjk: false,
        });
        assert_eq!(apply_mask_op(&op, "13812345678"), "138****5678");
        assert_eq!(apply_mask_op(&op, "1381234567"), "1381234567");  // 10 位原样
        assert_eq!(apply_mask_op(&op, "138123456789"), "138123456789"); // 12 位原样
        assert_eq!(apply_mask_op(&op, ""), "");
    }
    // bankcard —— 关键边界
    {
        let op = MaskOp::Template(TemplateOp {
            keep_prefix: 6,
            keep_suffix: 4,
            mask_char: '*',
            mask_min_len: 4,
            min_len: Some(16),
            max_len: Some(19),
            cjk: false,
        });
        assert_eq!(apply_mask_op(&op, "6225887654321098"), "622588******1098"); // 16 位
        assert_eq!(apply_mask_op(&op, "6225881234"), "6225881234");              // 10 位边界
        assert_eq!(apply_mask_op(&op, "622588123"), "622588123");                // 9 位 guard
        assert_eq!(apply_mask_op(&op, "6225887654321098765"), "622588*********8765"); // 19 位
        assert_eq!(apply_mask_op(&op, ""), "");
    }
    // customer_id —— 关键边界
    {
        let op = MaskOp::Template(TemplateOp {
            keep_prefix: 1,
            keep_suffix: 0,
            mask_char: '*',
            mask_min_len: 4,
            min_len: Some(2),
            max_len: None,
            cjk: false,
        });
        assert_eq!(apply_mask_op(&op, "12345678"), "1*******");
        assert_eq!(apply_mask_op(&op, "C1234"), "C****");
        assert_eq!(apply_mask_op(&op, "A"), "A");      // 1 字原样
        assert_eq!(apply_mask_op(&op, ""), "");        // 空串
    }
    // name（cjk 分支）
    {
        let op = MaskOp::Template(TemplateOp {
            keep_prefix: 1,
            keep_suffix: 1,
            mask_char: '*',
            mask_min_len: 1,
            min_len: None,
            max_len: None,
            cjk: true,
        });
        assert_eq!(apply_mask_op(&op, "张三"), "张*");
        assert_eq!(apply_mask_op(&op, "李四海"), "李*海");
        assert_eq!(apply_mask_op(&op, "欧阳四海"), "欧**海");
        assert_eq!(apply_mask_op(&op, "张"), "张");
        assert_eq!(apply_mask_op(&op, ""), "");
    }
    // email（split + cjk 分支）
    {
        use ruT0_data_kit_core::rules::{MaskOp, SplitTemplateOp, TemplateOp};
        let op = MaskOp::SplitTemplate(SplitTemplateOp {
            separator: "@".into(),
            segment_index: 0,
            inner: Box::new(TemplateOp {
                keep_prefix: 1,
                keep_suffix: 1,
                mask_char: '*',
                mask_min_len: 1,
                min_len: None,
                max_len: None,
                cjk: true,
            }),
        });
        assert_eq!(apply_mask_op(&op, "zhangsan@example.com"), "z******n@example.com");
        assert_eq!(apply_mask_op(&op, "zs@x.com"), "z*@x.com");
        assert_eq!(apply_mask_op(&op, "z@x.com"), "z@x.com");
        assert_eq!(apply_mask_op(&op, "notanemail"), "notanemail");
        assert_eq!(apply_mask_op(&op, ""), "");
    }
}

/// 验收项 20（v0.1.0 重构）：`MaskOp::RegexReplace` 通用算子基本契约 +
/// 算子清单只含 4 个通用算子。
///
/// 直接构造 `RegexReplace` 算子：
/// - `match_mode=All` + `replacement="$1****$2"` 对 11 位手机号 → `138****5678`。
/// - 非法 pattern 退化为原值返回（不 panic）。
/// - `apply_mask_op` 与 `MaskOp` impl `Masker` 行为一致。
/// - `list_mask_op_types` 只返回 4 个通用算子（无预置别名）。
#[test]
fn mask_op_regex_replace_dispatch() {
    use ruT0_data_kit_core::rules::{
        apply_mask_op, list_mask_op_types, MaskOp, MatchMode, RegexReplaceOp,
    };

    let op = MaskOp::RegexReplace(RegexReplaceOp::new(
        Some(r"(\d{3})\d{4}(\d{4})".into()),
        "$1****$2".into(),
        MatchMode::All,
    ));
    assert_eq!(apply_mask_op(&op, "13812345678"), "138****5678");
    // 无匹配返回原值
    assert_eq!(apply_mask_op(&op, "abc"), "abc");

    // 非法 pattern → re=None，原值返回
    let bad = MaskOp::RegexReplace(RegexReplaceOp::new(
        Some("(".into()),
        "x".into(),
        MatchMode::All,
    ));
    assert_eq!(apply_mask_op(&bad, "abc"), "abc");

    // MaskOp impl Masker
    use ruT0_data_kit_core::maskers::Masker;
    assert_eq!(op.mask("13812345678"), "138****5678");

    // list_mask_op_types 只含 4 个通用算子（v0.1.0 重构后无预置别名）
    let types = list_mask_op_types();
    let names: Vec<&str> = types.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        names,
        ["template", "split_template", "regex_replace", "const_replace"],
        "list_mask_op_types should only contain 4 generic operators",
    );
    // 反向校验：预置别名不在清单中
    for removed in [
        "idcard_mask",
        "phone_mask",
        "bankcard_mask",
        "email_mask",
        "name_mask",
        "customer_id_mask",
        "custom",
        "delete",
        "replace",
        "regex_extract",
    ] {
        assert!(
            !names.contains(&removed),
            "{removed} should NOT be in list_mask_op_types after refactor",
        );
    }
}

/// 验收项 21（v0.1.0 重构）：`ValidateOp::Regex` 通用算子基本契约 +
/// 算子清单只含 3 个通用算子。
///
/// 直接构造 `RegexOp`：
/// - 合法邮箱 valid==true。
/// - 非法值 valid==false。
/// - `list_validate_op_types` 只返回 3 个通用算子（无预置别名）。
#[test]
fn validate_op_regex_preset_equivalence() {
    use ruT0_data_kit_core::rules::{apply_validate_op, list_validate_op_types, RegexOp, ValidateOp};

    let op = ValidateOp::Regex(RegexOp::new(
        r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$",
        Some("email format invalid".into()),
        None,
    ));
    assert!(matches!(op, ValidateOp::Regex(_)));
    assert!(
        apply_validate_op(&op, "zhangsan@example.com").valid,
        "valid email should pass",
    );
    assert!(
        !apply_validate_op(&op, "invalid").valid,
        "plain word should fail",
    );
    assert!(
        !apply_validate_op(&op, "a@b").valid,
        "no TLD should fail",
    );
    assert!(
        !apply_validate_op(&op, "@x.com").valid,
        "empty local should fail",
    );

    // list_validate_op_types 只含 3 个通用算子（v0.1.0 重构后无预置别名）
    let types = list_validate_op_types();
    let names: Vec<&str> = types.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        names,
        ["regex", "algorithm", "regex_with_guard"],
        "list_validate_op_types should only contain 3 generic operators",
    );
    // 反向校验：预置别名不在清单中
    for removed in [
        "email",
        "username",
        "name",
        "idcard",
        "bankcard",
        "phone",
        "mac",
    ] {
        assert!(
            !names.contains(&removed),
            "{removed} should NOT be in list_validate_op_types after refactor",
        );
    }
}

/// 验收项 22（v0.1.0 重构）：`MaskOp::SplitTemplate` 与 `MaskOp::ConstReplace`
/// 通用算子契约。
///
/// - 手动构造 `MaskOp::SplitTemplate`（separator="@", segment_index=0, inner=
///   Template{keep_prefix:1, keep_suffix:1, mask_char:'*', mask_min_len:1, cjk:true}）
///   对 `"zhangsan@example.com"` → `"z******n@example.com"`。
/// - `MaskOp::ConstReplace { with: "REDACTED" }` 对任意输入返回 `"REDACTED"`；
///   `with: ""` 对任意输入返回 `""`（等价 delete）。
/// - 抽样断言 `apply_mask_op(&op, x)` 与 `MaskOp impl Masker` 的 `op.mask(x)`
///   行为一致（SplitTemplate + ConstReplace 两条）。
#[test]
fn mask_op_split_template_and_const_replace() {
    use ruT0_data_kit_core::rules::{
        apply_mask_op, ConstReplaceOp, MaskOp, SplitTemplateOp, TemplateOp,
    };
    use ruT0_data_kit_core::maskers::Masker;

    // SplitTemplate 手动构造
    let split = MaskOp::SplitTemplate(SplitTemplateOp {
        separator: "@".into(),
        segment_index: 0,
        inner: Box::new(TemplateOp {
            keep_prefix: 1,
            keep_suffix: 1,
            mask_char: '*',
            mask_min_len: 1,
            min_len: None,
            max_len: None,
            cjk: true,
        }),
    });
    assert_eq!(
        apply_mask_op(&split, "zhangsan@example.com"),
        "z******n@example.com",
    );
    assert_eq!(apply_mask_op(&split, "zs@x.com"), "z*@x.com");
    assert_eq!(apply_mask_op(&split, "z@x.com"), "z@x.com");
    assert_eq!(apply_mask_op(&split, "notanemail"), "notanemail");
    assert_eq!(apply_mask_op(&split, ""), "");

    // ConstReplace: with="REDACTED"
    let redact = MaskOp::ConstReplace(ConstReplaceOp {
        with: "REDACTED".into(),
    });
    assert_eq!(apply_mask_op(&redact, "anything"), "REDACTED");
    assert_eq!(apply_mask_op(&redact, ""), "REDACTED");
    assert_eq!(apply_mask_op(&redact, "13812345678"), "REDACTED");

    // ConstReplace: with="" 等价 delete
    let del = MaskOp::ConstReplace(ConstReplaceOp {
        with: String::new(),
    });
    assert_eq!(apply_mask_op(&del, "anything"), "");
    assert_eq!(apply_mask_op(&del, ""), "");

    // 抽样断言 apply_mask_op 与 MaskOp impl Masker 一致（SplitTemplate + 两条 ConstReplace）
    assert_eq!(split.mask("zhangsan@example.com"), "z******n@example.com");
    assert_eq!(redact.mask("anything"), "REDACTED");
    assert_eq!(del.mask("anything"), "");
}

/// 验收项 23（v0.1.0 重构）：`ValidateOp::Algorithm` 与
/// `ValidateOp::RegexWithGuard` 通用算子契约 + apply/build 等价。
///
/// v0.1.0 重构后无预置别名：直接构造 `AlgorithmOp` / `RegexWithGuardOp`
/// 验证：
/// - `AlgorithmOp { algo: IdCard }`：合法 `"286071197501111126"` valid==true，
///   非法 `"110101199001011230"` valid==false。
/// - `AlgorithmOp { algo: BankCard }`：Luhn 合法 `"6225887654321096"` valid==true。
/// - `RegexWithGuardOp { guard: PhonePrefix, pattern="^\d{11}$" }`：
///   `"13812345678"` valid==true，`"12345"` valid==false。
/// - `RegexWithGuardOp { guard: MacPrefix, pattern=MAC_REGEX }`：
///   `"AA:BB:CC:DD:EE:FF"` valid==true，`"notamac"` valid==false。
/// - 抽样断言 `apply_validate_op(&op, x).valid` 与
///   `build_validator(&FieldRule{ validator: "algorithm", params: Some(idcard_algorithm_params()) }).unwrap().validate(x).valid`
///   一致（idcard 一条）。
#[test]
fn validate_op_algorithm_and_guard_equivalence() {
    use ruT0_data_kit_core::rules::{
        apply_validate_op, build_validator, AlgorithmOp, AlgoKind, FieldRule, GuardKind,
        RegexWithGuardOp, ValidateOp, ValidatorRegistry,
    };

    // idcard → Algorithm
    let op = ValidateOp::Algorithm(AlgorithmOp { algo: AlgoKind::IdCard });
    assert!(matches!(op, ValidateOp::Algorithm(_)));
    assert!(
        apply_validate_op(&op, "286071197501111126").valid,
        "legal idcard should pass",
    );
    assert!(
        !apply_validate_op(&op, "110101199001011230").valid,
        "illegal idcard should fail",
    );

    // bankcard → Algorithm
    let op = ValidateOp::Algorithm(AlgorithmOp { algo: AlgoKind::BankCard });
    assert!(matches!(op, ValidateOp::Algorithm(_)));
    assert!(
        apply_validate_op(&op, "6225887654321096").valid,
        "Luhn-legal bankcard should pass",
    );

    // phone → RegexWithGuard
    let op = ValidateOp::RegexWithGuard(RegexWithGuardOp::new(
        GuardKind::PhonePrefix { prefix_set: "real".into() },
        r"^\d{11}$",
        None,
    ));
    assert!(matches!(op, ValidateOp::RegexWithGuard(_)));
    assert!(
        apply_validate_op(&op, "13812345678").valid,
        "legal phone should pass",
    );
    assert!(
        !apply_validate_op(&op, "12345").valid,
        "short non-phone should fail",
    );

    // mac → RegexWithGuard
    let op = ValidateOp::RegexWithGuard(RegexWithGuardOp::new(
        GuardKind::MacPrefix { prefix: None },
        r"^([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$",
        None,
    ));
    assert!(matches!(op, ValidateOp::RegexWithGuard(_)));
    assert!(
        apply_validate_op(&op, "AA:BB:CC:DD:EE:FF").valid,
        "legal mac should pass",
    );
    assert!(
        !apply_validate_op(&op, "notamac").valid,
        "non-mac should fail",
    );

    // 抽样等价：apply_validate_op vs build_validator（idcard 算子路径）。
    let reg = ValidatorRegistry::new();
    let rule = FieldRule {
        field: "x".into(),
        validator: "algorithm".into(),
        params: Some(idcard_algorithm_params()),
        regex: None,
        message: None,
        description: None,
    };
    let v = build_validator(&rule, &reg).expect("build_validator algorithm/idcard");
    let direct = ValidateOp::Algorithm(AlgorithmOp { algo: AlgoKind::IdCard });
    for input in ["286071197501111126", "110101199001011230", "12345"] {
        assert_eq!(
            apply_validate_op(&direct, input).valid,
            v.validate(input).valid,
            "apply_validate_op vs build_validator on {input:?}",
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// v0.2.0 日志切片全链路 e2e（T2-6 新增）
// ─────────────────────────────────────────────────────────────────────
//
// 24. `log_parse_full`：LogReader 读 tests/fixtures/samples/log/access.log，
//     断言 1860 行全解析成功、line_no 1..=1860 连续递增、每行 ip 非空。
// 25. `log_scan_full`：SignatureEngine + DefaultSensitiveScan + 默认
//     RuleSet 跑 scan_log，断言 Report.kind=="log_scan"、SQLi 命中 > 1000、
//     弱口令命中 ≥ 4、4 类新签名（union/extractvalue/sleep/tautology）
//     各至少命中 1 次。
// 26. `log_signatures_6_categories`：手工构造 6 条 LogEntry，每条触发 1 类
//     签名，断言 SignatureEngine 各产 ≥1 hit；并各配 1 条反例（不命中）。

/// v0.2.0 验收项 24：access.log 全 1860 行 100% 解析成功。
#[test]
fn log_parse_full() {
    let path = common::fixtures_dir().join("log/access.log");
    let r = LogReader::new().expect("LogReader");
    let entries = r.read(&path).expect("read access.log");
    // 文件行数
    let file_lines = std::fs::read_to_string(&path).unwrap().lines().count();
    assert_eq!(entries.len(), file_lines, "all lines parsed");
    assert_eq!(entries.len(), 1860, "fixture has 1860 lines (v0.2.0 追加 4 行后)");
    // line_no 连续递增
    for (i, e) in entries.iter().enumerate() {
        assert_eq!(e.line_no, i + 1, "line_no sequential at index {i}");
        assert!(!e.ip.is_empty(), "ip non-empty at line {}", e.line_no);
    }
}

/// v0.2.0 验收项 25：scan_log 全链路跑通 access.log，验证 summary 与 findings。
#[test]
fn log_scan_full() {
    let path = common::fixtures_dir().join("log/access.log");
    let r = LogReader::new().expect("LogReader");
    let entries = r.read(&path).expect("read access.log");
    assert_eq!(entries.len(), 1860);

    let eng = SignatureEngine::default();
    let scan = DefaultSensitiveScan::default();
    // 默认 RuleSet：validators 为空 → 敏感扫描段不产 finding，专注 SQLi + 弱口令。
    let rules = RuleSet {
        validators: vec![],
        maskers: vec![],
    };
    let report = scan_log(&entries, &eng, &scan, &rules);

    // Report 基本字段
    assert_eq!(report.kind, "log_scan");
    assert_eq!(report.source, "log");

    // summary 必为 mapping 且含 total_lines / sqli_hits / weak_password_hits
    let m = report
        .summary
        .as_mapping()
        .expect("summary is mapping");
    assert_eq!(
        m.get("total_lines").and_then(|v| v.as_u64()),
        Some(1860),
        "total_lines == 1860",
    );

    let sqli_hits = m
        .get("sqli_hits")
        .and_then(|v| v.as_u64())
        .expect("sqli_hits present");
    // fixture 含大量 SQLi 攻击行（T2-1 fixture 已有大量命中 + T2-6 追加 4 行）
    assert!(
        sqli_hits > 1000,
        "sqli_hits should be > 1000, got {sqli_hits}",
    );

    let wp_hits = m
        .get("weak_password_hits")
        .and_then(|v| v.as_u64())
        .expect("weak_password_hits present");
    // access.log 含大量 guest/admin/123456 等弱口令对，至少 4 条命中
    assert!(
        wp_hits >= 4,
        "weak_password_hits should be >= 4, got {wp_hits}",
    );

    // 4 类新签名（T2-6 追加 fixture 覆盖：union/extractvalue/sleep/tautology）
    // 各至少 1 次 SQLi finding（按 rule_id 去重计数）。
    let sqli_findings: Vec<&ruT0_data_kit_core::report::Finding> = report
        .findings
        .iter()
        .filter(|f| f.r#type == "sqli")
        .collect();
    let has_rule = |rid: &str| sqli_findings.iter().any(|f| f.value == rid);
    assert!(has_rule("sqli_union"), "union signature must hit at least once");
    assert!(has_rule("sqli_error_based"), "error_based signature must hit at least once");
    assert!(has_rule("sqli_time_based"), "time_based signature must hit at least once");
    assert!(has_rule("sqli_tautology"), "tautology signature must hit at least once");

    // T2-9：sqli finding 的 extra 字段已透传 parsed_payload，至少有一条 sqli
    // finding 的 extra.is_some()（fixture 含大量可被 parse_payload 解析的 payload）。
    assert!(
        sqli_findings.iter().any(|f| f.extra.is_some()),
        "at least one sqli finding must carry parsed_payload in extra",
    );
    // weak_password finding 的 extra 为 None（向后兼容）。
    let wp_findings: Vec<&ruT0_data_kit_core::report::Finding> = report
        .findings
        .iter()
        .filter(|f| f.r#type == "weak_password")
        .collect();
    assert!(
        wp_findings.iter().all(|f| f.extra.is_none()),
        "all weak_password findings must have extra == None",
    );

    // top_attack_ips 字段存在且为 sequence，长度 ≤ 5
    let top = m
        .get("top_attack_ips")
        .and_then(|v| v.as_sequence())
        .expect("top_attack_ips present");
    assert!(top.len() <= 5, "top_attack_ips truncated to 5");
}

/// v0.2.0 验收项 26：6 类签名各至少 1 正例 + 1 反例。
///
/// 正例（decoded query value 形态，与 sqli_signatures.yaml pattern 对齐）：
/// - blind_binary: `1' or ascii(substr((database()),1,1))>79#`
/// - union:         `1 union select 1,2,3`
/// - error_based:   `1 and extractvalue(1,concat(0x7e,version()))`
/// - time_based:    `1 and sleep(5)`
/// - tautology:     `1 or 1=1`
/// - comment:       `1'--`
///
/// 反例（正常请求，无注入特征）：每类配一条，断言该 rule_id 不命中。
#[test]
fn log_signatures_6_categories() {
    use ruT0_data_kit_core::log::LogEntry;

    fn mk(line_no: usize, query: &str) -> LogEntry {
        LogEntry {
            line_no,
            ip: "10.0.0.9".to_string(),
            timestamp: "10/Oct/2023:13:55:36 +0000".to_string(),
            method: "GET".to_string(),
            path: "/news.php".to_string(),
            query: Some(query.to_string()),
            status: 200,
            size: Some(100),
            user_agent: "curl/7.88.0".to_string(),
            raw: format!("GET /news.php?{query} HTTP/1.1"),
            decoded_path: ruT0_data_kit_core::log::url_decode_twice("/news.php"),
            decoded_query: {
                let pairs = ruT0_data_kit_core::log::parse_query(query);
                Some(
                    pairs
                        .iter()
                        .map(|(k, v)| {
                            if v.is_empty() {
                                k.clone()
                            } else {
                                format!("{k}={v}")
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("&"),
                )
            },
            decoded_ua: ruT0_data_kit_core::log::url_decode_twice("curl/7.88.0"),
        }
    }

    let eng = SignatureEngine::default();
    // 6 类正例
    let positives: &[(&str, &str)] = &[
        ("sqli_blind_binary", "id=1' or ascii(substr((database()),1,1))>79#"),
        ("sqli_union", "id=1 union select 1,2,3"),
        ("sqli_error_based", "id=1 and extractvalue(1,concat(0x7e,version()))"),
        ("sqli_time_based", "id=1 and sleep(5)"),
        ("sqli_tautology", "id=1 or 1=1"),
        ("sqli_comment", "id=1'--"),
    ];
    for (i, (rid, q)) in positives.iter().enumerate() {
        let e = mk(i + 1, q);
        let hits = eng.scan_log_entry(&e);
        assert!(
            hits.iter().any(|h| h.rule_id == *rid),
            "positive for {rid} must hit, hits={:?}",
            hits.iter().map(|h| &h.rule_id).collect::<Vec<_>>(),
        );
    }

    // 6 类反例：构造一个无任何注入特征的正常 query，断言所有 6 类均不命中。
    let normal = mk(100, "q=hello+world&page=1&sort=asc");
    let hits = eng.scan_log_entry(&normal);
    assert!(
        hits.is_empty(),
        "normal request must produce no signature hits, got {:?}",
        hits.iter().map(|h| &h.rule_id).collect::<Vec<_>>(),
    );

    // 额外反例：union 单独「select」关键字不构成 union 注入。
    let sel_only = mk(101, "kind=select-all");
    assert!(
        eng.scan_log_entry(&sel_only)
            .iter()
            .all(|h| h.rule_id != "sqli_union"),
        "select keyword alone must not trigger union",
    );

    // 额外反例：单独 `or 1` 不构成 `or 数字=数字` 恒真。
    let or_only = mk(102, "x=or 1");
    assert!(
        eng.scan_log_entry(&or_only)
            .iter()
            .all(|h| h.rule_id != "sqli_tautology"),
        "or 1 alone must not trigger tautology",
    );
}

/// v0.2.0 验收项 27（T2-9 新增）：sqli_union / sqli_error_based pattern
/// 扩变体回归 + extra/parsed_payload e2e 断言。
///
/// - `union all select 1,2,3` 命中 sqli_union（兼容 `union all select`）。
/// - `1 and extractvalue(1,concat(0x7e,user()))` 命中 sqli_error_based（原已支持，回归）。
/// - `1 and exp(~(select * from dual))` 命中 sqli_error_based（新增 exp ~ 变体）。
/// - `1 and floor(rand(0)*2)` 命中 sqli_error_based（floor rand 变体，回归）。
/// - 上述命中后 SignatureHit.parsed_payload.is_some()（T2-8 填充）。
#[test]
fn log_signatures_pattern_variants_and_parsed_payload() {
    use ruT0_data_kit_core::log::LogEntry;

    fn mk(line_no: usize, query: &str) -> LogEntry {
        LogEntry {
            line_no,
            ip: "10.0.0.9".to_string(),
            timestamp: "10/Oct/2023:13:55:36 +0000".to_string(),
            method: "GET".to_string(),
            path: "/news.php".to_string(),
            query: Some(query.to_string()),
            status: 200,
            size: Some(100),
            user_agent: "curl/7.88.0".to_string(),
            raw: format!("GET /news.php?{query} HTTP/1.1"),
            decoded_path: ruT0_data_kit_core::log::url_decode_twice("/news.php"),
            decoded_query: {
                let pairs = ruT0_data_kit_core::log::parse_query(query);
                Some(
                    pairs
                        .iter()
                        .map(|(k, v)| {
                            if v.is_empty() {
                                k.clone()
                            } else {
                                format!("{k}={v}")
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("&"),
                )
            },
            decoded_ua: ruT0_data_kit_core::log::url_decode_twice("curl/7.88.0"),
        }
    }

    let eng = SignatureEngine::default();

    // 1. union all select 命中 sqli_union（pattern 扩变体后兼容）。
    let union_all = mk(1, "id=1 union all select 1,2,3");
    let hits = eng.scan_log_entry(&union_all);
    assert!(
        hits.iter().any(|h| h.rule_id == "sqli_union"),
        "union all select must hit sqli_union, hits={:?}",
        hits.iter().map(|h| &h.rule_id).collect::<Vec<_>>(),
    );

    // 2. extractvalue 命中 sqli_error_based（原已支持，回归）。
    let extractvalue = mk(2, "id=1 and extractvalue(1,concat(0x7e,user()))");
    let hits = eng.scan_log_entry(&extractvalue);
    assert!(
        hits.iter().any(|h| h.rule_id == "sqli_error_based"),
        "extractvalue must hit sqli_error_based",
    );

    // 3. exp(~...) 命中 sqli_error_based（新增 exp ~ 变体）。
    let exp_bang = mk(3, "id=1 and exp(~(select * from dual))");
    let hits = eng.scan_log_entry(&exp_bang);
    assert!(
        hits.iter().any(|h| h.rule_id == "sqli_error_based"),
        "exp(~...) must hit sqli_error_based (new variant), hits={:?}",
        hits.iter().map(|h| &h.rule_id).collect::<Vec<_>>(),
    );

    // 4. floor(rand(0)*2) 命中 sqli_error_based（floor rand 变体，回归）。
    let floor_rand = mk(4, "id=1 and floor(rand(0)*2)");
    let hits = eng.scan_log_entry(&floor_rand);
    assert!(
        hits.iter().any(|h| h.rule_id == "sqli_error_based"),
        "floor(rand(0)*2) must hit sqli_error_based (floor rand variant)",
    );

    // 5. 命中后 parsed_payload.is_some()（T2-8 填充；union/error 均可被 parse_payload
    //    解析出结构化信息）。
    let union_hits = eng.scan_log_entry(&mk(5, "id=1 union select 1,2,3"));
    assert!(
        union_hits
            .iter()
            .any(|h| h.rule_id == "sqli_union" && h.parsed_payload.is_some()),
        "union hit must carry parsed_payload",
    );
    let err_hits = eng.scan_log_entry(&extractvalue);
    assert!(
        err_hits
            .iter()
            .any(|h| h.rule_id == "sqli_error_based" && h.parsed_payload.is_some()),
        "error_based hit must carry parsed_payload",
    );
}

// ─────────────────────────────────────────────────────────────────────
// 27. log_payload_parser_e2e（v0.2.1 T2-11）
//
// 端到端验证：盲注 entry 经 scan_log_entry 后，命中 SignatureHit 的
// parsed_payload.read_target == Some("database()")。这把 T2-8 的
// payload_parser 与 T2-1/T2-7 的 scan_log_entry 串起来，证明语义解析
// 在签名命中路径上确实被调用并填充正确字段。
// ─────────────────────────────────────────────────────────────────────
#[test]
fn log_payload_parser_e2e() {
    use ruT0_data_kit_core::log::LogEntry;

    fn mk(line_no: usize, query: &str) -> LogEntry {
        LogEntry {
            line_no,
            ip: "10.0.0.9".to_string(),
            timestamp: "10/Oct/2023:13:55:36 +0000".to_string(),
            method: "GET".to_string(),
            path: "/news.php".to_string(),
            query: Some(query.to_string()),
            status: 200,
            size: Some(100),
            user_agent: "curl/7.88.0".to_string(),
            raw: format!("GET /news.php?{query} HTTP/1.1"),
            decoded_path: ruT0_data_kit_core::log::url_decode_twice("/news.php"),
            decoded_query: {
                let pairs = ruT0_data_kit_core::log::parse_query(query);
                Some(
                    pairs
                        .iter()
                        .map(|(k, v)| {
                            if v.is_empty() {
                                k.clone()
                            } else {
                                format!("{k}={v}")
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("&"),
                )
            },
            decoded_ua: ruT0_data_kit_core::log::url_decode_twice("curl/7.88.0"),
        }
    }

    let eng = SignatureEngine::default();

    // 盲注二分 payload：ascii(substr((database()),1,1))>79，末尾 # 注释。
    let e = mk(1, "id=1' or ascii(substr((database()),1,1))>79#");
    let hits = eng.scan_log_entry(&e);

    // 必须命中 sqli_blind_binary。
    assert!(
        hits.iter().any(|h| h.rule_id == "sqli_blind_binary"),
        "blind payload must hit sqli_blind_binary, hits={:?}",
        hits.iter().map(|h| &h.rule_id).collect::<Vec<_>>(),
    );

    // 命中的 SignatureHit 必须携带 parsed_payload，且 read_target == database()。
    let blind_hit = hits
        .iter()
        .find(|h| h.rule_id == "sqli_blind_binary")
        .expect("sqli_blind_binary hit must exist");
    let parsed = blind_hit
        .parsed_payload
        .as_ref()
        .expect("blind hit must carry parsed_payload");
    assert_eq!(
        parsed.attack_type, "blind_boolean",
        "parsed attack_type must be blind_boolean, got {:?}",
        parsed.attack_type,
    );
    assert_eq!(
        parsed.read_target.as_deref(),
        Some("database()"),
        "parsed read_target must be database(), got {:?}",
        parsed.read_target,
    );
    assert_eq!(
        parsed.char_position, Some(1),
        "parsed char_position must be 1, got {:?}",
        parsed.char_position,
    );
    assert_eq!(
        parsed.comparator.as_deref(),
        Some(">"),
        "parsed comparator must be >, got {:?}",
        parsed.comparator,
    );
    assert_eq!(
        parsed.compared_ascii, Some(79),
        "parsed compared_ascii must be 79, got {:?}",
        parsed.compared_ascii,
    );
}

// ─────────────────────────────────────────────────────────────────────
// 28. log_decoded_query_plus_decode（v0.2.1 T2-11）
//
// 端到端验证 parse_line 对 query 中 `+` 的 form-urlencoded 还原：构造
// 一行 CLF/Nginx Combined 日志，其 query 段含字面 `+`（表空格），经
// LogReader::parse_line 后 entry.decoded_query 必须含空格、不含 `+`。
// 这证明 T2-7 的 parse_query `+`→space 语义在 parse_line 端到端路径上
// 生效，为 payload_parser 提供「已正确解码」输入。
// ─────────────────────────────────────────────────────────────────────
#[test]
fn log_decoded_query_plus_decode() {
    let reader = LogReader::new().expect("LogReader must compile");

    // CLF 行：query 含 `q=hello+world`（form-urlencoded 中 `+` 表空格）。
    let raw_line = r#"10.0.0.9 - - [10/Oct/2023:13:55:36 +0000] "GET /search?q=hello+world&sort=asc HTTP/1.1" 200 512 "-" "curl/7.88.0""#;

    let entry = reader
        .parse_line(raw_line, 1)
        .expect("CLF line with + in query must parse");

    // 原始 query 保留字面 `+`（不解码）。
    assert_eq!(
        entry.query.as_deref(),
        Some("q=hello+world&sort=asc"),
        "raw query must keep literal +, got {:?}",
        entry.query,
    );

    // decoded_query 必须把 `+` 解为空格、不再含字面 `+`。
    let decoded = entry
        .decoded_query
        .as_ref()
        .expect("decoded_query must be Some for entry with query");
    assert!(
        decoded.contains(' '),
        "decoded_query must contain space (from +), got {:?}",
        decoded,
    );
    assert!(
        !decoded.contains('+'),
        "decoded_query must not contain literal +, got {:?}",
        decoded,
    );
    assert!(
        decoded.contains("hello world"),
        "decoded_query must contain 'hello world', got {:?}",
        decoded,
    );

    // 同样验证 url-encoded 的 %20 也被双重解码为空格（与 + 等价语义）。
    let raw_pct = r#"10.0.0.9 - - [10/Oct/2023:13:55:36 +0000] "GET /search?q=hello%2520world HTTP/1.1" 200 512 "-" "curl/7.88.0""#;
    let entry_pct = reader
        .parse_line(raw_pct, 2)
        .expect("CLF line with %2520 in query must parse");
    let decoded_pct = entry_pct
        .decoded_query
        .as_ref()
        .expect("decoded_query must be Some for entry with query");
    assert!(
        decoded_pct.contains(' '),
        "decoded_query for %2520 must contain space (double decode), got {:?}",
        decoded_pct,
    );
    assert!(
        !decoded_pct.contains('%'),
        "decoded_query for %2520 must not contain %, got {:?}",
        decoded_pct,
    );
}
