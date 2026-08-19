//! 函数式校验器（v1.1.3 T55 新增）。
//!
//! 提取规则的「严格校验」层：提取正则宽松（召回优先），命中候选后用本模块的
//! 纯函数做严格校验。五条提取规则分别对应：
//! - `phone-extract` → [`is_valid_phone`]（11 位 + 纯数字；空名单 = 不过滤前缀）
//! - `bankcard-extract` → [`luhn_check`]（Luhn 算法）
//! - `ip4-extract` → [`is_valid_ipv4`]（段范围 + 前导零）
//! - `ip6-extract` → [`is_valid_ipv6`]（RFC 4291 严格）
//! - `idcard-extract` → [`is_valid_idcard`]（GB 11643-1999 校验码）
//!
//! T55b：原 `ip-extract` 拆为 `ip4-extract` + `ip6-extract` 两条独立规则，
//! `IpFamily` 枚举已删除。
//!
//! T55c：新增 `idcard-extract` 规则，`is_valid_idcard` 校验 18 位身份证号
//! 校验码（前 17 位加权求和 mod 11 查表），[`idcard_gender`] 推断性别
//! （第 17 位奇=男/偶=女）。性别联合校验（比对指定性别列）在
//! `extract_validate_to_new_sheet_inner` 中进行，不在此模块。
//!
//! T57：新增 5 个行级校验独立函数 [`is_valid_username`] / [`is_valid_sex`] /
//! [`is_valid_birth`] / [`is_valid_phone`] / [`is_valid_address`]，服务于
//! 行级多字段校验 → 双 Tab 输出。这些函数不依赖 `ExtractParams` / DB rule
//! 记录，直接对字段原值做严格校验；跨字段联合校验（sex vs idcard 性别、
//! birth vs idcard 出生日期码）在调用方进行。
//!
//! [`validate_extracted`] 按 `rule.params` 分发到对应函数；`params=None` →
//! `(true, "")`（仅正则提取，不额外校验，向后兼容 name-extract 等老规则）。
//!
//! v1.2.1：原 `func_validator.rs`（733 源码）按域拆分为 7 个子模块
//! （luhn / ip / idcard / personal / datetime / address / email / generic），
//! 本文件仅保留 dispatch + re-export。

pub mod address;
pub mod datetime;
pub mod email;
pub mod generic;
pub mod idcard;
pub mod ip;
pub mod luhn;
pub mod personal;

pub use address::{is_valid_address, ADDR_KEYWORDS};
pub use datetime::{clean_birth, is_valid_birth, is_valid_birth_format};
pub use email::is_valid_email;
pub use generic::is_valid_generic;
pub use idcard::{idcard_gender, is_valid_idcard};
pub use ip::{is_valid_ipv4, is_valid_ipv6};
pub use luhn::luhn_check;
pub use personal::{
    check_gender_consistency, check_phone_prefix, is_valid_phone, is_valid_sex,
    is_valid_username, normalize_gender,
};

use crate::processor::rules::{ExtractParams, Rule};

/// 按 `ExtractParams` 分发到对应函数式校验器（不依赖 `Rule`，直接接 `params`）。
///
/// v1.1.4 续轮 T70：从 [`validate_extracted`] 抽取核心逻辑，便于
/// `validate_multi_rules_to_two_sheets_inner` 接收 `params_override` 后直接
/// 校验，无需构造 `Rule`。
///
/// 返回 `(是否有效, 说明)`：
/// - `PhonePrefix{[]}` → 11 位纯数字即通过（空名单不过滤前缀）；非空 → 11 位 + 前缀白名单。
/// - `Luhn` → Luhn 算法；未通过说明 "未通过 Luhn 校验"。
/// - `Ipv4` → 段范围 0-255 + 禁前导零；未通过说明 "非合法 IPv4 地址"。
/// - `Ipv6` → `std::net::Ipv6Addr::from_str`（RFC 4291）；未通过说明 "非合法 IPv6 地址"。
/// - `IdCard` → GB 11643-1999 校验码；有效说明列写性别（"男"/"女"），
///   无效说明 "非合法身份证号"。
/// - `Username` / `Sex` → 对应行级校验函数。
/// - `Birth { formats }` → `formats` 空 = 全部接受（[`is_valid_birth`]，向后兼容）；
///   非空 = 仅接受指定格式之一（[`is_valid_birth_format`]，v1.1.5 T87 新增）。
/// - `Address` → 对应行级校验函数。
/// - `Email` → [`is_valid_email`]（v1.1.5 T81 新增，结构化邮箱校验）。
/// - `Generic` → [`is_valid_generic`]（字符类白名单 + 长度范围）。
pub fn validate_extracted_with_params(params: &ExtractParams, value: &str) -> (bool, String) {
    match params {
        ExtractParams::PhonePrefix { allowed_prefixes } => {
            if is_valid_phone(value, allowed_prefixes) {
                (true, String::new())
            } else {
                (false, "非合法手机号".to_string())
            }
        }
        ExtractParams::Luhn => {
            if luhn_check(value) {
                (true, String::new())
            } else {
                (false, "未通过 Luhn 校验".to_string())
            }
        }
        ExtractParams::Ipv4 => {
            if is_valid_ipv4(value) {
                (true, String::new())
            } else {
                (false, "非合法 IPv4 地址".to_string())
            }
        }
        ExtractParams::Ipv6 => {
            if is_valid_ipv6(value) {
                (true, String::new())
            } else {
                (false, "非合法 IPv6 地址".to_string())
            }
        }
        ExtractParams::IdCard => {
            if is_valid_idcard(value) {
                // 有效 → 说明列写推断的性别（性别联合校验在外层处理）
                let gender = idcard_gender(value)
                    .map(|c| c.to_string())
                    .unwrap_or_default();
                (true, gender)
            } else {
                (false, "非合法身份证号".to_string())
            }
        }
        // v1.1.4 T67：4 条行级校验变体分发
        ExtractParams::Username => {
            if is_valid_username(value) {
                (true, String::new())
            } else {
                (false, "用户名须为纯字母数字".to_string())
            }
        }
        ExtractParams::Sex => {
            if is_valid_sex(value) {
                (true, String::new())
            } else {
                (false, "性别须为「男」或「女」".to_string())
            }
        }
        ExtractParams::Birth { formats } => {
            if formats.is_empty() {
                // 向后兼容：clean_birth + 8 位校验（原逻辑）
                if is_valid_birth(value) {
                    (true, String::new())
                } else {
                    (
                        false,
                        "出生日期格式不符（清理后须为 8 位有效日期）".to_string(),
                    )
                }
            } else {
                // 仅接受指定格式
                if formats.iter().any(|f| is_valid_birth_format(value, f)) {
                    (true, String::new())
                } else {
                    (false, "出生日期格式不符（须为勾选的格式之一）".to_string())
                }
            }
        }
        ExtractParams::Address {
            min_hao,
            max_hao,
            min_shi,
            max_shi,
        } => {
            if is_valid_address(value, *min_hao, *max_hao, *min_shi, *max_shi) {
                (true, String::new())
            } else {
                (false, "地址格式不符（须全中文+地址关键词+号/室范围）".to_string())
            }
        }
        // v1.1.5 T81：邮箱校验变体
        ExtractParams::Email => {
            if is_valid_email(value) {
                (true, String::new())
            } else {
                (false, "邮箱格式不符".to_string())
            }
        }
        // v1.1.4 续轮 T70：通用校验变体（T77 改为自定义特殊字符白名单）
        ExtractParams::Generic {
            allow_digits,
            allow_letters,
            allow_special_chars,
            min_len,
            max_len,
        } => {
            if is_valid_generic(
                value,
                *allow_digits,
                *allow_letters,
                allow_special_chars,
                *min_len,
                *max_len,
            ) {
                (true, String::new())
            } else {
                (false, "通用校验未通过（字符类或长度不符）".to_string())
            }
        }
    }
}

/// 按 `rule.params` 分发到对应函数式校验器。
///
/// v1.1.4 续轮 T70：核心逻辑已抽取到 [`validate_extracted_with_params`]，
/// 本函数为保留向后兼容的 wrapper：`params=None` → `(true, "")`
/// （仅正则提取，不额外校验，向后兼容 name-extract）；`params=Some(p)` →
/// 委托 [`validate_extracted_with_params`]。
///
/// 不改 `extract_validate_to_new_sheet_inner`（保持列级提取不变）。
pub fn validate_extracted(rule: &Rule, value: &str) -> (bool, String) {
    match &rule.params {
        None => (true, String::new()),
        Some(params) => validate_extracted_with_params(params, value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processor::rules::{Rule, RuleKind};

    // ---- validate_extracted (分发) ----

    #[test]
    fn validate_extracted_none_passes() {
        // name-extract 风格：params=None → 直接通过
        let mut rule = like_name_extract();
        rule.params = None;
        assert_eq!(
            validate_extracted(&rule, "anything"),
            (true, "".to_string())
        );
    }

    #[test]
    fn validate_extracted_phone_prefix() {
        let mut rule = like_name_extract();
        rule.params = Some(ExtractParams::PhonePrefix {
            allowed_prefixes: vec!["134".into()],
        });
        assert!(validate_extracted(&rule, "13412345678").0);
        assert!(!validate_extracted(&rule, "15987654321").0);
        // 空列表 → 默认通过
        rule.params = Some(ExtractParams::PhonePrefix {
            allowed_prefixes: vec![],
        });
        assert!(validate_extracted(&rule, "13412345678").0);
    }

    #[test]
    fn validate_extracted_luhn() {
        let mut rule = like_name_extract();
        rule.params = Some(ExtractParams::Luhn);
        assert!(validate_extracted(&rule, "6222021234567890128").0);
        assert!(!validate_extracted(&rule, "6222021234567890123").0);
    }

    #[test]
    fn validate_extracted_ipv4_ipv6() {
        // T55b：Ipv4 / Ipv6 两个变体分别校验
        let mut rule_v4 = like_name_extract();
        rule_v4.params = Some(ExtractParams::Ipv4);
        assert!(validate_extracted(&rule_v4, "192.168.1.1").0);
        assert!(!validate_extracted(&rule_v4, "256.1.1.1").0); // 超范围
        assert!(!validate_extracted(&rule_v4, "192.168.01.1").0); // 前导零
        assert!(!validate_extracted(&rule_v4, "::1").0); // IPv6 不应通过 IPv4 校验

        let mut rule_v6 = like_name_extract();
        rule_v6.params = Some(ExtractParams::Ipv6);
        assert!(validate_extracted(&rule_v6, "::1").0);
        assert!(validate_extracted(&rule_v6, "2001:db8::1").0);
        assert!(!validate_extracted(&rule_v6, "192.168.1.1").0); // IPv4 不应通过 IPv6 校验
        assert!(!validate_extracted(&rule_v6, "1:2:3:4:5:6:7:8:9").0); // 段数超 8
    }

    #[test]
    fn validate_extracted_idcard() {
        // T55c：IdCard 变体校验码 + 性别推断
        let mut rule = like_name_extract();
        rule.params = Some(ExtractParams::IdCard);
        // 有效女（第 17 位 2 偶，校验码 X）
        let (ok, note) = validate_extracted(&rule, "11010519491231002X");
        assert!(ok);
        assert_eq!(note, "女");
        // 有效男（第 17 位 3 奇，校验码 8）
        let (ok, note) = validate_extracted(&rule, "110105194912310038");
        assert!(ok);
        assert_eq!(note, "男");
        // 无效（校验码错）
        let (ok, note) = validate_extracted(&rule, "110105194912310021");
        assert!(!ok);
        assert_eq!(note, "非合法身份证号");
        // 无效（长度不足）
        let (ok, _) = validate_extracted(&rule, "12345");
        assert!(!ok);
    }

    // ---- v1.1.4 T67：4 条新变体分发测试 ----

    #[test]
    fn validate_extracted_username() {
        // T67：Username 变体 → is_valid_username
        let mut rule = like_name_extract();
        rule.params = Some(ExtractParams::Username);
        // 有效（纯字母数字）
        assert!(validate_extracted(&rule, "admin").0);
        let (ok, note) = validate_extracted(&rule, "lufe1jian");
        assert!(ok);
        assert_eq!(note, "");
        // 无效（含点）
        let (ok, note) = validate_extracted(&rule, "ab.cd");
        assert!(!ok);
        assert_eq!(note, "用户名须为纯字母数字");
        // 无效（含下划线）
        let (ok, _) = validate_extracted(&rule, "ad_1in");
        assert!(!ok);
        // 无效（空串）
        let (ok, _) = validate_extracted(&rule, "");
        assert!(!ok);
    }

    #[test]
    fn validate_extracted_sex() {
        // T67：Sex 变体 → is_valid_sex
        let mut rule = like_name_extract();
        rule.params = Some(ExtractParams::Sex);
        // 有效
        let (ok, note) = validate_extracted(&rule, "男");
        assert!(ok);
        assert_eq!(note, "");
        assert!(validate_extracted(&rule, "女").0);
        // trim 后匹配
        assert!(validate_extracted(&rule, " 男 ").0);
        // 无效
        let (ok, note) = validate_extracted(&rule, "male");
        assert!(!ok);
        assert_eq!(note, "性别须为「男」或「女」");
        let (ok, _) = validate_extracted(&rule, "");
        assert!(!ok);
        let (ok, _) = validate_extracted(&rule, "未知");
        assert!(!ok);
    }

    #[test]
    fn validate_extracted_birth() {
        // T67 + T70：Birth 变体 → is_valid_birth（清理分隔符后校验）
        let mut rule = like_name_extract();
        rule.params = Some(ExtractParams::Birth { formats: vec![] });
        // 有效（8 位纯数字）
        let (ok, note) = validate_extracted(&rule, "19491231");
        assert!(ok);
        assert_eq!(note, "");
        assert!(validate_extracted(&rule, "20000101").0);
        // T70：有效（含分隔符，清理后 8 位）
        assert!(validate_extracted(&rule, "1949-12-31").0);
        assert!(validate_extracted(&rule, "2003/12/23").0);
        // 无效（月 13）
        let (ok, _) = validate_extracted(&rule, "20031323");
        assert!(!ok);
        // 无效（日 0）
        let (ok, _) = validate_extracted(&rule, "20031200");
        assert!(!ok);
        // 无效（清理后 6 位）
        let (ok, _) = validate_extracted(&rule, "2003-12");
        assert!(!ok);
        // 无效（长度超）
        let (ok, _) = validate_extracted(&rule, "194912311");
        assert!(!ok);
        // 无效（空串）
        let (ok, _) = validate_extracted(&rule, "");
        assert!(!ok);
    }

    #[test]
    fn validate_extracted_address() {
        // T67 + T70：Address 变体 → is_valid_address（结构化校验）
        let mut rule = like_name_extract();
        rule.params = Some(ExtractParams::Address {
            min_hao: None,
            max_hao: None,
            min_shi: None,
            max_shi: None,
        });
        // 有效
        let (ok, note) =
            validate_extracted(&rule, "内蒙古自治区呼和浩特市玉泉区大南街街道1340号540室");
        assert!(ok);
        assert_eq!(note, "");
        assert!(validate_extracted(&rule, "北京市朝阳区1号101室").0);
        // T70：原号超 1500 → 现在有效（结构化校验不限制号范围）
        assert!(validate_extracted(&rule, "北京市朝阳区1501号101室").0);
        // T70：原室不足 101 → 现在有效
        assert!(validate_extracted(&rule, "北京市朝阳区1号100室").0);
        // T70：原室超 999 → 现在有效
        assert!(validate_extracted(&rule, "北京市朝阳区1号1000室").0);
        // T70：「1234号101室」含号/室 CJK + 关键词 → 现在有效
        assert!(validate_extracted(&rule, "1234号101室").0);
        // 无效（无中文，无地址关键词）
        let (ok, _) = validate_extracted(&rule, "hello world");
        assert!(!ok);
        // 无效（空串）
        let (ok, _) = validate_extracted(&rule, "");
        assert!(!ok);
        // 无效（2 CJK 但无地址关键词）
        let (ok, _) = validate_extracted(&rule, "张三");
        assert!(!ok);
        // v1.2.2：含英文字母 → 不通过
        let (ok, _) = validate_extracted(&rule, "吉林省长春T朝阳区前进街道4342号1323室");
        assert!(!ok);
    }

    // ---- v1.1.4 续轮 T70：validate_extracted_with_params ----

    #[test]
    fn validate_extracted_with_params_generic() {
        // T70：Generic 分支分发（T77 改为自定义特殊字符白名单）
        let params = ExtractParams::Generic {
            allow_digits: true,
            allow_letters: true,
            allow_special_chars: "".into(),
            min_len: Some(3),
            max_len: None,
        };
        // 有效（数字+字母，长度 6 ≥ 3）
        let (ok, note) = validate_extracted_with_params(&params, "abc123");
        assert!(ok);
        assert_eq!(note, "");
        // 无效（含特殊字符 @，白名单为空）
        let (ok, note) = validate_extracted_with_params(&params, "abc@123");
        assert!(!ok);
        assert_eq!(note, "通用校验未通过（字符类或长度不符）");
        // 无效（长度 2 < 3）
        let (ok, _) = validate_extracted_with_params(&params, "ab");
        assert!(!ok);
        // 无效（空串）
        let (ok, _) = validate_extracted_with_params(&params, "");
        assert!(!ok);
        // T77：白名单包含 @ → abc@123 通过
        let params2 = ExtractParams::Generic {
            allow_digits: true,
            allow_letters: true,
            allow_special_chars: "@".into(),
            min_len: Some(3),
            max_len: None,
        };
        let (ok, _) = validate_extracted_with_params(&params2, "abc@123");
        assert!(ok);
        // T77：白名单不含 # → abc#123 不通过
        let (ok, _) = validate_extracted_with_params(&params2, "abc#123");
        assert!(!ok);
    }

    #[test]
    fn validate_extracted_with_params_birth_with_separators() {
        // T70：Birth 分支 + clean_birth 支持；T87：formats 空 = 全部接受（向后兼容）
        let params = ExtractParams::Birth { formats: vec![] };
        // 有效（含分隔符）
        let (ok, _) = validate_extracted_with_params(&params, "1949-12-31");
        assert!(ok);
        // 有效（纯数字）
        let (ok, _) = validate_extracted_with_params(&params, "19491231");
        assert!(ok);
        // 无效（月 13）
        let (ok, note) = validate_extracted_with_params(&params, "20031323");
        assert!(!ok);
        assert_eq!(note, "出生日期格式不符（清理后须为 8 位有效日期）");
    }

    #[test]
    fn validate_extracted_with_params_birth_formats() {
        // v1.1.5 T87：formats 非空 → 仅接受指定格式之一
        // 仅接受 yyyy-mm-dd
        let params = ExtractParams::Birth {
            formats: vec!["yyyy-mm-dd".into()],
        };
        let (ok, _) = validate_extracted_with_params(&params, "1949-12-31");
        assert!(ok);
        // yyyymmdd 不在勾选格式内 → 不通过
        let (ok, note) = validate_extracted_with_params(&params, "19491231");
        assert!(!ok);
        assert_eq!(note, "出生日期格式不符（须为勾选的格式之一）");
        // 多格式：yyyymmdd + yyyy/mm/dd 均接受
        let params2 = ExtractParams::Birth {
            formats: vec!["yyyymmdd".into(), "yyyy/mm/dd".into()],
        };
        let (ok, _) = validate_extracted_with_params(&params2, "19491231");
        assert!(ok);
        let (ok, _) = validate_extracted_with_params(&params2, "1949/12/31");
        assert!(ok);
        // yyyy-mm-dd 不在勾选列表 → 不通过
        let (ok, _) = validate_extracted_with_params(&params2, "1949-12-31");
        assert!(!ok);
        // 无效日期（即便格式匹配）→ 不通过
        let (ok, _) = validate_extracted_with_params(&params2, "20031323");
        assert!(!ok);
    }

    #[test]
    fn validate_extracted_with_params_address_structured() {
        // T70：Address 分支 + 结构化校验（v1.2.2 增加号/室范围）
        let params = ExtractParams::Address {
            min_hao: None,
            max_hao: None,
            min_shi: None,
            max_shi: None,
        };
        // 有效
        let (ok, _) = validate_extracted_with_params(&params, "北京市朝阳区建国路88号");
        assert!(ok);
        // 有效（原号超 1500 → 现在有效）
        let (ok, _) = validate_extracted_with_params(&params, "北京市朝阳区1501号101室");
        assert!(ok);
        // 无效（无中文）
        let (ok, note) = validate_extracted_with_params(&params, "hello world");
        assert!(!ok);
        assert_eq!(note, "地址格式不符（须全中文+地址关键词+号/室范围）");
        // 无效（空串）
        let (ok, _) = validate_extracted_with_params(&params, "");
        assert!(!ok);
        // v1.2.2：含英文字母 → 不通过
        let (ok, _) =
            validate_extracted_with_params(&params, "内蒙古自治区呼和O特市托克托县古城镇1319号139室");
        assert!(!ok);

        // v1.2.2：号范围校验
        let params_range = ExtractParams::Address {
            min_hao: Some(1),
            max_hao: Some(1500),
            min_shi: Some(101),
            max_shi: Some(999),
        };
        // 号/室都在范围内 → 通过
        let (ok, _) =
            validate_extracted_with_params(&params_range, "重庆市江津区几江街道260号228室");
        assert!(ok);
        // 号超出范围 → 不通过
        let (ok, _) =
            validate_extracted_with_params(&params_range, "天津市河西区下瓦房街道5189号375室");
        assert!(!ok);
        // 室超出范围 → 不通过
        let (ok, _) = validate_extracted_with_params(&params_range, "北京市朝阳区1号1000室");
        assert!(!ok);
    }

    /// 测试辅助：构造一个最小 Rule（params 可后续覆盖）。
    fn like_name_extract() -> Rule {
        Rule {
            id: "test".into(),
            name: "test".into(),
            kind: RuleKind::Extract,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: String::new(),
            template: None,
            params: None,
        }
    }
}
