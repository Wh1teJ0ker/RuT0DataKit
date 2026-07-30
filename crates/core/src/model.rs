//! 公共数据结构骨架。
//!
//! v1.0.0: 仅定义字段占位，具体语义/校验推迟到 T5（导入流）填充。
//! 嵌套 struct 一律 `#[serde(rename_all = "camelCase")]`，对齐前端契约。

use serde::{Deserialize, Serialize};

/// 单条记录（按列名到值的映射）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub fields: std::collections::HashMap<String, String>,
}

/// Sheet 元信息骨架。字段在 T5 导入流中填充。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sheet {
    pub id: i64,
    pub session_id: i64,
    pub name: String,
    pub position: i64,
    pub column_order: Vec<String>,
    pub column_visibility: std::collections::HashMap<String, bool>,
}

/// 操作日志骨架（对应 `operations` 表）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub id: i64,
    pub sheet_id: Option<i64>,
    pub kind: String,
    pub params_json: Option<String>,
    pub result_snapshot_json: Option<String>,
}

/// 列定义骨架。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    pub name: String,
    pub visible: bool,
}
