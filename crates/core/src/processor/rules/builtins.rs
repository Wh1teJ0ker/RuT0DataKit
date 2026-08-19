//! 内置规则表（18 条）。
//!
//! v1.2.1：18 个 `pub fn xxx_rule() -> Rule` 构造器收敛为表驱动
//! `LazyLock<Vec<Rule>>`，`RuleRegistry::with_defaults()` 遍历 `all()` 注册。
//! 历史版本变更见 `RuleRegistry::with_defaults` 文档注释。

use std::sync::LazyLock;

use crate::processor::rules::extract_params::ExtractParams;
use crate::processor::rules::rule::{Rule, RuleKind};
use crate::processor::rules::template::{SegmentTemplate, SimpleTemplate, TemplateParams};

/// 内置规则表。启动时由 `RuleRegistry::with_defaults()` 遍历注册到 DB。
///
/// 顺序：3 姓名 + 2 脱敏 + 6 提取 + 7 校验 = 18 条。
static BUILTIN_RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    vec![
        // ---- 姓名（校验 / 脱敏 / 提取）----
        Rule { id: "name-validate".into(), name: "姓名校验".into(), kind: RuleKind::Validate, field: Some("name".into()), pattern: Some(r"^[\u4e00-\u9fa5]{2,4}$".into()), replacement: None, enabled: true, description: "校验姓名为 2-4 位中文字符".into(), template: None, params: None },
        // name-mask: 无 template → SimpleMasker 旧逻辑（保留首尾各 1）
        Rule { id: "name-mask".into(), name: "姓名脱敏".into(), kind: RuleKind::Mask, field: Some("name".into()), pattern: None, replacement: None, enabled: true, description: "保留首尾字符，中间以 * 替换".into(), template: None, params: None },
        Rule { id: "name-extract".into(), name: "姓名提取".into(), kind: RuleKind::Extract, field: Some("name".into()), pattern: Some(r"[\u4e00-\u9fa5]{2,4}".into()), replacement: None, enabled: true, description: "提取姓名候选：2-4 位连续中文字符".into(), template: None, params: None },
        // ---- 整段 / 分段脱敏（空模板 = 透传，前端选预设后填充）----
        Rule { id: "simple-mask".into(), name: "整段脱敏".into(), kind: RuleKind::Mask, field: None, pattern: None, replacement: None, enabled: true, description: "整段模板脱敏：选预设（身份证/手机/出生日期/银行卡）或自定义参数".into(), template: Some(TemplateParams::Simple(SimpleTemplate::default())), params: None },
        Rule { id: "segment-mask".into(), name: "分段脱敏".into(), kind: RuleKind::Mask, field: None, pattern: None, replacement: None, enabled: true, description: "分段模板脱敏：按分隔符拆分值，对指定段保留首尾脱敏".into(), template: Some(TemplateParams::Segment(SegmentTemplate::default())), params: None },
        // ---- 提取规则（宽松正则召回 + 函数式严格校验）----
        // phone: \b[1-9]\d{10}\b, PhonePrefix{[]}（空名单=不过滤前缀）
        Rule { id: "phone-extract".into(), name: "手机号提取".into(), kind: RuleKind::Extract, field: None, pattern: Some(r"\b[1-9]\d{10}\b".into()), replacement: None, enabled: true, description: "提取 11 位手机号（可选前缀白名单校验，留空=不限）".into(), template: None, params: Some(ExtractParams::PhonePrefix { allowed_prefixes: Vec::new() }) },
        // bankcard: \b[1-9]\d{12,18}\b, Luhn
        Rule { id: "bankcard-extract".into(), name: "银行卡号提取".into(), kind: RuleKind::Extract, field: None, pattern: Some(r"\b[1-9]\d{12,18}\b".into()), replacement: None, enabled: true, description: "提取 13-19 位银行卡号（首位非 0），Luhn 严格校验".into(), template: None, params: Some(ExtractParams::Luhn) },
        // ip4: \b(?:\d{1,3}\.){3}\d{1,3}\b, Ipv4
        Rule { id: "ip4-extract".into(), name: "IPv4地址提取".into(), kind: RuleKind::Extract, field: None, pattern: Some(r"\b(?:\d{1,3}\.){3}\d{1,3}\b".into()), replacement: None, enabled: true, description: "提取 IPv4 地址，段范围+前导零严格校验".into(), template: None, params: Some(ExtractParams::Ipv4) },
        // ip6: (?:[0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}, Ipv6
        Rule { id: "ip6-extract".into(), name: "IPv6地址提取".into(), kind: RuleKind::Extract, field: None, pattern: Some(r"(?:[0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}".into()), replacement: None, enabled: true, description: "提取 IPv6 地址，RFC 4291 严格校验".into(), template: None, params: Some(ExtractParams::Ipv6) },
        // idcard: \b[1-9]\d{16}[\dXx]\b, IdCard（GB 11643-1999 校验码 + 性别推断）
        Rule { id: "idcard-extract".into(), name: "身份证号提取".into(), kind: RuleKind::Extract, field: None, pattern: Some(r"\b[1-9]\d{16}[\dXx]\b".into()), replacement: None, enabled: true, description: "提取 18 位身份证号，校验码 + 性别严格校验".into(), template: None, params: Some(ExtractParams::IdCard) },
        // ---- 函数式校验规则（kind=Validate, 带 params 走 validate_extracted 分发）----
        Rule { id: "username-validate".into(), name: "用户名校验".into(), kind: RuleKind::Validate, field: None, pattern: None, replacement: None, enabled: true, description: "校验用户名为纯字母数字".into(), template: None, params: Some(ExtractParams::Username) },
        Rule { id: "sex-validate".into(), name: "性别校验".into(), kind: RuleKind::Validate, field: None, pattern: None, replacement: None, enabled: true, description: "校验性别为「男」或「女」".into(), template: None, params: Some(ExtractParams::Sex) },
        Rule { id: "birth-validate".into(), name: "出生日期校验".into(), kind: RuleKind::Validate, field: None, pattern: None, replacement: None, enabled: true, description: "校验出生日期（清理分隔符后 8 位有效日期）".into(), template: None, params: Some(ExtractParams::Birth { formats: vec![] }) },
        Rule { id: "idcard-validate".into(), name: "身份证号校验".into(), kind: RuleKind::Validate, field: None, pattern: None, replacement: None, enabled: true, description: "校验 18 位身份证号（GB 11643-1999 校验码）；可选跨字段比对性别/出生日期".into(), template: None, params: Some(ExtractParams::IdCard) },
        Rule { id: "phone-validate".into(), name: "手机号校验".into(), kind: RuleKind::Validate, field: None, pattern: None, replacement: None, enabled: true, description: "校验 11 位手机号（可选前缀白名单，留空=不限）".into(), template: None, params: Some(ExtractParams::PhonePrefix { allowed_prefixes: Vec::new() }) },
        Rule { id: "address-validate".into(), name: "地址校验".into(), kind: RuleKind::Validate, field: None, pattern: None, replacement: None, enabled: true, description: "校验地址格式（全中文+地址关键词；可选号/室数字范围）".into(), template: None, params: Some(ExtractParams::Address { min_hao: None, max_hao: None, min_shi: None, max_shi: None }) },
        Rule { id: "generic-validate".into(), name: "通用校验".into(), kind: RuleKind::Validate, field: None, pattern: None, replacement: None, enabled: true, description: "通用校验：可选字符类（数字/字母）+ 自定义特殊符号白名单 + 长度限制".into(), template: None, params: Some(ExtractParams::Generic { allow_digits: true, allow_letters: true, allow_special_chars: String::new(), min_len: None, max_len: None }) },
        Rule { id: "email-validate".into(), name: "邮箱校验".into(), kind: RuleKind::Validate, field: None, pattern: None, replacement: None, enabled: true, description: "校验邮箱地址格式（local@domain）".into(), template: None, params: Some(ExtractParams::Email) },
    ]
});

/// 内置规则集合（表驱动）。
pub struct BuiltinRules;

impl BuiltinRules {
    /// 返回全部内置规则的静态引用切片。
    pub fn all() -> &'static [Rule] {
        &BUILTIN_RULES
    }

    /// 按 id 查找内置规则（返回克隆）。
    pub fn get(id: &str) -> Option<Rule> {
        BUILTIN_RULES.iter().find(|r| r.id == id).cloned()
    }
}
