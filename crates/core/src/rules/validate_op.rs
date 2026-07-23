//! 通用校验算子模型：`ValidateOp` 枚举 + 3 个变体 + `apply_validate_op` dispatch。
//!
//! v0.1.0 重构：把所有校验逻辑收敛为通用算子模型，**只保留 3 个校验通用
//! 算子，删除所有预置别名**（用户要求「只做规则模版」）。
//! - 校验：[`ValidateOp::Regex`] / [`ValidateOp::Algorithm`] /
//!   [`ValidateOp::RegexWithGuard`]
//!
//! 每条规则的所有参数均由调用方通过 `FieldRule.params` 显式提供
//! （pattern / message / empty_message / algo / guard / prefix / prefix_set 等；
//! phone 守卫的 `prefix_set` 自 v0.4.3 起被忽略，仅向后兼容）。
//! [`ValidateOp::from_rule`] 仅按通用算子名 + params 构造，未知名返回 `None`。
//!
//! 对外暴露 [`apply_validate_op`] 公共函数，供前端试运行与下拉源使用。
//! `ValidateOp` 自身 impl [`crate::validators::Validator`]，可直接作为
//! `Box<dyn Validator>` 注入 pipeline。

use regex::Regex;
use serde_yml::Value;

use crate::rules::types::FieldRule;
use crate::validators::{ValidationResult, Validator};

/// 内置算法校验种类：委托现有具体 validator struct。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgoKind {
    IdCard,
    BankCard,
}

/// RegexWithGuard 守卫种类：phone（向后兼容，仅按 `patterns::PHONE.validate` 校验）/ mac 前缀匹配。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardKind {
    /// 手机号守卫（向后兼容）。自 v0.4.3 起 PhoneValidator 已去绝对化
    /// （仅 `patterns::PHONE.validate`），`prefix_set` 参数仅为兼容旧 YAML 保留，
    /// 不再影响校验结果。
    PhonePrefix { prefix_set: String },
    /// MAC 地址前缀守卫。`prefix == None` 时不做前缀校验。
    MacPrefix { prefix: Option<String> },
}

/// 正则校验算子：按 `pattern` 整段 `is_match`，可选空值 guard 与 message。
#[derive(Debug, Clone)]
pub struct RegexOp {
    pub pattern: String,
    pub message: Option<String>,
    /// 非空 guard：值为空时返回 `empty_message`（若 `Some`）。
    pub empty_message: Option<String>,
    re: Option<Regex>,
}

impl PartialEq for RegexOp {
    fn eq(&self, other: &Self) -> bool {
        self.pattern == other.pattern
            && self.message == other.message
            && self.empty_message == other.empty_message
    }
}

impl RegexOp {
    /// 构造一个 `RegexOp`。`pattern` 非法时 `re=None`，运行时一律返回 fail。
    pub fn new(
        pattern: impl Into<String>,
        message: Option<String>,
        empty_message: Option<String>,
    ) -> Self {
        let pattern: String = pattern.into();
        let re = Regex::new(&pattern).ok();
        Self {
            pattern,
            message,
            empty_message,
            re,
        }
    }
}

/// 算法校验算子：委托内置具体 validator struct（IdCard / BankCard）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgorithmOp {
    pub algo: AlgoKind,
}

/// 守卫 + 正则校验算子：先过守卫（phone 向后兼容 / mac 前缀），再过正则。
#[derive(Debug, Clone)]
pub struct RegexWithGuardOp {
    pub guard: GuardKind,
    pub pattern: String,
    pub message: Option<String>,
    // 守卫+正则算子目前委托旧 validator struct 实现，预留 re 字段供未来纯算法实现使用。
    #[allow(dead_code)]
    re: Option<Regex>,
}

impl PartialEq for RegexWithGuardOp {
    fn eq(&self, other: &Self) -> bool {
        self.guard == other.guard
            && self.pattern == other.pattern
            && self.message == other.message
    }
}

impl RegexWithGuardOp {
    pub fn new(
        guard: GuardKind,
        pattern: impl Into<String>,
        message: Option<String>,
    ) -> Self {
        let pattern: String = pattern.into();
        let re = Regex::new(&pattern).ok();
        Self {
            guard,
            pattern,
            message,
            re,
        }
    }
}

/// 收敛后的通用校验算子枚举。
#[derive(Debug, Clone, PartialEq)]
pub enum ValidateOp {
    Regex(RegexOp),
    Algorithm(AlgorithmOp),
    RegexWithGuard(RegexWithGuardOp),
}

impl ValidateOp {
    /// 从 `FieldRule` 构造 `ValidateOp`。
    ///
    /// v0.4.4 重构：按 `rule.scope`（数据类型）匹配通用算子名，不再读
    /// `rule.regex`（字段已删除）。识别 3 个通用算子名：
    /// - `regex`：pattern / message / empty_message（空值 guard，可空）。
    /// - `algorithm`：`algo` 参数取 `"idcard"` / `"bankcard"`（不区分大小写），
    ///   缺省 `"idcard"`。
    /// - `regex_with_guard`：`guard` 参数取 `"phone"` / `"mac"`（不区分大小写），
    ///   缺省 `"phone"`；其余参数 `pattern` / `prefix` / `message` 从 params /
    ///   rule 同名字段读取。`prefix_set`（phone 守卫，v0.4.3 起被忽略，
    ///   仅向后兼容）仍可出现在 params 中但不再影响校验结果。
    ///
    /// 未知名返回 `None`，由调用方决定如何报错（通常回退到 ValidatorRegistry）。
    pub fn from_rule(rule: &FieldRule) -> Option<Self> {
        // v0.4.4：按 rule.scope 名匹配通用算子（取代旧 rule.validator）。
        let params = rule.params.clone().unwrap_or_default();
        Some(match rule.scope.as_str() {
            "regex" => {
                let pattern = params
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let empty_message = params
                    .get("empty_message")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                ValidateOp::Regex(RegexOp::new(
                    pattern,
                    rule.message.clone().or_else(|| {
                        params
                            .get("message")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    }),
                    empty_message,
                ))
            }
            "algorithm" => {
                let algo = params
                    .get("algo")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_lowercase())
                    .filter(|s| s == "bankcard")
                    .map(|_| AlgoKind::BankCard)
                    .unwrap_or(AlgoKind::IdCard);
                ValidateOp::Algorithm(AlgorithmOp { algo })
            }
            "regex_with_guard" => {
                let guard_name = params
                    .get("guard")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_else(|| "phone".to_string());
                let pattern = params
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let guard = match guard_name.as_str() {
                    "mac" => {
                        let prefix = params
                            .get("prefix")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        GuardKind::MacPrefix { prefix }
                    }
                    _ => {
                        // 缺省 / "phone"：自 v0.4.3 起 PhoneValidator 已去绝对化，
                        // prefix_set 仅为兼容旧 YAML 保留，运行时被忽略。
                        let prefix_set = params
                            .get("prefix_set")
                            .and_then(|v| v.as_str())
                            .unwrap_or("real")
                            .to_string();
                        GuardKind::PhonePrefix { prefix_set }
                    }
                };
                ValidateOp::RegexWithGuard(RegexWithGuardOp::new(
                    guard,
                    pattern,
                    rule.message.clone().or_else(|| {
                        params
                            .get("message")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    }),
                ))
            }
            _ => return None,
        })
    }
}

impl Validator for ValidateOp {
    fn validate(&self, value: &str) -> ValidationResult {
        apply_validate_op(self, value)
    }
}

/// 对 `value` 应用 [`ValidateOp`]，返回校验结果。
pub fn apply_validate_op(op: &ValidateOp, value: &str) -> ValidationResult {
    match op {
        ValidateOp::Regex(r) => apply_regex(r, value),
        ValidateOp::Algorithm(a) => apply_algorithm(a, value),
        ValidateOp::RegexWithGuard(g) => apply_regex_with_guard(g, value),
    }
}

fn apply_regex(r: &RegexOp, value: &str) -> ValidationResult {
    let v = value.trim();
    if let Some(msg) = &r.empty_message {
        if v.is_empty() {
            return ValidationResult::fail(msg.clone());
        }
    }
    let Some(re) = r.re.as_ref() else {
        return ValidationResult::fail(
            r.message
                .clone()
                .unwrap_or_else(|| "regex does not match".to_string()),
        );
    };
    if re.is_match(v) {
        ValidationResult::ok()
    } else {
        ValidationResult::fail(
            r.message
                .clone()
                .unwrap_or_else(|| "regex does not match".to_string()),
        )
    }
}

fn apply_algorithm(a: &AlgorithmOp, value: &str) -> ValidationResult {
    use std::collections::HashMap;
    match a.algo {
        AlgoKind::IdCard => crate::validators::idcard::IdCardValidator::new(HashMap::new())
            .validate(value),
        AlgoKind::BankCard => {
            crate::validators::bankcard::BankCardValidator::new(HashMap::new()).validate(value)
        }
    }
}

fn apply_regex_with_guard(g: &RegexWithGuardOp, value: &str) -> ValidationResult {
    match &g.guard {
        GuardKind::PhonePrefix { prefix_set: _ } => {
            use std::collections::HashMap;
            // 委托 PhoneValidator：自 v0.4.3 起仅按 patterns::PHONE.validate 校验，
            // prefix_set 参数被忽略（向后兼容旧 YAML）。
            crate::validators::phone::PhoneValidator::new(HashMap::new()).validate(value)
        }
        GuardKind::MacPrefix { prefix } => {
            use std::collections::HashMap;
            let mut params = HashMap::new();
            if let Some(p) = prefix {
                params.insert("prefix".to_string(), Value::String(p.clone()));
            }
            crate::validators::mac::MacValidator::new(params).validate(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn validate_regex_preset_email() {
        let op = ValidateOp::Regex(RegexOp::new(
            r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$",
            Some("email format invalid".into()),
            None,
        ));
        assert!(apply_validate_op(&op, "zhangsan@example.com").valid);
        assert!(!apply_validate_op(&op, "invalid").valid);
    }

    #[test]
    fn validate_algorithm_idcard() {
        let op = ValidateOp::Algorithm(AlgorithmOp {
            algo: AlgoKind::IdCard,
        });
        assert!(apply_validate_op(&op, "286071197501111126").valid);
        assert!(!apply_validate_op(&op, "801615200409127668").valid);
    }

    #[test]
    fn validate_op_from_rule_regex_with_params() {
        // regex 算子从 params.pattern 构造。
        let mut params = HashMap::new();
        params.insert(
            "pattern".into(),
            Value::String(r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$".into()),
        );
        params.insert("message".into(), Value::String("email format invalid".into()));
        let rule = FieldRule {
            field: "x".into(),
            scope: "regex".into(), tag: "validate".into(),
            params: Some(params),
            message: None,
            description: None,        };
        let op = ValidateOp::from_rule(&rule).expect("regex op");
        assert!(matches!(op, ValidateOp::Regex(_)));
        assert!(apply_validate_op(&op, "zhangsan@example.com").valid);
        assert!(!apply_validate_op(&op, "invalid").valid);
    }

    #[test]
    fn validate_op_from_rule_algorithm_with_algo_param() {
        // algorithm 算子从 params.algo 构造。
        for (algo_str, kind) in [
            ("idcard", AlgoKind::IdCard),
            ("bankcard", AlgoKind::BankCard),
            ("IDCARD", AlgoKind::IdCard),
            ("BankCard", AlgoKind::BankCard),
        ] {
            let mut params = HashMap::new();
            params.insert("algo".into(), Value::String(algo_str.into()));
            let rule = FieldRule {
                field: "x".into(),
                scope: "algorithm".into(), tag: "validate".into(),
                params: Some(params),
                message: None,
                description: None,            };
            let op = ValidateOp::from_rule(&rule).expect("algorithm op");
            match &op {
                ValidateOp::Algorithm(a) => assert_eq!(a.algo, kind, "algo {algo_str}"),
                _ => panic!("expected Algorithm"),
            }
        }

        // algo 缺省 → IdCard
        let rule = FieldRule {
            field: "x".into(),
            scope: "algorithm".into(), tag: "validate".into(),
            params: None,
            message: None,
            description: None,        };
        let op = ValidateOp::from_rule(&rule).expect("algorithm default");
        assert!(matches!(op, ValidateOp::Algorithm(AlgorithmOp { algo: AlgoKind::IdCard })));

        // 未知名返回 None
        let rule = FieldRule {
            field: "x".into(),
            scope: "idcard".into(), tag: "validate".into(),
            params: None,
            message: None,
            description: None,        };
        assert!(ValidateOp::from_rule(&rule).is_none());
    }

    #[test]
    fn validate_op_from_rule_regex_with_guard_params() {
        // guard=phone
        let mut params = HashMap::new();
        params.insert("guard".into(), Value::String("phone".into()));
        params.insert("pattern".into(), Value::String(r"^\d{11}$".into()));
        params.insert("prefix_set".into(), Value::String("real".into()));
        let rule = FieldRule {
            field: "x".into(),
            scope: "regex_with_guard".into(), tag: "validate".into(),
            params: Some(params),
            message: None,
            description: None,        };
        let op = ValidateOp::from_rule(&rule).expect("guard phone op");
        assert!(matches!(op, ValidateOp::RegexWithGuard(_)));
        assert!(apply_validate_op(&op, "13812345678").valid);
        assert!(!apply_validate_op(&op, "12345").valid);

        // guard=mac + prefix
        let mut params = HashMap::new();
        params.insert("guard".into(), Value::String("mac".into()));
        params.insert("pattern".into(), Value::String(r"^([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$".into()));
        params.insert("prefix".into(), Value::String("AA:BB".into()));
        let rule = FieldRule {
            field: "x".into(),
            scope: "regex_with_guard".into(), tag: "validate".into(),
            params: Some(params),
            message: None,
            description: None,        };
        let op = ValidateOp::from_rule(&rule).expect("guard mac op");
        assert!(matches!(op, ValidateOp::RegexWithGuard(_)));
        assert!(apply_validate_op(&op, "AA:BB:CC:DD:EE:FF").valid);
        assert!(!apply_validate_op(&op, "notamac").valid);
    }
}
