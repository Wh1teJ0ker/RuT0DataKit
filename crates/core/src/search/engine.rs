//! 查询引擎：keyword (AND/OR) / regex / exact_field 三种查询实现。
//!
//! - keyword：走倒排索引取 term 的 (row, col) 倒排表，AND 取交集、OR 取并集。
//! - regex：线性扫描全表（不走索引），用 `regex::Regex`；非法 pattern 返回
//!   [`CoreError::InvalidInput`]，不 panic。
//! - exact_field：按 field 名定位列 + value 精确匹配（大小写敏感）。
//!
//! snippet：命中片段 ±20 字符上下文，按 char 索引不按 byte，避免 UTF-8 截断 panic。

use std::collections::HashSet;

use regex::Regex;

use crate::error::CoreError;
use crate::readers::Records;

use super::inverted_index::SearchIndex;
use super::{SearchHit, SearchMode};

/// keyword 查询：取每个 term 的倒排表，按 mode 合并。
///
/// terms 为空 / 任意 term 未命中时按 mode 语义处理：
/// - AND：空 terms → 空；任一 term 未命中 → 空（交集为空）。
/// - OR：空 terms → 空；未命中的 term 直接跳过。
pub fn search_keyword(
    index: &SearchIndex,
    terms: &[String],
    mode: SearchMode,
) -> Vec<(usize, usize)> {
    if terms.is_empty() {
        return Vec::new();
    }
    let lower: Vec<String> = terms
        .iter()
        .map(|t| t.to_lowercase())
        .collect();
    match mode {
        SearchMode::And => {
            // 任一 term 未命中 → 空集（AND 语义要求全部命中）。
            // 先收集所有 term 的倒排表；任一缺失直接返回空。
            let sets: Vec<HashSet<(usize, usize)>> = lower
                .iter()
                .map(|t| {
                    index
                        .postings
                        .get(t)
                        .map(|l| l.iter().cloned().collect())
                        .unwrap_or_default()
                })
                .collect();
            if sets.iter().any(|s| s.is_empty()) {
                return Vec::new();
            }
            let mut iter = sets.into_iter();
            let first = iter.next().unwrap_or_default();
            let mut acc = first;
            for next in iter {
                acc = acc.intersection(&next).cloned().collect();
                if acc.is_empty() {
                    return Vec::new();
                }
            }
            acc.into_iter().collect()
        }
        SearchMode::Or => {
            let mut acc: HashSet<(usize, usize)> = HashSet::new();
            for t in &lower {
                if let Some(list) = index.postings.get(t) {
                    acc.extend(list.iter().cloned());
                }
            }
            acc.into_iter().collect()
        }
    }
}

/// regex 查询：线性扫描每个 cell，匹配 `pattern` 的返回命中的 (row, col)。
///
/// 非法正则返回 [`CoreError::InvalidInput`]，不 panic。
pub fn search_regex(
    records: &Records,
    pattern: &str,
) -> Result<Vec<(usize, usize)>, CoreError> {
    let re = Regex::new(pattern).map_err(|e| CoreError::InvalidInput(format!("regex error: {e}")))?;
    let mut hits = Vec::new();
    for (ri, row) in records.rows.iter().enumerate() {
        for (ci, cell) in row.iter().enumerate() {
            if re.is_match(cell) {
                hits.push((ri, ci));
            }
        }
    }
    Ok(hits)
}

/// exact_field 查询：按 `field` 名定位列 + `value` 精确匹配。
///
/// `field` 不在 headers 时返回空结果（不报错，调用方按需处理）。
/// 大小写敏感精确相等；如需不敏感可后续扩 `SearchQuery`（v0.4.0 out_of_scope）。
pub fn search_exact_field(
    records: &Records,
    field: &str,
    value: &str,
) -> Vec<(usize, usize)> {
    let Some(ci) = records.headers.iter().position(|h| h == field) else {
        return Vec::new();
    };
    let mut hits = Vec::new();
    for (ri, row) in records.rows.iter().enumerate() {
        if row.get(ci).map(|v| v.as_str()) == Some(value) {
            hits.push((ri, ci));
        }
    }
    hits
}

/// 构造命中：把 (row, col) 列表转成 [`SearchHit`]，并填充 snippet。
///
/// snippet：命中 cell 内容前后各 ±20 字符上下文，按 char 索引切片，避免 UTF-8
/// 截断 panic。field 从 records.headers 取，value 从 cell 取。
pub fn build_hits(records: &Records, mut cells: Vec<(usize, usize)>) -> Vec<SearchHit> {
    // 排序保证输出稳定（多 term OR 后顺序依赖 HashMap，不稳定）。
    cells.sort_unstable();
    cells.dedup();
    let mut hits = Vec::with_capacity(cells.len());
    for (ri, ci) in cells {
        let row = match records.rows.get(ri) {
            Some(r) => r,
            None => continue,
        };
        let cell = row.get(ci).cloned().unwrap_or_default();
        let field = records.headers.get(ci).cloned().unwrap_or_default();
        hits.push(SearchHit {
            row: ri,
            col: ci,
            field,
            value: cell.clone(),
            snippet: snippet(&cell, &cell, 20),
        });
    }
    hits
}

/// 生成命中片段：以 `needle` 在 `haystack` 中的首次出现为中心，前后各取
/// `context` 字符上下文。按 char 索引切片，避免 UTF-8 边界 panic。
///
/// `needle` 为空 / 不在 `haystack` 中时直接返回 `haystack` 前后各 `context`
/// 字符的片段（兜底为整段 haystack）。
fn snippet(haystack: &str, needle: &str, context: usize) -> String {
    let chars: Vec<char> = haystack.chars().collect();
    if needle.is_empty() {
        return haystack.to_string();
    }
    let needle_chars: Vec<char> = needle.chars().collect();
    let pos = (0..=chars.len().saturating_sub(needle_chars.len()))
        .find(|&i| chars[i..i + needle_chars.len()] == needle_chars[..]);
    let Some(center) = pos else {
        return haystack.to_string();
    };
    let start = center.saturating_sub(context);
    let end = (center + needle_chars.len() + context).min(chars.len());
    let s: String = chars[start..end].iter().collect();
    // 标注省略号表明截断。
    let prefix = if start > 0 { "…" } else { "" };
    let suffix = if end < chars.len() { "…" } else { "" };
    format!("{prefix}{s}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readers::Records;

    fn sample() -> Records {
        Records {
            headers: vec!["name".into(), "email".into()],
            rows: vec![
                vec!["Alice".into(), "alice@example.com".into()],
                vec!["Bob Smith".into(), "bob@example.com".into()],
                vec!["中文名字".into(), "cn@example.com".into()],
            ],
        }
    }

    #[test]
    fn keyword_and() {
        let idx = SearchIndex::build(&sample());
        // "bob" 和 "smith" 都在 (1,0)
        let hits = search_keyword(&idx, &["bob".into(), "smith".into()], SearchMode::And);
        assert_eq!(hits, vec![(1, 0)]);
    }

    #[test]
    fn keyword_or() {
        let idx = SearchIndex::build(&sample());
        // "alice" 同时出现在 (0,0) "Alice" 和 (0,1) "alice@example.com"；
        // "bob" 出现在 (1,0) "Bob Smith" 和 (1,1) "bob@example.com"。
        let hits = search_keyword(&idx, &["alice".into(), "bob".into()], SearchMode::Or);
        let mut h = hits.clone();
        h.sort();
        assert_eq!(h, vec![(0, 0), (0, 1), (1, 0), (1, 1)]);
    }

    #[test]
    fn keyword_and_missing_term_yields_empty() {
        let idx = SearchIndex::build(&sample());
        // "zzznotexist" 不在任何 cell 里，AND 应得空集。
        let hits = search_keyword(&idx, &["zzznotexist".into()], SearchMode::And);
        assert!(hits.is_empty());
    }

    #[test]
    fn regex_hits() {
        let records = sample();
        let hits = search_regex(&records, r"@example\.com$").unwrap();
        let mut h = hits.clone();
        h.sort();
        assert_eq!(h, vec![(0, 1), (1, 1), (2, 1)]);
    }

    #[test]
    fn regex_invalid_returns_error() {
        let records = sample();
        let err = search_regex(&records, r"[invalid").unwrap_err();
        match err {
            CoreError::InvalidInput(_) => {}
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn exact_field_match() {
        let records = sample();
        let hits = search_exact_field(&records, "name", "Bob Smith");
        assert_eq!(hits, vec![(1, 0)]);
    }

    #[test]
    fn exact_field_unknown_header_yields_empty() {
        let records = sample();
        let hits = search_exact_field(&records, "nope", "x");
        assert!(hits.is_empty());
    }

    #[test]
    fn snippet_handles_utf8_boundary() {
        let s = snippet("中文测试字段值", "字段", 2);
        // 应该是 "…试字段值…" 类似带省略号的形式，关键是不 panic 且包含 needle。
        assert!(s.contains("字段"));
        assert!(s.contains('…'));
    }

    #[test]
    fn snippet_handles_empty_needle() {
        let s = snippet("hello", "", 20);
        assert_eq!(s, "hello");
    }

    #[test]
    fn build_hits_dedup_and_sort() {
        let records = sample();
        let hits = build_hits(&records, vec![(2, 0), (0, 0), (1, 0), (2, 0)]);
        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].row, 0);
        assert_eq!(hits[2].row, 2);
        assert_eq!(hits[0].field, "name");
    }
}
