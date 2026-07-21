//! 倒排索引构建 + 分词。
//!
//! v0.4.0 最小实现：按「非字母数字」字符切分 + 小写化（UTF-8 友好，按 `char`
//! 迭代），term → 倒排表 `HashMap<String, Vec<(usize, usize)>>`（行号, 列号）。
//! 不引入第三方分词库 / 拼音 / 模糊匹配（HANDOFF out_of_scope）。
//!
//! 每条 term 对应的 (row, col) 列表保持插入顺序、不去重——同一 cell 里多次出现
//! 同一 term 时也只记一次（用 `(row, col)` 集合语义去重），因为命中粒度只到 cell。

use std::collections::HashMap;

use crate::readers::Records;

/// 倒排索引：term → 命中过的 (row, col) 列表（已按 (row, col) 去重，但未排序）。
///
/// 另存 `headers` 副本便于 exact_field 查询时按字段名定位列。
#[derive(Debug, Clone)]
pub struct SearchIndex {
    /// term（小写）→ (row, col) 列表。
    pub postings: HashMap<String, Vec<(usize, usize)>>,
    /// 表头副本（exact_field 用）。
    pub headers: Vec<String>,
    /// 行数（性能基线 / 早期返回用）。
    pub row_count: usize,
}

impl SearchIndex {
    /// 对 [`Records`] 的每个 cell 分词构建倒排索引。
    ///
    /// 空 cell 不产生 term。同一 (row, col) 上重复出现的 term 只记一次。
    pub fn build(records: &Records) -> SearchIndex {
        let mut postings: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        for (ri, row) in records.rows.iter().enumerate() {
            for (ci, cell) in row.iter().enumerate() {
                if cell.is_empty() {
                    continue;
                }
                // 同一 cell 内的 term 去重：用临时 set。
                let mut seen: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                for term in tokenize(cell) {
                    if seen.insert(term.clone()) {
                        postings
                            .entry(term)
                            .or_default()
                            .push((ri, ci));
                    }
                }
            }
        }
        SearchIndex {
            postings,
            headers: records.headers.clone(),
            row_count: records.rows.len(),
        }
    }
}

/// 分词：按「非字母数字」字符切分 + 小写化。
///
/// UTF-8 友好：按 `char` 迭代而非 byte。中文等非 ASCII 字母数字字符按
/// `char::is_alphanumeric` 判定（Unicode 感知），连续中文会被聚成一个 term
/// （v0.4.0 最小行为，不做 CJK 词法切分——HANDOFF out_of_scope）。
pub fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            for c in ch.to_lowercase() {
                cur.push(c);
            }
        } else {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_splits_on_non_alphanumeric() {
        assert_eq!(tokenize("Hello, World!"), vec!["hello", "world"]);
        assert_eq!(tokenize("foo-bar_baz"), vec!["foo", "bar", "baz"]);
        assert_eq!(tokenize("中文 测试"), vec!["中文", "测试"]);
        assert_eq!(tokenize(""), Vec::<String>::new());
        assert_eq!(tokenize("a1 b2"), vec!["a1", "b2"]);
    }

    #[test]
    fn build_index_dedupes_within_cell() {
        let records = Records {
            headers: vec!["name".into()],
            rows: vec![vec!["foo foo foo".into()]],
        };
        let idx = SearchIndex::build(&records);
        assert_eq!(idx.postings.get("foo").unwrap(), &vec![(0, 0)]);
    }

    #[test]
    fn build_index_multi_cells() {
        let records = Records {
            headers: vec!["a".into(), "b".into()],
            rows: vec![
                vec!["foo".into(), "bar".into()],
                vec!["baz".into(), "foo".into()],
            ],
        };
        let idx = SearchIndex::build(&records);
        let mut foo = idx.postings.get("foo").unwrap().clone();
        foo.sort();
        assert_eq!(foo, vec![(0, 0), (1, 1)]);
        assert_eq!(idx.postings.get("bar").unwrap(), &vec![(0, 1)]);
        assert_eq!(idx.postings.get("baz").unwrap(), &vec![(1, 0)]);
        assert_eq!(idx.row_count, 2);
    }
}
