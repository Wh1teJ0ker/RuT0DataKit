//! SQL 解析工具模块（v0.5.0 T5-9）。
//!
//! 把 v0.2.4 的盲注探针提取 + 聚类 + 数据库还原从 `LogEntry` 耦合里解耦出来，
//! 接受**纯 SQL 文本列表**输入，自动识别 7 类 SQLi 探针并还原结构化数据库。
//!
//! ## 与 `log_scan` 的关系
//!
//! - v0.2.4 `pipeline::log_scan::scan_log` 继续走
//!   [`BlindAggregator::collect_from_entries`]（吃 `LogEntry`），本任务 T5-9
//!   不动它。
//! - 本模块复用 [`crate::logsign::extract_blind_probe`]（同一份「SQL 文本 →
//!   BlindProbe」逻辑），通过 [`BlindAggregator::collect_from_probes`] 进入
//!   同一聚合 / 还原算法，保证 v0.2.4 fixture 在两条路径下还原结果一致。
//!
//! ## 7 类探针支持
//!
//! - **boolean**（`ascii(substr((<rt>),<pos>,1))<cmp><thr>`）：进聚合，按位置
//!   二分还原字符。
//! - **equality**（`substr((<rt>),<pos>,1)='c'` 或 `=char(N)`）：进聚合，单探针
//!   直接得字符。
//! - **length**（`length((<rt>))<cmp><thr>`）：进聚合，按 threshold 二分还原
//!   长度数值。
//! - **time**（`sleep(N)` / `benchmark(...)` / `pg_sleep(N)`）：HANDOFF risks
//!   指出无 `char_position` 无法拼字符串，本任务最小实现是仅产 [`ParsedPayload`]
//!   （`sleep_seconds` 汇总），不进聚合。
//! - **error / union / tautology / comment**：无二分序列，仅产 [`ParsedPayload`]，
//!   不进聚合。
//!
//! [`ParsedPayload`] 由 [`crate::logsign::payload_parser::parse_payload`] 复用
//! 产出，按 6 类首命中规则归类（time / error / union / tautology / comment
//! 覆盖 5 类非盲注探针，blind_boolean 在 payload_parser 里也归一类但本模块
//! 优先用 BlindProbe 表达，避免重复）。
//!
//! ## v0.7.1：自动 URL 解码
//!
//! `parse_sqls` 内部用 [`crate::log::looks_like_url_encoded`] 检测输入是否含
//! `%XX` hex 序列，命中则先调 [`crate::log::url_decode_twice`] 双重 URL 解码
//! 再喂探针正则。caller 可直接传 raw（如 `username=1'%20or%20...%23`），
//! 无需自行解码。对已解码输入零影响（向后兼容 v0.5.0 ~ v0.7.0 行为）。

use crate::logsign::payload_parser::parse_payload;
use crate::logsign::{
    extract_blind_probe, AggregatedResult, BlindAggregator, BlindProbe, ParsedPayload,
    ReconstructedDatabase,
};
use crate::log::{looks_like_url_encoded, url_decode_twice};

/// 单条 SQL 解析输入（v0.5.0 T5-9）。
///
/// 与 `LogEntry` 解耦：caller 只需提供 SQL 文本 + 可选响应 body
/// 大小 + 可选来源 IP。`response_body_size` 为 `None` 时该 input 跳过盲注
/// 探针提取（无 body size 无法判真假），但仍会尝试 payload_parser 6 类语义
/// 解析兜底。
///
/// **v0.7.1**：`sql` 字段可传 raw 或已解码形态——[`parse_sqls`] 内部用
/// [`looks_like_url_encoded`] 检测 `%XX` 序列，命中则自动调
/// [`url_decode_twice`] 解码后再喂探针正则。caller 无需自行解码。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SqlParseInput {
    /// SQL 文本（v0.7.1 起 `parse_sqls` 自动检测 `%XX` 并解码，caller 可直传 raw）。
    pub sql: String,
    /// HTTP 响应 body 字节数；`None` 表示未知（盲注探针跳过该 input）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body_size: Option<u64>,
    /// 来源 IP；`None` 视为空串。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ip: Option<String>,
}

/// SQL 解析聚合结果（v0.5.0 T5-9）。
///
/// - `probes`：从所有 input 抽取的盲注探针（boolean / equality / length）。
/// - `aggregated`：按 `(read_target, source_ip, ProbeKind)` 聚类后的还原结果。
/// - `reconstructed`：交叉关联 4 类标准 read_target 重建的结构化数据库。
/// - `parsed_payloads`：兜底字段，记录 time / error / union / tautology /
///   comment 等无二分序列的探针（HANDOFF risks 指出这些类不进聚合）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct SqlParseResult {
    /// 所有 input 抽取的盲注探针（boolean/equality/length）。
    pub probes: Vec<BlindProbe>,
    /// 聚合还原结果（每 `(read_target, source_ip, ProbeKind)` 一项）。
    pub aggregated: Vec<AggregatedResult>,
    /// 交叉关联重建的结构化数据库。
    pub reconstructed: ReconstructedDatabase,
    /// 兜底：6 类 payload 语义解析（time/error/union/tautology/comment 等
    /// 无二分序列的探针）；盲注探针也会在此出现（payload_parser 首命中 blind_boolean）。
    pub parsed_payloads: Vec<ParsedPayload>,
}

/// 对一批纯 SQL 文本做盲注探针提取 + 聚类 + 数据库还原（v0.5.0 T5-9）。
///
/// 流程：
/// 1. 对每条 input 调 [`extract_blind_probe`] 抽取盲注探针（boolean/equality/
///    length），同时调 [`parse_payload`] 兜底产出 6 类语义解析。
/// 2. 用 [`BlindAggregator::collect_from_probes`] 构建聚合器 → [`aggregate`]
///    → [`reconstruct_database`]。
///
/// [`aggregate`]: BlindAggregator::aggregate
/// [`reconstruct_database`]: BlindAggregator::reconstruct_database
pub fn parse_sqls(inputs: Vec<SqlParseInput>) -> SqlParseResult {
    let mut probes: Vec<BlindProbe> = Vec::new();
    let mut parsed_payloads: Vec<ParsedPayload> = Vec::new();

    for input in &inputs {
        // v0.7.1：自动识别 URL-encoded 输入，命中 `%XX` hex 序列则先双重 URL
        // 解码再喂探针正则。对已解码输入零影响（looks_like_url_encoded 仅在
        // 含合法 `%XX` 时返回 true）。
        let decoded_sql = if looks_like_url_encoded(&input.sql) {
            url_decode_twice(&input.sql)
        } else {
            input.sql.clone()
        };

        // 盲注探针：boolean/equality/length（无 response_body_size 跳过）。
        let mut p = extract_blind_probe(
            &decoded_sql,
            input.response_body_size,
            input.source_ip.as_deref(),
        );
        probes.append(&mut p);

        // 6 类语义解析兜底：time/error/union/tautology/comment 等无二分序列
        // 的探针在这里被记录。blind_boolean 也会出现，作为 parsed_payload
        // 副产物（与 probes 不冲突，下游前端可按需展示）。
        if let Some(pp) = parse_payload(&decoded_sql) {
            parsed_payloads.push(pp);
        }
    }

    let aggregator = BlindAggregator::collect_from_probes(probes.clone());
    let aggregated = aggregator.aggregate();
    let reconstructed = BlindAggregator::reconstruct_database(&aggregated);

    SqlParseResult {
        probes,
        aggregated,
        reconstructed,
        parsed_payloads,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logsign::ProbeKind;

    /// 构造一条 ascii_binary 二分序列，还原出单个字符 ascii=target_ascii。
    /// 生成 3 个 true 探针（thr < target_ascii，body=true_size）+
    /// 2 个 false 探针（thr >= target_ascii，body=false_size），
    /// 保证 max(true_thr)+1 == min(false_thr) == target_ascii 自洽。
    fn ascii_binary_probes_for_char(
        rt: &str,
        pos: u32,
        target_ascii: u32,
        true_size: u64,
        false_size: u64,
        ip: &str,
    ) -> Vec<SqlParseInput> {
        let mut inputs = Vec::new();
        // true 探针：thr < target_ascii（取 target_ascii-3 / -2 / -1，确保至少 3 个）。
        let true_thrs = if target_ascii >= 3 {
            vec![target_ascii - 3, target_ascii - 2, target_ascii - 1]
        } else {
            (0..target_ascii).collect()
        };
        for thr in true_thrs {
            let sql = format!("ascii(substr(({rt}),{pos},1))>{thr}");
            inputs.push(SqlParseInput {
                sql,
                response_body_size: Some(true_size),
                source_ip: Some(ip.to_string()),
            });
        }
        // false 探针：thr >= target_ascii（取 target_ascii / +1）。
        for thr in [target_ascii, target_ascii + 1] {
            let sql = format!("ascii(substr(({rt}),{pos},1))>{thr}");
            inputs.push(SqlParseInput {
                sql,
                response_body_size: Some(false_size),
                source_ip: Some(ip.to_string()),
            });
        }
        inputs
    }

    #[test]
    fn extract_ascii_binary_probe_from_pure_sql() {
        // 单条 ascii_binary SQL → 1 个 BlindProbe（AsciiBinary）。
        let probes = extract_blind_probe(
            "1' or ascii(substr((database()),1,1))>79#",
            Some(862),
            Some("1.1.1.1"),
        );
        assert_eq!(probes.len(), 1);
        assert_eq!(probes[0].probe_kind, ProbeKind::AsciiBinary);
        assert_eq!(probes[0].read_target, "database()");
        assert_eq!(probes[0].char_position, 1);
        assert_eq!(probes[0].threshold, 79);
        assert_eq!(probes[0].body_size, 862);
        assert_eq!(probes[0].source_ip, "1.1.1.1");
    }

    #[test]
    fn extract_equality_probe_from_pure_sql() {
        // substr((database()),1,1)='p' → Equality 探针。
        let probes = extract_blind_probe(
            "substr((database()),1,1)='p'",
            Some(862),
            Some("1.1.1.1"),
        );
        assert_eq!(probes.len(), 1);
        assert_eq!(probes[0].probe_kind, ProbeKind::Equality);
        assert_eq!(probes[0].equality_char, Some('p'));
        assert_eq!(probes[0].char_position, 1);

        // char(N) 形态。
        let probes = extract_blind_probe(
            "substr((database()),1,1)=char(112)",
            Some(862),
            None,
        );
        assert_eq!(probes.len(), 1);
        assert_eq!(probes[0].equality_char, Some('p'));
        assert_eq!(probes[0].source_ip, "");
    }

    #[test]
    fn extract_length_probe_from_pure_sql() {
        // length((database()))>N → Length 探针。
        let probes = extract_blind_probe(
            "length((database()))>5",
            Some(862),
            Some("1.1.1.1"),
        );
        assert_eq!(probes.len(), 1);
        assert_eq!(probes[0].probe_kind, ProbeKind::Length);
        assert_eq!(probes[0].char_position, 0);
        assert_eq!(probes[0].threshold, 5);
    }

    #[test]
    fn extract_returns_empty_when_no_body_size() {
        // response_body_size=None → 空探针（与 v0.2.4 LogEntry.size=None 跳过一致）。
        let probes = extract_blind_probe(
            "ascii(substr((database()),1,1))>79",
            None,
            Some("1.1.1.1"),
        );
        assert!(probes.is_empty());
    }

    #[test]
    fn parse_sqls_clusters_and_reconstructs_person_database() {
        // 合成 v0.2.4 fixture 的 4 类 read_target 探针序列，验证纯 SQL 路径
        // 能还原出 schema=person / 1 表 person_data / 7 列。
        let true_size = 862u64;
        let false_size = 875u64;
        let ip = "10.112.16.207";

        // 1) database() → "person"（6 字符 p/e/r/s/o/n）。
        let mut inputs: Vec<SqlParseInput> = Vec::new();
        let person_chars: [u8; 6] = *b"person";
        for (i, b) in person_chars.iter().enumerate() {
            inputs.extend(ascii_binary_probes_for_char(
                "database()",
                (i + 1) as u32,
                *b as u32,
                true_size,
                false_size,
                ip,
            ));
        }

        // 2) table_name → "person_data"（11 字符）。
        let rt_tables =
            "select group_concat(table_name) from information_schema.tables where table_schema=database()";
        let table_chars: [u8; 11] = *b"person_data";
        for (i, b) in table_chars.iter().enumerate() {
            inputs.extend(ascii_binary_probes_for_char(
                rt_tables,
                (i + 1) as u32,
                *b as u32,
                true_size,
                false_size,
                ip,
            ));
        }

        // 3) column_name → "id,username,password,sex,birth,idcard,phone"。
        let rt_cols =
            "select group_concat(column_name) from information_schema.columns where table_name='person_data'";
        let col_str = "id,username,password,sex,birth,idcard,phone";
        for (i, ch) in col_str.chars().enumerate() {
            inputs.extend(ascii_binary_probes_for_char(
                rt_cols,
                (i + 1) as u32,
                ch as u32,
                true_size,
                false_size,
                ip,
            ));
        }

        // 4) 行数据 group_concat(id,0x7e,username,0x7e,idcard) from person_data →
        //    "1~zhangsan~IDCARD1,2~lisi~IDCARD2"。
        let rt_rows =
            "select group_concat(id,0x7e,username,0x7e,idcard) from person_data";
        let row_str = "1~zhangsan~IDCARD1,2~lisi~IDCARD2";
        for (i, ch) in row_str.chars().enumerate() {
            inputs.extend(ascii_binary_probes_for_char(
                rt_rows,
                (i + 1) as u32,
                ch as u32,
                true_size,
                false_size,
                ip,
            ));
        }

        let result = parse_sqls(inputs);

        // 还原断言。
        assert_eq!(result.reconstructed.schema.as_deref(), Some("person"));
        assert_eq!(result.reconstructed.tables.len(), 1, "expect 1 table");
        let table = &result.reconstructed.tables[0];
        assert_eq!(table.name, "person_data");
        assert_eq!(
            table.columns,
            vec!["id", "username", "password", "sex", "birth", "idcard", "phone"],
        );
        assert_eq!(table.column_separator, Some('~'));
        assert_eq!(table.rows.len(), 2, "2 rows");
        assert_eq!(table.rows[0].cells[0].as_deref(), Some("1"));
        assert_eq!(table.rows[0].cells[1].as_deref(), Some("zhangsan"));
        assert_eq!(table.rows[0].cells[5].as_deref(), Some("IDCARD1"));
        assert_eq!(table.rows[1].cells[0].as_deref(), Some("2"));
        assert_eq!(table.rows[1].cells[1].as_deref(), Some("lisi"));
        assert_eq!(table.rows[1].cells[5].as_deref(), Some("IDCARD2"));
    }

    #[test]
    fn parse_sqls_records_time_error_union_tautology_comment_payloads() {
        // 无二分序列的 5 类探针不进聚合，但在 parsed_payloads 兜底。
        let inputs = vec![
            SqlParseInput {
                sql: "1 and sleep(5)".to_string(),
                response_body_size: Some(1000),
                source_ip: Some("1.1.1.1".to_string()),
            },
            SqlParseInput {
                sql: "1 and extractvalue(1,concat(0x7e,user()))".to_string(),
                response_body_size: Some(1000),
                source_ip: Some("1.1.1.1".to_string()),
            },
            SqlParseInput {
                sql: "1 union select 1,2,3".to_string(),
                response_body_size: Some(1000),
                source_ip: Some("1.1.1.1".to_string()),
            },
            SqlParseInput {
                sql: "1 or 1=1".to_string(),
                response_body_size: Some(1000),
                source_ip: Some("1.1.1.1".to_string()),
            },
            SqlParseInput {
                sql: "1'--".to_string(),
                response_body_size: Some(1000),
                source_ip: Some("1.1.1.1".to_string()),
            },
        ];

        let result = parse_sqls(inputs);
        // 这 5 类无盲注探针（ascii_binary/equality/length 正则不命中）。
        assert!(
            result.probes.is_empty(),
            "non-blind probes must not produce BlindProbe, got {:?}",
            result.probes
        );
        assert!(result.aggregated.is_empty());
        // parsed_payloads 兜底 5 类。
        let attack_types: Vec<&str> = result
            .parsed_payloads
            .iter()
            .map(|p| p.attack_type.as_str())
            .collect();
        assert!(attack_types.contains(&"time"), "{:?}", attack_types);
        assert!(attack_types.contains(&"error"), "{:?}", attack_types);
        assert!(attack_types.contains(&"union"), "{:?}", attack_types);
        assert!(attack_types.contains(&"tautology"), "{:?}", attack_types);
        assert!(attack_types.contains(&"comment"), "{:?}", attack_types);
    }

    #[test]
    fn parse_sqls_empty_inputs() {
        let result = parse_sqls(Vec::new());
        assert!(result.probes.is_empty());
        assert!(result.aggregated.is_empty());
        assert!(result.reconstructed.tables.is_empty());
        assert!(result.parsed_payloads.is_empty());
    }

    #[test]
    fn parse_sqls_mixed_blind_and_non_blind_no_cross_contamination() {
        // 同一批 input 含 ascii_binary + sleep：盲注进聚合，sleep 只进 parsed_payloads。
        let mut inputs = ascii_binary_probes_for_char(
            "database()",
            1,
            b'p' as u32,
            862,
            875,
            "1.1.1.1",
        );
        inputs.push(SqlParseInput {
            sql: "1 and sleep(5)".to_string(),
            response_body_size: Some(1000),
            source_ip: Some("1.1.1.1".to_string()),
        });

        let result = parse_sqls(inputs);
        // ascii_binary 探针存在；sleep 不产 BlindProbe。
        assert!(result.probes.iter().all(|p| p.probe_kind == ProbeKind::AsciiBinary));
        assert_eq!(result.aggregated.len(), 1);
        let r = &result.aggregated[0];
        assert_eq!(r.kind.as_deref(), Some("ascii_binary"));
        assert!(r.decoded_string.starts_with('p'), "got {}", r.decoded_string);
        // sleep 在 parsed_payloads 里。
        assert!(result
            .parsed_payloads
            .iter()
            .any(|p| p.attack_type == "time"));
    }

    #[test]
    fn parse_sqls_separates_sources_by_ip() {
        // 同 read_target 不同 source_ip → 两个独立聚合分组。
        let mut inputs = ascii_binary_probes_for_char(
            "database()",
            1,
            b'p' as u32,
            862,
            875,
            "1.1.1.1",
        );
        inputs.extend(ascii_binary_probes_for_char(
            "database()",
            1,
            b'x' as u32,
            862,
            875,
            "2.2.2.2",
        ));

        let result = parse_sqls(inputs);
        // 两个分组（同 read_target 但 source_ip 不同）。
        let mut ips: Vec<String> = result
            .aggregated
            .iter()
            .flat_map(|r| r.source_ips.clone())
            .collect();
        ips.sort();
        assert_eq!(ips, vec!["1.1.1.1".to_string(), "2.2.2.2".to_string()]);
    }

    #[test]
    fn parse_sqls_decodes_url_encoded_ascii_binary() {
        // v0.7.1：用户报告的 URL-encoded payload，`%20`→space / `%3E`→`>` /
        // `%23`→`#`，解码后命中 ascii_binary 探针。
        let inputs = vec![SqlParseInput {
            sql: "username=1'%20or%20ascii(substr((database()),1,1))%3E79%23&password=1"
                .to_string(),
            response_body_size: Some(862),
            source_ip: Some("1.1.1.1".to_string()),
        }];

        let result = parse_sqls(inputs);
        assert_eq!(result.probes.len(), 1, "probes = {:?}", result.probes);
        assert_eq!(result.probes[0].probe_kind, ProbeKind::AsciiBinary);
        assert_eq!(result.probes[0].read_target, "database()");
        assert_eq!(result.probes[0].char_position, 1);
        assert_eq!(result.probes[0].threshold, 79);
        assert_eq!(result.probes[0].body_size, 862);
    }

    #[test]
    fn parse_sqls_decodes_url_encoded_equality() {
        // `%27p%27` → `'p'`，解码后命中 Equality 探针。
        let inputs = vec![SqlParseInput {
            sql: "substr((database()),1,1)=%27p%27".to_string(),
            response_body_size: Some(862),
            source_ip: Some("1.1.1.1".to_string()),
        }];

        let result = parse_sqls(inputs);
        assert_eq!(result.probes.len(), 1, "probes = {:?}", result.probes);
        assert_eq!(result.probes[0].probe_kind, ProbeKind::Equality);
        assert_eq!(result.probes[0].equality_char, Some('p'));
        assert_eq!(result.probes[0].char_position, 1);
    }

    #[test]
    fn parse_sqls_passes_plain_sql_unchanged() {
        // 已解码纯文本输入：looks_like_url_encoded 返回 false，走原路径，
        // 行为与 v0.7.0 一致（回归保护）。
        let inputs = vec![SqlParseInput {
            sql: "1' or ascii(substr((database()),1,1))>79#".to_string(),
            response_body_size: Some(862),
            source_ip: Some("1.1.1.1".to_string()),
        }];

        let result = parse_sqls(inputs);
        assert_eq!(result.probes.len(), 1);
        assert_eq!(result.probes[0].probe_kind, ProbeKind::AsciiBinary);
        assert_eq!(result.probes[0].read_target, "database()");
        assert_eq!(result.probes[0].threshold, 79);
    }

    #[test]
    fn parse_sqls_url_encoded_time_payload() {
        // `1'%20or%20sleep(5)%23` → `1' or sleep(5)#`，解码后命中 time 类。
        let inputs = vec![SqlParseInput {
            sql: "1'%20or%20sleep(5)%23".to_string(),
            response_body_size: Some(1000),
            source_ip: Some("1.1.1.1".to_string()),
        }];

        let result = parse_sqls(inputs);
        assert!(result
            .parsed_payloads
            .iter()
            .any(|p| p.attack_type == "time"),
            "parsed_payloads = {:?}",
            result.parsed_payloads);
    }
}
