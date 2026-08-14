//! 脱敏器 trait + 原型实现。
//!
//! v1.1.0：`Masker` trait + `SimpleMasker`。
//! v1.1.3：`SimpleMasker` 扩展通用模板分支（`rule.template`）。
//! v1.1.3 T49：空模板（`TemplateParams` 所有字段 `None`）→ 不脱敏（透传，
//! 原样返回），承载 `simple-mask` / `segment-mask` 规则的"未选预设"语义。
//! v1.1.3 T54：原 `general-mask` 拆为 `simple-mask`（整段脱敏，Simple 模板）+
//! `segment-mask`（分段脱敏，Segment 模板），dispatch 按 template 变体分流
//! （不看 rule.id）。
//! v1.1.3 T53：`reverse=true` 反向脱敏——掩码首尾、保留中间。
//!
//! 四种脱敏模式：
//! - **模板驱动**（`rule.kind == Mask` + `rule.template.is_some()` 且**非空模板**）：
//!   保留前 `keep_prefix` 字符 + 后 `keep_suffix` 字符，中间替换为 `mask_char`
//!   （至少 `mask_min_len` 个）。对齐 v0.8.0 `TemplateOp`。前端选预设（身份证 /
//!   手机 / 出生日期 / 银行卡）填充 `simple-mask` 的 `template` 后走此分支。
//!   T53 `reverse=true`：反向脱敏——掩码首 `keep_prefix` + 尾 `keep_suffix` 字符，
//!   中间原样保留（如 `13812345678` + kp=3,ks=4 → `***1234****`）。
//! - **空模板透传**（`rule.kind == Mask` + `rule.template` 为**空模板**，
//!   即 `TemplateParams::is_empty()`）：T49 新增。原样返回，不脱敏。
//!   承载 `simple-mask` / `segment-mask` 规则"未选预设 → 不脱敏"的语义。
//! - **规则驱动（无模板，v1.1.0 语义）**（`rule.kind == Mask` + `rule.template.is_none()`）：
//!   - 长度 ≥ 3：保留首尾各 1 字符，中间用「掩码字符」替换（「张三丰」→「张*丰」）。
//!   - 长度 2：保留首字符，末位用掩码字符替换（「张三」→「张*」）。
//!   - 长度 1：单个掩码字符；空串：空串。
//!   - 掩码字符取自 `rule.replacement`（首个字符；`None`/空 → 默认 `*`）。
//! - **无规则（通用脱敏）**：保留首尾各 1 字符，中间用 `*` 替换（长度 ≤ 2 全 `*`）。
//!
//! 掩码字符优先级：`replacement`（临时覆盖） > `template.mask_char` > 默认 `*`。

use crate::error::CoreResult;
use crate::processor::rules::{
    Rule, RuleKind, SegmentMask, SegmentTemplate, SimpleTemplate, TemplateParams,
};

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
/// - **模板驱动**（`rule.kind == Mask` + `rule.template` 非空，v1.1.3）：
///   保留前 `keep_prefix` + 后 `keep_suffix` 字符，中间替换为 `mask_char`（至少
///   `mask_min_len` 个）。对齐 v0.8.0 `TemplateOp`。前端选预设填充
///   `simple-mask` 的 `template` 后走此分支。
/// - **空模板透传**（`rule.template` 为空模板，T49）：原样返回，不脱敏。
/// - **规则驱动（无模板，v1.1.0）**（`rule.kind == Mask` + `rule.template.is_none()`）：
///   - 长度 ≥ 3：保留首尾各 1 字符，中间用「掩码字符」替换（「张三丰」→「张*丰」）。
///   - 长度 2：保留首字符，末位用掩码字符替换（「张三」→「张*」）。
///   - 长度 1：单个掩码字符；空串：空串。
///   - 掩码字符取自 `rule.replacement` 的首个字符；`None`/空 → 默认 `*`。
/// - **无规则（通用脱敏语义）**：保留首尾各 1 字符，中间用 `*` 替换。
///   长度 ≤ 2 时全部 `*`。
///
/// 掩码字符优先级：`replacement`（临时覆盖） > `template.mask_char` > 默认 `*`。
pub struct SimpleMasker;

impl Masker for SimpleMasker {
    fn mask(&self, input: &str, rule: Option<&Rule>) -> CoreResult<MaskResult> {
        let chars: Vec<char> = input.chars().collect();
        let n = chars.len();

        if let Some(r) = rule {
            if r.kind == RuleKind::Mask {
                // 掩码字符优先级：replacement（临时覆盖）> template.mask_char > 默认 `*`。
                let mask_char = r
                    .replacement
                    .as_deref()
                    .and_then(|s| s.chars().next())
                    .or_else(|| r.template.as_ref().and_then(|t| t.mask_char()))
                    .unwrap_or('*');

                // v1.1.3 通用模板分支：rule.template 非空时走模板脱敏。
                if let Some(tpl) = &r.template {
                    // T49：空模板（所有字段 None / Segment 段空）→ 不脱敏（透传）。
                    if tpl.is_empty() {
                        return Ok(MaskResult {
                            output: input.to_string(),
                            rule_id: r.id.clone(),
                        });
                    }
                    return Ok(match tpl {
                        TemplateParams::Simple(s) => apply_template(s, &chars, n, mask_char, &r.id),
                        TemplateParams::Segment(s) => {
                            apply_segment_template(s, input, mask_char, &r.id)
                        }
                    });
                }

                // v1.1.0 旧逻辑（无模板）：保留首尾各 1，中间用掩码字符替换。
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

/// 应用通用模板脱敏（v1.1.3）。保留前 `keep_prefix` + 后 `keep_suffix` 字符，
/// 中间替换为 `mask_char`（至少 `mask_min_len` 个）。
///
/// - 保留段重叠（`keep_prefix + keep_suffix >= n`）→ 仅输出 `mask_min_len` 个 `mask_char`。
/// - T53 `reverse=true`：反向脱敏——掩码首 `keep_prefix` + 尾 `keep_suffix` 字符，
///   中间原样保留。`keep_prefix`/`keep_suffix` 语义变为「首尾脱码位数」。
///   重叠（`kp+ks >= n`）→ 整段脱敏，输出 `max(n, mask_min_len)` 个 `mask_char`。
///
/// 对齐 v0.8.0 `apply_template`（仅去掉 cjk 分支——本实现用「无模板旧逻辑」承载姓名脱敏）。
fn apply_template(
    tpl: &SimpleTemplate,
    chars: &[char],
    n: usize,
    mask_char: char,
    rule_id: &str,
) -> MaskResult {
    let kp = tpl.keep_prefix.unwrap_or(0);
    let ks = tpl.keep_suffix.unwrap_or(0);
    let mml = tpl.mask_min_len.unwrap_or(1);

    // T53：reverse=true → 反向脱敏（掩码首尾，保留中间）
    if tpl.reverse.unwrap_or(false) {
        if n == 0 {
            return MaskResult {
                output: String::new(),
                rule_id: rule_id.to_string(),
            };
        }
        let head_mask_end = kp.min(n);
        let tail_mask_start = n.saturating_sub(ks);
        if tail_mask_start <= head_mask_end {
            // kp+ks >= n → 整段脱敏，至少 mml 个 mask_char
            let mask_len = n.max(mml);
            return MaskResult {
                output: mask_char.to_string().repeat(mask_len),
                rule_id: rule_id.to_string(),
            };
        }
        let head_mask = mask_char.to_string().repeat(head_mask_end);
        let middle: String = chars[head_mask_end..tail_mask_start].iter().collect();
        let tail_mask = mask_char.to_string().repeat(n - tail_mask_start);
        return MaskResult {
            output: format!("{head_mask}{middle}{tail_mask}"),
            rule_id: rule_id.to_string(),
        };
    }

    // 正向：保留首尾，掩码中间
    let head_end = kp.min(n);
    let tail_start = n.saturating_sub(ks);
    // 保留段重叠（含 kp+ks==n 边界：中间段长度为 0）→ 仅输出 mask_min_len 个 mask_char。
    if tail_start <= head_end {
        let mask = mask_char.to_string().repeat(mml);
        return MaskResult {
            output: mask,
            rule_id: rule_id.to_string(),
        };
    }
    let mid_len = tail_start - head_end;
    let mask_len = mid_len.max(mml);
    let head: String = chars[..head_end].iter().collect();
    let tail: String = chars[tail_start..].iter().collect();
    let mask = mask_char.to_string().repeat(mask_len);
    MaskResult {
        output: format!("{head}{mask}{tail}"),
        rule_id: rule_id.to_string(),
    }
}

/// 应用分段脱敏模板（v1.1.3 T52）。按 `delimiter` 拆分输入值，
/// 对 `segments` 中列出的段（按 0-based `index`）做保留首尾脱敏，其余段原样保留。
///
/// - `delimiter` 为空 或 `segments` 为空 → 透传（原样返回）。
/// - 段 `index` 超出拆分后的段数 → 该配置被忽略（不影响其他段）。
/// - 单段脱敏复用 `apply_segment_part`（保留前 `keep_prefix` + 后 `keep_suffix` 字符，
///   中间替换为 `mask_char`，至少 `mask_min_len` 个）。
fn apply_segment_template(
    tpl: &SegmentTemplate,
    input: &str,
    mask_char: char,
    rule_id: &str,
) -> MaskResult {
    if tpl.delimiter.is_empty() || tpl.segments.is_empty() {
        return MaskResult {
            output: input.to_string(),
            rule_id: rule_id.to_string(),
        };
    }
    let parts: Vec<&str> = input.split(&tpl.delimiter).collect();
    let out: Vec<String> = parts
        .iter()
        .enumerate()
        .map(|(i, part)| {
            if let Some(cfg) = tpl.segments.iter().find(|c| c.index == i) {
                apply_segment_part(part, cfg, mask_char)
            } else {
                part.to_string()
            }
        })
        .collect();
    MaskResult {
        output: out.join(&tpl.delimiter),
        rule_id: rule_id.to_string(),
    }
}

/// 对单段值应用保留首尾脱敏（v1.1.3 T52，`apply_segment_template` 的单段辅助）。
///
/// 与 `apply_template` 的保留段逻辑一致，但**不带 min/max guard**
/// （guard 是整段模板的语义，分段模板对每段独立脱敏）。保留段重叠时仅输出
/// `mask_min_len` 个 `mask_char`。
fn apply_segment_part(part: &str, cfg: &SegmentMask, mask_char: char) -> String {
    let chars: Vec<char> = part.chars().collect();
    let n = chars.len();
    let kp = cfg.keep_prefix.unwrap_or(0);
    let ks = cfg.keep_suffix.unwrap_or(0);
    let mml = cfg.mask_min_len.unwrap_or(1);
    if n == 0 {
        return mask_char.to_string().repeat(mml);
    }
    let head_end = kp.min(n);
    let tail_start = n.saturating_sub(ks);
    if tail_start <= head_end {
        return mask_char.to_string().repeat(mml);
    }
    let mid_len = tail_start - head_end;
    let mask_len = mid_len.max(mml);
    let head: String = chars[..head_end].iter().collect();
    let tail: String = chars[tail_start..].iter().collect();
    let mask = mask_char.to_string().repeat(mask_len);
    format!("{head}{mask}{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processor::rules::{
        bankcard_preset, birthdate_preset, idcard_preset, phone_preset, Rule, RuleKind,
        RuleRegistry, SegmentTemplate, SimpleTemplate, TemplateParams,
    };

    /// 构造 simple-mask 规则 + 指定模板（T54：整段脱敏规则持 Simple 模板）。
    fn simple_mask_with_template(tpl: TemplateParams) -> Rule {
        let mut r = RuleRegistry::simple_mask_rule();
        r.template = Some(tpl);
        r
    }

    /// 构造 segment-mask 规则 + 指定模板（T54：分段脱敏规则持 Segment 模板）。
    fn segment_mask_with_template(tpl: TemplateParams) -> Rule {
        let mut r = RuleRegistry::segment_mask_rule();
        r.template = Some(tpl);
        r
    }

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
            template: None,
            params: None,
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
            template: None,
            params: None,
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
            template: None,
            params: None,
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

    // ---- v1.1.3 通用模板脱敏测试（T49 预设 + T54 拆分 simple-mask/segment-mask）----

    #[test]
    fn template_empty_passthrough() {
        // T49：simple-mask 持空 Simple 模板 → 不脱敏（透传）。
        let m = SimpleMasker;
        let rule = RuleRegistry::simple_mask_rule();
        assert_eq!(
            m.mask("110101199001011234", Some(&rule)).unwrap().output,
            "110101199001011234"
        );
        assert_eq!(
            m.mask("13812345678", Some(&rule)).unwrap().output,
            "13812345678"
        );
        assert_eq!(m.mask("", Some(&rule)).unwrap().output, "");
        assert_eq!(m.mask("张三丰", Some(&rule)).unwrap().output, "张三丰");
    }

    #[test]
    fn template_idcard_keeps_6_4() {
        // 18 位身份证 → 保留前 6 + 后 4，中间 8 位 *（idcard 预设）
        let m = SimpleMasker;
        let rule = simple_mask_with_template(idcard_preset());
        assert_eq!(
            m.mask("110101199001011234", Some(&rule)).unwrap().output,
            "110101********1234"
        );
    }

    #[test]
    fn template_idcard_short_passthrough() {
        // 15 位身份证 → 不在 [18,18] guard 区间，原样返回
        let m = SimpleMasker;
        let rule = simple_mask_with_template(idcard_preset());
        assert_eq!(
            m.mask("110101900101123", Some(&rule)).unwrap().output,
            "110101900101123"
        );
    }

    #[test]
    fn template_phone_keeps_3_4() {
        // 11 位手机号 → 保留前 3 + 后 4，中间 4 位 *（phone 预设）
        let m = SimpleMasker;
        let rule = simple_mask_with_template(phone_preset());
        assert_eq!(
            m.mask("13812345678", Some(&rule)).unwrap().output,
            "138****5678"
        );
    }

    #[test]
    fn template_phone_short_passthrough() {
        // 10 位手机号 → 不在 [11,11] guard 区间，原样返回
        let m = SimpleMasker;
        let rule = simple_mask_with_template(phone_preset());
        assert_eq!(
            m.mask("1381234567", Some(&rule)).unwrap().output,
            "1381234567"
        );
    }

    #[test]
    fn template_birthdate_keeps_year_month() {
        // YYYY-MM-DD（10 字符）→ 保留前 8（YYYY-MM-），后 2 位 *（birthdate 预设）
        let m = SimpleMasker;
        let rule = simple_mask_with_template(birthdate_preset());
        assert_eq!(
            m.mask("1990-01-15", Some(&rule)).unwrap().output,
            "1990-01-**"
        );
        assert_eq!(
            m.mask("1985-12-03", Some(&rule)).unwrap().output,
            "1985-12-**"
        );
    }

    #[test]
    fn template_birthdate_short_passthrough() {
        // 非 10 字符 → 原样返回
        let m = SimpleMasker;
        let rule = simple_mask_with_template(birthdate_preset());
        assert_eq!(m.mask("1990-01", Some(&rule)).unwrap().output, "1990-01");
    }

    #[test]
    fn template_bankcard_keeps_4_4_19_digits() {
        // 19 位银行卡 → 保留前 4 + 后 4，中间 11 位 *（bankcard 预设，无长度 guard）
        let m = SimpleMasker;
        let rule = simple_mask_with_template(bankcard_preset());
        assert_eq!(
            m.mask("6222021234567890123", Some(&rule)).unwrap().output,
            "6222***********0123"
        );
    }

    #[test]
    fn template_bankcard_keeps_4_4_16_digits() {
        // 16 位银行卡 → 保留前 4 + 后 4，中间 8 位 *
        let m = SimpleMasker;
        let rule = simple_mask_with_template(bankcard_preset());
        assert_eq!(
            m.mask("4367421234567890", Some(&rule)).unwrap().output,
            "4367********7890"
        );
    }

    #[test]
    fn template_replacement_overrides_mask_char() {
        // idcard 预设 + replacement="#" → 掩码字符临时覆盖为 #
        let m = SimpleMasker;
        let mut rule = simple_mask_with_template(idcard_preset());
        rule.replacement = Some("#".into());
        assert_eq!(
            m.mask("110101199001011234", Some(&rule)).unwrap().output,
            "110101########1234"
        );
    }

    #[test]
    fn template_mask_char_in_preset_used() {
        // 预设内 mask_char 优先于默认 *：idcard_preset().with_mask_char('#')
        let m = SimpleMasker;
        let mut rule = RuleRegistry::simple_mask_rule();
        rule.template = Some(idcard_preset().with_mask_char('#'));
        assert_eq!(
            m.mask("110101199001011234", Some(&rule)).unwrap().output,
            "110101########1234"
        );
    }

    #[test]
    fn name_mask_no_template_unchanged() {
        // name-mask 无 template → 仍走旧逻辑（保留首尾各 1），向后兼容
        let m = SimpleMasker;
        let rule = crate::processor::rules::RuleRegistry::name_mask_rule();
        assert!(rule.template.is_none());
        assert_eq!(m.mask("张三丰", Some(&rule)).unwrap().output, "张*丰");
        assert_eq!(
            m.mask("13812345678", Some(&rule)).unwrap().output,
            "1*********8"
        );
    }

    // ---- v1.1.3 T52 分段脱敏测试 ----

    #[test]
    fn segment_email_mask() {
        // @ 分隔，第 0 段保留首尾各 1，中间用 * 替换（mml=1，mask_len=mid_len）。
        // "zhangsan"（8 字符）→ 保留首尾各 1，中间 6 字符 → z******n@example.com
        let m = SimpleMasker;
        let rule = segment_mask_with_template(TemplateParams::Segment(
            SegmentTemplate::new("@").with_segment(0, 1, 1, 1),
        ));
        assert_eq!(
            m.mask("zhangsan@example.com", Some(&rule)).unwrap().output,
            "z******n@example.com"
        );
    }

    #[test]
    fn segment_email_short_local_part() {
        // user@qq.com → @ 前段 "user"（4 字符）保留首尾各 1，中间 2 字符 → u**r@qq.com
        let m = SimpleMasker;
        let rule = segment_mask_with_template(TemplateParams::Segment(
            SegmentTemplate::new("@").with_segment(0, 1, 1, 1),
        ));
        assert_eq!(
            m.mask("user@qq.com", Some(&rule)).unwrap().output,
            "u**r@qq.com"
        );
    }

    #[test]
    fn segment_ip_mask() {
        // 192.168.11.1 → 按点分隔，第 2 段 "11" 整段脱敏（kp=0,ks=0,mml=2）→ 192.168.**.1
        let m = SimpleMasker;
        let rule = segment_mask_with_template(TemplateParams::Segment(
            SegmentTemplate::new(".").with_segment(2, 0, 0, 2),
        ));
        assert_eq!(
            m.mask("192.168.11.1", Some(&rule)).unwrap().output,
            "192.168.**.1"
        );
    }

    #[test]
    fn segment_multiple_segments_masked() {
        // 192.168.11.1 → 同时脱敏第 2、3 段（"11"→"**"、"1"→"**"）
        let m = SimpleMasker;
        let rule = segment_mask_with_template(TemplateParams::Segment(
            SegmentTemplate::new(".")
                .with_segment(2, 0, 0, 2)
                .with_segment(3, 0, 0, 2),
        ));
        assert_eq!(
            m.mask("192.168.11.1", Some(&rule)).unwrap().output,
            "192.168.**.**"
        );
    }

    #[test]
    fn segment_empty_delimiter_passthrough() {
        // delimiter 空 → 透传
        let m = SimpleMasker;
        let rule = segment_mask_with_template(TemplateParams::Segment(
            SegmentTemplate::new("").with_segment(0, 1, 1, 4),
        ));
        assert_eq!(
            m.mask("zhangsan@example.com", Some(&rule)).unwrap().output,
            "zhangsan@example.com"
        );
    }

    #[test]
    fn segment_no_segments_passthrough() {
        // segments 空 → 透传
        let m = SimpleMasker;
        let rule = segment_mask_with_template(TemplateParams::Segment(SegmentTemplate::new("@")));
        assert_eq!(
            m.mask("zhangsan@example.com", Some(&rule)).unwrap().output,
            "zhangsan@example.com"
        );
    }

    #[test]
    fn segment_unknown_index_untouched() {
        // 段索引超出范围 → 该段不动，其他段正常脱敏
        let m = SimpleMasker;
        let rule = segment_mask_with_template(TemplateParams::Segment(
            SegmentTemplate::new("@").with_segment(5, 1, 1, 4),
        ));
        // index=5 不存在 → 原样返回
        assert_eq!(
            m.mask("zhangsan@example.com", Some(&rule)).unwrap().output,
            "zhangsan@example.com"
        );
    }

    // ---- v1.1.3 T53 反向脱敏测试（reverse=true：掩码首尾，保留中间）----

    #[test]
    fn template_reverse_mask_head_tail() {
        // reverse=true, kp=3, ks=4 → 13812345678（11位）→ ***1234****
        let m = SimpleMasker;
        let rule = simple_mask_with_template(TemplateParams::Simple(
            SimpleTemplate::new(3, 4, 1).with_reverse(true),
        ));
        assert_eq!(
            m.mask("13812345678", Some(&rule)).unwrap().output,
            "***1234****"
        );
    }

    #[test]
    fn template_reverse_keeps_middle() {
        // reverse=true, kp=2, ks=2 → abcdef（6位）→ **cd**
        let m = SimpleMasker;
        let rule = simple_mask_with_template(TemplateParams::Simple(
            SimpleTemplate::new(2, 2, 1).with_reverse(true),
        ));
        assert_eq!(m.mask("abcdef", Some(&rule)).unwrap().output, "**cd**");
    }

    #[test]
    fn template_reverse_full_mask_on_overlap() {
        // reverse=true, kp=10, ks=10, n=11 → kp+ks>=n → 整段脱敏 11 个 *
        let m = SimpleMasker;
        let rule = simple_mask_with_template(TemplateParams::Simple(
            SimpleTemplate::new(10, 10, 1).with_reverse(true),
        ));
        assert_eq!(
            m.mask("13812345678", Some(&rule)).unwrap().output,
            "***********"
        );
    }

    #[test]
    fn template_reverse_with_mask_char() {
        // reverse=true + mask_char='#' → 用 # 脱码首尾
        let m = SimpleMasker;
        let rule = simple_mask_with_template(TemplateParams::Simple(
            SimpleTemplate::new(3, 4, 1)
                .with_reverse(true)
                .with_mask_char('#'),
        ));
        assert_eq!(
            m.mask("13812345678", Some(&rule)).unwrap().output,
            "###1234####"
        );
    }

    #[test]
    fn template_reverse_masks_short_input_without_guard() {
        // reverse=true（无长度 guard）→ 15 位输入仍被反向脱敏
        // kp=3, ks=4 → head=***, middle=10190010, tail=****
        let m = SimpleMasker;
        let rule = simple_mask_with_template(TemplateParams::Simple(
            SimpleTemplate::new(3, 4, 1).with_reverse(true),
        ));
        assert_eq!(
            m.mask("110101900101123", Some(&rule)).unwrap().output,
            "***10190010****"
        );
    }

    #[test]
    fn template_reverse_default_is_forward() {
        // reverse=None → 走正向（默认不反转）：kp=3,ks=4 → 138****5678
        let m = SimpleMasker;
        let rule = simple_mask_with_template(TemplateParams::Simple(SimpleTemplate::new(3, 4, 4)));
        let s = match rule.template.as_ref().unwrap() {
            TemplateParams::Simple(s) => s,
            _ => panic!(),
        };
        assert_eq!(s.reverse, None);
        assert_eq!(
            m.mask("13812345678", Some(&rule)).unwrap().output,
            "138****5678"
        );
    }

    #[test]
    fn segment_mask_char_from_template() {
        // 模板 mask_char=# 优先于默认 *（mml=1 → mask_len=mid_len=6）
        let m = SimpleMasker;
        let rule = segment_mask_with_template(TemplateParams::Segment(
            SegmentTemplate::new("@")
                .with_segment(0, 1, 1, 1)
                .with_mask_char('#'),
        ));
        assert_eq!(
            m.mask("zhangsan@example.com", Some(&rule)).unwrap().output,
            "z######n@example.com"
        );
    }

    #[test]
    fn segment_replacement_overrides_mask_char() {
        // replacement=# 优先于 template.mask_char
        let m = SimpleMasker;
        let mut rule = RuleRegistry::segment_mask_rule();
        rule.template = Some(TemplateParams::Segment(
            SegmentTemplate::new("@")
                .with_segment(0, 1, 1, 1)
                .with_mask_char('*'),
        ));
        rule.replacement = Some("#".into());
        assert_eq!(
            m.mask("zhangsan@example.com", Some(&rule)).unwrap().output,
            "z######n@example.com"
        );
    }
}
