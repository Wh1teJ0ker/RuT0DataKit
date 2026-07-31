//! 公共数据结构骨架。
//!
//! v1.0.0: 仅定义 `Record`，由各 datasource 子模块使用。
//! `Sheet` / `Operation` / `Column` 推迟到 v1.1+ 真实需要时再定义
//! （当前前端契约由 serde_json Value + commands 层处理）。
//! 嵌套 struct 一律 `#[serde(rename_all = "camelCase")]`，对齐前端契约。

use serde::{Deserialize, Serialize};

/// 单条记录（按列名到值的映射）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub fields: std::collections::HashMap<String, String>,
}
