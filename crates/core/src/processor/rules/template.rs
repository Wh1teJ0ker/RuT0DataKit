//! 通用模板脱敏参数（v1.1.3 新增，T52 改为 untagged enum）。
//!
//! 两种变体：
//! - `Simple`：整段脱敏（原 v1.1.3 T48 逻辑）。保留前 `keep_prefix` 字符 +
//!   后 `keep_suffix` 字符，中间替换为 `mask_char`（至少 `mask_min_len` 个）。
//!   `min_len` / `max_len` 为值总字符数 guard。所有字段 `Option`，`None` 取语义
//!   默认值（0/0/*/1/None/None）。
//! - `Segment`：分段脱敏（T52 新增）。按 `delimiter` 拆分值，对 `segments` 中
//!   列出的段（按 0-based `index`）做保留首尾脱敏，其余段原样保留。
//!
//! serde untagged：旧 DB 里的 flat JSON（无 `delimiter` 字段）反序列化为 `Simple`，
//! 新 JSON（含 `delimiter`）反序列化为 `Segment`。`SimpleTemplate` 所有字段
//! `Option` + `SegmentTemplate` 需要 `delimiter: String`（非 Option）→ serde
//! 先尝试 `Simple`（命中旧 JSON），再尝试 `Segment`（命中含 `delimiter` 的新 JSON）。
//!
//! T49 子规则化：`general-mask` 规则持有**空模板**（`TemplateParams::default()`，
//! 即 `Simple` 全 `None`）→ `SimpleMasker` 视为不脱敏（透传）。前端选预设
//! （`idcard_preset()` 等）或配置分段模板后填充 `template`，再经 `mask_column`
//! 的 `template` 临时参数覆盖，或经 `update_rule_template` 持久化到 DB。
//!
//! v1.2.0 T93：从 `rules.rs` 拆出，承载模板类型 + 构造器 + 预设常量。

use serde::{Deserialize, Serialize};

/// 通用模板脱敏参数（v1.1.3 新增，T52 改为 untagged enum）。
///
/// 两种变体：
/// - `Simple`：整段脱敏（原 v1.1.3 T48 逻辑）。保留前 `keep_prefix` 字符 +
///   后 `keep_suffix` 字符，中间替换为 `mask_char`（至少 `mask_min_len` 个）。
///   `min_len` / `max_len` 为值总字符数 guard。所有字段 `Option`，`None` 取语义
///   默认值（0/0/*/1/None/None）。
/// - `Segment`：分段脱敏（T52 新增）。按 `delimiter` 拆分值，对 `segments` 中
///   列出的段（按 0-based `index`）做保留首尾脱敏，其余段原样保留。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", untagged)]
pub enum TemplateParams {
    /// 整段脱敏（v1.1.3 T48 原逻辑）。
    Simple(SimpleTemplate),
    /// 分段脱敏（v1.1.3 T52）：按 `delimiter` 拆分，对指定段做保留首尾脱敏。
    Segment(SegmentTemplate),
}

/// 整段脱敏模板参数（v1.1.3 T48，T52 从 `TemplateParams` 拆出作为 `Simple` 变体）。
///
/// `#[serde(deny_unknown_fields)]`：untagged enum 反序列化时，含 `delimiter` /
/// `segments` 字段的 Segment JSON 不会误匹配 Simple（否则 Simple 全 Option 字段
/// 会"吞掉"额外字段，导致 Segment 永远不被尝试）。
///
/// T53：`reverse: Option<bool>` 反向脱敏标志。`None`/`false` = 正向（保留首尾、
/// 掩码中间）；`true` = 反向（掩码首尾、保留中间）。反向时 `keep_prefix` /
/// `keep_suffix` 语义变为「首部脱码位数」/「尾部脱码位数」。旧 JSON 无此字段 →
/// `None` → 正向，向后兼容。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SimpleTemplate {
    /// 保留前缀字符数（默认 0）。`reverse=true` 时变为「首部脱码位数」。
    pub keep_prefix: Option<usize>,
    /// 保留后缀字符数（默认 0）。`reverse=true` 时变为「尾部脱码位数」。
    pub keep_suffix: Option<usize>,
    /// 掩码字符（默认 `*`）。
    pub mask_char: Option<char>,
    /// 脱敏段至少插入多少个掩码字符（默认 1）。正向 = 中间段最小掩码长度；
    /// 反向 = 重叠全脱敏时的最小长度兜底。
    pub mask_min_len: Option<usize>,
    /// 值总字符数下限 guard（默认 None = 不限制）。
    pub min_len: Option<usize>,
    /// 值总字符数上限 guard（默认 None = 不限制）。
    pub max_len: Option<usize>,
    /// 反向脱敏标志（T53）。`None`/`false` = 正向（保留首尾、掩码中间）；
    /// `true` = 反向（掩码首尾、保留中间）。旧 JSON 无此字段 → `None` → 正向。
    pub reverse: Option<bool>,
}

/// 分段脱敏模板参数（v1.1.3 T52）。
///
/// 按 `delimiter` 把值拆成多段，对 `segments` 中列出的段（按 0-based `index`）
/// 做保留首尾脱敏；不在列表中的段原样保留。例：
/// - `zhangsan@example.com` + delimiter=`@` + segments=[{0,1,1,4}]
///   → `z*****n@example.com`
/// - `192.168.11.1` + delimiter=`.` + segments=[{3,0,0,2}]
///   → `192.168.**.1`
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SegmentTemplate {
    /// 掩码字符（默认 `*`）。
    pub mask_char: Option<char>,
    /// 分隔符（如 `@` / `.` / `-`）。空 → 透传（不脱敏）。
    pub delimiter: String,
    /// 段配置列表。按 `index`（0-based）匹配拆分后的段。
    pub segments: Vec<SegmentMask>,
}

/// 单段脱敏配置（v1.1.3 T52，`SegmentTemplate.segments` 元素）。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SegmentMask {
    /// 段下标（0-based，对应 `input.split(delimiter)` 后的位置）。
    pub index: usize,
    /// 该段保留前缀字符数（默认 0）。
    pub keep_prefix: Option<usize>,
    /// 该段保留后缀字符数（默认 0）。
    pub keep_suffix: Option<usize>,
    /// 该段至少插入多少个掩码字符（默认 1）。
    pub mask_min_len: Option<usize>,
}

impl Default for TemplateParams {
    /// 默认 = 空 Simple 模板（全 `None`，透传）。
    fn default() -> Self {
        TemplateParams::Simple(SimpleTemplate::default())
    }
}

impl TemplateParams {
    /// 构造一个 Simple 模板（keep_prefix/keep_suffix/mask_min_len 显式给定）。
    pub fn new(keep_prefix: usize, keep_suffix: usize, mask_min_len: usize) -> Self {
        TemplateParams::Simple(SimpleTemplate {
            keep_prefix: Some(keep_prefix),
            keep_suffix: Some(keep_suffix),
            mask_char: None,
            mask_min_len: Some(mask_min_len),
            min_len: None,
            max_len: None,
            reverse: None,
        })
    }

    /// 链式设置 mask_char（仅对 Simple 变体有效；Segment 变体忽略）。
    pub fn with_mask_char(self, c: char) -> Self {
        match self {
            TemplateParams::Simple(mut s) => {
                s.mask_char = Some(c);
                TemplateParams::Simple(s)
            }
            other => other,
        }
    }

    /// 链式设置 min_len / max_len guard（仅对 Simple 变体有效）。
    pub fn with_len_range(self, min: usize, max: usize) -> Self {
        match self {
            TemplateParams::Simple(mut s) => {
                s.min_len = Some(min);
                s.max_len = Some(max);
                TemplateParams::Simple(s)
            }
            other => other,
        }
    }

    /// 是否为空模板（透传，不脱敏）。
    ///
    /// - `Simple` → 6 字段全 `None`。
    /// - `Segment` → `delimiter` 为空 或 `segments` 为空。
    pub fn is_empty(&self) -> bool {
        match self {
            TemplateParams::Simple(s) => {
                s.keep_prefix.is_none()
                    && s.keep_suffix.is_none()
                    && s.mask_char.is_none()
                    && s.mask_min_len.is_none()
                    && s.min_len.is_none()
                    && s.max_len.is_none()
            }
            TemplateParams::Segment(s) => s.delimiter.is_empty() || s.segments.is_empty(),
        }
    }

    /// 取模板的掩码字符（Simple / Segment 共用）。
    pub fn mask_char(&self) -> Option<char> {
        match self {
            TemplateParams::Simple(s) => s.mask_char,
            TemplateParams::Segment(s) => s.mask_char,
        }
    }
}

impl SimpleTemplate {
    /// 构造一个 keep_prefix/keep_suffix/mask_min_len 都显式给定、其余默认的模板。
    pub fn new(keep_prefix: usize, keep_suffix: usize, mask_min_len: usize) -> Self {
        Self {
            keep_prefix: Some(keep_prefix),
            keep_suffix: Some(keep_suffix),
            mask_char: None,
            mask_min_len: Some(mask_min_len),
            min_len: None,
            max_len: None,
            reverse: None,
        }
    }

    /// 链式设置 mask_char。
    pub fn with_mask_char(mut self, c: char) -> Self {
        self.mask_char = Some(c);
        self
    }

    /// 链式设置 min_len / max_len guard（长度上下限，含端点）。
    pub fn with_len_range(mut self, min: usize, max: usize) -> Self {
        self.min_len = Some(min);
        self.max_len = Some(max);
        self
    }

    /// 链式设置反向脱敏标志（T53）。`true` = 掩码首尾、保留中间。
    pub fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse = Some(reverse);
        self
    }
}

impl SegmentTemplate {
    /// 构造一个指定分隔符的空分段模板（无段配置 → 透传）。
    pub fn new(delimiter: impl Into<String>) -> Self {
        Self {
            mask_char: None,
            delimiter: delimiter.into(),
            segments: Vec::new(),
        }
    }

    /// 链式添加一段脱敏配置。
    pub fn with_segment(
        mut self,
        index: usize,
        keep_prefix: usize,
        keep_suffix: usize,
        mask_min_len: usize,
    ) -> Self {
        self.segments.push(SegmentMask {
            index,
            keep_prefix: Some(keep_prefix),
            keep_suffix: Some(keep_suffix),
            mask_min_len: Some(mask_min_len),
        });
        self
    }

    /// 链式设置 mask_char。
    pub fn with_mask_char(mut self, c: char) -> Self {
        self.mask_char = Some(c);
        self
    }
}

// ---- T49 通用脱敏预设（general-mask 的子规则）----
//
// 4 条预设对应原 v1.1.3 独立规则（idcard/phone/birthdate/bankcard）的模板参数，
// 现作为 `general-mask` 的子规则供前端选择。前端选预设 → 填充 6 个可编辑参数框
// → 用户可继续修改 → 执行脱敏时把模板透传给 `mask_column`。
// 预设只是 `TemplateParams` 常量构造器，不再单独 seed 到 DB。

/// 身份证号预设：保留前 6 位地区码 + 后 4 位校验码，中间 8 位用 `*` 替换。
/// `min_len=max_len=18` guard → 非 18 位原样返回。
pub fn idcard_preset() -> TemplateParams {
    TemplateParams::new(6, 4, 8).with_len_range(18, 18)
}

/// 手机号预设：保留前 3 位 + 后 4 位，中间 4 位用 `*` 替换。
/// `min_len=max_len=11` guard → 非 11 位原样返回。
pub fn phone_preset() -> TemplateParams {
    TemplateParams::new(3, 4, 4).with_len_range(11, 11)
}

/// 出生日期预设：保留年份和月份（前 8 字符 `YYYY-MM-`），日期 2 位用 `*` 替换。
/// `min_len=max_len=10` guard → 非 10 位原样返回。例：`1990-01-15` → `1990-01-**`。
pub fn birthdate_preset() -> TemplateParams {
    TemplateParams::new(8, 0, 2).with_len_range(10, 10)
}

/// 银行卡号预设：保留前 4 位 + 后 4 位，中间位数用 `*` 替换。
/// 不设长度 guard（银行卡号长度 15~19 位不等）。例：`6222021234567890123` → `6222***********0123`。
pub fn bankcard_preset() -> TemplateParams {
    TemplateParams::new(4, 4, 1)
}
