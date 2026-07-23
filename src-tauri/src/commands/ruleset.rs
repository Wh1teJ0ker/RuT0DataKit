//! 规则集 YAML 存取 + 标签清单 + 内置规则集命令（4 命令）。

use ruT0_data_kit_core::rules::{builtin_ruleset};
use serde_json::Value;

use super::parse_ruleset_json;

/// 把 `rules_json`（RuleSet 的 JSON 序列化）反序列化校验后用 `serde_yml`
/// 序列化为 YAML 写盘。`RuleSet` 本身只 derive `Deserialize`，这里先解析
/// 成 `serde_json::Value`（既能校验 JSON 合法性，又能直接给 serde_yml 序列化）。
#[tauri::command]
pub fn save_ruleset(rules_json: String, out_path: String) -> Result<(), String> {
    // 先反序列化为 RuleSet 校验结构合法性（field/scope/tag 字段齐全）。
    let _rules = parse_ruleset_json(&rules_json)?;
    // 再把原始 JSON 解析为 Value，交给 serde_yml 输出 YAML。这样无需 RuleSet
    // 实现 Serialize（core 端 RuleSet 只 derive Deserialize，T0-16 不动 core）。
    let value: Value = serde_json::from_str(&rules_json)
        .map_err(|e| format!("rules_json 解析失败: {e}"))?;
    let yaml = serde_yml::to_string(&value).map_err(|e| format!("YAML 序列化失败: {e}"))?;
    std::fs::write(&out_path, yaml).map_err(|e| format!("写入失败: {e}"))
}

/// 读取 YAML 规则集文件内容为字符串返回。前端用 js-yaml 解析后 dispatch SET_RULES。
///
/// 只做读盘，不做结构校验；校验交给前端的 SET_RULES reducer 与再次 save_ruleset
/// 时的 parse_ruleset_json。这样允许用户加载部分可解析的 YAML（例如含注释）。
#[tauri::command]
pub fn read_ruleset(path: String) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("读取失败: {e}"))
}

/// 返回规则标签清单（静态三选一）：`["extract","mask","validate"]`。
///
/// v0.4.4 规则引擎重构后，规则池初始为空，`tag` 为单值字段（非多标签），
/// 三选一对应「数据提取 / 数据脱敏 / 数据校验」三种用途。后续版本接入
/// 规则添加入口时，前端 RulesView 的 tag 下拉源使用此清单。
///
/// `rules_json` 参数保留（向后兼容前端调用签名），但当前实现不读取它
/// （标签为静态清单）。后续若需从用户 ruleset 合并自定义 tag，可在此
/// 扩展。
#[tauri::command]
pub fn list_rule_tags(_rules_json: Option<String>) -> Result<Vec<String>, String> {
    Ok(vec![
        "extract".to_string(),
        "mask".to_string(),
        "validate".to_string(),
    ])
}

/// 返回内置规则集（v0.4.5 起逐步接入）的 JSON 序列化。
///
/// v0.4.5：内置规则集含手机号提取（scope="phone", tag="extract"）一条。
/// 前端 App 启动时调此命令拉取，写入 `state.rules`，让 RulesView /
/// ExtractView / MaskView / ValidateView 可见可用。
///
/// 返回 `serde_json::Value`（对象 `{ maskers: [], validators: [...] }`），
/// 前端直接 `JSON.parse` 后 dispatch `SET_RULES`。
#[tauri::command]
pub fn list_builtin_rules() -> Result<Value, String> {
    let rs = builtin_ruleset();
    // 用 serde_json 序列化 RuleSet（与 extract_text 的 rules_json 路径一致）。
    serde_json::to_value(&rs).map_err(|e| format!("序列化内置规则集失败: {e}"))
}
