//! 个人信息规范——手机号码校验器。
//!
//! 与 `validators::phone::PhoneValidator`（通用手机号：11 位、首位 1，去绝对化、
//! 不带号段白名单，供 extract 路径使用）不同，本校验器**严格按规范 spec**：
//! 11 位 10 进制数字，且前 3 位必须落在规范指定的 52 个「虚假号段」集合内。
//!
//! 规范明确说明：这些号段是「为避免与现实中存在冲突」而设计的虚假号段，
//! 仅限本规范的数据校验场景使用，不替代通用 `phone` scope。
//!
//! 与 v0.4.3「删除绝对化内容」不冲突：那次删除的是散布在通用 phone 校验器里的
//! 硬编码 *真实* 号段白名单；这里是另一份规则文档明确定义的*虚假*号段集合，
//! 作为独立的 `pinfo_phone` scope 接入，通用 `phone` scope 行为保持不变。
//!
//! v0.4.4：52 个前缀不再硬编码不可变——`params.prefixes`（YAML 字符串序列）
//! 可覆盖默认集合，缺省回退到内置 52 前缀。这让用户能在 RulesView 等前端
//! 直接编辑号段集合，无需改 Rust。

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::OnceLock;

use serde_yml::Value;

use crate::validators::{ValidationResult, Validator};

/// 规范指定的虚假号段集合（前 3 位，共 52 个）。
///
/// 来源：规范「手机号码(phone)」节，按出现顺序原样罗列。当 `params.prefixes`
/// 未提供时作为默认集合使用。
const PINFO_PHONE_PREFIXES: [&str; 52] = [
    "734", "735", "736", "737", "738", "739", "747", "748", "750", "751", "752", "757", "758",
    "759", "772", "778", "782", "783", "784", "787", "788", "795", "798", "730", "731", "732",
    "740", "745", "746", "755", "756", "766", "767", "771", "775", "776", "785", "786", "796",
    "733", "749", "753", "773", "774", "777", "780", "781", "789", "790", "791", "793", "799",
];

static DEFAULT_PREFIX_SET: OnceLock<HashSet<&'static str>> = OnceLock::new();

fn default_prefix_set() -> &'static HashSet<&'static str> {
    DEFAULT_PREFIX_SET.get_or_init(|| PINFO_PHONE_PREFIXES.iter().copied().collect())
}

/// 个人信息规范手机号码校验器。
///
/// `prefixes` 为 `None` 时使用默认 52 前缀；`Some(set)` 时用自定义集合。
pub struct PInfoPhoneValidator {
    prefixes: Option<HashSet<String>>,
}

impl PInfoPhoneValidator {
    /// 从 params 构造。识别 `prefixes` 参数（YAML 字符串序列）；
    /// 未提供或缺省时 `prefixes=None`（走默认集合）。
    pub fn new(params: HashMap<String, Value>) -> Self {
        let prefixes = params
            .get("prefixes")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect::<HashSet<String>>()
            })
            .filter(|s| !s.is_empty());
        Self { prefixes }
    }

    /// 命中判定：优先用自定义集合，否则默认集合。
    fn prefix_matches(&self, prefix: &str) -> bool {
        match &self.prefixes {
            Some(custom) => custom.contains(prefix),
            None => default_prefix_set().contains(prefix),
        }
    }
}

impl Validator for PInfoPhoneValidator {
    fn validate(&self, value: &str) -> ValidationResult {
        let v = value.trim();
        if v.len() != 11 || !v.chars().all(|c| c.is_ascii_digit()) {
            return ValidationResult::fail("phone must be 11 decimal digits");
        }
        let prefix = &v[..3];
        if self.prefix_matches(prefix) {
            ValidationResult::ok()
        } else {
            ValidationResult::fail("phone prefix not in pinfo spec set")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v() -> PInfoPhoneValidator {
        PInfoPhoneValidator::new(HashMap::new())
    }

    #[test]
    fn spec_example_valid() {
        // 规范示例行：78813630178（前缀 788 ∈ 集合）
        assert!(v().validate("78813630178").valid);
        // 注：规范另一示例 81825660184 前缀 818 不在集合内，与 spec 矛盾，
        // 本校验器按 spec 集合判定 → 失败。
        assert!(!v().validate("81825660184").valid);
    }

    #[test]
    fn prefix_in_set_positive() {
        for p in ["734", "799", "777", "780", "791"] {
            let n = format!("{p}12345678");
            assert!(v().validate(&n).valid, "{n} should be valid");
        }
    }

    #[test]
    fn prefix_not_in_set_fails() {
        // 138 / 159 是真实号段，不在虚假号段集合 → 失败
        assert!(!v().validate("13812345678").valid);
        assert!(!v().validate("15987654321").valid);
        // 818 也不在集合（规范示例行 81825660184 与 spec 集合矛盾，按 spec 判定）
        assert!(!v().validate("81825660184").valid);
    }

    #[test]
    fn wrong_length_fails() {
        assert!(!v().validate("7341234567").valid); // 10
        assert!(!v().validate("734123456789").valid); // 12
    }

    #[test]
    fn non_digit_fails() {
        assert!(!v().validate("7341234567a").valid);
        assert!(!v().validate("734-2345678").valid);
    }

    #[test]
    fn empty_fails() {
        assert!(!v().validate("").valid);
    }

    #[test]
    fn default_prefix_set_has_52_entries() {
        assert_eq!(PINFO_PHONE_PREFIXES.len(), 52);
        assert_eq!(default_prefix_set().len(), 52, "no duplicates expected");
    }

    // -------- 自定义前缀参数测试 --------

    fn make_params(prefixes: &[&str]) -> HashMap<String, Value> {
        let seq = Value::Sequence(
            prefixes
                .iter()
                .map(|s| Value::String((*s).to_string()))
                .collect(),
        );
        let mut m = HashMap::new();
        m.insert("prefixes".to_string(), seq);
        m
    }

    #[test]
    fn custom_prefixes_override_default() {
        // 只放 138 / 159（真实号段，不在号段集合）→ 自定义后应通过。
        let v = PInfoPhoneValidator::new(make_params(&["138", "159"]));
        assert!(v.validate("13812345678").valid);
        assert!(v.validate("15987654321").valid);
        // 788 在默认集合但不在自定义集合 → 失败。
        assert!(!v.validate("78813630178").valid);
    }

    #[test]
    fn empty_prefixes_param_falls_back_to_default() {
        // prefixes=[] 或单元素空 → 视作未提供，走默认集合。
        let empty_seq = Value::Sequence(vec![]);
        let mut m = HashMap::new();
        m.insert("prefixes".to_string(), empty_seq);
        let v = PInfoPhoneValidator::new(m);
        assert!(v.validate("78813630178").valid, "empty prefixes should fall back");
        assert!(!v.validate("13812345678").valid);
    }

    #[test]
    fn no_prefixes_param_uses_default() {
        let v = PInfoPhoneValidator::new(HashMap::new());
        assert!(v.validate("78813630178").valid);
        assert!(!v.validate("13812345678").valid);
    }
}
