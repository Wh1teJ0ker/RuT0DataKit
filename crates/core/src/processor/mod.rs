//! 处理器模块：脱敏 / 校验 / 提取 / 规则管理。
//!
//! v1.1.0 原型实现：
//! - `rules` → `Rule` 结构 + `RuleRegistry` + 三条姓名相关内置规则（脱敏/校验/提取各一条）
//! - `validator` → `Validator` trait + `RegexValidator`（承载姓名校验）
//! - `masker` → `Masker` trait + `SimpleMasker`（姓名脱敏：≥3 保留首尾 + 中间掩码字符；2 保留首字符；通用：首尾保留）
//! - `extractor` → `Extractor` trait + `PiiExtractor`（手机/邮箱/身份证）
//!
//! 规则持久化到 DB（`rules` 表），启动时若 DB 无规则则 seed 三条内置规则。
//!
//! 设计原则（延续 02-技术设计文档 §2.1）：
//! - trait 边界隔离纯逻辑与 IO；
//! - 能力注册化——通过 `RuleRegistry` 管理规则，不硬编码到调用链。

pub mod extractor;
pub mod masker;
pub mod rules;
pub mod validator;

pub use extractor::{ExtractItem, Extractor, PiiExtractor};
pub use masker::{MaskResult, Masker, SimpleMasker};
pub use rules::{Rule, RuleKind, RuleRegistry};
pub use validator::{RegexValidator, ValidationResult, Validator};
