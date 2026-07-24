//! 内置规则集（v0.4.5 起逐步接入）。
//!
//! v0.4.4 规则引擎重构后规则池初始为空，只留好 `scope`/`tag` 字段接口。
//! v0.4.5 接入数据提取规则三条：手机号（scope="phone"）、银行卡号
//! （scope="bankcard"）、IP 地址（scope="ip"），覆盖数据格式规范定义的三类需识别提取的数据。
//!
//! v0.5.x：接入 4 条通用脱敏模版（template / split_template / regex_replace /
//! const_replace，tag="mask"，params=None），用户在 RulesView 试运行或 MaskView
//! 按列填参数。
//!
//! v0.5.x：接入 4 条数据校验规则（tag="validate"），覆盖个人信息规范定义的
//! 4 类字段：用户名（username）/姓名（name）/身份证号（idcard）/
//! 手机号码（pinfo_phone）。这些规则供 `validate_pipeline` 按 field 命中列后
//! 逐 cell 校验。
//!
//! `builtin_ruleset()` 返回当前所有内置规则组成的 `RuleSet`，供：
//! - `extract::extract_text` / `extract_file` 在未传 rules_json 时作为默认规则集
//!   （这样无需用户配置即可提取手机号 / 银行卡号 / IP）
//! - tauri `list_builtin_rules` 命令序列化为 JSON 返回给前端，启动时加载到
//!   `state.rules`，让 RulesView / ExtractView / ValidateView 显示内置规则
//!
//! 后续版本按数据类型逐条接入：email / mac / ...
//! 每条只需在此追加一个 `FieldRule`，无需改 scan/extract 算法。

use crate::rules::{FieldRule, MaskRule, RuleSet};

/// 返回内置规则集（v0.4.5：phone / bankcard / ip 三条数据提取规则；
/// v0.5.x：template / split_template / regex_replace / const_replace 四条
/// 通用脱敏模版，params 全为 None，用户在 RulesView 试运行或 MaskView 按列填参数；
/// v0.5.x：username / name / idcard / pinfo_phone 四条数据校验规则）。
///
/// 内置规则是「出厂自带」的，与用户在 RulesView 创建的规则区分。前端启动
/// 时通过 `list_builtin_rules` 命令拉取并写入 `state.rules`，用户可见可用
/// 但不可编辑删除（后续版本若支持用户规则，再叠加合并）。
pub fn builtin_ruleset() -> RuleSet {
    RuleSet {
        validators: vec![
            // 数据提取（tag="extract"）
            phone_extract_rule(),
            bankcard_extract_rule(),
            ip_extract_rule(),
            // 数据校验（tag="validate"，个人信息规范）
            username_validate_rule(),
            name_validate_rule(),
            idcard_validate_rule(),
            pinfo_phone_validate_rule(),
        ],
        maskers: vec![
            template_mask_rule(),
            split_template_mask_rule(),
            regex_replace_mask_rule(),
            const_replace_mask_rule(),
        ],
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

// ── v0.5.x 内置数据校验规则（4 条，tag="validate"，个人信息规范）──────────
//
// 这 4 条对应规范定义的 4 类需校验字段：
// - 用户名(username)：仅字母数字
// - 姓名(name)：全中文
// - 身份证号(idcard)：18 位 + GB11643 校验码算法
// - 手机号码(pinfo_phone)：11 位 + 规范指定的 52 个虚假号段前缀集合
//
// `scope` 同时是 `ValidatorRegistry::get` 查键：username / name / idcard 复用
// 既有 validator；pinfo_phone 走 `PInfoPhoneValidator`（独立于通用 phone）。
//
// `field` 用规范列名（中文），这样用户导入符合规范列结构的
// CSV 后，validate_pipeline 按 header 名命中即可直接生效，无需用户手填 field。
// 若用户列名不同，可在 RulesView 复制本规则并改 field。

/// 用户名校验规则：scope="username"，tag="validate"，field="用户名"。
///
/// spec：只能由数字字母组成。错误示例 ab.cd、ad_1in、a-123。
pub fn username_validate_rule() -> FieldRule {
    FieldRule {
        field: "用户名".into(),
        scope: "username".into(),
        tag: "validate".into(),
        params: None,
        message: None,
        description: Some("用户名校验（仅字母数字）".into()),
    }
}

/// 姓名校验规则：scope="name"，tag="validate"，field="姓名"。
///
/// spec：只能由全中文组成。错误示例 张3、li四。
pub fn name_validate_rule() -> FieldRule {
    FieldRule {
        field: "姓名".into(),
        scope: "name".into(),
        tag: "validate".into(),
        params: None,
        message: None,
        description: Some("姓名校验（全中文）".into()),
    }
}

/// 身份证号校验规则：scope="idcard"，tag="validate"，field="身份证号"。
///
/// spec：18 位 = 6 地址码 + 8 出生日期码 + 3 顺序码 + 1 校验码；
/// 校验码 = 前 17 位加权和（系数 7,9,10,5,8,4,2,1,6,3,7,9,10,5,8,4,2）mod 11
/// 查表（10X98765432）。倒数第 2 位奇=男偶=女（描述用，校验不强制）。
pub fn idcard_validate_rule() -> FieldRule {
    FieldRule {
        field: "身份证号".into(),
        scope: "idcard".into(),
        tag: "validate".into(),
        params: None,
        message: None,
        description: Some("身份证号校验（18 位 + GB11643 校验码）".into()),
    }
}

/// 手机号码校验规则：scope="pinfo_phone"，tag="validate"，field="手机号码"。
///
/// spec：11 位 10 进制数字，前 3 位必须在规范指定的 52 个虚假号段集合内
/// （734/735/.../799，详见 `validators::pinfo_phone::PINFO_PHONE_PREFIXES`）。
///
/// 与通用 `phone` scope（仅校验首位 1，去绝对化、无号段白名单，用于 extract）
/// 区分：本校验器严格按虚假号段集合，仅用于个人信息规范的校验场景。
/// field 用规范列名「手机号码」（注意：通用 phone 提取规则的 field 是 "phone"）。
pub fn pinfo_phone_validate_rule() -> FieldRule {
    FieldRule {
        field: "手机号码".into(),
        scope: "pinfo_phone".into(),
        tag: "validate".into(),
        params: None,
        message: None,
        description: Some("手机号码校验（11 位 + 52 个虚假号段前缀）".into()),
    }
}

// ── v0.5.x 内置脱敏模版（4 条，tag="mask"，params=None）──────────────────
//
// 这 4 条是「替换模版」家族的四种 scope，覆盖数据脱敏规范全部
// 脱敏场景。params 全为 None——模版本身不写死参数，用户在 RulesView 试运行
// 或 MaskView 按列填 keep_prefix/keep_suffix/... 等参数后再作用于数据。
// 启动时经 `list_builtin_rules` 序列化到前端 `state.rules.maskers`，RulesView
// 按 tag="mask" 过滤即可展示这 4 条；每条带「试运行」入口（trial_mask 命令
// 调 apply_mask_op，用户填参数 + 样例值 → 看脱敏结果）。
//
// field 用中文占位（与规范列名一致），仅文档用途，实际命中靠 MaskView 按列
// 配置时填入的真实 field。

/// 替换模版（最通用）：scope="template"，tag="mask"。
///
/// 参数（用户运行期填）：keep_prefix / keep_suffix / mask_char / mask_min_len /
/// min_len / max_len / cjk。覆盖姓名(cjk)、身份证号、手机号、出生日期、银行卡号
/// 等全部脱敏类型。
pub fn template_mask_rule() -> MaskRule {
    MaskRule {
        field: "替换模版".into(),
        scope: "template".into(),
        tag: "mask".into(),
        params: None,
        message: None,
        description: Some(
            "通用替换脱敏：保留前 N / 后 M 位，中间用掩码字符填充。支持 cjk 中文姓名分支。".into(),
        ),
    }
}

/// 分段替换模版：scope="split_template"，tag="mask"。
///
/// 参数：separator / segment_index + template 族参数（keep_prefix/keep_suffix/...）。
/// 适用于分隔符分隔的多段字段（如 `192.168.1.1` 脱某段、`a|b|c` 脱第二段）。
pub fn split_template_mask_rule() -> MaskRule {
    MaskRule {
        field: "分段替换模版".into(),
        scope: "split_template".into(),
        tag: "mask".into(),
        params: None,
        message: None,
        description: Some(
            "按分隔符拆分后对指定段做替换脱敏，其余段原样保留。适用于 IP、多值字段。".into(),
        ),
    }
}

/// 正则替换模版：scope="regex_replace"，tag="mask"。
///
/// 参数：pattern / replacement / match_mode。用户自定义正则匹配 + 替换串，
/// 适用于 template 无法覆盖的复杂结构（如邮箱 `(\w{1,2})[\w.]*@` → `$1***@`）。
pub fn regex_replace_mask_rule() -> MaskRule {
    MaskRule {
        field: "正则替换模版".into(),
        scope: "regex_replace".into(),
        tag: "mask".into(),
        params: None,
        message: None,
        description: Some(
            "正则匹配 + 替换串脱敏，支持全局/首次匹配模式。适用于邮箱、自定义结构。".into(),
        ),
    }
}

/// 常量替换模版：scope="const_replace"，tag="mask"。
///
/// 参数：with。整字段替换为固定常量（如 `***` / `REDACTED`），不保留任何原文。
pub fn const_replace_mask_rule() -> MaskRule {
    MaskRule {
        field: "常量替换模版".into(),
        scope: "const_replace".into(),
        tag: "mask".into(),
        params: None,
        message: None,
        description: Some(
            "整字段替换为固定常量（如 *** / REDACTED），不保留原文。".into(),
        ),
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
        // 3 extract + 4 validate = 7 validators
        assert_eq!(rs.validators.len(), 7);
        assert_eq!(rs.maskers.len(), 4);
        // phone extract
        assert_eq!(rs.validators[0].field, "phone");
        assert_eq!(rs.validators[0].scope, "phone");
        assert_eq!(rs.validators[0].tag, "extract");
        // bankcard extract
        assert_eq!(rs.validators[1].field, "bankcard");
        assert_eq!(rs.validators[1].scope, "bankcard");
        assert_eq!(rs.validators[1].tag, "extract");
        // ip extract
        assert_eq!(rs.validators[2].field, "ip");
        assert_eq!(rs.validators[2].scope, "ip");
        assert_eq!(rs.validators[2].tag, "extract");
        // username validate
        assert_eq!(rs.validators[3].field, "用户名");
        assert_eq!(rs.validators[3].scope, "username");
        assert_eq!(rs.validators[3].tag, "validate");
        // name validate
        assert_eq!(rs.validators[4].field, "姓名");
        assert_eq!(rs.validators[4].scope, "name");
        assert_eq!(rs.validators[4].tag, "validate");
        // idcard validate
        assert_eq!(rs.validators[5].field, "身份证号");
        assert_eq!(rs.validators[5].scope, "idcard");
        assert_eq!(rs.validators[5].tag, "validate");
        // pinfo_phone validate
        assert_eq!(rs.validators[6].field, "手机号码");
        assert_eq!(rs.validators[6].scope, "pinfo_phone");
        assert_eq!(rs.validators[6].tag, "validate");
        // 4 mask templates
        assert_eq!(rs.maskers[0].scope, "template");
        assert_eq!(rs.maskers[1].scope, "split_template");
        assert_eq!(rs.maskers[2].scope, "regex_replace");
        assert_eq!(rs.maskers[3].scope, "const_replace");
        for m in &rs.maskers {
            assert_eq!(m.tag, "mask");
            assert!(m.params.is_none());
        }
    }

    #[test]
    fn builtin_mask_rules_fields() {
        let rs = builtin_ruleset();
        let t = &rs.maskers[0];
        assert_eq!(t.scope, "template");
        assert_eq!(t.tag, "mask");
        assert!(t.params.is_none());
        assert!(t.description.is_some());
        let s = &rs.maskers[1];
        assert_eq!(s.scope, "split_template");
        assert_eq!(s.tag, "mask");
        let r = &rs.maskers[2];
        assert_eq!(r.scope, "regex_replace");
        assert_eq!(r.tag, "mask");
        let c = &rs.maskers[3];
        assert_eq!(c.scope, "const_replace");
        assert_eq!(c.tag, "mask");
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

    /// 非首位 1 的 11 位数字不记 phone finding（PhoneValidator 校验失败）。
    #[test]
    fn builtin_ruleset_skips_non_1_prefix_phone() {
        let rs = builtin_ruleset();
        let scan = DefaultSensitiveScan::new();
        let findings = scan.scan("nope 22345678901", &rs);
        let phones: Vec<&str> = findings
            .iter()
            .filter(|f| f.r#type == "phone")
            .map(|f| f.value.as_str())
            .collect();
        assert!(
            !phones.contains(&"22345678901"),
            "non-1-prefix should not be extracted as phone: {:?}",
            phones
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

    /// 未通过 Luhn 校验的卡号不记 bankcard finding（BankCardValidator 校验失败）。
    /// 合成号 6222021234567890124（19 位，未通过 Luhn）。
    #[test]
    fn builtin_ruleset_skips_invalid_bankcard() {
        let rs = builtin_ruleset();
        let scan = DefaultSensitiveScan::new();
        let findings = scan.scan("card 6222021234567890124 end", &rs);
        let cards: Vec<&str> = findings
            .iter()
            .filter(|f| f.r#type == "bankcard")
            .map(|f| f.value.as_str())
            .collect();
        assert!(
            !cards.contains(&"6222021234567890124"),
            "invalid luhn bankcard should not be extracted as bankcard: {:?}",
            cards
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

    /// 段 > 255 的非法 IP 不记 ip finding（IpValidator 校验失败）。
    /// 256.1.1.1 第一段 256 > 255。
    #[test]
    fn builtin_ruleset_skips_invalid_ip() {
        let rs = builtin_ruleset();
        let scan = DefaultSensitiveScan::new();
        let findings = scan.scan("bad 256.1.1.1 here", &rs);
        let ips: Vec<&str> = findings
            .iter()
            .filter(|f| f.r#type == "ip")
            .map(|f| f.value.as_str())
            .collect();
        assert!(
            !ips.contains(&"256.1.1.1"),
            "invalid ip (>255) should not be extracted as ip: {:?}",
            ips
        );
    }

    // ── 数据校验规则（tag="validate"，个人信息规范）──────────

    #[test]
    fn username_validate_rule_fields() {
        let r = username_validate_rule();
        assert_eq!(r.field, "用户名");
        assert_eq!(r.scope, "username");
        assert_eq!(r.tag, "validate");
        assert!(r.params.is_none());
        assert!(r.message.is_none());
        assert!(r.description.is_some());
    }

    #[test]
    fn name_validate_rule_fields() {
        let r = name_validate_rule();
        assert_eq!(r.field, "姓名");
        assert_eq!(r.scope, "name");
        assert_eq!(r.tag, "validate");
        assert!(r.params.is_none());
        assert!(r.message.is_none());
        assert!(r.description.is_some());
    }

    #[test]
    fn idcard_validate_rule_fields() {
        let r = idcard_validate_rule();
        assert_eq!(r.field, "身份证号");
        assert_eq!(r.scope, "idcard");
        assert_eq!(r.tag, "validate");
        assert!(r.params.is_none());
        assert!(r.message.is_none());
        assert!(r.description.is_some());
    }

    #[test]
    fn pinfo_phone_validate_rule_fields() {
        let r = pinfo_phone_validate_rule();
        assert_eq!(r.field, "手机号码");
        assert_eq!(r.scope, "pinfo_phone");
        assert_eq!(r.tag, "validate");
        assert!(r.params.is_none());
        assert!(r.message.is_none());
        assert!(r.description.is_some());
    }

    /// validate_pipeline 用内置规则集校验符合规范列结构的 CSV：合法行全 true。
    ///
    /// 使用规范示例行（前缀 788 ∈ 虚假号段集合）。
    #[test]
    fn builtin_ruleset_validates_spec_rows() {
        use crate::pipeline::validate::validate_pipeline;
        use crate::readers::Records;

        let records = Records {
            headers: vec![
                "编号".into(),
                "用户名".into(),
                "姓名".into(),
                "身份证号".into(),
                "手机号码".into(),
            ],
            rows: vec![vec![
                "1".into(),
                "GAn4NPxq5omi".into(),
                "广怀萍".into(),
                "779200198010124642".into(),
                "79180497760".into(),
            ]],
        };
        let rs = builtin_ruleset();
        let r = validate_pipeline(&records, &rs).expect("validate");
        // 用户名（纯字母数字）、姓名（全中文）、身份证号（校验码合法）、
        // 手机号码（791 ∈ 虚假号段集合）均合法 → 整行 valid。
        assert!(
            r.valid_matrix[0].iter().all(|&v| v),
            "expected all-valid row, got {:?}",
            r.valid_matrix[0]
        );
        assert_eq!(r.summary.invalid_rows, 0);
    }

    /// validate_pipeline 用内置规则集标记非法 cell：用户名含下划线 / 姓名
    /// 含数字 / 手机号用真实号段（138 不在号段集合）。
    #[test]
    fn builtin_ruleset_marks_invalid_pinfo_cells() {
        use crate::pipeline::validate::validate_pipeline;
        use crate::readers::Records;

        let records = Records {
            headers: vec![
                "编号".into(),
                "用户名".into(),
                "姓名".into(),
                "身份证号".into(),
                "手机号码".into(),
            ],
            rows: vec![vec![
                "1".into(),
                "ad_1in".into(),      // 含下划线 → 非法
                "张3".into(),         // 含数字 → 非法
                "779200198010124642".into(), // 合法
                "13812345678".into(),  // 真实号段，不在号段集合 → 非法
            ]],
        };
        let rs = builtin_ruleset();
        let r = validate_pipeline(&records, &rs).expect("validate");
        // 找到各列的 col idx
        let h_idx: std::collections::HashMap<&str, usize> = records
            .headers
            .iter()
            .enumerate()
            .map(|(i, h)| (h.as_str(), i))
            .collect();
        assert!(!r.valid_matrix[0][h_idx["用户名"]], "username should be invalid");
        assert!(!r.valid_matrix[0][h_idx["姓名"]], "name should be invalid");
        assert!(r.valid_matrix[0][h_idx["身份证号"]], "idcard should be valid");
        assert!(!r.valid_matrix[0][h_idx["手机号码"]], "pinfo_phone should be invalid");
        assert_eq!(r.summary.invalid_rows, 1);
    }
}
