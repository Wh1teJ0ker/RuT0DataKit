//! 脱敏 / 校验算子元信息（v0.1.0 重构：删除所有预置别名，只保留通用算子）。
//!
//! v0.1.0 重构：用户要求「只做规则模版」，因此本模块不再维护旧 masker /
//! validator 名 → 算子配置的映射表，预置别名（`idcard_mask` / `phone_mask` /
//! `email` / `username` / `name` / `idcard` / `bankcard` / `phone` / `mac` 等）
//! 全部删除。所有规则均通过通用算子 + 显式 params 配置：
//! - 脱敏：`template` / `split_template` / `regex_replace` / `const_replace`
//! - 校验：`regex` / `algorithm` / `regex_with_guard`
//!
//! [`list_mask_op_types`] / [`list_validate_op_types`] 仅返回这 4 + 3 个通用
//! 算子，供前端下拉源使用。每条规则的所有参数（keep_prefix / keep_suffix /
//! mask_char / min_len / max_len / cjk / pattern / replacement / match_mode /
//! algo / guard / prefix_set / prefix / message 等）均由调用方显式提供。
//!
//! 旧正则常量（EMAIL_REGEX / USERNAME_REGEX 等）作为「常用正则样例」保留导出，
//! 便于 [`crate::rules::operator::RegexOp`] 使用方在 params.pattern 中引用，
//! 但不再绑定任何预置别名。

/// 邮箱常用正则样例（供用户在 `regex` 算子的 params.pattern 中直接引用）。
pub const EMAIL_REGEX: &str = r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$";
/// 用户名常用正则样例（字母数字）。
pub const USERNAME_REGEX: &str = r"^[A-Za-z0-9]+$";
/// 中文姓名常用正则样例。
pub const NAME_REGEX: &str = r"^[\x{4e00}-\x{9fa5}]+$";
/// 11 位手机号格式正则样例。
pub const PHONE_REGEX: &str = r"^\d{11}$";
/// MAC 地址格式正则样例。
pub const MAC_REGEX: &str = r"^([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$";

/// 生日格式正则样例（YYYYMMDD，月份 01-12、日期 01-31，不精确校验闰年）。
pub const BIRTH_REGEX: &str = r"^\d{4}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])$";
/// 中文地址正则样例（包含中文字符 + 省/市/区/县/镇/村/街/路/号 任一关键字）。
pub const ADDRESS_REGEX: &str = r"^[\x{4e00}-\x{9fa5}]+.*(省|市|区|县|镇|村|街|路|号).*$";
/// 密码格式正则样例（长度 8-32，仅字母数字）。
///
/// 注：`regex` crate 不支持 look-around，无法在单条正则中同时强制
/// 「至少含字母 + 至少含数字」。本样例仅约束长度与字符集；如需强制
/// 字母+数字混合，请配合 `algorithm` / `regex_with_guard` 算子做附加检查。
pub const PASSWORD_REGEX: &str = r"^[A-Za-z0-9]{8,32}$";
/// IPv4 格式正则样例（四段 0-255）。
pub const IP_REGEX: &str =
    r"^((25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)\.){3}(25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)$";

/// 返回脱敏算子类型清单（供前端下拉源）。
///
/// v0.1.0 重构后仅含 4 个通用算子（无预置别名）：
/// - `template` / `split_template` / `regex_replace` / `const_replace`
///
/// 元组格式：`(op_name, label)`。
pub fn list_mask_op_types() -> Vec<(&'static str, &'static str)> {
    vec![
        ("template", "通用模板脱敏"),
        ("split_template", "切分模板脱敏"),
        ("regex_replace", "正则替换脱敏"),
        ("const_replace", "常量替换脱敏"),
    ]
}

/// 返回校验算子类型清单（供前端下拉源）。
///
/// v0.1.0 重构后仅含 3 个通用算子（无预置别名）：
/// - `regex` / `algorithm` / `regex_with_guard`
pub fn list_validate_op_types() -> Vec<(&'static str, &'static str)> {
    vec![
        ("regex", "正则校验"),
        ("algorithm", "算法校验"),
        ("regex_with_guard", "守卫+正则校验"),
    ]
}

/// 一个预置模板条目：只导出元信息，不绑定具体 `MaskOp` / `ValidateOp` 实例，
/// 避免与 `operator.rs` 耦合。T5-4 GUI 消费此结构按标签分组渲染常用模板。
///
/// 字段说明：
/// - `name`：模板短名（用作规则 `masker` / `validator` 字段或模板引用 key）。
/// - `label`：人类可读名称（GUI 标签）。
/// - `tags`：分组标签（如 `mask` / `validate` / `sensitive` / `search` / `sql_parse`）。
/// - `params_template`：参数样例（YAML/JSON 片段的弱类型映射），供 GUI 预填。
///   `None` 表示该模板无需预置参数（由调用方按算子类型自行编辑）。
#[derive(Debug, Clone, PartialEq)]
pub struct PresetEntry {
    pub name: &'static str,
    pub label: &'static str,
    pub tags: &'static [&'static str],
    pub params_template: Option<serde_yml::Mapping>,
}

/// 内部静态描述：与 [`PresetEntry`] 同构但 `params_template` 用 `&[&str]`
/// 键值对表示，便于在 `static` 上下文声明，运行期再组装为 `Mapping`。
struct PresetSpec {
    name: &'static str,
    label: &'static str,
    tags: &'static [&'static str],
    /// `(key, raw_yaml_value)` 对；`raw_yaml_value` 会经 `serde_yml` 解析为
    /// `serde_yml::Value`，支持数字 / 字符串 / 布尔。
    params: &'static [(&'static str, &'static str)],
}

/// 内置预置模板全集（按标签分组）。
///
/// 维护原则：
/// - 不硬编码 `MaskOp` / `ValidateOp` 实例，只导出元信息 + 参数样例。
/// - `params` 中值字符串经 `serde_yml::from_str` 解析为 `Value`，键名与
///   `default_mask.yaml` / 通用算子 params 对齐。
/// - 新增标签（如 `search` / `sql_parse`）时在此追加 `PresetSpec`。
static PRESET_SPECS: &[PresetSpec] = &[
    // ===== mask 标签：脱敏侧常用模板 =====
    PresetSpec {
        name: "template_idcard",
        label: "身份证号脱敏（保留前6后4）",
        tags: &["mask", "sensitive"],
        params: &[
            ("keep_prefix", "6"),
            ("keep_suffix", "4"),
            ("mask_char", r#""*""#),
            ("mask_min_len", "8"),
            ("min_len", "18"),
            ("max_len", "18"),
            ("cjk", "false"),
        ],
    },
    PresetSpec {
        name: "template_phone",
        label: "手机号脱敏（保留前3后4）",
        tags: &["mask", "sensitive"],
        params: &[
            ("keep_prefix", "3"),
            ("keep_suffix", "4"),
            ("mask_char", r#""*""#),
            ("mask_min_len", "4"),
            ("min_len", "11"),
            ("max_len", "11"),
            ("cjk", "false"),
        ],
    },
    PresetSpec {
        name: "template_name_cjk",
        label: "中文姓名脱敏（CJK 分支）",
        tags: &["mask", "sensitive"],
        params: &[
            ("keep_prefix", "1"),
            ("keep_suffix", "1"),
            ("mask_char", r#""*""#),
            ("mask_min_len", "1"),
            ("cjk", "true"),
        ],
    },
    PresetSpec {
        name: "split_template_email",
        label: "邮箱本地部分脱敏",
        tags: &["mask", "sensitive"],
        params: &[
            ("separator", r#""@""#),
            ("segment_index", "0"),
            ("keep_prefix", "1"),
            ("keep_suffix", "1"),
            ("mask_char", r#""*""#),
            ("mask_min_len", "1"),
            ("cjk", "true"),
        ],
    },
    // ===== validate 标签：校验侧常用模板 =====
    PresetSpec {
        name: "validate_idcard",
        label: "身份证号格式校验",
        tags: &["validate", "sensitive"],
        params: &[("pattern", r"'^\d{17}[0-9Xx]$'")],
    },
    PresetSpec {
        name: "validate_phone",
        label: "手机号格式校验",
        tags: &["validate", "sensitive"],
        params: &[("pattern", r"'^\d{11}$'")],
    },
    PresetSpec {
        name: "validate_email",
        label: "邮箱格式校验",
        tags: &["validate", "sensitive"],
        // EMAIL_REGEX 是 const &str，无法在 static 切片字面量里内联拼接，
        // 直接写字面量正则；值与 EMAIL_REGEX 保持同步。
        // 用单引号 YAML 字符串：反斜杠按字面量处理，无需双重转义。
        params: &[(
            "pattern",
            r"'^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$'",
        )],
    },
    PresetSpec {
        name: "validate_bankcard",
        label: "银行卡号格式校验",
        tags: &["validate", "sensitive"],
        params: &[("pattern", r"'^\d{16,19}$'")],
    },
    // ===== sensitive 标签：由 mask / validate 条目同时携带，
    // 通过 list_tagged_presets("sensitive") 可一次性取回所有敏感字段模板。
    // 这里不再重复列举，filter 已覆盖。=====
];

/// 把 `PresetSpec.params` 组装为 `serde_yml::Mapping`。
///
/// 值字符串走 `serde_yml::from_str` 解析，因此 `"6"` → 整数 6、`"true"` →
/// 布尔、`r#""*""#` → 字符串 `"*"`，与 `default_mask.yaml` 类型一致。
fn build_params_template(spec: &PresetSpec) -> Option<serde_yml::Mapping> {
    if spec.params.is_empty() {
        return None;
    }
    let mut m = serde_yml::Mapping::new();
    for (k, raw_v) in spec.params {
        let value: serde_yml::Value = serde_yml::from_str(raw_v).expect("PresetSpec params raw YAML must parse");
        m.insert(serde_yml::Value::String((*k).into()), value);
    }
    Some(m)
}

/// 把 `PresetSpec` 转为运行期 `PresetEntry`。
fn spec_to_entry(spec: &PresetSpec) -> PresetEntry {
    PresetEntry {
        name: spec.name,
        label: spec.label,
        tags: spec.tags,
        params_template: build_params_template(spec),
    }
}

/// 按标签返回预置模板清单。
///
/// 至少覆盖 `mask` / `validate` / `sensitive` 三类标签。返回条目的 `tags`
/// 任一匹配即纳入。未匹配标签返回空 Vec（不报错）。本函数**不构造任何
/// `MaskOp` / `ValidateOp` 实例**，仅返回元信息 + 参数样例，下游可基于
/// `params_template` 渲染 GUI 表单或生成规则草稿。
pub fn list_tagged_presets(tag: &str) -> Vec<PresetEntry> {
    PRESET_SPECS
        .iter()
        .filter(|s| s.tags.iter().any(|t| *t == tag))
        .map(spec_to_entry)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_op_types_only_general_operators() {
        let mask = list_mask_op_types();
        let mask_names: Vec<&str> = mask.iter().map(|(n, _)| *n).collect();
        assert_eq!(mask_names, ["template", "split_template", "regex_replace", "const_replace"]);

        let val = list_validate_op_types();
        let val_names: Vec<&str> = val.iter().map(|(n, _)| *n).collect();
        assert_eq!(val_names, ["regex", "algorithm", "regex_with_guard"]);
    }

    #[test]
    fn list_tagged_presets_covers_mask_validate_sensitive() {
        // mask 标签：至少 4 条脱敏模板。
        let mask_presets = list_tagged_presets("mask");
        assert!(mask_presets.len() >= 4, "mask presets: {}", mask_presets.len());
        assert!(mask_presets.iter().all(|p| p.tags.contains(&"mask")));

        // validate 标签：至少 4 条校验模板。
        let validate_presets = list_tagged_presets("validate");
        assert!(validate_presets.len() >= 4, "validate presets: {}", validate_presets.len());
        assert!(validate_presets.iter().all(|p| p.tags.contains(&"validate")));

        // sensitive 标签：mask + validate 中带 sensitive 的并集，至少 8 条。
        let sensitive_presets = list_tagged_presets("sensitive");
        assert!(sensitive_presets.len() >= 8, "sensitive presets: {}", sensitive_presets.len());
        assert!(sensitive_presets.iter().all(|p| p.tags.contains(&"sensitive")));

        // mask / validate 应不重叠（name 不同），sensitive 与二者交集一致。
        let mask_names: Vec<&str> = mask_presets.iter().map(|p| p.name).collect();
        let val_names: Vec<&str> = validate_presets.iter().map(|p| p.name).collect();
        for n in &mask_names {
            assert!(!val_names.contains(n), "mask preset {n} should not appear in validate");
        }

        // 每条 mask/validate 模板都带 params_template（非 None），便于 GUI 预填。
        for p in mask_presets.iter().chain(validate_presets.iter()) {
            assert!(p.params_template.is_some(), "preset {} should carry params_template", p.name);
        }

        // 未知标签返回空 Vec（不报错）。
        assert!(list_tagged_presets("nonexistent_tag").is_empty());
    }

    #[test]
    fn preset_entry_params_template_roundtrips_through_serde() {
        // params_template 序列化后能反序列化为 Mapping，确保 GUI 拿到的参数样例可用。
        let mask_presets = list_tagged_presets("mask");
        let p = mask_presets
            .iter()
            .find(|p| p.name == "template_idcard")
            .expect("template_idcard preset");
        let m = p.params_template.as_ref().expect("params_template");
        let s = serde_yml::to_string(m).unwrap();
        let back: serde_yml::Mapping = serde_yml::from_str(&s).unwrap();
        assert_eq!(back, *m);
    }

    #[test]
    fn regex_constants_still_available() {
        // 常用正则常量保留导出，便于用户在 regex 算子 params.pattern 中引用。
        assert!(EMAIL_REGEX.contains("@"));
        assert!(PHONE_REGEX.starts_with(r"^\d{11}$"));
        assert!(MAC_REGEX.contains("A-Fa-f"));
        assert!(USERNAME_REGEX.chars().any(|c| c == '+'));
        assert!(NAME_REGEX.contains("4e00"));
    }

    #[test]
    fn birth_regex_matches_valid_and_invalid() {
        use regex::Regex;
        let re = Regex::new(BIRTH_REGEX).unwrap();
        assert!(re.is_match("20240115"));
        assert!(re.is_match("19991231"));
        assert!(!re.is_match("20241301")); // 月份 13 非法
        assert!(!re.is_match("20241232")); // 日期 32 非法
        assert!(!re.is_match("2024-01-15")); // 含分隔符
        assert!(!re.is_match("abcd0101")); // 非数字
    }

    #[test]
    fn address_regex_matches_valid_and_invalid() {
        use regex::Regex;
        let re = Regex::new(ADDRESS_REGEX).unwrap();
        assert!(re.is_match("广东省深圳市南山区"));
        assert!(re.is_match("北京市朝阳区"));
        assert!(re.is_match("四川省成都市武侯区科华路1号"));
        assert!(re.is_match("广东省")); // 关键字「省」+ 中文，符合样例约束
        assert!(!re.is_match("123456")); // 纯数字
        assert!(!re.is_match("New York City")); // 非中文
    }

    #[test]
    fn password_regex_matches_valid_and_invalid() {
        use regex::Regex;
        let re = Regex::new(PASSWORD_REGEX).unwrap();
        // 正例：长度 8-32 且仅字母数字
        assert!(re.is_match("abc12345"));
        assert!(re.is_match("Password12345"));
        assert!(re.is_match("A1B2C3D4"));
        // 反例：长度/字符集不符
        assert!(!re.is_match("abc123")); // 长度 < 8
        assert!(!re.is_match("abc 12345")); // 含空格（不在字符集）
        assert!(!re.is_match("abcdefghijklmnopqrstuvwxyz1234567")); // 长度 > 32
        assert!(!re.is_match("password!@#")); // 含非字母数字
    }

    #[test]
    fn ip_regex_matches_valid_and_invalid() {
        use regex::Regex;
        let re = Regex::new(IP_REGEX).unwrap();
        assert!(re.is_match("192.168.1.1"));
        assert!(re.is_match("255.255.255.255"));
        assert!(re.is_match("0.0.0.0"));
        assert!(re.is_match("10.0.0.1"));
        assert!(!re.is_match("256.1.1.1")); // 段 > 255
        assert!(!re.is_match("192.168.1")); // 段数不足
        assert!(!re.is_match("192.168.1.1.1")); // 段数过多
        assert!(!re.is_match("a.b.c.d")); // 非数字
    }
}
