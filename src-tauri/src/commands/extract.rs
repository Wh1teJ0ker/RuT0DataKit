//! 文本/文件 PII 提取命令（2 命令）+ 提取 helpers + 单元测试。

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use ruT0_data_kit_core::extract;
use ruT0_data_kit_core::rules::RuleSet;
use serde_json::{json, Value};

use super::parse_ruleset_json;

/// 提取结果条目（前端友好结构）。
#[derive(serde::Serialize, serde::Deserialize)]
struct ExtractItem {
    #[serde(rename = "type")]
    r#type: String,
    value: String,
}

/// 从文本提取 PII，去重后返回 findings + counts。
///
/// `rules_json` 为 Some 时按用户传入的 RuleSet 提取（前端从「规则管理」池勾选）；
/// 为 None 时回退到 builtin（phone/bankcard/ip 三类），保持 v0.4.3 行为。
#[tauri::command]
pub fn extract_text(content: String, rules_json: Option<String>) -> Result<Value, String> {
    let rules = resolve_extract_rules(rules_json)?;
    let raw = extract::extract_text_with_rules(&content, &rules);
    Ok(package_extract_result(raw))
}

/// 从 .txt 文件提取 PII，去重后返回 findings + counts。
///
/// `rules_json` 语义同 [`extract_text`]。
#[tauri::command]
pub fn extract_file(path: String, rules_json: Option<String>) -> Result<Value, String> {
    let rules = resolve_extract_rules(rules_json)?;
    let raw = extract::extract_file_with_rules(Path::new(&path), &rules)
        .map_err(|e| e.to_string())?;
    Ok(package_extract_result(raw))
}

/// 解析 extract 命令的 rules_json：Some -> 反序列化用户 RuleSet；
/// None -> 空规则集（v0.4.4：`builtin_extract_ruleset` 已删除，规则池初始
/// 为空，等价于 `RuleSet::default()`，返回空 findings）。
fn resolve_extract_rules(
    rules_json: Option<String>,
) -> Result<RuleSet, String> {
    match rules_json {
        Some(json) => parse_ruleset_json(&json),
        None => Ok(RuleSet::default()),
    }
}

/// 把 core Finding 列表去重 + 打包成 { findings, counts } JSON。
///
/// v0.4.4 T10-2：counts 从硬编码 phone/bankcard/ip 改为按 `f.r#type` 动态聚合
/// （BTreeMap 保字典序），以支持用户自定义 RuleSet 产生的任意 type（idcard/email
/// /mac/username/name 或 rule.field 等）。
fn package_extract_result(raw: Vec<ruT0_data_kit_core::report::Finding>) -> Value {
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut items: Vec<ExtractItem> = Vec::new();
    let mut counts: BTreeMap<String, i64> = BTreeMap::new();
    for f in raw {
        let key = (f.r#type.clone(), f.value.clone());
        if seen.insert(key) {
            *counts.entry(f.r#type.clone()).or_insert(0) += 1;
            items.push(ExtractItem {
                r#type: f.r#type,
                value: f.value,
            });
        }
    }
    json!({
        "findings": items,
        "counts": counts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 写一份临时 CSV 测试数据并返回路径。调用方负责清理。
    fn write_temp_csv(headers: &[&str], rows: &[Vec<&str>]) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "rut0_t5_8_{}.csv",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file = std::fs::File::create(&dir).unwrap();
        let mut wtr = csv::Writer::from_writer(file);
        wtr.write_record(headers).unwrap();
        for r in rows {
            wtr.write_record(r).unwrap();
        }
        wtr.flush().unwrap();
        dir
    }

    fn empty_ruleset() -> String {
        serde_json::json!({ "maskers": [], "validators": [] }).to_string()
    }

    /// 验证 JSON 行数 = 原始行数，key = headers 子集（按勾选列过滤）。
    #[test]
    fn export_records_json_filters_columns_and_preserves_rows() {
        let csv_path = write_temp_csv(
            &["id", "name", "phone"],
            &[vec!["1", "alice", "138"], vec!["2", "bob", "139"]],
        );
        let out = csv_path.with_extension("json");
        let selected = vec!["id".to_string(), "phone".to_string()];
        let order = vec!["phone".to_string(), "id".to_string()];
        crate::commands::export_records_json(
            csv_path.to_string_lossy().into_owned(),
            empty_ruleset(),
            selected,
            order,
            None,
            out.to_string_lossy().into_owned(),
        )
        .unwrap();

        let body = std::fs::read_to_string(&out).unwrap();
        let parsed: Vec<HashMap<String, String>> = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed.len(), 2, "行数应等于原始行数");
        // key 应是 selected_columns 子集（不含 name）
        for row in &parsed {
            assert!(row.contains_key("id"));
            assert!(row.contains_key("phone"));
            assert!(!row.contains_key("name"));
        }
        // column_order 控制顺序：phone 在 id 之前（HashMap 无序但 values 应正确）
        assert_eq!(parsed[0].get("id").unwrap(), "1");
        assert_eq!(parsed[0].get("phone").unwrap(), "138");
        assert_eq!(parsed[1].get("id").unwrap(), "2");
        assert_eq!(parsed[1].get("phone").unwrap(), "139");

        let _ = std::fs::remove_file(&csv_path);
        let _ = std::fs::remove_file(&out);
    }

    /// 验证 selected_row_indices 只保留指定行。
    #[test]
    fn export_records_json_filters_rows_by_indices() {
        let csv_path = write_temp_csv(
            &["id", "name"],
            &[vec!["1", "a"], vec!["2", "b"], vec!["3", "c"]],
        );
        let out = csv_path.with_extension("json");
        crate::commands::export_records_json(
            csv_path.to_string_lossy().into_owned(),
            empty_ruleset(),
            vec!["id".to_string(), "name".to_string()],
            vec![],
            Some(vec![0, 2]),
            out.to_string_lossy().into_owned(),
        )
        .unwrap();

        let body = std::fs::read_to_string(&out).unwrap();
        let parsed: Vec<HashMap<String, String>> = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed.len(), 2, "只保留索引 0/2 两行");
        assert_eq!(parsed[0].get("id").unwrap(), "1");
        assert_eq!(parsed[1].get("id").unwrap(), "3");

        let _ = std::fs::remove_file(&csv_path);
        let _ = std::fs::remove_file(&out);
    }

    // ─── v0.4.4 T10-2 extract 命令 rules_json + 动态计数 测试 ───

    /// 传 rules_json = 含 email 规则的 RuleSet，验证 extract_text 能提取 email
    /// （空规则集不提取任何类型，传 rules_json 才能提取 email）。
    #[test]
    fn extract_text_with_rules_json_extracts_custom_type() {
        let rules_json = serde_json::json!({
            "maskers": [],
            "validators": [
                { "field": "email", "scope": "email", "tag": "extract" }
            ]
        })
        .to_string();
        let content = "mail test@example.com, also hello@x.cn".to_string();
        let result = extract_text(content, Some(rules_json)).expect("extract_text");
        let findings = result["findings"].as_array().expect("findings array");
        assert!(
            findings
                .iter()
                .any(|f| f["type"] == "email" && f["value"] == "test@example.com"),
            "email not extracted: {:?}",
            findings
        );
        assert!(
            findings
                .iter()
                .any(|f| f["type"] == "email" && f["value"] == "hello@x.cn"),
            "second email not extracted: {:?}",
            findings
        );
        // 不应含 phone（rules_json 未指定 phone）
        assert!(
            !findings.iter().any(|f| f["type"] == "phone"),
            "phone should not be extracted: {:?}",
            findings
        );
    }

    /// 不传 rules_json（None）走空规则集（v0.4.4：builtin 已删除），
    /// 不提取任何类型，findings 为空。
    #[test]
    fn extract_text_without_rules_json_returns_empty() {
        // 用合成测试号（非真实 PII）
        let content = "phone 13812345678, ip 192.168.1.1".to_string();
        let result = extract_text(content, None).expect("extract_text");
        let findings = result["findings"].as_array().expect("findings array");
        // v0.4.4：空规则集 -> 空 findings（内置规则集已删除）
        assert!(
            findings.is_empty(),
            "empty ruleset should produce no findings, got: {:?}",
            findings
        );
        // counts 为空对象
        assert!(
            result["counts"].as_object().unwrap().is_empty(),
            "counts should be empty, got: {:?}",
            result["counts"]
        );
    }

    /// package_extract_result 对任意 type 动态计数（不只 phone/bankcard/ip）。
    #[test]
    fn package_extract_result_counts_dynamic_types() {
        use ruT0_data_kit_core::report::Finding;
        let raw = vec![
            Finding { r#type: "phone".into(), value: "13812345678".into(), location: None, valid: Some(true), context: None, extra: None },
            Finding { r#type: "email".into(), value: "a@b.cn".into(), location: None, valid: Some(true), context: None, extra: None },
            Finding { r#type: "email".into(), value: "c@d.cn".into(), location: None, valid: Some(true), context: None, extra: None },
            Finding { r#type: "mac".into(), value: "00:11:22:33:44:55".into(), location: None, valid: Some(true), context: None, extra: None },
            // 重复项应被去重
            Finding { r#type: "email".into(), value: "a@b.cn".into(), location: None, valid: Some(true), context: None, extra: None },
        ];
        let result = package_extract_result(raw);
        // findings 去重后 4 条（email a@b.cn 只算一次）
        assert_eq!(result["findings"].as_array().unwrap().len(), 4);
        // counts 动态聚合：email=2（a@b.cn 去重 + c@d.cn）、phone=1、mac=1
        assert_eq!(result["counts"]["email"], 2);
        assert_eq!(result["counts"]["phone"], 1);
        assert_eq!(result["counts"]["mac"], 1);
        // 不应再硬编码 bankcard/ip=0（BTreeMap 只含实际出现过的 type）
        assert!(result["counts"].get("bankcard").is_none());
        assert!(result["counts"].get("ip").is_none());
    }
}
