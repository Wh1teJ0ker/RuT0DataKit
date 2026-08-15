//! 提取规则的函数式校验参数（v1.1.3 T55 新增）。
//!
//! 提取正则宽松（召回优先），严格性由 [`crate::processor::func_validator`]
//! 的对应函数兜底。serde 用 `tag = "validator"` 内部标签，DB 存为 `params TEXT`
//! JSON 列。现有非提取规则（name-extract 等）`params = None`，行为不变。
//!
//! JSON 示例：
//! - 银行卡 Luhn：`{"validator":"luhn"}`
//! - 手机号前缀：`{"validator":"phonePrefix","allowedPrefixes":["134","159"]}`
//! - IPv4 地址：`{"validator":"ipv4"}`
//! - IPv6 地址：`{"validator":"ipv6"}`
//!
//! v1.2.0 T93：从 `rules.rs` 拆出，承载 `ExtractParams` 枚举。

use serde::{Deserialize, Serialize};

/// 提取规则的函数式校验参数（v1.1.3 T55 新增）。
///
/// 提取正则宽松（召回优先），严格性由 [`crate::processor::func_validator`]
/// 的对应函数兜底。serde 用 `tag = "validator"` 内部标签，DB 存为 `params TEXT`
/// JSON 列。现有非提取规则（name-extract 等）`params = None`，行为不变。
///
/// JSON 示例：
/// - 银行卡 Luhn：`{"validator":"luhn"}`
/// - 手机号前缀：`{"validator":"phonePrefix","allowedPrefixes":["134","159"]}`
/// - IPv4 地址：`{"validator":"ipv4"}`
/// - IPv6 地址：`{"validator":"ipv6"}`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "validator", rename_all = "camelCase")]
pub enum ExtractParams {
    /// 手机号前缀校验。`allowed_prefixes` 空 = 默认（首位必须为 1，标准中国手机号）；
    /// 非空 = 前 3 位必须在列表内（用户自定义前缀范围，不再强制 1 开头）。
    #[serde(rename_all = "camelCase")]
    PhonePrefix {
        /// 向后兼容：旧 DB 存的是 snake_case `allowed_prefixes`，alias 让旧数据也能反序列化。
        #[serde(alias = "allowed_prefixes")]
        allowed_prefixes: Vec<String>,
    },
    /// 银行卡 Luhn 严格校验（右起偶数位 ×2，>9 则数位和，总和 %10==0）。
    Luhn,
    /// IPv4 地址严格解析（4 段 0-255 + 禁前导零）。
    Ipv4,
    /// IPv6 地址严格解析（走 `std::net::Ipv6Addr::from_str`）。
    Ipv6,
    /// 身份证号校验码严格校验（GB 11643-1999：前 17 位加权求和 mod 11 查表）。
    /// 性别（第 17 位奇=男/偶=女）由 `validate_extracted` 返回，性别联合校验
    /// （比对指定性别列）在 `extract_validate_to_new_sheet_inner` 中进行。
    #[serde(rename = "idcard")]
    IdCard,
    /// 用户名：纯字母数字（admin / lufe1jian / 91xxev）。
    ///
    /// v1.1.4 T67：用于 `username-validate` 函数式校验规则（kind=Validate，
    /// 带 params 走 `validate_extracted` 分发）。
    Username,
    /// 性别：仅「男」/「女」。
    ///
    /// v1.1.4 T67：用于 `sex-validate` 函数式校验规则。
    Sex,
    /// 出生日期：支持多格式可选校验（v1.1.5 T87 改为 struct variant）。
    ///
    /// v1.1.4 T67 新增（unit variant）；T70 续轮改进为先 `clean_birth` 清理分隔符再校验；
    /// v1.1.5 T87 改为 struct variant，`formats` 为空 = 全部接受（向后兼容）。
    /// 用于 `birth-validate` 函数式校验规则。
    ///
    /// 支持的格式标识：
    /// - `"yyyymmdd"` — 8 位纯数字
    /// - `"yyyy-mm-dd"` — 连字符分隔
    /// - `"yyyy/mm/dd"` — 斜杠分隔
    /// - `"yyyy.mm.dd"` — 点号分隔
    Birth {
        /// 接受的格式列表。空 = 全部接受（向后兼容）。
        #[serde(default)]
        formats: Vec<String>,
    },
    /// 地址：结构化校验（中文 ≥ 2 + 地址关键词）。
    ///
    /// v1.1.4 T67 新增；T70 续轮放宽为结构化校验（原严格正则号1-1500+
    /// 室101-999 已废弃）。用于 `address-validate` 函数式校验规则。
    Address,
    /// 邮箱地址：结构化校验（local@domain，RFC 5321 简化）。
    ///
    /// v1.1.5 T81 新增。用于 `email-validate` 函数式校验规则（kind=Validate，
    /// 带 params 走 `validate_extracted` 分发）。校验规则：
    /// - 含恰好 1 个 `@`
    /// - local 部分非空、≤64 字符、仅允许 `[a-zA-Z0-9._%+-]`
    /// - domain 部分非空、含至少 1 个 `.`、每段非空、仅允许 `[a-zA-Z0-9.-]`
    /// - 总长度 ≤ 254
    Email,
    /// 通用校验：字符类白名单 + 长度范围（v1.1.4 续轮 T70 新增；T77 改为
    /// 自定义特殊字符白名单）。
    ///
    /// `allow_digits` / `allow_letters` 为布尔开关；`allow_special_chars`
    /// 是用户自由填写的特殊字符白名单（空串 = 不允许任何特殊字符；
    /// 非空如 `"_-.@"` = 仅允许这些字符）。三个字符类至少有一个非空/为 true
    /// （全 false / 全空直接判否）。`min_len` / `max_len` 为 `None` 时不限。
    /// 用于 `generic-validate` 函数式校验规则，前端可发送 `params_override`
    /// 覆盖 DB 默认值。
    #[serde(rename = "generic", rename_all = "camelCase")]
    Generic {
        allow_digits: bool,
        allow_letters: bool,
        #[serde(default)]
        allow_special_chars: String,
        min_len: Option<usize>,
        max_len: Option<usize>,
    },
}
