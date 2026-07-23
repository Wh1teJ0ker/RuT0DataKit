//! 通用脱敏算子模型：`MaskOp` 枚举 + 4 个变体 + `apply_mask_op` dispatch。
//!
//! v0.1.0 重构：把所有脱敏逻辑收敛为通用算子模型，**只保留 4 个脱敏通用
//! 算子，删除所有预置别名**（用户要求「只做规则模版」）。
//! - 脱敏：[`MaskOp::Template`] / [`MaskOp::SplitTemplate`] /
//!   [`MaskOp::RegexReplace`] / [`MaskOp::ConstReplace`]
//!
//! 每条规则的所有参数均由调用方通过 `MaskRule.params` 显式提供
//! （keep_prefix / keep_suffix / mask_char / mask_min_len / min_len / max_len /
//! cjk / pattern / replacement / match_mode / with 等）。
//! [`MaskOp::from_rule`] 仅按通用算子名 + params 构造，未知名返回 `None`。
//!
//! 对外暴露 [`apply_mask_op`] 公共函数，供前端试运行与下拉源使用。`MaskOp`
//! 自身 impl [`crate::maskers::Masker`]，可直接作为 `Box<dyn Masker>` 注入
//! pipeline。

use std::collections::HashMap;

use regex::Regex;
use serde_yml::Value;

use crate::maskers::Masker;

/// 正则替换的匹配模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchMode {
    /// 替换所有匹配子串（对应 `Regex::replace_all`）。
    All,
    /// 仅处理第一个匹配：若 `replacement == "$0"` 则提取首个匹配子串原文，
    /// 否则按 `Regex::replace` 替换首个匹配。
    First,
}

/// 通用模板脱敏算子：保留前 `keep_prefix` 字符 + 后 `keep_suffix` 字符，
/// 中间替换为 `mask_char`。
///
/// - `mask_min_len`：脱敏段至少插入多少个 `mask_char`（实际脱敏段长度 =
///   `max(原中间长度, mask_min_len)`）。
/// - `min_len` / `max_len`：值总字符数（按 `char` 计）不在
///   `[min_len, max_len]` 区间时原样返回（guard）。`None` 表示该侧不限制。
/// - `cjk`：`true` 时跳过 `keep_prefix` / `keep_suffix`，改走 NameMask/EmailMask
///   同款分支（n=0 空 / n=1 原样 / n=2 `首*` / n>=3 `首+*(n-2)+末`），用于复现
///   旧实现的 2 字不保留末位语义。`false` 时走通用 keep_prefix/keep_suffix 模板。
///
/// 与旧 `CustomMask` / `IdCardMask` / `PhoneMask` / `BankCardMask` /
/// `CustomerIdMask` / `NameMask` 行为等价。
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateOp {
    pub keep_prefix: usize,
    pub keep_suffix: usize,
    pub mask_char: char,
    pub mask_min_len: usize,
    pub min_len: Option<usize>,
    pub max_len: Option<usize>,
    /// `true` 时切换到 CJK 姓名/邮箱本地部分同款分支（见结构体 doc）。
    pub cjk: bool,
}

/// 切分模板脱敏算子：按 `separator` 把值切成多段，对 `segment_index` 段应用
/// `inner` 算子，其余段保持原样后重新拼接。
///
/// 用于 `email_mask`：`separator="@"`，`segment_index=0`（本地部分），
/// `inner=Template{keep_prefix=1, keep_suffix=1, mask_char='*', mask_min_len=1}`。
#[derive(Debug, Clone, PartialEq)]
pub struct SplitTemplateOp {
    pub separator: String,
    pub segment_index: usize,
    pub inner: Box<TemplateOp>,
}

/// 正则替换脱敏算子。
///
/// - `pattern` 缺失或非法时退化为原值返回（不 panic）。
/// - `match_mode == All`：替换所有匹配（`replace_all`）。
/// - `match_mode == First` 且 `replacement == "$0"`：返回第一个匹配子串原文
///   （等价旧 `regex_extract`）。
/// - `match_mode == First` 且 `replacement != "$0"`：替换首个匹配
///   （`replace`）。
#[derive(Debug, Clone)]
pub struct RegexReplaceOp {
    pub pattern: Option<String>,
    pub replacement: String,
    pub match_mode: MatchMode,
    // 预编译缓存：构造时尝试编译，运行时直接用。
    re: Option<Regex>,
}

impl PartialEq for RegexReplaceOp {
    fn eq(&self, other: &Self) -> bool {
        self.pattern == other.pattern
            && self.replacement == other.replacement
            && self.match_mode == other.match_mode
    }
}

/// 常量替换脱敏算子：对任意输入返回固定字符串 `with`。
///
/// `with == ""` 时等价于旧 `delete`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstReplaceOp {
    pub with: String,
}

impl RegexReplaceOp {
    /// 公开构造函数：自动尝试编译 `pattern`，非法或缺失时 `re=None`（运行时原值返回）。
    pub fn new(
        pattern: Option<String>,
        replacement: String,
        match_mode: MatchMode,
    ) -> Self {
        let re = pattern.as_ref().and_then(|p| Regex::new(p).ok());
        Self {
            pattern,
            replacement,
            match_mode,
            re,
        }
    }
}

/// 收敛后的通用脱敏算子枚举。
#[derive(Debug, Clone, PartialEq)]
pub enum MaskOp {
    Template(TemplateOp),
    SplitTemplate(SplitTemplateOp),
    RegexReplace(RegexReplaceOp),
    ConstReplace(ConstReplaceOp),
}

impl MaskOp {
    /// 从 `MaskRule` 构造 `MaskOp`。
    ///
    /// v0.1.0 重构后只识别 4 个通用算子名（无预置别名）：
    /// - `template`：keep_prefix / keep_suffix / mask_char / mask_min_len /
    ///   min_len / max_len / cjk
    /// - `split_template`：separator / segment_index + 内层 Template 全部参数
    ///   （keep_prefix / keep_suffix / mask_char / mask_min_len / min_len /
    ///   max_len / cjk，扁平化传入）
    /// - `regex_replace`：pattern / replacement / match_mode
    ///   （`match_mode` 取 `"all"` 或 `"first"`，缺省 `"all"`）
    /// - `const_replace`：with
    ///
    /// 未知名返回 `None`，由调用方决定如何报错。
    pub fn from_rule(
        name: &str,
        params: Option<&HashMap<String, Value>>,
    ) -> Option<Self> {
        let params = params.cloned().unwrap_or_default();
        Some(match name {
            "template" => MaskOp::Template(template_from_params(&params)),
            "split_template" => MaskOp::SplitTemplate(split_template_from_params(&params)),
            "regex_replace" => MaskOp::RegexReplace(regex_replace_from_params(
                &params,
                parse_match_mode(&params).unwrap_or(MatchMode::All),
            )),
            "const_replace" => MaskOp::ConstReplace(ConstReplaceOp {
                with: params
                    .get("with")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            }),
            _ => return None,
        })
    }
}

impl Masker for MaskOp {
    fn mask(&self, value: &str) -> String {
        apply_mask_op(self, value)
    }
}

/// 对 `value` 应用 [`MaskOp`]，返回脱敏后的字符串。
pub fn apply_mask_op(op: &MaskOp, value: &str) -> String {
    match op {
        MaskOp::Template(t) => apply_template(t, value),
        MaskOp::SplitTemplate(s) => apply_split_template(s, value),
        MaskOp::RegexReplace(r) => apply_regex_replace(r, value),
        MaskOp::ConstReplace(c) => c.with.clone(),
    }
}

fn apply_template(t: &TemplateOp, value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let n = chars.len();
    // guard：长度不匹配原样返回。
    if let Some(min) = t.min_len {
        if n < min {
            return value.to_string();
        }
    }
    if let Some(max) = t.max_len {
        if n > max {
            return value.to_string();
        }
    }
    // cjk 分支：复现旧 NameMask 行为（n=0 空、n=1 原样、n=2 `首*`、n>=3 `首+*(n-2)+末`）。
    if t.cjk {
        return match n {
            0 => String::new(),
            1 => value.to_string(),
            2 => format!("{}*", chars[0]),
            _ => {
                let head = chars[0];
                let tail = chars[n - 1];
                let stars = "*".repeat(n - 2);
                format!("{head}{stars}{tail}")
            }
        };
    }
    let head_end = t.keep_prefix.min(n);
    let tail_start = n.saturating_sub(t.keep_suffix);
    // 保留段重叠（含 kp+ks==n 边界：此时中间段长度为 0，仍按规格
    // 「至少插入 mask_min_len 个 mask_char，仅输出脱敏段」处理，避免
    // 错误地拼接 head + mask + 空 tail 产生形如 "李四海*" 的输出）。
    if tail_start <= head_end {
        let mask: String = std::iter::repeat(t.mask_char).take(t.mask_min_len).collect();
        return mask;
    }
    let mid_len = tail_start - head_end;
    let mask_len = mid_len.max(t.mask_min_len);
    let mask: String = std::iter::repeat(t.mask_char).take(mask_len).collect();
    let head: String = chars[..head_end].iter().collect();
    let tail: String = chars[tail_start..].iter().collect();
    format!("{head}{mask}{tail}")
}

fn apply_split_template(s: &SplitTemplateOp, value: &str) -> String {
    // 使用 rfind 以与旧 email 行为一致（多个分隔符时取最后一个）。
    let sep = s.separator.as_str();
    let idx = match value.rfind(sep) {
        Some(i) => i,
        None => return value.to_string(),
    };
    let (head, tail_with_sep) = value.split_at(idx);
    // tail_with_sep 起始即为 separator。
    let segments_before = if s.segment_index == 0 {
        // 对第一段应用 inner，其余段（含 separator 之后内容）保持原样。
        let masked = apply_template(&s.inner, head);
        return format!("{masked}{tail_with_sep}");
    } else {
        head
    };
    // segment_index > 0 时：把 head 按 separator 切分（最多 segment_index+1 段），
    // 对指定段应用 inner，其它段保持原样。
    let parts: Vec<&str> = segments_before.split(sep).collect();
    if s.segment_index >= parts.len() {
        return value.to_string();
    }
    let target = parts[s.segment_index];
    let masked_target = apply_template(&s.inner, target);
    let mut rebuilt = String::new();
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            rebuilt.push_str(sep);
        }
        if i == s.segment_index {
            rebuilt.push_str(&masked_target);
        } else {
            rebuilt.push_str(p);
        }
    }
    rebuilt.push_str(tail_with_sep);
    rebuilt
}

fn apply_regex_replace(r: &RegexReplaceOp, value: &str) -> String {
    let Some(re) = r.re.as_ref() else {
        return value.to_string();
    };
    match r.match_mode {
        MatchMode::All => re.replace_all(value, r.replacement.as_str()).into_owned(),
        MatchMode::First => {
            if r.replacement == "$0" {
                // 提取首个匹配子串原文。
                match re.find(value) {
                    Some(m) => m.as_str().to_string(),
                    None => value.to_string(),
                }
            } else {
                re.replace(value, r.replacement.as_str()).into_owned()
            }
        }
    }
}

fn template_from_params(params: &HashMap<String, Value>) -> TemplateOp {
    TemplateOp {
        keep_prefix: parse_usize(params, "keep_prefix").unwrap_or(0),
        keep_suffix: parse_usize(params, "keep_suffix").unwrap_or(0),
        mask_char: params
            .get("mask_char")
            .and_then(|v| v.as_str())
            .and_then(|s| s.chars().next())
            .unwrap_or('*'),
        mask_min_len: parse_usize(params, "mask_min_len").unwrap_or(1),
        min_len: parse_usize(params, "min_len"),
        max_len: parse_usize(params, "max_len"),
        cjk: parse_bool(params, "cjk").unwrap_or(false),
    }
}

/// 从 params 构造 `SplitTemplateOp`。
///
/// 内层 Template 的参数扁平化传入（与外层共用同一 params 表）。`separator`
/// 缺省为 `"@"`，`segment_index` 缺省为 0，便于邮箱等常见场景零配置。
fn split_template_from_params(params: &HashMap<String, Value>) -> SplitTemplateOp {
    let separator = params
        .get("separator")
        .and_then(|v| v.as_str())
        .unwrap_or("@")
        .to_string();
    let segment_index = parse_usize(params, "segment_index").unwrap_or(0);
    let inner = template_from_params(params);
    SplitTemplateOp {
        separator,
        segment_index,
        inner: Box::new(inner),
    }
}

fn regex_replace_from_params(
    params: &HashMap<String, Value>,
    match_mode: MatchMode,
) -> RegexReplaceOp {
    let pattern = params
        .get("pattern")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let replacement = params
        .get("replacement")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let re = pattern.as_ref().and_then(|p| Regex::new(p).ok());
    RegexReplaceOp {
        pattern,
        replacement,
        match_mode,
        re,
    }
}

/// 解析 `match_mode` 参数：`"all"`（缺省）或 `"first"`。
pub(super) fn parse_match_mode(params: &HashMap<String, Value>) -> Option<MatchMode> {
    match params.get("match_mode")? {
        Value::String(s) => match s.to_lowercase().as_str() {
            "all" => Some(MatchMode::All),
            "first" => Some(MatchMode::First),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn parse_usize(params: &HashMap<String, Value>, key: &str) -> Option<usize> {
    match params.get(key)? {
        Value::Number(n) => n.as_u64().map(|n| n as usize),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

pub(super) fn parse_bool(params: &HashMap<String, Value>, key: &str) -> Option<bool> {
    match params.get(key)? {
        Value::Bool(b) => Some(*b),
        Value::String(s) => match s.to_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_idcard_equivalence() {
        let op = MaskOp::Template(TemplateOp {
            keep_prefix: 6,
            keep_suffix: 4,
            mask_char: '*',
            mask_min_len: 8,
            min_len: Some(18),
            max_len: Some(18),
            cjk: false,
        });
        assert_eq!(apply_mask_op(&op, "110101199001011234"), "110101********1234");
        // 非 18 位原样返回
        assert_eq!(apply_mask_op(&op, "11010119900101"), "11010119900101");
    }

    #[test]
    fn regex_replace_all_phone() {
        let op = MaskOp::RegexReplace(RegexReplaceOp {
            pattern: Some(r"(\d{3})\d{4}(\d{4})".into()),
            replacement: "$1****$2".into(),
            match_mode: MatchMode::All,
            re: Regex::new(r"(\d{3})\d{4}(\d{4})").ok(),
        });
        assert_eq!(apply_mask_op(&op, "13812345678"), "138****5678");
    }

    #[test]
    fn regex_extract_first_match() {
        let op = MaskOp::RegexReplace(RegexReplaceOp {
            pattern: Some(r"\d{11}".into()),
            replacement: "$0".into(),
            match_mode: MatchMode::First,
            re: Regex::new(r"\d{11}").ok(),
        });
        assert_eq!(apply_mask_op(&op, "tel:13812345678"), "13812345678");
        assert_eq!(apply_mask_op(&op, "no digits"), "no digits");
    }

    #[test]
    fn split_template_email() {
        let op = MaskOp::SplitTemplate(SplitTemplateOp {
            separator: "@".into(),
            segment_index: 0,
            inner: Box::new(TemplateOp {
                keep_prefix: 1,
                keep_suffix: 1,
                mask_char: '*',
                mask_min_len: 1,
                min_len: None,
                max_len: None,
                // 用 cjk=true 分支复现旧 EmailMask 的 n=2 `首*` 行为。
                cjk: true,
            }),
        });
        assert_eq!(apply_mask_op(&op, "zhangsan@example.com"), "z******n@example.com");
        assert_eq!(apply_mask_op(&op, "zs@x.com"), "z*@x.com");
        assert_eq!(apply_mask_op(&op, "z@x.com"), "z@x.com");
        assert_eq!(apply_mask_op(&op, "notanemail"), "notanemail");
    }

    #[test]
    fn const_replace_delete_and_replace() {
        let del = MaskOp::ConstReplace(ConstReplaceOp {
            with: String::new(),
        });
        assert_eq!(apply_mask_op(&del, "anything"), "");
        let rep = MaskOp::ConstReplace(ConstReplaceOp {
            with: "REDACTED".into(),
        });
        assert_eq!(apply_mask_op(&rep, "anything"), "REDACTED");
    }

    /// v0.1.0 重构：`from_rule` 按通用算子名 + params 直接构造，不再有预置别名。
    #[test]
    fn mask_op_from_rule_template_with_params() {
        // template + 全部 params → 与旧 idcard_mask 预置等价。
        let mut params = HashMap::new();
        params.insert("keep_prefix".into(), Value::Number(6.into()));
        params.insert("keep_suffix".into(), Value::Number(4.into()));
        params.insert("mask_char".into(), Value::String("*".into()));
        params.insert("mask_min_len".into(), Value::Number(8.into()));
        params.insert("min_len".into(), Value::Number(18.into()));
        params.insert("max_len".into(), Value::Number(18.into()));
        params.insert("cjk".into(), Value::Bool(false));
        let op = MaskOp::from_rule("template", Some(&params)).expect("template op");
        assert_eq!(apply_mask_op(&op, "110101199001011234"), "110101********1234");
        assert_eq!(apply_mask_op(&op, "11010119900101"), "11010119900101");

        // 未知名返回 None
        assert!(MaskOp::from_rule("idcard_mask", None).is_none());
        assert!(MaskOp::from_rule("custom", None).is_none());
    }

    #[test]
    fn mask_op_from_rule_split_template_with_params() {
        // split_template + 参数 → 与旧 email_mask 预置等价。
        let mut params = HashMap::new();
        params.insert("separator".into(), Value::String("@".into()));
        params.insert("segment_index".into(), Value::Number(0.into()));
        params.insert("keep_prefix".into(), Value::Number(1.into()));
        params.insert("keep_suffix".into(), Value::Number(1.into()));
        params.insert("mask_char".into(), Value::String("*".into()));
        params.insert("mask_min_len".into(), Value::Number(1.into()));
        params.insert("cjk".into(), Value::Bool(true));
        let op = MaskOp::from_rule("split_template", Some(&params)).expect("split_template op");
        assert_eq!(apply_mask_op(&op, "zhangsan@example.com"), "z******n@example.com");
        assert_eq!(apply_mask_op(&op, "zs@x.com"), "z*@x.com");
        assert_eq!(apply_mask_op(&op, "notanemail"), "notanemail");
    }

    #[test]
    fn mask_op_from_rule_regex_replace_with_match_mode() {
        // match_mode=first + replacement="$0" → 提取首个匹配（等价旧 regex_extract）。
        let mut params = HashMap::new();
        params.insert("pattern".into(), Value::String(r"\d{11}".into()));
        params.insert("replacement".into(), Value::String("$0".into()));
        params.insert("match_mode".into(), Value::String("first".into()));
        let op = MaskOp::from_rule("regex_replace", Some(&params)).expect("regex_replace first");
        assert_eq!(apply_mask_op(&op, "tel:13812345678"), "13812345678");

        // match_mode=all + replacement → 替换全部。
        let mut params = HashMap::new();
        params.insert("pattern".into(), Value::String(r"(\d{3})\d{4}(\d{4})".into()));
        params.insert("replacement".into(), Value::String("$1****$2".into()));
        params.insert("match_mode".into(), Value::String("all".into()));
        let op = MaskOp::from_rule("regex_replace", Some(&params)).expect("regex_replace all");
        assert_eq!(apply_mask_op(&op, "13812345678"), "138****5678");
    }

    #[test]
    fn mask_op_from_rule_const_replace_with_empty() {
        // const_replace + with="" → 等价 delete。
        let mut params = HashMap::new();
        params.insert("with".into(), Value::String("".into()));
        let op = MaskOp::from_rule("const_replace", Some(&params)).expect("const_replace empty");
        assert_eq!(apply_mask_op(&op, "secret"), "");

        // const_replace + with="REDACTED"
        let mut params = HashMap::new();
        params.insert("with".into(), Value::String("REDACTED".into()));
        let op = MaskOp::from_rule("const_replace", Some(&params)).expect("const_replace redacted");
        assert_eq!(apply_mask_op(&op, "anything"), "REDACTED");
    }
}

#[cfg(test)]
mod bug_repro_tests {
    use super::*;

    fn t(kp: usize, ks: usize, mml: usize, cjk: bool) -> TemplateOp {
        TemplateOp {
            keep_prefix: kp,
            keep_suffix: ks,
            mask_char: '*',
            mask_min_len: mml,
            min_len: None,
            max_len: None,
            cjk,
        }
    }

    #[test]
    fn repro_lisihai_variants() {
        // 回归验证：kp+ks==n 边界（如 kp=3,ks=0,n=3）以前会输出 "李四海*"，
        // 现在应走重叠分支，输出纯脱敏段 "*"。
        for kp in 0..=3 {
            for ks in 0..=3 {
                for mml in 0..=3 {
                    for cjk in [false, true] {
                        let op = MaskOp::Template(t(kp, ks, mml, cjk));
                        let out = apply_mask_op(&op, "李四海");
                        assert_ne!(
                            out, "李四海*",
                            "kp={kp} ks={ks} mml={mml} cjk={cjk} 不应输出 \"李四海*\""
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn template_kp_plus_ks_equals_n_outputs_only_mask() {
        // kp=3, ks=0, n=3 → 中间段长度为 0，应仅输出 mask_min_len 个 *。
        let op = MaskOp::Template(t(3, 0, 1, false));
        assert_eq!(apply_mask_op(&op, "李四海"), "*");
        // kp=2, ks=1, n=3 → 同样 kp+ks==n，仅输出脱敏段。
        let op = MaskOp::Template(t(2, 1, 2, false));
        assert_eq!(apply_mask_op(&op, "abc"), "**");
        // kp=1, ks=2, n=3 → 仅输出脱敏段。
        let op = MaskOp::Template(t(1, 2, 1, false));
        assert_eq!(apply_mask_op(&op, "abc"), "*");
        // ASCII 长串：kp+ks==n 的边界（n=18, kp+ks=18, mml=8 → 仅脱敏段 8 个 *）。
        let op = MaskOp::Template(t(6, 12, 8, false));
        assert_eq!(apply_mask_op(&op, "110101199001011234"), "********");
    }
}
