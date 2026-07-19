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
    fn regex_constants_still_available() {
        // 常用正则常量保留导出，便于用户在 regex 算子 params.pattern 中引用。
        assert!(EMAIL_REGEX.contains("@"));
        assert!(PHONE_REGEX.starts_with(r"^\d{11}$"));
        assert!(MAC_REGEX.contains("A-Fa-f"));
        assert!(USERNAME_REGEX.chars().any(|c| c == '+'));
        assert!(NAME_REGEX.contains("4e00"));
    }
}
