//! 规则管理基础结构。
//!
//! v1.1.0：`Rule` 结构 + `RuleRegistry` + 三条姓名相关内置规则
//! （脱敏 / 校验 / 提取各一条）。规则持久化到 DB（`rules` 表），
//! 启动时若 DB 无规则则 seed 这三条内置规则。
//! v1.1.3：`Rule` 新增 `template: Option<TemplateParams>` 字段（通用模板脱敏参数，
//! JSON 存储到 `rules.template` 列）。T49 子规则化：4 条原独立脱敏规则
//! （身份证 / 手机 / 出生日期 / 银行卡）收敛为预设（`TemplateParams` 常量），
//! 不再单独 seed。T54 拆分：原 `general-mask` 一条规则拆为**两条独立规则**
//! `simple-mask`（整段脱敏，持 Simple 模板）+ `segment-mask`（分段脱敏，
//! 持 Segment 模板），`with_defaults()` 共 5 条（3 name + simple-mask + segment-mask）。
//! 旧 `general-mask` 由 `cleanup_deprecated_rules()` 删除。
//!
//! `Rule` 经 serde camelCase 序列化与前端对齐：
//! ```json
//! { "id": "name-validate", "name": "姓名校验", "kind": "validate",
//!   "field": "name", "pattern": "^[\\u4e00-\\u9fa5]{2,4}$",
//!   "replacement": null, "enabled": true, "description": "..." }
//! ```
//! v1.1.3 `simple-mask` / `segment-mask` 规则各持**空模板**（对应变体的
//! `default()`，所有字段 `None` / 空）→ 前端选预设填充参数，不选则不脱敏（透传）。
//! 预设示例（仅 `simple-mask` 适用，`segment-mask` 不内置预设）：
//! ```json
//! { "id": "simple-mask", ..., "template": { "keepPrefix": 6, "keepSuffix": 4,
//!   "maskChar": "*", "maskMinLen": 8 } }
//! ```
//! v1.1.3 T52：`TemplateParams` 改为 untagged enum（`Segment` / `Simple`）。
//! - `Simple` = 原 flat 结构（向后兼容旧 DB JSON，多余字段被忽略）。
//! - `Segment` = 按 `delimiter` 拆分值，对 `segments` 中列出的段做保留首尾脱敏
//!   （如 `zhangsan@example.com` 按 `@` 拆分，对第 0 段保留首尾各 1）。
//!
//! v1.1.3 T53：`SimpleTemplate` 新增 `reverse: Option<bool>`（反向脱敏：
//!   保留中间，对首 N 位和后 N 位脱敏）。
//!
//! v1.1.3 T55：新增 3 条**提取规则**（`phone-extract` / `bankcard-extract` /
//! `ip-extract`），每条持 `params: Option<ExtractParams>` 描述函数式校验器
//! （Luhn / IPv4+IPv6 / 手机号前缀列表）。提取正则宽松（召回优先），严格性
//! 由 `func_validator::validate_extracted` 兜底。工作流 = 提取候选 → 函数式
//! 校验 → 结果落新 Tab → 导出。`with_defaults()` 由 5 条 → 8 条。
//!
//! v1.1.3 T55b：拆分 `ip-extract` 为 `ip4-extract` + `ip6-extract` 两条独立
//! 规则，删除 `IpFamily` 枚举，`ExtractParams` 新增 `Ipv4` / `Ipv6` unit 变体。
//! `with_defaults()` 由 8 条 → 9 条。旧 `ip-extract` 由
//! `cleanup_deprecated_rules()` 删除。
//!
//! v1.1.3 T55c：新增 `idcard-extract` 规则（18 位身份证号提取 + 校验码严格
//! 校验），`ExtractParams` 新增 `IdCard` unit 变体。校验码算法（GB 11643-1999）
//! 由 `func_validator::is_valid_idcard` 实现。性别联合校验（手动指定性别列）
//! 在 `extract_validate_to_new_sheet_inner` 中进行，不存规则配置。`with_defaults()`
//! 由 9 条 → 10 条。
//!
//! v1.2.0 T93：将 `TemplateParams` / `SimpleTemplate` / `SegmentTemplate` /
//! `SegmentMask` + 构造器 + 预设拆到 [`template`] 子模块，`ExtractParams` 拆到
//! [`extract_params`] 子模块。本模块保留 `RuleKind` / `Rule` / `RuleRegistry`
//! + 内置规则构造器 + 测试。通过 `pub use` 再导出，保持路径兼容。

pub mod extract_params;
pub mod template;

pub use extract_params::ExtractParams;
pub use template::{
    bankcard_preset, birthdate_preset, idcard_preset, phone_preset, SegmentMask, SegmentTemplate,
    SimpleTemplate, TemplateParams,
};

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

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

/// 规则注册表。应用启动时从 DB 加载，内置规则集 seed 到 DB。
#[derive(Debug, Clone)]
pub struct RuleRegistry {
    rules: Vec<Rule>,
}

impl RuleRegistry {
    /// 创建空注册表。
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// 加载内置规则集。
    /// v1.1.0 三条姓名相关规则（脱敏/校验/提取各一条）。
    /// v1.1.3 T49：4 条原独立脱敏规则（身份证 / 手机 / 出生日期 / 银行卡）
    /// 收敛为预设（`idcard_preset()` 等，不再单独 seed）。T54 拆分：原
    /// `general-mask` 一条规则拆为 `simple-mask`（整段脱敏）+ `segment-mask`
    /// （分段脱敏）两条独立规则。`with_defaults()` 共 5 条（3 name +
    /// simple-mask + segment-mask）。旧 `general-mask` 由
    /// `cleanup_deprecated_rules()` 删除。
    /// v1.1.3 T55：新增 3 条提取规则（`phone-extract` / `bankcard-extract` /
    /// `ip-extract`），各持 `params` 函数式校验器。`with_defaults()` 共 8 条。
    /// v1.1.3 T55b：拆分 `ip-extract` 为 `ip4-extract`（IPv4）+ `ip6-extract`
    /// （IPv6）两条独立规则，`with_defaults()` 共 9 条。旧 `ip-extract` 由
    /// `cleanup_deprecated_rules()` 删除。
    /// v1.1.3 T55c：新增 `idcard-extract`（18 位身份证号 + 校验码 + 性别推断），
    /// `with_defaults()` 共 10 条。
    /// v1.1.4 T67：新增 6 条函数式校验规则（`username-validate` / `sex-validate` /
    /// `birth-validate` / `idcard-validate` / `phone-validate` / `address-validate`），
    /// kind=Validate 且带 `params` 走 `validate_extracted` 分发，
    /// `with_defaults()` 共 16 条。
    /// v1.1.4 续轮 T70：新增 `generic-validate`（字符类白名单 + 长度范围），
    /// `with_defaults()` 共 17 条。
    /// v1.1.5 T81：新增 `email-validate`（邮箱地址结构化校验），
    /// `with_defaults()` 共 18 条。
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        reg.register(Self::name_validate_rule());
        reg.register(Self::name_mask_rule());
        reg.register(Self::name_extract_rule());
        // v1.1.3 T54：整段脱敏 + 分段脱敏拆为两条独立规则（各持空模板）
        reg.register(Self::simple_mask_rule());
        reg.register(Self::segment_mask_rule());
        // v1.1.3 T55：3 条提取规则（手机号 / 银行卡 / IP），各持函数式校验参数
        reg.register(Self::phone_extract_rule());
        reg.register(Self::bankcard_extract_rule());
        // v1.1.3 T55b：原 ip-extract 拆为 ipv4 / ipv6 两条独立规则
        reg.register(Self::ip4_extract_rule());
        reg.register(Self::ip6_extract_rule());
        // v1.1.3 T55c：身份证号提取 + 校验码严格校验
        reg.register(Self::idcard_extract_rule());
        // v1.1.4 T67：6 条函数式校验规则（kind=Validate，带 params 走
        // validate_extracted 分发）。
        reg.register(Self::username_validate_rule());
        reg.register(Self::sex_validate_rule());
        reg.register(Self::birth_validate_rule());
        reg.register(Self::idcard_validate_rule());
        reg.register(Self::phone_validate_rule());
        reg.register(Self::address_validate_rule());
        // v1.1.4 续轮 T70：通用校验规则（字符类白名单 + 长度范围）。
        // `with_defaults()` 共 17 条。
        reg.register(Self::generic_validate_rule());
        // v1.1.5 T81：邮箱校验规则（kind=Validate，带 params 走
        // validate_extracted 分发）。`with_defaults()` 共 18 条。
        reg.register(Self::email_validate_rule());
        reg
    }

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
            template: Some(TemplateParams::Simple(SimpleTemplate::default())),
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
    /// `func_validator` 的 `check_phone_prefix` 兜底：空前缀列表 → 须 1 开头
    /// （标准中国手机号）；非空 → 前 3 位必须在列表内（用户自定义范围）。
    /// `params = PhonePrefix{[]}` → 空前缀列表 = 默认须 1 开头。前端可在
    /// RulesPanel 配置 `allowedPrefixes`（如 `["799","786"]`）→ 放行非标准前缀。
    pub fn phone_extract_rule() -> Rule {
        Rule {
            id: "phone-extract".into(),
            name: "手机号提取".into(),
            kind: RuleKind::Extract,
            field: None,
            pattern: Some(r"\b[1-9]\d{10}\b".into()),
            replacement: None,
            enabled: true,
            description: "提取 11 位手机号（默认 1 开头，可选前缀白名单校验）".into(),
            template: None,
            params: Some(ExtractParams::PhonePrefix {
                allowed_prefixes: Vec::new(),
            }),
        }
    }

    /// 银行卡号提取内置规则（v1.1.3 T55）：13-19 位数字，首位非 0。
    ///
    /// 提取正则 `\b[1-9]\d{12,18}\b`（宽松召回），严格校验由 `func_validator`
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
    /// 由 `func_validator::is_valid_ipv4` 兜底（段范围 0-255 + 禁前导零）。
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
    /// 的 hex 段，严格校验由 `func_validator::is_valid_ipv6` 兜底（走
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
    /// 严格校验由 `func_validator::is_valid_idcard` 兜底（GB 11643-1999 校验码
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
    // `func_validator::validate_extracted` 分发）。与 name-validate 不同，这些
    // 规则不带正则 pattern（field=None），由调用方（T68 新命令）直接对单元格原值
    // 调 `validate_extracted`。idcard/phone 复用现有 IdCard / PhonePrefix 变体：
    // idcard-validate 的 IdCard 分支返回 (true, gender) 供跨字段比对；
    // phone-validate 用 PhonePrefix 变体，但 validate 语义是整串校验而非提取
    // 前缀（PhonePrefix 分支只调 check_phone_prefix 查前缀，整串严格校验在
    // T68 新命令里直接调 is_valid_phone）。----

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
    /// 走 `validate_extracted` 的 PhonePrefix 分支 → `check_phone_prefix`。
    /// 注意：PhonePrefix 分支只查前缀白名单，不检查 11 位（这是提取规则
    /// 的兜底语义）。整串严格校验（11 位 + 纯数字 + 默认 1 开头 + 前缀白名单）在
    /// T68 新命令里直接调 `is_valid_phone`，不走 `validate_extracted` 的 PhonePrefix 分支。
    pub fn phone_validate_rule() -> Rule {
        Rule {
            id: "phone-validate".into(),
            name: "手机号校验".into(),
            kind: RuleKind::Validate,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: "校验 11 位手机号（默认 1 开头，可选前缀白名单）".into(),
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

    /// 注册一条规则（若 id 已存在则覆盖）。
    pub fn register(&mut self, rule: Rule) {
        if let Some(existing) = self.rules.iter_mut().find(|r| r.id == rule.id) {
            *existing = rule;
        } else {
            self.rules.push(rule);
        }
    }

    /// 列出全部规则（按注册顺序）。
    pub fn list(&self) -> &[Rule] {
        &self.rules
    }

    /// 按 id 查找规则。
    pub fn get(&self, id: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.id == id)
    }

    /// 切换规则启用状态。不存在则返回 `Err`。
    pub fn toggle(&mut self, id: &str, enabled: bool) -> Result<(), CoreError> {
        let rule = self
            .rules
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| CoreError::Processor(format!("rule not found: {id}")))?;
        rule.enabled = enabled;
        Ok(())
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::template::{SegmentMask, SegmentTemplate, SimpleTemplate, TemplateParams};
    use super::extract_params::ExtractParams;

    /// 测试辅助：断言模板是 Simple 变体并返回内部 `&SimpleTemplate`。
    fn expect_simple(tpl: &TemplateParams) -> &SimpleTemplate {
        match tpl {
            TemplateParams::Simple(s) => s,
            TemplateParams::Segment(_) => panic!("expected Simple, got Segment"),
        }
    }

    #[test]
    fn with_defaults_loads_ten_rules() {
        let reg = RuleRegistry::with_defaults();
        let rules = reg.list();
        // v1.1.4 续轮 T70：3 条姓名 + simple-mask + segment-mask + 5 条 extract
        // （name-extract + phone/bankcard/ip4/ip6/idcard）+ 7 条 validate
        // （name + username/sex/birth/idcard/phone/address + generic）
        // v1.1.5 T81：+ email-validate = 18 条
        assert_eq!(rules.len(), 18);
        // 脱敏 / 校验 / 提取 三种 kind 都存在
        let kinds: Vec<RuleKind> = rules.iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&RuleKind::Mask));
        assert!(kinds.contains(&RuleKind::Validate));
        assert!(kinds.contains(&RuleKind::Extract));
        // 3 条 mask 规则（name-mask + simple-mask + segment-mask）
        let mask_count = kinds.iter().filter(|k| **k == RuleKind::Mask).count();
        assert_eq!(mask_count, 3);
        // 6 条 extract 规则（name-extract + phone/bankcard/ip4/ip6/idcard）
        let extract_count = kinds.iter().filter(|k| **k == RuleKind::Extract).count();
        assert_eq!(extract_count, 6);
        // v1.1.5 T81：9 条 validate 规则（name-validate + 6 条 T67 + generic + email）
        let validate_count = kinds.iter().filter(|k| **k == RuleKind::Validate).count();
        assert_eq!(validate_count, 9);
    }

    #[test]
    fn name_validate_rule_pattern_is_chinese_range() {
        let r = RuleRegistry::name_validate_rule();
        assert_eq!(r.id, "name-validate");
        assert_eq!(r.kind, RuleKind::Validate);
        assert_eq!(r.field.as_deref(), Some("name"));
        assert!(r.pattern.is_some());
        assert!(r.enabled);
        // name 规则无 template / params
        assert!(r.template.is_none());
        assert!(r.params.is_none());
    }

    #[test]
    fn name_mask_rule_keeps_first_char_semantic() {
        let r = RuleRegistry::name_mask_rule();
        assert_eq!(r.id, "name-mask");
        assert_eq!(r.kind, RuleKind::Mask);
        assert_eq!(r.field.as_deref(), Some("name"));
        // replacement=None → SimpleMasker 默认掩码字符 `*`
        assert!(r.replacement.is_none());
        // 无 template / params → 走 SimpleMasker 旧逻辑
        assert!(r.template.is_none());
        assert!(r.params.is_none());
    }

    #[test]
    fn name_extract_rule_has_chinese_pattern() {
        let r = RuleRegistry::name_extract_rule();
        assert_eq!(r.id, "name-extract");
        assert_eq!(r.kind, RuleKind::Extract);
        assert_eq!(r.field.as_deref(), Some("name"));
        assert!(r.pattern.is_some());
        // T55：name-extract 无 params（仅正则提取，不做函数式校验）
        assert!(r.params.is_none());
    }

    #[test]
    fn simple_mask_rule_has_empty_template() {
        let r = RuleRegistry::simple_mask_rule();
        assert_eq!(r.id, "simple-mask");
        assert_eq!(r.kind, RuleKind::Mask);
        // 持空 Simple 模板 → is_empty()==true
        let tpl = r.template.expect("simple-mask must have template");
        assert!(matches!(tpl, TemplateParams::Simple(_)));
        assert!(tpl.is_empty());
        assert!(r.params.is_none());
    }

    #[test]
    fn segment_mask_rule_has_empty_template() {
        let r = RuleRegistry::segment_mask_rule();
        assert_eq!(r.id, "segment-mask");
        assert_eq!(r.kind, RuleKind::Mask);
        // 持空 Segment 模板 → is_empty()==true
        let tpl = r.template.expect("segment-mask must have template");
        assert!(matches!(tpl, TemplateParams::Segment(_)));
        assert!(tpl.is_empty());
        assert!(r.params.is_none());
    }

    #[test]
    fn phone_extract_rule_has_phone_prefix_params() {
        let r = RuleRegistry::phone_extract_rule();
        assert_eq!(r.id, "phone-extract");
        assert_eq!(r.kind, RuleKind::Extract);
        assert!(r.pattern.is_some());
        // T55：params = PhonePrefix{[]}（空前缀列表 = 默认须 1 开头）
        let params = r.params.as_ref().expect("phone-extract must have params");
        match params {
            ExtractParams::PhonePrefix { allowed_prefixes } => {
                assert!(allowed_prefixes.is_empty());
            }
            _ => panic!("expected PhonePrefix, got {params:?}"),
        }
    }

    #[test]
    fn bankcard_extract_rule_has_luhn_params() {
        let r = RuleRegistry::bankcard_extract_rule();
        assert_eq!(r.id, "bankcard-extract");
        assert_eq!(r.kind, RuleKind::Extract);
        assert!(r.pattern.is_some());
        // T55：params = Luhn
        let params = r
            .params
            .as_ref()
            .expect("bankcard-extract must have params");
        assert!(matches!(params, ExtractParams::Luhn));
    }

    #[test]
    fn ip4_extract_rule_has_ipv4_params() {
        let r = RuleRegistry::ip4_extract_rule();
        assert_eq!(r.id, "ip4-extract");
        assert_eq!(r.kind, RuleKind::Extract);
        assert!(r.pattern.is_some());
        // T55b：params = Ipv4
        let params = r.params.as_ref().expect("ip4-extract must have params");
        assert!(matches!(params, ExtractParams::Ipv4));
    }

    #[test]
    fn ip6_extract_rule_has_ipv6_params() {
        let r = RuleRegistry::ip6_extract_rule();
        assert_eq!(r.id, "ip6-extract");
        assert_eq!(r.kind, RuleKind::Extract);
        assert!(r.pattern.is_some());
        // T55b：params = Ipv6
        let params = r.params.as_ref().expect("ip6-extract must have params");
        assert!(matches!(params, ExtractParams::Ipv6));
    }

    #[test]
    fn idcard_extract_rule_has_idcard_params() {
        let r = RuleRegistry::idcard_extract_rule();
        assert_eq!(r.id, "idcard-extract");
        assert_eq!(r.kind, RuleKind::Extract);
        assert_eq!(r.pattern.as_deref(), Some(r"\b[1-9]\d{16}[\dXx]\b"));
        // T55c：params = IdCard
        let params = r.params.as_ref().expect("idcard-extract must have params");
        assert!(matches!(params, ExtractParams::IdCard));
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

    #[test]
    fn generic_validate_rule_default_params() {
        // v1.1.4 续轮 T70：generic-validate 默认参数
        let r = RuleRegistry::generic_validate_rule();
        assert_eq!(r.id, "generic-validate");
        assert_eq!(r.kind, RuleKind::Validate);
        assert!(r.pattern.is_none());
        assert!(r.template.is_none());
        let params = r
            .params
            .as_ref()
            .expect("generic-validate must have params");
        match params {
            ExtractParams::Generic {
                allow_digits,
                allow_letters,
                allow_special_chars,
                min_len,
                max_len,
            } => {
                assert!(*allow_digits, "default allow_digits = true");
                assert!(*allow_letters, "default allow_letters = true");
                assert!(
                    allow_special_chars.is_empty(),
                    "default allow_special_chars = empty"
                );
                assert_eq!(*min_len, None, "default min_len = None");
                assert_eq!(*max_len, None, "default max_len = None");
            }
            other => panic!("expected Generic, got {other:?}"),
        }
    }

    #[test]
    fn rule_params_field_serde_skip_when_none() {
        // name-mask 无 params → JSON 不输出 params 字段（向后兼容）
        let r = RuleRegistry::name_mask_rule();
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("params"));
    }

    #[test]
    fn rule_params_field_serde_present_when_some() {
        // phone-extract 有 params → JSON 输出 params 字段
        let r = RuleRegistry::phone_extract_rule();
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
    fn idcard_preset_params() {
        let tpl = idcard_preset();
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
        let tpl = phone_preset();
        let s = expect_simple(&tpl);
        assert_eq!(s.keep_prefix, Some(3));
        assert_eq!(s.keep_suffix, Some(4));
        assert_eq!(s.mask_min_len, Some(4));
        assert_eq!(s.reverse, None);
    }

    #[test]
    fn birthdate_preset_params() {
        let tpl = birthdate_preset();
        let s = expect_simple(&tpl);
        assert_eq!(s.keep_prefix, Some(8));
        assert_eq!(s.keep_suffix, Some(0));
        assert_eq!(s.mask_min_len, Some(2));
        assert_eq!(s.reverse, None);
    }

    #[test]
    fn bankcard_preset_params() {
        let tpl = bankcard_preset();
        let s = expect_simple(&tpl);
        assert_eq!(s.keep_prefix, Some(4));
        assert_eq!(s.keep_suffix, Some(4));
        assert_eq!(s.mask_min_len, Some(1));
        assert_eq!(s.reverse, None);
    }

    #[test]
    fn template_params_serde_roundtrip() {
        let tpl = idcard_preset();
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
    fn rule_template_field_serde_skip_when_none() {
        // name-mask 无 template → JSON 不输出 template 字段（向后兼容 v1.1.2 前端）
        let r = RuleRegistry::name_mask_rule();
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("template"));
    }

    #[test]
    fn rule_template_field_serde_present_when_some() {
        // simple-mask 有 template（空 Simple 模板）→ JSON 输出 template 字段
        let r = RuleRegistry::simple_mask_rule();
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

    #[test]
    fn get_by_id_works() {
        let reg = RuleRegistry::with_defaults();
        assert!(reg.get("name-validate").is_some());
        assert!(reg.get("name-mask").is_some());
        assert!(reg.get("name-extract").is_some());
        assert!(reg.get("simple-mask").is_some());
        assert!(reg.get("segment-mask").is_some());
        // T55b：4 条新 extract 规则（phone / bankcard / ip4 / ip6）
        assert!(reg.get("phone-extract").is_some());
        assert!(reg.get("bankcard-extract").is_some());
        assert!(reg.get("ip4-extract").is_some());
        assert!(reg.get("ip6-extract").is_some());
        // T55c：idcard-extract
        assert!(reg.get("idcard-extract").is_some());
        // v1.1.4 T67：6 条新 validate 规则
        assert!(reg.get("username-validate").is_some());
        assert!(reg.get("sex-validate").is_some());
        assert!(reg.get("birth-validate").is_some());
        assert!(reg.get("idcard-validate").is_some());
        assert!(reg.get("phone-validate").is_some());
        assert!(reg.get("address-validate").is_some());
        // v1.1.4 续轮 T70：generic-validate
        assert!(reg.get("generic-validate").is_some());
        // v1.1.5 T81：email-validate
        assert!(reg.get("email-validate").is_some());
        // T55b：旧 ip-extract id 已不存在（拆分后由 cleanup_deprecated_rules 删除）
        assert!(reg.get("ip-extract").is_none());
        // T54：旧 general-mask id 已不存在（由 cleanup_deprecated_rules 删除）
        assert!(reg.get("general-mask").is_none());
        // 旧 id 已不存在
        assert!(reg.get("idcard-mask").is_none());
        assert!(reg.get("phone-mask").is_none());
        assert!(reg.get("birthdate-mask").is_none());
        assert!(reg.get("bankcard-mask").is_none());
        assert!(reg.get("nope").is_none());
    }

    #[test]
    fn toggle_disables_rule() {
        let mut reg = RuleRegistry::with_defaults();
        assert!(reg.get("name-validate").unwrap().enabled);
        reg.toggle("name-validate", false).unwrap();
        assert!(!reg.get("name-validate").unwrap().enabled);
    }

    #[test]
    fn toggle_unknown_returns_err() {
        let mut reg = RuleRegistry::with_defaults();
        assert!(reg.toggle("nope", true).is_err());
    }

    #[test]
    fn register_overwrites_same_id() {
        let mut reg = RuleRegistry::with_defaults();
        let mut r = RuleRegistry::name_validate_rule();
        r.description = "updated".into();
        reg.register(r);
        assert_eq!(reg.list().len(), 18);
        assert_eq!(reg.get("name-validate").unwrap().description, "updated");
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
}
