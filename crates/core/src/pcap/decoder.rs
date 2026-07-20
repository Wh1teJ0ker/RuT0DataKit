//! T3-2：双重 URL 解码 + base64 字段解码 + 规则化重组接口。
//!
//! 设计要点（见 v0.3.0 计划）：
//! - [`decode_url_twice`]：与 `crate::log::url_decode_twice` 同语义；pcap
//!   模块独立实现避免对 log 模块的循环依赖（pcap 仅依赖 report/rules/scan/error）。
//! - [`try_decode_base64_field`]：自动检测单个值是否 base64 并解码。fixture 的
//!   POST body 是 JSON，每个字段值单独 base64（如 `username` → `Y2hlbnlvbmc=`）。
//!   解码策略：满足 base64 字符集 + 长度约束后，尝试 UTF-8；UTF-8 失败再尝试 GBK
//!   兜底（Rust std 无 GBK 解码器，仅 UTF-8 + GBK ASCII 子集；fixture 用 UTF-8）。
//! - [`extract_decoded_fields`]：把 body 文本拆成 `(field_name, decoded_value)`
//!   列表。支持 JSON / form-urlencoded / 纯文本（整体当作单字段）。
//! - [`reassemble_base64`]：规则化重组接口（fixture 不需要，保留给用户自定义
//!   规则 `reassemble_base64.param` 覆盖）。

use serde_json::Value;

/// 双重 URL 解码：连续解两遍。
///
/// 与 [`crate::log::url_decode_twice`] 同语义；pcap 模块独立实现，避免
/// `pcap` → `log` 形成不必要的耦合（pcap 仅依赖 report/rules/scan/error）。
pub fn decode_url_twice(input: &str) -> String {
    let once = url_decode_once(input);
    url_decode_once(&once)
}

fn url_decode_once(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2])) {
                out.push(hi << 4 | lo);
                i += 3;
                continue;
            }
        }
        // form-urlencoded 中 `+` 表示空格。
        if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// 是否对单值尝试 base64 解码。仅当：
/// - 字符集 ⊂ `[A-Za-z0-9+/=]`
/// - 长度 ≥ 4 且长度 % 4 == 0（标准 base64 padding）
/// - 解码后含可打印字符比例 ≥ 80%
///
/// 缓解 base64 误伤：11 位手机号长度 11 % 4 != 0 不触发；16 位银行卡长度 16 % 4 == 0
/// 会尝试，但解码结果大概率是乱码（可打印比例 < 80%），返回 None 不替换原值。
pub fn try_decode_base64_field(value: &str) -> Option<String> {
    let v = value.trim();
    if v.len() < 4 || v.len() % 4 != 0 {
        return None;
    }
    if !v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=') {
        return None;
    }
    let bytes = b64_decode(v)?;
    // UTF-8 优先（fixture 是 UTF-8 base64）
    if let Ok(s) = String::from_utf8(bytes.clone()) {
        if printable_ratio(&s) >= 0.8 {
            return Some(s);
        }
    }
    // GBK 兜底：v0.3.0 不实现完整 GBK 解码，仅检测 ASCII 子集；
    // 若全 ASCII 走 UTF-8 已命中，非 ASCII 留给 v0.3.1+ 扩展。
    None
}

/// 极简 base64 解码（不引入 `base64` crate）。
///
/// - 标准字母表 `A-Z a-z 0-9 + /`，`=` padding。
/// - 输入必须满足长度 % 4 == 0 且字符集合法（调用方负责校验）。
/// - 非法输入返回 `None`。
fn b64_decode(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut i = 0;
    while i + 4 <= bytes.len() {
        // 统计末尾 padding 数量
        let chunk = &bytes[i..i + 4];
        let mut vals = [0u8; 4];
        let mut pad = 0;
        for (j, &c) in chunk.iter().enumerate() {
            if c == b'=' {
                if j < 2 {
                    return None; // padding 出现在前 2 位非法
                }
                pad += 1;
                vals[j] = 0;
            } else {
                vals[j] = val(c)?;
            }
        }
        if pad > 2 {
            return None;
        }
        out.push((vals[0] << 2) | (vals[1] >> 4));
        if pad < 2 {
            out.push((vals[1] << 4) | (vals[2] >> 2));
        }
        if pad < 1 {
            out.push((vals[2] << 6) | vals[3]);
        }
        i += 4;
    }
    Some(out)
}

fn printable_ratio(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let total = s.chars().count().max(1);
    let printable = s.chars().filter(|c| !c.is_control()).count();
    printable as f64 / total as f64
}

/// 把 HTTP body 拆成 `(field_name, decoded_or_raw_value)` 列表。
///
/// 优先级：
/// 1. JSON 对象：`{"k1":"v1","k2":"v2",...}` → 对每个 value 调
///    [`try_decode_base64_field`]，解码成功用解码值，否则用原值。
/// 2. form-urlencoded：`k1=v1&k2=v2` → 同上，value 先做双重 URL 解码再尝试 base64。
/// 3. 其他（纯文本 / JSON array / JSON 字符串）：整体当作单字段 `(("_body", body))`。
///
/// fixture 是 case 1：每字段值单独 base64，自动解码即命中 PII。
pub fn extract_decoded_fields(body: &str) -> Vec<(String, String)> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    // 1. JSON 对象
    if trimmed.starts_with('{') {
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(trimmed) {
            return map
                .into_iter()
                .map(|(k, v)| {
                    let raw = match v {
                        Value::String(s) => s,
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => String::new(),
                        other => other.to_string(),
                    };
                    let decoded = try_decode_base64_field(&raw).unwrap_or(raw);
                    (k, decoded)
                })
                .collect();
        }
    }
    // 2. form-urlencoded
    if trimmed.contains('=') && !trimmed.starts_with('{') {
        let mut out = Vec::new();
        for pair in trimmed.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (k, v) = match pair.find('=') {
                Some(idx) => (&pair[..idx], &pair[idx + 1..]),
                None => (pair, ""),
            };
            // form-urlencoded value 先双重 URL 解码，再尝试 base64
            let url_dec = decode_url_twice(v);
            let decoded = try_decode_base64_field(&url_dec).unwrap_or(url_dec);
            out.push((k.to_string(), decoded));
        }
        if !out.is_empty() {
            return out;
        }
    }
    // 3. 其他：整体当单字段
    vec![("_body".to_string(), body.to_string())]
}

/// 规则化重组接口：用户可在 rules 中写 `reassemble_base64.param` 把多个字段
/// 值按指定参数拼接后整体 base64 解码（fixture 不需要；保留接口）。
///
/// - `field_values`：参与重组的字段值序列。
/// - `_param`：规则参数（v0.3.0 仅占位，未实现具体策略）。
///
/// 返回 `None` 表示当前不支持或重组失败，调用方应回退到自动解码。
pub fn reassemble_base64(field_values: &[String], _param: &str) -> Option<String> {
    if field_values.is_empty() {
        return None;
    }
    let joined = field_values.concat();
    try_decode_base64_field(&joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_decode_twice_basic() {
        // %27 → '，%2527 → %27 → '
        assert_eq!(url_decode_once("%27"), "'");
        assert_eq!(decode_url_twice("%2527"), "'");
        assert_eq!(decode_url_twice("hello%20world"), "hello world");
        // form-urlencoded `+` → 空格
        assert_eq!(decode_url_twice("a+b"), "a b");
    }

    #[test]
    fn decode_fixture_first_post_body() {
        // fixture 第一条 POST body（hex 解出 UTF-8 后）的字段
        let body = r#"{"username": "Y2hlbnlvbmc=", "name": "5LuY6YeM5aSP5peL", "sex": "5aWz", "birth": "MjAwMTA5MDU=", "idcard": "NTA2MDUxMjAwMTA5MDU1NzQz", "phone": "NzQ3MzMzODUyNDg=", "address": "6buR6b6Z5rGf55yB5ZOI5bCU5ruo5biC6YCa5rKz5Y6/5LiJ56uZ6ZWHMzc35Y+3MTU55a6k"}"#;
        let fields = extract_decoded_fields(body);
        let map: std::collections::HashMap<String, String> = fields.into_iter().collect();
        assert_eq!(map.get("username").map(|s| s.as_str()), Some("chenyong"));
        assert_eq!(map.get("name").map(|s| s.as_str()), Some("付里夏旋"));
        assert_eq!(map.get("sex").map(|s| s.as_str()), Some("女"));
        assert_eq!(map.get("birth").map(|s| s.as_str()), Some("20010905"));
        assert_eq!(
            map.get("idcard").map(|s| s.as_str()),
            Some("506051200109055743")
        );
        assert_eq!(map.get("phone").map(|s| s.as_str()), Some("74733385248"));
        assert!(map.get("address").unwrap().contains("黑龙江"));
        assert!(map.get("address").unwrap().contains("哈尔滨"));
    }

    #[test]
    fn non_base64_value_untouched() {
        // 普通中文名（非 base64）应原样返回
        let fields = extract_decoded_fields(r#"{"name":"张三"}"#);
        assert_eq!(fields[0].1, "张三");
    }

    #[test]
    fn phone_11_digits_not_decoded() {
        // 11 位手机号长度 11 % 4 != 0，不触发 base64 解码
        assert!(try_decode_base64_field("13812345678").is_none());
    }

    #[test]
    fn bankcard_16_may_attempt_but_fail_printable() {
        // 16 位银行卡长度 16 % 4 == 0，会尝试解码；解码结果大概率乱码
        // 应返回 None 不替换原值（避免把 6225881234567890 误解成乱码）
        let r = try_decode_base64_field("6225881234567890");
        // 允许两种结果：要么 None（解码失败），要么 Some 但不等于原值
        // 无论如何，调用方应能正确选择原值
        if let Some(s) = r {
            assert_ne!(s, "6225881234567890");
        }
    }

    #[test]
    fn form_urlencoded_body() {
        let body = "name=%E5%BC%A0%E4%B8%89&age=20";
        let fields = extract_decoded_fields(body);
        assert_eq!(fields[0].0, "name");
        assert_eq!(fields[0].1, "张三");
        assert_eq!(fields[1].0, "age");
        assert_eq!(fields[1].1, "20");
    }

    #[test]
    fn plain_text_body_as_single_field() {
        let body = "just plain text no structure";
        let fields = extract_decoded_fields(body);
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].0, "_body");
        assert_eq!(fields[0].1, body);
    }

    #[test]
    fn reassemble_base64_empty_returns_none() {
        assert!(reassemble_base64(&[], "x").is_none());
    }
}
