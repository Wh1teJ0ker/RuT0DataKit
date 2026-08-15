//! 处理器模块：脱敏 / 规则管理。
//!
//! v1.1.0 原型实现：
//! - `rules` → `Rule` 结构 + `RuleRegistry` + 三条姓名相关内置规则（脱敏/校验/提取各一条）
//! - `masker` → `Masker` trait + `SimpleMasker`（姓名脱敏：≥3 保留首尾 + 中间掩码字符；2 保留首字符；通用：首尾保留）
//!
//! 规则持久化到 DB（`rules` 表），启动时若 DB 无规则则 seed 三条内置规则。
//!
//! v1.1.3 T55：新增函数式校验器（Luhn / IPv4 / IPv6 /
//! 手机号前缀），供提取规则的 `validate_extracted` 严格兜底。T55b 拆分
//! `ip-extract` 为 `ip4-extract` + `ip6-extract` 两条独立规则（删除 `IpFamily`）。
//! T55c 新增 `idcard-extract` 规则：18 位身份证号召回 `\b[1-9]\d{16}[\dXx]\b`（首位非零），
//! 校验码严格校验（GB 11643-1999，加权求和 mod 11 查表），第 17 位性别推断
//! 奇=男/偶=女。性别联合校验（比对用户在提取时指定的性别列）在
//! `extract_validate_to_new_sheet_inner` 中进行——性别列是提取时参数而非规则
//! 配置，不同 sheet 列名不同，不落 DB。
//!
//! v1.2.1：函数式校验器重命名为 `validators`，按域拆分为 8 个子模块
//! （luhn / ip / idcard / personal / datetime / address / email / generic），
//! 原 `rules/mod.rs` 拆为 `rule.rs` / `registry.rs` / `builtins.rs`。
//!
//! 设计原则（延续 02-技术设计文档 §2.1）：
//! - trait 边界隔离纯逻辑与 IO；
//! - 能力注册化——通过 `RuleRegistry` 管理规则，不硬编码到调用链。

pub mod masker;
pub mod rules;
pub mod validators;

pub use masker::{MaskResult, Masker, SimpleMasker};
pub use rules::{
    ExtractParams, Rule, RuleKind, RuleRegistry, SegmentMask, SegmentTemplate, SimpleTemplate,
    TemplateParams,
};
