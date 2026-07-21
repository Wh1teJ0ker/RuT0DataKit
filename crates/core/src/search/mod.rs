//! 搜索模块：倒排索引 + 三种查询（keyword / regex / exact_field）。
//!
//! v0.4.0 最小实现：
//! - [`SearchIndex::build`]：对所有 cell 分词构建 term → (row, col) 倒排表。
//! - [`SearchQuery`]：三种查询枚举，带 [`SearchMode`]（And/Or）控制 keyword 合并。
//! - [`search_records`]：统一入口，dispatch 到 [`engine`] 三个实现并组装 [`SearchResult`]。
//!
//! 不实现模糊搜索 / 拼音 / 分词库（HANDOFF out_of_scope）。不做持久化索引——
//! 每次调用现建 [`SearchIndex`]；调用方若想缓存可自行持有 [`SearchIndex`]。

use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::readers::Records;

pub mod engine;
pub mod inverted_index;

pub use engine::{build_hits, search_exact_field, search_keyword, search_regex};
pub use inverted_index::SearchIndex;

/// keyword 查询的多关键词合并模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    /// 所有 term 都命中的 (row, col) 交集。
    And,
    /// 任一 term 命中的 (row, col) 并集。
    Or,
}

/// 查询类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SearchQuery {
    /// 关键词匹配：对 `terms` 每个 term 走倒排索引，按 `mode` 合并。
    Keyword {
        terms: Vec<String>,
        mode: SearchMode,
    },
    /// 正则匹配：线性扫全表，`pattern` 为 `regex` crate 语法。
    Regex {
        pattern: String,
    },
    /// 精确字段匹配：`field` 列上等于 `value` 的 cell。
    ExactField {
        field: String,
        value: String,
    },
}

/// 单条命中。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    /// 命中行号（0-based，对应 `records.rows`）。
    pub row: usize,
    /// 命中列号（0-based，对应 `records.headers`）。
    pub col: usize,
    /// 命中字段名（来自 `records.headers[col]`）。
    pub field: String,
    /// 命中 cell 原值。
    pub value: String,
    /// 命中片段 ±20 字符上下文（按 char 索引，UTF-8 友好）。
    pub snippet: String,
}

/// 查询结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResult {
    /// 全部命中（已按 (row, col) 去重 + 排序，输出稳定）。
    pub hits: Vec<SearchHit>,
}

/// 统一搜索入口：按 `query` 分支构建索引 / 线性扫，组装 [`SearchResult`]。
///
/// - keyword：调 [`SearchIndex::build`] + [`engine::search_keyword`]。
/// - regex：直接线性扫，非法 pattern 返回 [`CoreError::InvalidInput`]。
/// - exact_field：按 field 名定位列 + 精确匹配。
///
/// 同一调用内现建索引，不缓存。大文件（≥10w 行）性能基线见 `mod.rs` 的
/// `#[ignore]` 测试 `big_search_baseline`。
pub fn search_records(records: &Records, query: &SearchQuery) -> Result<SearchResult, CoreError> {
    let cells = match query {
        SearchQuery::Keyword { terms, mode } => {
            let index = SearchIndex::build(records);
            search_keyword(&index, terms, *mode)
        }
        SearchQuery::Regex { pattern } => search_regex(records, pattern)?,
        SearchQuery::ExactField { field, value } => search_exact_field(records, field, value),
    };
    Ok(SearchResult {
        hits: build_hits(records, cells),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn sample() -> Records {
        Records {
            headers: vec!["name".into(), "email".into(), "city".into()],
            rows: vec![
                vec!["Alice Lee".into(), "alice@example.com".into(), "北京".into()],
                vec!["Bob Smith".into(), "bob@example.com".into(), "上海".into()],
                vec!["Carol Diaz".into(), "carol@example.com".into(), "广州".into()],
                vec!["David".into(), "david@example.com".into(), "深圳".into()],
            ],
        }
    }

    #[test]
    fn keyword_and_via_entry() {
        let r = sample();
        let res = search_records(
            &r,
            &SearchQuery::Keyword {
                terms: vec!["bob".into(), "smith".into()],
                mode: SearchMode::And,
            },
        )
        .unwrap();
        assert_eq!(res.hits.len(), 1);
        assert_eq!(res.hits[0].row, 1);
        assert_eq!(res.hits[0].col, 0);
        assert_eq!(res.hits[0].field, "name");
    }

    #[test]
    fn keyword_or_via_entry() {
        let r = sample();
        let res = search_records(
            &r,
            &SearchQuery::Keyword {
                terms: vec!["alice".into(), "carol".into()],
                mode: SearchMode::Or,
            },
        )
        .unwrap();
        // alice 在 (0,0)+(0,1)；carol 在 (2,0)+(2,1) → 共 4 命中。
        assert_eq!(res.hits.len(), 4);
        let mut rows: Vec<usize> = res.hits.iter().map(|h| h.row).collect();
        rows.sort();
        assert_eq!(rows, vec![0, 0, 2, 2]);
    }

    #[test]
    fn keyword_empty_result() {
        let r = sample();
        let res = search_records(
            &r,
            &SearchQuery::Keyword {
                terms: vec!["nobody".into()],
                mode: SearchMode::Or,
            },
        )
        .unwrap();
        assert!(res.hits.is_empty());
    }

    #[test]
    fn regex_via_entry() {
        let r = sample();
        let res = search_records(
            &r,
            &SearchQuery::Regex {
                pattern: r"@example\.com$".into(),
            },
        )
        .unwrap();
        assert_eq!(res.hits.len(), 4);
        assert!(res.hits.iter().all(|h| h.field == "email"));
    }

    #[test]
    fn regex_invalid_returns_error_no_panic() {
        let r = sample();
        let err = search_records(
            &r,
            &SearchQuery::Regex {
                pattern: r"[invalid".into(),
            },
        )
        .unwrap_err();
        match err {
            CoreError::InvalidInput(_) => {}
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn exact_field_via_entry() {
        let r = sample();
        let res = search_records(
            &r,
            &SearchQuery::ExactField {
                field: "city".into(),
                value: "上海".into(),
            },
        )
        .unwrap();
        assert_eq!(res.hits.len(), 1);
        assert_eq!(res.hits[0].row, 1);
        assert_eq!(res.hits[0].col, 2);
        assert_eq!(res.hits[0].value, "上海");
    }

    #[test]
    fn snippet_present_on_hit() {
        let r = sample();
        let res = search_records(
            &r,
            &SearchQuery::Keyword {
                terms: vec!["alice".into()],
                mode: SearchMode::Or,
            },
        )
        .unwrap();
        // "alice" 出现在 (0,0) "Alice Lee" 和 (0,1) "alice@example.com" → 2 命中。
        assert_eq!(res.hits.len(), 2);
        assert!(res.hits.iter().all(|h| !h.snippet.is_empty()));
    }

    #[test]
    fn exact_field_unknown_header_yields_empty_via_entry() {
        let r = sample();
        let res = search_records(
            &r,
            &SearchQuery::ExactField {
                field: "nope".into(),
                value: "x".into(),
            },
        )
        .unwrap();
        assert!(res.hits.is_empty());
    }

    /// 生成 ≥10w 行 × 10 列 Records（代码内生成，不提交大 CSV 到仓库）。
    fn big_records(rows: usize) -> Records {
        let headers: Vec<String> = (0..10).map(|i| format!("col{i}")).collect();
        let mut out = Vec::with_capacity(rows);
        for r in 0..rows {
            let row: Vec<String> = (0..10)
                .map(|c| format!("r{r}c{c}-needle-{c}"))
                .collect();
            out.push(row);
        }
        Records { headers, rows: out }
    }

    #[test]
    #[ignore = "大文件性能基线，避免每次 cargo test 跑"]
    fn big_search_baseline() {
        let records = big_records(100_000);
        // build 基线：5s
        let t0 = Instant::now();
        let index = SearchIndex::build(&records);
        let build_ms = t0.elapsed().as_millis();
        assert!(build_ms < 5_000, "build too slow: {build_ms}ms");

        // keyword 单次查询：1s（走索引，但命中 1_000_000 个 cell 时
        // HashSet 组装 + Vec 收集本身有成本；宽松上限避免 CI 抖动）。
        let t1 = Instant::now();
        let hits = search_keyword(&index, &["needle".into()], SearchMode::Or);
        let kw_ms = t1.elapsed().as_millis();
        assert!(kw_ms < 1_000, "keyword query too slow: {kw_ms}ms");
        // 10w 行 × 10 列每个 cell 都含 needle → 1_000_000 命中。
        assert_eq!(hits.len(), 1_000_000);

        // 通过统一入口再跑一次 keyword（含 build_hits）。命中 1_000_000 个 cell
        // 时 sort + dedup + 1M SearchHit 构造（每条 clone value/field/snippet）
        // 是主要成本；放宽到 8s 避免 CI 抖动。
        let t2 = Instant::now();
        let res = search_records(
            &records,
            &SearchQuery::Keyword {
                terms: vec!["needle".into()],
                mode: SearchMode::Or,
            },
        )
        .unwrap();
        let full_kw_ms = t2.elapsed().as_millis();
        assert!(full_kw_ms < 8_000, "full keyword too slow: {full_kw_ms}ms");
        assert_eq!(res.hits.len(), 1_000_000);

        // regex 单次查询：3s（线性扫 1_000_000 cell + 1M build_hits；
        // regex 引擎 is_match 在长字符串上有成本，宽松上限避免 CI 抖动）。
        let t3 = Instant::now();
        let _res = search_records(
            &records,
            &SearchQuery::Regex {
                pattern: r"needle-\d".into(),
            },
        )
        .unwrap();
        let re_ms = t3.elapsed().as_millis();
        assert!(re_ms < 3_000, "regex query too slow: {re_ms}ms");
    }
}
