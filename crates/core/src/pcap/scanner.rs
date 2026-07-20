//! T3-3：PcapScanner::scan — 把 reader + decoder + SensitiveScan 串成
//! `kind="pcap_scan"` 的 [`Report`]。
//!
//! 设计要点（见 v0.3.0 计划）：
//! - 仅做敏感扫描，不跑 SQLi 签名（fixture 仅含 PII）。
//! - 对每条 [`HttpRequest`]：构造扫描文本 = 双重 URL 解码后的 uri +
//!   `extract_decoded_fields(body)` 解出的字段值，复用
//!   [`SensitiveScan::scan`] 跑身份证/手机/邮箱/MAC/姓名 等。
//! - finding.location 补 `frame:<N>`；context 补 `<METHOD> <HOST> <SRC_IP>`。
//! - summary 含 total_requests / sensitive_hits / decoded_fragments /
//!   top_src_ips（按命中数排序取前 5）。

use std::collections::HashMap;
use std::path::Path;

use serde_yml::Value;

use crate::error::CoreError;
use crate::pcap::decoder::{decode_url_twice, extract_decoded_fields};
use crate::pcap::reader::PcapReader;
use crate::report::{Finding, Report};
use crate::rules::RuleSet;
use crate::scan::{DefaultSensitiveScan, SensitiveScan};

/// pcap 扫描器：无状态，依赖调用方注入 `scan` + `rules`。
pub struct PcapScanner;

impl PcapScanner {
    /// 对 pcap 文件跑敏感扫描，产出 `kind="pcap_scan"` 的 [`Report`]。
    ///
    /// - tshark 缺失 → [`CoreError::DependencyMissing("tshark")`](crate::error::CoreError) 透传。
    /// - `scan` / `rules` 由调用方注入，GUI 多次扫描场景友好。
    pub fn scan(
        path: &Path,
        scan: &DefaultSensitiveScan,
        rules: &RuleSet,
    ) -> Result<Report, CoreError> {
        let requests = PcapReader::read(path)?;
        let mut findings: Vec<Finding> = Vec::new();
        let mut ip_counts: HashMap<String, u64> = HashMap::new();
        let mut sensitive_hits: u64 = 0;
        let mut decoded_fragments: u64 = 0;

        for req in &requests {
            // 扫描文本 = 双重 URL 解码后的 uri + body 解码出的字段值
            let mut scan_text = decode_url_twice(&req.uri);
            if let Some(body) = req.body.as_deref() {
                let fields = extract_decoded_fields(body);
                for (_field, value) in &fields {
                    if !value.is_empty() {
                        scan_text.push(' ');
                        scan_text.push_str(value);
                        decoded_fragments += 1;
                    }
                }
            }

            let sens = scan.scan(&scan_text, rules);
            let frame_hits = sens.len() as u64;
            for mut f in sens {
                f.location = Some(format!("frame:{}", req.frame_no));
                f.context = Some(format!("{} {} -> {}", req.method, req.host, req.src_ip));
                f.extra = None;
                findings.push(f);
                sensitive_hits += 1;
            }
            if frame_hits > 0 {
                *ip_counts.entry(req.src_ip.clone()).or_insert(0) += frame_hits;
            }
        }

        let mut ip_vec: Vec<(String, u64)> = ip_counts.into_iter().collect();
        ip_vec.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let top_src_ips: Vec<Value> = ip_vec
            .into_iter()
            .take(5)
            .map(|(ip, hits)| {
                let mut m = serde_yml::Mapping::new();
                m.insert(Value::String("ip".into()), Value::String(ip));
                m.insert(Value::String("hits".into()), Value::Number(hits.into()));
                Value::Mapping(m)
            })
            .collect();

        let mut summary = serde_yml::Mapping::new();
        summary.insert(
            Value::String("total_requests".into()),
            Value::Number((requests.len() as u64).into()),
        );
        summary.insert(
            Value::String("sensitive_hits".into()),
            Value::Number(sensitive_hits.into()),
        );
        summary.insert(
            Value::String("decoded_fragments".into()),
            Value::Number(decoded_fragments.into()),
        );
        summary.insert(
            Value::String("top_src_ips".into()),
            Value::Sequence(top_src_ips),
        );

        Ok(Report {
            source: path.to_string_lossy().into_owned(),
            kind: "pcap_scan".to_string(),
            summary: Value::Mapping(summary),
            findings,
            extra: Value::Null,
        })
    }
}
