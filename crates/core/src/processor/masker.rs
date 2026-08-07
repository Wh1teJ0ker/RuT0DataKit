//! 脱敏器 trait + 原型实现。
//!
//! v1.1.0：`Masker` trait + `SimpleMasker`。
//! 两种脱敏模式：
//! - **规则驱动**（`rule.kind == Mask`）：
//!   - 长度 ≥ 3：保留首尾各 1 字符，中间用「掩码字符」替换（「张三丰」→「张*丰」）。
//!   - 长度 2：保留首字符，末位用掩码字符替换（「张三」→「张*」）。
//!   - 长度 1：单个掩码字符；空串：空串。
//!   - 掩码字符取自 `rule.replacement`（首个字符；`None`/空 → 默认 `*`）。
//! - **无规则（通用脱敏）**：保留首尾各 1 字符，中间用 `*` 替换（长度 ≤ 2 全 `*`）。

use crate::error::CoreResult;
use crate::processor::rules::{Rule, RuleKind};

/// 单值脱敏结果。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskResult {
    /// 脱敏后输出值。
    pub output: String,
    /// 触发规则 ID（无规则时为 `"simple"`）。
    pub rule_id: String,
}

/// 脱敏器 trait。纯逻辑，不持有状态。
pub trait Masker {
    /// 脱敏单值。`rule.kind` 应为 `Mask` 或 `None`-语义（v1.1.0 内置脱敏不依赖规则）。
    fn mask(&self, input: &str, rule: Option<&Rule>) -> CoreResult<MaskResult>;
}

/// 原型脱敏器。
///
/// - **规则驱动**（`rule.kind == Mask`）：
///   - 长度 ≥ 3：保留首尾各 1 字符，中间用「掩码字符」替换（「张三丰」→「张*丰」）。
///   - 长度 2：保留首字符，末位用掩码字符替换（「张三」→「张*」）。
///   - 长度 1：单个掩码字符；空串：空串。
///   - 掩码字符取自 `rule.replacement` 的首个字符；`None`/空 → 默认 `*`。
/// - **无规则（通用脱敏语义）**：保留首尾各 1 字符，中间用 `*` 替换。
///   长度 ≤ 2 时全部 `*`。
pub struct SimpleMasker;

impl Masker for SimpleMasker {
    fn mask(&self, input: &str, rule: Option<&Rule>) -> CoreResult<MaskResult> {
        let chars: Vec<char> = input.chars().collect();
        let n = chars.len();

        if let Some(r) = rule {
            if r.kind == RuleKind::Mask {
                // 掩码字符：取 replacement 首个字符；None/空 → 默认 `*`。
                let mask_char = r
                    .replacement
                    .as_deref()
                    .and_then(|s| s.chars().next())
                    .unwrap_or('*');
                let mask_str = mask_char.to_string();
                let output = if n == 0 {
                    String::new()
                } else if n == 1 {
                    mask_str
                } else if n == 2 {
                    // 2 字符：保留首字符，末位用掩码字符替换。
                    format!("{}{}", chars[0], mask_str)
                } else {
                    // ≥3 字符：保留首尾各 1 字符，中间用掩码字符替换。
                    let mid = mask_str.repeat(n - 2);
                    format!("{}{}{}", chars[0], mid, chars[n - 1])
                };
                return Ok(MaskResult {
                    output,
                    rule_id: r.id.clone(),
                });
            }
            // 非 mask 规则 → 落到无规则通用逻辑（不报错，保持原型可用性）。
        }

        // 无规则通用脱敏：保留首尾各 1 字符，中间 `*` 替换。
        let output = if n == 0 {
            String::new()
        } else if n <= 2 {
            "*".repeat(n)
        } else {
            let mid = "*".repeat(n - 2);
            format!("{}{}{}", chars[0], mid, chars[n - 1])
        };
        let rule_id = rule
            .map(|r| r.id.clone())
            .unwrap_or_else(|| "simple".into());
        Ok(MaskResult { output, rule_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processor::rules::{Rule, RuleKind};

    #[test]
    fn masks_long_value_keeps_ends() {
        let m = SimpleMasker;
        let r = m.mask("13812345678", None).unwrap();
        assert_eq!(r.output, "1*********8");
        assert_eq!(r.rule_id, "simple");
    }

    #[test]
    fn masks_short_value_all_stars() {
        let m = SimpleMasker;
        assert_eq!(m.mask("ab", None).unwrap().output, "**");
        assert_eq!(m.mask("a", None).unwrap().output, "*");
    }

    #[test]
    fn masks_empty_string_returns_empty() {
        let m = SimpleMasker;
        assert_eq!(m.mask("", None).unwrap().output, "");
    }

    #[test]
    fn masks_chinese_value_keeps_ends() {
        let m = SimpleMasker;
        // "张三丰四" → "张**四"
        assert_eq!(m.mask("张三丰四", None).unwrap().output, "张**四");
    }

    #[test]
    fn rule_with_mask_char_uses_that_char() {
        // replacement 非空 → 取首个字符作为掩码字符。
        let m = SimpleMasker;
        let rule = Rule {
            id: "test-mask".into(),
            name: "测试脱敏".into(),
            kind: RuleKind::Mask,
            field: None,
            pattern: None,
            replacement: Some("#".into()),
            enabled: true,
            description: String::new(),
        };
        // "13812345678"（11 字符）→ "1" + 9 个 # + "8"
        let r = m.mask("13812345678", Some(&rule)).unwrap();
        assert_eq!(r.output, "1#########8");
        assert_eq!(r.rule_id, "test-mask");
    }

    #[test]
    fn rule_with_empty_replacement_defaults_star() {
        // replacement 空串 → 默认掩码字符 `*`。
        let m = SimpleMasker;
        let rule = Rule {
            id: "test-mask".into(),
            name: "测试脱敏".into(),
            kind: RuleKind::Mask,
            field: None,
            pattern: None,
            replacement: Some(String::new()),
            enabled: true,
            description: String::new(),
        };
        let r = m.mask("13812345678", Some(&rule)).unwrap();
        assert_eq!(r.output, "1*********8");
        assert_eq!(r.rule_id, "test-mask");
    }

    #[test]
    fn rule_without_replacement_defaults_star() {
        // replacement=None → 默认掩码字符 `*`。
        let m = SimpleMasker;
        let rule = Rule {
            id: "test-mask".into(),
            name: "测试脱敏".into(),
            kind: RuleKind::Mask,
            field: None,
            pattern: None,
            replacement: None,
            enabled: true,
            description: String::new(),
        };
        // "13812345678"（11 字符）→ "1" + 9 个 * + "8"
        let r = m.mask("13812345678", Some(&rule)).unwrap();
        assert_eq!(r.output, "1*********8");
        assert_eq!(r.rule_id, "test-mask");
    }

    #[test]
    fn name_mask_rule_keeps_surname() {
        // 姓名脱敏：≥3 字符保留首尾，中间 * 替换；2 字符保留首字符末位 *。
        let m = SimpleMasker;
        let rule = crate::processor::rules::RuleRegistry::name_mask_rule();
        assert_eq!(m.mask("张三", Some(&rule)).unwrap().output, "张*");
        assert_eq!(m.mask("张三丰", Some(&rule)).unwrap().output, "张*丰");
        assert_eq!(m.mask("欧阳修", Some(&rule)).unwrap().output, "欧*修");
        assert_eq!(m.mask("司马相如", Some(&rule)).unwrap().output, "司**如");
        assert_eq!(m.mask("赵", Some(&rule)).unwrap().output, "*");
        assert_eq!(m.mask("", Some(&rule)).unwrap().output, "");
    }
}
