//! 内置规则构造器（18 条）。
//!
//! v1.2.1：从 `rules/mod.rs` 拆出，承载所有内置规则的 `Rule` 构造函数。
//! `RuleRegistry::with_defaults()` 调用这些构造器 seed 到注册表。
//!
//! 历史版本变更见 `RuleRegistry::with_defaults` 文档注释。

use crate::processor::rules::extract_params::ExtractParams;
use crate::processor::rules::rule::{Rule, RuleKind};
use crate::processor::rules::template::{
    SegmentTemplate, TemplateParams,
};

/// 内置规则构造器集合（18 条）。
///
/// 所有方法返回 `Rule` 值，供 `RuleRegistry::with_defaults()` 注册。
/// 方法名与规则 id 一一对应（如 `name_validate_rule()` → `id = "name-validate"`）。
pub struct BuiltinRules;

impl BuiltinRules {
    /// 姓名校验内置规则：2-4 位中文字符。
    pub fn name_validate_rule() -> Rule {
        Rule {
            id: "name-validate".into(),
            name: "姓名校验".into(),
            kind: RuleKind::Validate,
            field: Some("name".into()),
            pattern: Some(r"^[\u4e00-\u9fa5]{2,4}$".into()),
            replacement: None,
            enabled: true,
            description: "校验姓名为 2-4 位中文字符".into(),
            template: None,
            params: None,
        }
    }

    /// 姓名脱敏内置规则：≥3 字符保留首尾，中间 `*` 替换；2 字符保留首字符末位 `*`。
    /// `replacement=None` → `SimpleMasker` 默认掩码字符 `*`。
    /// 无 `template` → 走 `SimpleMasker` 旧逻辑（保留首尾各 1）。
    pub fn name_mask_rule() -> Rule {
        Rule {
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
        }
    }

    /// 姓名提取内置规则：2-4 位连续中文字符。
    /// 前端测试时用此 pattern 做正则提取预览。
    pub fn name_extract_rule() -> Rule {
        Rule {
            id: "name-extract".into(),
            name: "姓名提取".into(),
            kind: RuleKind::Extract,
            field: Some("name".into()),
            pattern: Some(r"[\u4e00-\u9fa5]{2,4}".into()),
            replacement: None,
            enabled: true,
            description: "提取姓名候选：2-4 位连续中文字符".into(),
            template: None,
            params: None,
        }
    }

    /// 整段脱敏内置规则（v1.1.3 T54 由原 `general-mask` 拆分而来）：持**空 Simple
    /// 模板**（`TemplateParams::Simple(SimpleTemplate::default())`，所有字段
    /// `None`）→ 视为不脱敏（透传，原样返回）。
    ///
    /// 前端选预设（身份证 / 手机 / 出生日期 / 银行卡 / 自定义）→ 填充 7 个可编辑
    /// 参数框（含 T53 反向脱敏开关）→ 执行脱敏时把模板透传给
    /// `mask_column(template=...)` 临时覆盖，或经 `update_rule_template`
    /// 持久化到 DB。不选预设 → 空模板 → 不脱敏。
    ///
    /// 4 个预设（`idcard_preset()` 等）仍由 `TemplateParams::Simple` 承载，
    /// 仅适用于本规则。
    pub fn simple_mask_rule() -> Rule {
        Rule {
            id: "simple-mask".into(),
            name: "整段脱敏".into(),
            kind: RuleKind::Mask,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: "整段模板脱敏：选预设（身份证/手机/出生日期/银行卡）或自定义参数".into(),
            template: Some(TemplateParams::Simple(
                crate::processor::rules::template::SimpleTemplate::default(),
            )),
            params: None,
        }
    }

    /// 分段脱敏内置规则（v1.1.3 T54 由原 `general-mask` 拆分而来）：持**空
    /// Segment 模板**（`TemplateParams::Segment(SegmentTemplate::default())`，
    /// `delimiter` 空 + `segments` 空）→ 视为不脱敏（透传，原样返回）。
    ///
    /// 前端配置分隔符 + 段配置（每段 `index` + `keep_prefix`/`keep_suffix`/
    /// `mask_min_len`）→ 执行脱敏时把模板透传给 `mask_column(template=...)`
    /// 临时覆盖，或经 `update_rule_template` 持久化到 DB。空分隔符 / 空段列表
    /// → 不脱敏。不内置预设。
    pub fn segment_mask_rule() -> Rule {
        Rule {
            id: "segment-mask".into(),
            name: "分段脱敏".into(),
            kind: RuleKind::Mask,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: "分段模板脱敏：按分隔符拆分值，对指定段保留首尾脱敏".into(),
            template: Some(TemplateParams::Segment(SegmentTemplate::default())),
            params: None,
        }
    }

    /// 手机号提取内置规则（v1.1.3 T55）：11 位纯数字，首位非零。
    ///
    /// 提取正则 `\b[1-9]\d{10}\b`（宽松召回，与 bankcard-extract 一致使用
    /// `[1-9]` 首位而非 `1`，以兼容非标准前缀的测试数据）。严格校验由
    /// `validators` 的 `is_valid_phone` 兜底：空前缀列表 → 不过滤前缀
    /// （11 位纯数字均放行）；非空 → 前 3 位必须在列表内（用户自定义范围）。
    /// `params = PhonePrefix{[]}` → 空前缀列表 = 不过滤前缀。
    /// 前端可在 RulesPanel 配置 `allowedPrefixes`（如 `["799","786"]`）→ 限定非标准前缀。
    pub fn phone_extract_rule() -> Rule {
        Rule {
            id: "phone-extract".into(),
            name: "手机号提取".into(),
            kind: RuleKind::Extract,
            field: None,
            pattern: Some(r"\b[1-9]\d{10}\b".into()),
            replacement: None,
            enabled: true,
            description: "提取 11 位手机号（可选前缀白名单校验，留空=不限）".into(),
            template: None,
            params: Some(ExtractParams::PhonePrefix {
                allowed_prefixes: Vec::new(),
            }),
        }
    }

    /// 银行卡号提取内置规则（v1.1.3 T55）：13-19 位数字，首位非 0。
    ///
    /// 提取正则 `\b[1-9]\d{12,18}\b`（宽松召回），严格校验由 `validators`
    /// 的 `luhn_check` 兜底（右起偶数位 ×2，>9 则数位和，总和 %10==0）。
    /// 有效例：`6222021234567890123`；无效例：`6222021234567890124`。
    pub fn bankcard_extract_rule() -> Rule {
        Rule {
            id: "bankcard-extract".into(),
            name: "银行卡号提取".into(),
            kind: RuleKind::Extract,
            field: None,
            pattern: Some(r"\b[1-9]\d{12,18}\b".into()),
            replacement: None,
            enabled: true,
            description: "提取 13-19 位银行卡号（首位非 0），Luhn 严格校验".into(),
            template: None,
            params: Some(ExtractParams::Luhn),
        }
    }

    /// IPv4 地址提取内置规则（v1.1.3 T55b，由原 `ip-extract` 拆分）。
    ///
    /// 提取正则 `\b(?:\d{1,3}\.){3}\d{1,3}\b` 宽松召回 4 段点分数字，严格校验
    /// 由 `validators::is_valid_ipv4` 兜底（段范围 0-255 + 禁前导零）。
    /// `params = Ipv4`。有效例：`192.168.1.1` / `10.0.0.1` / `255.255.255.255`；
    /// 无效例：`256.1.1.1`（超范围）/ `192.168.01.1`（前导零）。
    pub fn ip4_extract_rule() -> Rule {
        Rule {
            id: "ip4-extract".into(),
            name: "IPv4地址提取".into(),
            kind: RuleKind::Extract,
            field: None,
            pattern: Some(r"\b(?:\d{1,3}\.){3}\d{1,3}\b".into()),
            replacement: None,
            enabled: true,
            description: "提取 IPv4 地址，段范围+前导零严格校验".into(),
            template: None,
            params: Some(ExtractParams::Ipv4),
        }
    }

    /// IPv6 地址提取内置规则（v1.1.3 T55b，由原 `ip-extract` 拆分）。
    ///
    /// 提取正则 `(?:[0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}` 宽松召回冒号分隔
    /// 的 hex 段，严格校验由 `validators::is_valid_ipv6` 兜底（走
    /// `std::net::Ipv6Addr::from_str`，RFC 4291 严格）。
    /// `params = Ipv6`。有效例：`::1` / `2001:db8::1`；无效例：`1:2:3:4:5:6:7:8:9`
    /// （段数超 8）。
    pub fn ip6_extract_rule() -> Rule {
        Rule {
            id: "ip6-extract".into(),
            name: "IPv6地址提取".into(),
            kind: RuleKind::Extract,
            field: None,
            pattern: Some(r"(?:[0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}".into()),
            replacement: None,
            enabled: true,
            description: "提取 IPv6 地址，RFC 4291 严格校验".into(),
            template: None,
            params: Some(ExtractParams::Ipv6),
        }
    }

    /// 身份证号提取内置规则（v1.1.3 T55c 新增）。
    ///
    /// 提取正则 `\b[1-9]\d{16}[\dXx]\b` 宽松召回 18 位身份证号（首位非零，末位可为 X/x），
    /// 严格校验由 `validators::is_valid_idcard` 兜底（GB 11643-1999 校验码
    /// 算法：前 17 位乘系数 `[7,9,10,5,8,4,2,1,6,3,7,9,10,5,8,4,2]`，加权和
    /// mod 11 查表 `[1,0,X,9,8,7,6,5,4,3,2]` 得第 18 位校验码）。性别（第 17 位
    /// 奇=男/偶=女）由 `validate_extracted` 返回到说明列；性别联合校验（比对
    /// 手动指定的性别列）在 `extract_validate_to_new_sheet_inner` 中进行，不存
    /// 规则配置。`params = IdCard`。
    pub fn idcard_extract_rule() -> Rule {
        Rule {
            id: "idcard-extract".into(),
            name: "身份证号提取".into(),
            kind: RuleKind::Extract,
            field: None,
            pattern: Some(r"\b[1-9]\d{16}[\dXx]\b".into()),
            replacement: None,
            enabled: true,
            description: "提取 18 位身份证号，校验码 + 性别严格校验".into(),
            template: None,
            params: Some(ExtractParams::IdCard),
        }
    }

    // ---- v1.1.4 T67：6 条函数式校验规则（kind=Validate，带 params 走
    // `validators::validate_extracted` 分发）。与 name-validate 不同，这些
    // 规则不带正则 pattern（field=None），由调用方直接对单元格原值
    // 调 `validate_extracted`。idcard/phone 复用现有 IdCard / PhonePrefix 变体：
    // idcard-validate 的 IdCard 分支返回 (true, gender) 供跨字段比对；
    // phone-validate 用 PhonePrefix 变体，validate 语义是整串校验而非提取
    // 前缀（PhonePrefix 分支调 is_valid_phone 做完整校验）。----

    /// 用户名校验内置规则（v1.1.4 T67 新增）：`params = Username`，
    /// 走 `validate_extracted` 的 Username 分支 → `is_valid_username`（纯字母数字）。
    pub fn username_validate_rule() -> Rule {
        Rule {
            id: "username-validate".into(),
            name: "用户名校验".into(),
            kind: RuleKind::Validate,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: "校验用户名为纯字母数字".into(),
            template: None,
            params: Some(ExtractParams::Username),
        }
    }

    /// 性别校验内置规则（v1.1.4 T67 新增）：`params = Sex`，
    /// 走 `validate_extracted` 的 Sex 分支 → `is_valid_sex`（仅「男」/「女」）。
    pub fn sex_validate_rule() -> Rule {
        Rule {
            id: "sex-validate".into(),
            name: "性别校验".into(),
            kind: RuleKind::Validate,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: "校验性别为「男」或「女」".into(),
            template: None,
            params: Some(ExtractParams::Sex),
        }
    }

    /// 出生日期校验内置规则（v1.1.4 T67 新增）：`params = Birth`，
    /// 走 `validate_extracted` 的 Birth 分支 → `is_valid_birth`
    /// （清理分隔符后 8 位数字 + 日期有效性，T70 续轮改进）。
    pub fn birth_validate_rule() -> Rule {
        Rule {
            id: "birth-validate".into(),
            name: "出生日期校验".into(),
            kind: RuleKind::Validate,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: "校验出生日期（清理分隔符后 8 位有效日期）".into(),
            template: None,
            params: Some(ExtractParams::Birth { formats: vec![] }),
        }
    }

    /// 身份证号校验内置规则（v1.1.4 T67 新增）：`params = IdCard`（复用现有变体），
    /// 走 `validate_extracted` 的 IdCard 分支 → `is_valid_idcard`
    /// （GB 11643-1999 校验码）。有效时返回的 gender 字符串供 T68 新命令做
    /// 跨字段比对（idcard 性别 vs 指定性别列）。
    pub fn idcard_validate_rule() -> Rule {
        Rule {
            id: "idcard-validate".into(),
            name: "身份证号校验".into(),
            kind: RuleKind::Validate,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: "校验 18 位身份证号（GB 11643-1999 校验码）；可选跨字段比对性别/出生日期"
                .into(),
            template: None,
            params: Some(ExtractParams::IdCard),
        }
    }

    /// 手机号校验内置规则（v1.1.4 T67 新增）：`params = PhonePrefix{[]}`，
    /// 走 `validate_extracted_with_params` 的 PhonePrefix 分支 → `is_valid_phone`
    /// （11 位 + 纯数字 + 前缀白名单，空名单=不过滤前缀）。
    pub fn phone_validate_rule() -> Rule {
        Rule {
            id: "phone-validate".into(),
            name: "手机号校验".into(),
            kind: RuleKind::Validate,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: "校验 11 位手机号（可选前缀白名单，留空=不限）".into(),
            template: None,
            params: Some(ExtractParams::PhonePrefix {
                allowed_prefixes: Vec::new(),
            }),
        }
    }

    /// 地址校验内置规则（v1.1.4 T67 新增）：`params = Address`，
    /// 走 `validate_extracted` 的 Address 分支 → `is_valid_address`
    /// （结构化校验：中文 ≥ 2 + 地址关键词，T70 续轮放宽原严格正则）。
    pub fn address_validate_rule() -> Rule {
        Rule {
            id: "address-validate".into(),
            name: "地址校验".into(),
            kind: RuleKind::Validate,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: "校验地址格式（中文+地址关键词）".into(),
            template: None,
            params: Some(ExtractParams::Address),
        }
    }

    /// 通用校验内置规则（v1.1.4 续轮 T70 新增）：`params = Generic`，
    /// 走 `validate_extracted` 的 Generic 分支 → `is_valid_generic`
    /// （字符类白名单 + 长度范围）。默认允许数字+字母，不限长度。
    /// 前端可发送 `params_override` 覆盖 DB 默认值（如临时加 `min_len` /
    /// `max_len` 或非空 `allow_special_chars`）。
    pub fn generic_validate_rule() -> Rule {
        Rule {
            id: "generic-validate".into(),
            name: "通用校验".into(),
            kind: RuleKind::Validate,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: "通用校验：可选字符类（数字/字母）+ 自定义特殊符号白名单 + 长度限制"
                .into(),
            template: None,
            params: Some(ExtractParams::Generic {
                allow_digits: true,
                allow_letters: true,
                allow_special_chars: String::new(),
                min_len: None,
                max_len: None,
            }),
        }
    }

    /// 邮箱校验内置规则（v1.1.5 T81 新增）：`params = Email`，
    /// 走 `validate_extracted` 的 Email 分支 → `is_valid_email`
    /// （结构化校验：local@domain，local ≤64，domain 含 `.`，总长 ≤254）。
    pub fn email_validate_rule() -> Rule {
        Rule {
            id: "email-validate".into(),
            name: "邮箱校验".into(),
            kind: RuleKind::Validate,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: "校验邮箱地址格式（local@domain）".into(),
            template: None,
            params: Some(ExtractParams::Email),
        }
    }
}
