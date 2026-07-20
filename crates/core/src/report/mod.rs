//! 统一报告（Report）schema：`Report` + `Finding`。
//!
//! T0-5 落地 `kind="csv_mask"` 的脱敏报告；T0-6 敏感扫描会复用同一 schema
//! 填 `kind="log_scan"` / `"pcap_scan"` 等。`summary` / `extra` 使用弱类型
//! `serde_yml::Value`，各 kind 各自决定结构。

pub mod csv_report;

pub use csv_report::{build_csv_mask_report, write_masked_csv};

/// 单条 finding：扫描到的敏感项。
#[derive(Debug, Clone, serde::Serialize)]
pub struct Finding {
    /// 敏感类型，如 "idcard" / "phone" / "bankcard"。
    #[serde(rename = "type")]
    pub r#type: String,
    /// 命中的原始值（脱敏报告通常不产出 finding，扫描报告产出）。
    pub value: String,
    /// 命中位置（行号 / offset / 字段名等），可空。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// 校验结果（如 idcard 校验通过/不通过），可空。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid: Option<bool>,
    /// 额外上下文，可空。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// 弱类型扩展字段，可空。log_scan 场景塞 ParsedPayload 序列化值
    /// （由 pipeline/log_scan.rs 把 SignatureHit.parsed_payload 序列化为
    /// `serde_yml::Value` 透传给前端）；csv_report / sensitive 场景为 `None`
    /// （向后兼容：`skip_serializing_if` 保证无 extra 时序列化形态不变，旧
    /// JSON 消费者读到 extra 为 undefined 不报错）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_yml::Value>,
}

/// 统一报告。
#[derive(Debug, Clone, serde::Serialize)]
pub struct Report {
    /// 源路径或标识。
    pub source: String,
    /// 报告种类：`csv_mask` / `log_scan` / `pcap_scan`。
    pub kind: String,
    /// 弱类型汇总，各 kind 自填。
    pub summary: serde_yml::Value,
    /// 命中 finding 列表（脱敏场景通常空）。
    pub findings: Vec<Finding>,
    /// 额外元数据（如 skipped_fields / 脱敏参数等）。
    pub extra: serde_yml::Value,
}
