//! 内置规则集（v0.4.5 起逐步接入）。
//!
//! v0.4.4 规则引擎重构后规则池初始为空，只留好 `scope`/`tag` 字段接口。
//! v0.4.5 接入数据提取规则三条：手机号（scope="phone"）、银行卡号
//! （scope="bankcard"）、IP 地址（scope="ip"），覆盖 PDF《数据格式规范文档》
//! 定义的三类需识别提取的数据。
//!
//! `builtin_ruleset()` 返回当前所有内置规则组成的 `RuleSet`，供：
//! - `extract::extract_text` / `extract_file` 在未传 rules_json 时作为默认规则集
//!   （这样无需用户配置即可提取手机号 / 银行卡号 / IP）
//! - tauri `list_builtin_rules` 命令序列化为 JSON 返回给前端，启动时加载到
//!   `state.rules`，让 RulesView / ExtractView 显示内置规则
//!
//! 后续版本按数据类型逐条接入：idcard / email / mac / ...
//! 每条只需在此追加一个 `FieldRule`，无需改 scan/extract 算法。

use crate::rules::{FieldRule, RuleSet};

/// 返回内置规则集（v0.4.5：phone / bankcard / ip 三条数据提取规则）。
///
/// 内置规则是「出厂自带」的，与用户在 RulesView 创建的规则区分。前端启动
/// 时通过 `list_builtin_rules` 命令拉取并写入 `state.rules`，用户可见可用
/// 但不可编辑删除（后续版本若支持用户规则，再叠加合并）。
pub fn builtin_ruleset() -> RuleSet {
    RuleSet {
        validators: vec![phone_extract_rule(), bankcard_extract_rule(), ip_extract_rule()],
        maskers: vec![],
    }
}

/// 手机号提取规则：scope="phone"，tag="extract"。
///
/// - `scope="phone"` 作为 `scan::extract_pattern` 查键（`\b\d{11}\b`）+
///   `ValidatorRegistry::get("phone")` 查键（`patterns::PHONE.validate` 校验首位为 1）。
/// - `tag="extract"` 标记为数据提取用途（RulesView 按标签筛选 / ExtractView
///   勾选规则时按 tag 过滤）。
/// - `field="phone"` 为列名占位（extract 不依赖列名，但字段必填）。
pub fn phone_extract_rule() -> FieldRule {
    FieldRule {
        field: "phone".into(),
        scope: "phone".into(),
        tag: "extract".into(),
        params: None,
        message: None,
        description: Some("手机号提取（11 位数字，首位 1）".into()),
    }
}

/// 银行卡号提取规则：scope="bankcard"，tag="extract"。
///
/// - `scope="bankcard"` 作为 `scan::extract_pattern` 查键（`\b\d{13,19}\b`）+
///   `ValidatorRegistry::get("bankcard")` 查键（`BankCardValidator` 做 Luhn
///   校验：从右起第 1 位为校验位，偶数位乘 2 超 9 减 9，总和对 10 取模为 0）。
/// - `tag="extract"` 标记为数据提取用途。
/// - `field="bankcard"` 为列名占位（extract 不依赖列名，但字段必填）。
pub fn bankcard_extract_rule() -> FieldRule {
    FieldRule {
        field: "bankcard".into(),
        scope: "bankcard".into(),
        tag: "extract".into(),
        params: None,
        message: None,
        description: Some("银行卡号提取（13-19 位数字，Luhn 校验）".into()),
    }
}

/// IP 地址提取规则：scope="ip"，tag="extract"。
///
/// - `scope="ip"` 作为 `scan::extract_pattern` 查键（IPv4 四段 0-255 正则）+
///   `ValidatorRegistry::get("ip")` 查键（`IpValidator` 用 `IP_REGEX` 二次
///   校验，四段 0-255）。
/// - `tag="extract"` 标记为数据提取用途。
/// - `field="ip"` 为列名占位（extract 不依赖列名，但字段必填）。
pub fn ip_extract_rule() -> FieldRule {
    FieldRule {
        field: "ip".into(),
        scope: "ip".into(),
        tag: "extract".into(),
        params: None,
        message: None,
        description: Some("IP 地址提取（IPv4 四段 0-255，拒绝前导零）".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::extract_text_with_rules;
    use crate::scan::{DefaultSensitiveScan, SensitiveScan};

    #[test]
    fn builtin_ruleset_has_three_extract_rules() {
        let rs = builtin_ruleset();
        assert_eq!(rs.validators.len(), 3);
        assert_eq!(rs.maskers.len(), 0);
        // phone
        assert_eq!(rs.validators[0].field, "phone");
        assert_eq!(rs.validators[0].scope, "phone");
        assert_eq!(rs.validators[0].tag, "extract");
        // bankcard
        assert_eq!(rs.validators[1].field, "bankcard");
        assert_eq!(rs.validators[1].scope, "bankcard");
        assert_eq!(rs.validators[1].tag, "extract");
        // ip
        assert_eq!(rs.validators[2].field, "ip");
        assert_eq!(rs.validators[2].scope, "ip");
        assert_eq!(rs.validators[2].tag, "extract");
    }

    #[test]
    fn phone_extract_rule_fields() {
        let r = phone_extract_rule();
        assert_eq!(r.field, "phone");
        assert_eq!(r.scope, "phone");
        assert_eq!(r.tag, "extract");
        assert!(r.params.is_none());
        assert!(r.message.is_none());
        assert!(r.description.is_some());
    }

    #[test]
    fn bankcard_extract_rule_fields() {
        let r = bankcard_extract_rule();
        assert_eq!(r.field, "bankcard");
        assert_eq!(r.scope, "bankcard");
        assert_eq!(r.tag, "extract");
        assert!(r.params.is_none());
        assert!(r.message.is_none());
        assert!(r.description.is_some());
    }

    #[test]
    fn ip_extract_rule_fields() {
        let r = ip_extract_rule();
        assert_eq!(r.field, "ip");
        assert_eq!(r.scope, "ip");
        assert_eq!(r.tag, "extract");
        assert!(r.params.is_none());
        assert!(r.message.is_none());
        assert!(r.description.is_some());
    }

    /// 内置规则集能从文本提取手机号（scan 路径：scope -> extract_pattern
    /// -> find_iter -> PhoneValidator 校验首位 1 -> valid==true 记 finding）。
    #[test]
    fn builtin_ruleset_extracts_phone_from_text() {
        let rs = builtin_ruleset();
        let scan = DefaultSensitiveScan::new();
        // 合成测试号（非真实 PII）：首位 1 + 11 位
        let findings = scan.scan("contact 13812345678 or 15500001111", &rs);
        let phones: Vec<&str> = findings
            .iter()
            .filter(|f| f.r#type == "phone")
            .map(|f| f.value.as_str())
            .collect();
        assert!(phones.contains(&"13812345678"), "got {:?}", phones);
        assert!(phones.contains(&"15500001111"), "got {:?}", phones);
    }

    /// 非首位 1 的 11 位数字不记 finding（PhoneValidator 校验失败）。
    #[test]
    fn builtin_ruleset_skips_non_1_prefix_phone() {
        let rs = builtin_ruleset();
        let scan = DefaultSensitiveScan::new();
        let findings = scan.scan("nope 22345678901", &rs);
        assert!(
            findings.iter().all(|f| f.value != "22345678901"),
            "non-1-prefix should not be extracted: {:?}",
            findings
        );
    }

    /// extract_text_with_rules 传内置规则集等价于 extract_text 默认行为。
    #[test]
    fn extract_text_with_builtin_ruleset_finds_phone() {
        let rs = builtin_ruleset();
        let findings = extract_text_with_rules("call 13812345678", &rs);
        assert!(findings.iter().any(|f| f.r#type == "phone" && f.value == "13812345678"));
    }

    /// 内置规则集能从文本提取通过 Luhn 校验的银行卡号。
    /// 合成号 62258800000000002（17 位，非真实 PII，通过 Luhn）。
    #[test]
    fn builtin_ruleset_extracts_bankcard_from_text() {
        let rs = builtin_ruleset();
        let scan = DefaultSensitiveScan::new();
        let findings = scan.scan("card 62258800000000002 end", &rs);
        let cards: Vec<&str> = findings
            .iter()
            .filter(|f| f.r#type == "bankcard")
            .map(|f| f.value.as_str())
            .collect();
        assert!(cards.contains(&"62258800000000002"), "got {:?}", cards);
    }

    /// 未通过 Luhn 校验的卡号不记 finding（BankCardValidator 校验失败）。
    /// 合成号 6222021234567890124（19 位，PDF 示例，未通过 Luhn）。
    #[test]
    fn builtin_ruleset_skips_invalid_bankcard() {
        let rs = builtin_ruleset();
        let scan = DefaultSensitiveScan::new();
        let findings = scan.scan("card 6222021234567890124 end", &rs);
        assert!(
            findings
                .iter()
                .all(|f| f.value != "6222021234567890124"),
            "invalid luhn bankcard should not be extracted: {:?}",
            findings
        );
    }

    /// 内置规则集能从文本提取 IPv4 地址（192.168.1.1）。
    #[test]
    fn builtin_ruleset_extracts_ip_from_text() {
        let rs = builtin_ruleset();
        let scan = DefaultSensitiveScan::new();
        let findings = scan.scan("server 192.168.1.1 up", &rs);
        let ips: Vec<&str> = findings
            .iter()
            .filter(|f| f.r#type == "ip")
            .map(|f| f.value.as_str())
            .collect();
        assert!(ips.contains(&"192.168.1.1"), "got {:?}", ips);
    }

    /// 段 > 255 的非法 IP 不记 finding（IpValidator 校验失败）。
    /// 256.1.1.1 第一段 256 > 255。
    #[test]
    fn builtin_ruleset_skips_invalid_ip() {
        let rs = builtin_ruleset();
        let scan = DefaultSensitiveScan::new();
        let findings = scan.scan("bad 256.1.1.1 here", &rs);
        assert!(
            findings.iter().all(|f| f.value != "256.1.1.1"),
            "invalid ip (>255) should not be extracted: {:?}",
            findings
        );
    }
}
