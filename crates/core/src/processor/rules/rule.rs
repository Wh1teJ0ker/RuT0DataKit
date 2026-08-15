//! 规则定义：`Rule` 结构 + `RuleKind` 枚举。
//!
//! v1.2.1：从 `rules/mod.rs` 拆出，承载 `Rule` / `RuleKind` 类型定义。
//! `RuleRegistry` 见 [`crate::processor::rules::registry`]，
//! 内置规则构造器见 [`crate::processor::rules::builtins`]。

use serde::{Deserialize, Serialize};

use crate::processor::rules::extract_params::ExtractParams;
use crate::processor::rules::template::TemplateParams;

/// 规则类型。决定规则被哪个处理器消费。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleKind {
    /// 脱敏规则（被 `Masker` 消费）
    Mask,
    /// 校验规则（被 `Validator` 消费）
    Validate,
    /// 提取规则（前端测试时用 pattern 做正则提取预览）
    Extract,
}

impl std::fmt::Display for RuleKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleKind::Mask => write!(f, "mask"),
            RuleKind::Validate => write!(f, "validate"),
            RuleKind::Extract => write!(f, "extract"),
        }
    }
}

impl RuleKind {
    /// 从 lowercase 字符串解析（DB 列反序列化用）。
    pub fn from_str_lowercase(s: &str) -> Option<Self> {
        match s {
            "mask" => Some(Self::Mask),
            "validate" => Some(Self::Validate),
            "extract" => Some(Self::Extract),
            _ => None,
        }
    }
}

/// 规则定义。前后端经 serde camelCase 序列化统一。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    /// 规则 ID（唯一标识，如 `"name-validate"`）。
    pub id: String,
    /// 规则显示名（如 `"姓名校验"`）。
    pub name: String,
    /// 规则类型。
    pub kind: RuleKind,
    /// 目标列名（如 `"name"`）。`None` 表示不限定列。
    pub field: Option<String>,
    /// 正则模式串（校验/提取用，脱敏可选）。`None` 表示不使用正则。
    pub pattern: Option<String>,
    /// 脱敏掩码字符（取首个字符；`None`/空 → 默认 `*`）。
    /// 由 `SimpleMasker` 在「保留首字符 + 其余用掩码字符替换」逻辑中消费，
    /// 也可临时覆盖模板的 `mask_char`（优先级：replacement > template.mask_char > 默认 `*`）。
    pub replacement: Option<String>,
    /// 启用/禁用。
    pub enabled: bool,
    /// 规则说明。
    pub description: String,
    /// 通用模板脱敏参数（v1.1.3 新增）。`None` → 走 `SimpleMasker` 旧逻辑
    /// （保留首尾各 1），向后兼容 v1.1.0~v1.1.2 的 name-mask 规则。
    /// T49：`general-mask` 规则持空模板（`TemplateParams::default()`，
    /// 所有字段 `None`）→ `SimpleMasker` 视为不脱敏（透传）。前端选预设后
    /// 填充字段 → 走模板脱敏。DB 存为 `template TEXT`（JSON 串）。
    /// `#[serde(default, skip_serializing_if = "Option::is_none")]` 保证旧 Rule JSON
    /// （无 template 字段）反序列化时 `template=None`，且序列化时不输出空字段，
    /// 前端拿到的 name-mask 规则 JSON 形态不变。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<TemplateParams>,
    /// 提取规则的函数式校验参数（v1.1.3 T55 新增）。`None` = 仅正则提取，
    /// 不做额外校验（向后兼容 name-extract 等老规则）。
    /// serde `default` + `skip_serializing_if = "Option::is_none"` 保证旧 Rule JSON
    /// （无 params 字段）反序列化时 `params=None`，且序列化时不输出空字段。
    /// DB 存为 `params TEXT`（`ExtractParams` 的 JSON 串）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<ExtractParams>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::template::{SegmentMask, SegmentTemplate, SimpleTemplate, TemplateParams};
    use super::super::extract_params::ExtractParams;

    /// 测试辅助：断言模板是 Simple 变体并返回内部 `&SimpleTemplate`。
    fn expect_simple(tpl: &TemplateParams) -> &SimpleTemplate {
        match tpl {
            TemplateParams::Simple(s) => s,
            TemplateParams::Segment(_) => panic!("expected Simple, got Segment"),
        }
    }

    #[test]
    fn rulekind_serializes_lowercase() {
        let json = serde_json::to_string(&RuleKind::Validate).unwrap();
        assert_eq!(json, "\"validate\"");
        let m: RuleKind = serde_json::from_str("\"mask\"").unwrap();
        assert_eq!(m, RuleKind::Mask);
    }

    #[test]
    fn rulekind_from_str_lowercase_round_trip() {
        for k in [RuleKind::Mask, RuleKind::Validate, RuleKind::Extract] {
            let s = k.to_string();
            assert_eq!(RuleKind::from_str_lowercase(&s), Some(k));
        }
        assert_eq!(RuleKind::from_str_lowercase("nope"), None);
    }

    #[test]
    fn rule_params_field_serde_skip_when_none() {
        // name-mask 无 params → JSON 不输出 params 字段（向后兼容）
        let r = Rule {
            id: "name-mask".into(),
            name: "姓名脱敏".into(),
            kind: RuleKind::Mask,
            field: Some("name".into()),
            pattern: None,
            replacement: None,
            enabled: true,
            description: "保留首尾字符，中间以 * 替换".into(),
            template: None,
            params: None,
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("params"));
    }

    #[test]
    fn rule_params_field_serde_present_when_some() {
        // phone-extract 有 params → JSON 输出 params 字段
        let r = Rule {
            id: "phone-extract".into(),
            name: "手机号提取".into(),
            kind: RuleKind::Extract,
            field: None,
            pattern: Some(r"\b[1-9]\d{10}\b".into()),
            replacement: None,
            enabled: true,
            description: "提取 11 位手机号".into(),
            template: None,
            params: Some(ExtractParams::PhonePrefix {
                allowed_prefixes: Vec::new(),
            }),
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("params"));
    }

    #[test]
    fn rule_params_field_deserialize_missing_as_none() {
        // 模拟 v1.1.4 旧 JSON（无 params 字段）→ 反序列化 params=None
        let old_json = r#"{"id":"name-mask","name":"姓名脱敏","kind":"mask","field":"name","pattern":null,"replacement":null,"enabled":true,"description":"保留首尾字符，中间以 * 替换"}"#;
        let r: Rule = serde_json::from_str(old_json).unwrap();
        assert_eq!(r.id, "name-mask");
        assert!(r.params.is_none());
    }

    #[test]
    fn rule_template_field_serde_skip_when_none() {
        // name-mask 无 template → JSON 不输出 template 字段（向后兼容 v1.1.2 前端）
        let r = Rule {
            id: "name-mask".into(),
            name: "姓名脱敏".into(),
            kind: RuleKind::Mask,
            field: Some("name".into()),
            pattern: None,
            replacement: None,
            enabled: true,
            description: "保留首尾字符，中间以 * 替换".into(),
            template: None,
            params: None,
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("template"));
    }

    #[test]
    fn rule_template_field_serde_present_when_some() {
        // simple-mask 有 template（空 Simple 模板）→ JSON 输出 template 字段
        let r = Rule {
            id: "simple-mask".into(),
            name: "整段脱敏".into(),
            kind: RuleKind::Mask,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: "整段模板脱敏".into(),
            template: Some(TemplateParams::Simple(SimpleTemplate::default())),
            params: None,
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("template"));
    }

    #[test]
    fn rule_template_field_deserialize_missing_as_none() {
        // 模拟 v1.1.2 旧 JSON（无 template 字段）→ 反序列化 template=None
        let old_json = r#"{"id":"name-mask","name":"姓名脱敏","kind":"mask","field":"name","pattern":null,"replacement":null,"enabled":true,"description":"保留首尾字符，中间以 * 替换"}"#;
        let r: Rule = serde_json::from_str(old_json).unwrap();
        assert_eq!(r.id, "name-mask");
        assert!(r.template.is_none());
    }

    // ---- 模板预设参数测试（从原 mod.rs 迁移）----

    #[test]
    fn idcard_preset_params() {
        let tpl = super::super::template::idcard_preset();
        let s = expect_simple(&tpl);
        assert_eq!(s.keep_prefix, Some(6));
        assert_eq!(s.keep_suffix, Some(4));
        assert_eq!(s.mask_min_len, Some(8));
        // T53：预设不反转（正向）
        assert_eq!(s.reverse, None);
        assert!(!tpl.is_empty());
    }

    #[test]
    fn phone_preset_params() {
        let tpl = super::super::template::phone_preset();
        let s = expect_simple(&tpl);
        assert_eq!(s.keep_prefix, Some(3));
        assert_eq!(s.keep_suffix, Some(4));
        assert_eq!(s.mask_min_len, Some(4));
        assert_eq!(s.reverse, None);
    }

    #[test]
    fn birthdate_preset_params() {
        let tpl = super::super::template::birthdate_preset();
        let s = expect_simple(&tpl);
        assert_eq!(s.keep_prefix, Some(8));
        assert_eq!(s.keep_suffix, Some(0));
        assert_eq!(s.mask_min_len, Some(2));
        assert_eq!(s.reverse, None);
    }

    #[test]
    fn bankcard_preset_params() {
        let tpl = super::super::template::bankcard_preset();
        let s = expect_simple(&tpl);
        assert_eq!(s.keep_prefix, Some(4));
        assert_eq!(s.keep_suffix, Some(4));
        assert_eq!(s.mask_min_len, Some(1));
        assert_eq!(s.reverse, None);
    }

    #[test]
    fn template_params_serde_roundtrip() {
        let tpl = super::super::template::idcard_preset();
        let json = serde_json::to_string(&tpl).unwrap();
        // camelCase 序列化
        assert!(json.contains("keepPrefix"));
        assert!(json.contains("keepSuffix"));
        assert!(json.contains("maskMinLen"));
        let back: TemplateParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back, tpl);
    }

    #[test]
    fn reverse_template_serde_roundtrip() {
        // T53：reverse=true 的 Simple 模板序列化/反序列化闭环
        let tpl = TemplateParams::Simple(SimpleTemplate::new(3, 4, 1).with_reverse(true));
        let json = serde_json::to_string(&tpl).unwrap();
        assert!(json.contains("reverse"));
        assert!(json.contains("\"reverse\":true"));
        let back: TemplateParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back, tpl);
        let s = expect_simple(&back);
        assert_eq!(s.reverse, Some(true));
    }

    #[test]
    fn segment_template_serde_roundtrip() {
        let tpl = TemplateParams::Segment(
            SegmentTemplate::new("@")
                .with_segment(0, 1, 1, 4)
                .with_mask_char('*'),
        );
        let json = serde_json::to_string(&tpl).unwrap();
        assert!(json.contains("delimiter"));
        assert!(json.contains("segments"));
        let back: TemplateParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back, tpl);
        assert!(!tpl.is_empty());
    }

    #[test]
    fn old_flat_json_deserializes_to_simple() {
        // 旧 DB 里的 flat JSON（无 delimiter/reverse，但含已删除的 minLen/maxLen）
        // → Segment 变体无 delimiter 失败 → 回退到 Simple，多余字段被忽略
        let old = r#"{"keepPrefix":6,"keepSuffix":4,"maskChar":"*","maskMinLen":8,"minLen":18,"maxLen":18}"#;
        let tpl: TemplateParams = serde_json::from_str(old).unwrap();
        match tpl {
            TemplateParams::Simple(s) => {
                assert_eq!(s.keep_prefix, Some(6));
                assert_eq!(s.keep_suffix, Some(4));
                // T53：旧 JSON 无 reverse 字段 → None → 正向
                assert_eq!(s.reverse, None);
            }
            TemplateParams::Segment(_) => panic!("expected Simple, got Segment"),
        }
    }

    #[test]
    fn segment_is_empty_when_no_segments() {
        let tpl = TemplateParams::Segment(SegmentTemplate::new("@"));
        assert!(tpl.is_empty());
    }

    #[test]
    fn segment_is_empty_when_empty_delimiter() {
        let tpl = TemplateParams::Segment(SegmentTemplate::new("").with_segment(0, 1, 1, 1));
        assert!(tpl.is_empty());
    }

    #[test]
    fn extract_params_serde_roundtrip() {
        // v1.1.5 T81：ExtractParams 十一变体 serde 闭环
        let cases = vec![
            ExtractParams::PhonePrefix {
                allowed_prefixes: vec!["134".into(), "159".into()],
            },
            ExtractParams::Luhn,
            ExtractParams::Ipv4,
            ExtractParams::Ipv6,
            ExtractParams::IdCard,
            ExtractParams::Username,
            ExtractParams::Sex,
            ExtractParams::Birth { formats: vec![] },
            ExtractParams::Address,
            ExtractParams::Email,
            ExtractParams::Generic {
                allow_digits: true,
                allow_letters: true,
                allow_special_chars: "_-.@".into(),
                min_len: Some(3),
                max_len: None,
            },
        ];
        for p in &cases {
            let json = serde_json::to_string(p).unwrap();
            let back: ExtractParams = serde_json::from_str(&json).unwrap();
            assert_eq!(back, *p, "roundtrip failed for {json}");
        }
        // 验证内部标签格式
        assert!(serde_json::to_string(&ExtractParams::Luhn)
            .unwrap()
            .contains("\"validator\":\"luhn\""));
        assert!(serde_json::to_string(&ExtractParams::PhonePrefix {
            allowed_prefixes: vec![]
        })
        .unwrap()
        .contains("\"validator\":\"phonePrefix\""));
        assert!(serde_json::to_string(&ExtractParams::Ipv4)
            .unwrap()
            .contains("\"validator\":\"ipv4\""));
        assert!(serde_json::to_string(&ExtractParams::Ipv6)
            .unwrap()
            .contains("\"validator\":\"ipv6\""));
        assert_eq!(
            serde_json::to_string(&ExtractParams::IdCard).unwrap(),
            r#"{"validator":"idcard"}"#
        );
        // v1.1.4 T67：4 个新变体 camelCase 标签
        assert!(serde_json::to_string(&ExtractParams::Username)
            .unwrap()
            .contains("\"validator\":\"username\""));
        assert!(serde_json::to_string(&ExtractParams::Sex)
            .unwrap()
            .contains("\"validator\":\"sex\""));
        assert!(
            serde_json::to_string(&ExtractParams::Birth { formats: vec![] })
                .unwrap()
                .contains("\"validator\":\"birth\"")
        );
        // v1.1.5 T87：Birth 向后兼容 —— {"validator":"birth"} 可反序列化为 Birth { formats: vec![] }
        let birth_json: ExtractParams = serde_json::from_str(r#"{"validator":"birth"}"#).unwrap();
        assert!(matches!(birth_json, ExtractParams::Birth { .. }));
        // T87：formats 字段被序列化
        assert!(serde_json::to_string(&ExtractParams::Birth {
            formats: vec!["yyyymmdd".into()]
        })
        .unwrap()
        .contains("\"formats\""));
        assert!(serde_json::to_string(&ExtractParams::Address)
            .unwrap()
            .contains("\"validator\":\"address\""));
        // v1.1.5 T81：Email 变体 camelCase 标签
        assert!(serde_json::to_string(&ExtractParams::Email)
            .unwrap()
            .contains("\"validator\":\"email\""));
        // v1.1.4 续轮 T70：Generic 变体 camelCase 标签 + 字段
        // T77：allow_special: bool → allow_special_chars: String（自定义白名单）
        let generic_json = serde_json::to_string(&ExtractParams::Generic {
            allow_digits: true,
            allow_letters: true,
            allow_special_chars: "_-.@".into(),
            min_len: Some(3),
            max_len: None,
        })
        .unwrap();
        assert!(generic_json.contains("\"validator\":\"generic\""));
        assert!(generic_json.contains("\"allowDigits\":true"));
        assert!(generic_json.contains("\"allowLetters\":true"));
        assert!(generic_json.contains("\"allowSpecialChars\":\"_-.@\""));
        assert!(generic_json.contains("\"minLen\":3"));
        assert!(generic_json.contains("\"maxLen\":null"));
    }
}
