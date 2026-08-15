//! IPv4 / IPv6 地址校验。

use std::str::FromStr;

/// IPv4 严格校验：4 段点号分隔，每段 0-255，禁前导零（`"0"` 单独允许，
/// `"01"` / `"00"` 拒）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::is_valid_ipv4;
/// assert!(is_valid_ipv4("192.168.1.1"));
/// assert!(is_valid_ipv4("0.0.0.0"));
/// assert!(is_valid_ipv4("255.255.255.255"));
/// assert!(!is_valid_ipv4("256.1.1.1"));       // 超范围
/// assert!(!is_valid_ipv4("192.168.1"));       // 段数不足
/// assert!(!is_valid_ipv4("192.168.01.1"));    // 前导零
/// ```
pub fn is_valid_ipv4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    for p in parts {
        if p.is_empty() {
            return false;
        }
        // 前导零：长度 > 1 且首字符为 '0' → 非法（"0" 单独允许）
        if p.len() > 1 && p.starts_with('0') {
            return false;
        }
        // 必须全 ASCII 数字
        if !p.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        let n: u32 = match p.parse() {
            Ok(n) => n,
            Err(_) => return false,
        };
        if n > 255 {
            return false;
        }
    }
    true
}

/// IPv6 严格校验：委托 `std::net::Ipv6Addr::from_str`（RFC 4291，支持 `::`
/// 缩写、全写、IPv4-mapped 等所有合法形式）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::is_valid_ipv6;
/// assert!(is_valid_ipv6("::1"));
/// assert!(is_valid_ipv6("2001:db8::1"));
/// assert!(is_valid_ipv6("fe80::1"));
/// assert!(!is_valid_ipv6("::g"));              // 非法十六进制
/// assert!(!is_valid_ipv6("2001:db8:::1"));     // 多重缩写
/// ```
pub fn is_valid_ipv6(s: &str) -> bool {
    std::net::Ipv6Addr::from_str(s).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_valid_samples() {
        // 用户给的有效样例
        assert!(is_valid_ipv4("192.168.1.1"));
        assert!(is_valid_ipv4("10.0.0.1"));
        assert!(is_valid_ipv4("255.255.255.255"));
        // 边界
        assert!(is_valid_ipv4("0.0.0.0"));
        assert!(is_valid_ipv4("1.2.3.4"));
    }

    #[test]
    fn ipv4_invalid_samples() {
        // 用户给的无效样例
        assert!(!is_valid_ipv4("256.1.1.1")); // 超范围
        assert!(!is_valid_ipv4("192.168.1")); // 段数不足
        assert!(!is_valid_ipv4("192.168.01.1")); // 前导零
                                                 // 其他非法
        assert!(!is_valid_ipv4("192.168.1.1.1")); // 段数过多
        assert!(!is_valid_ipv4("192.168..1")); // 空段
        assert!(!is_valid_ipv4("192.168.a.1")); // 非数字
        assert!(!is_valid_ipv4("192.168.1.999")); // 超范围
        assert!(!is_valid_ipv4("")); // 空串
        assert!(!is_valid_ipv4("192.168.1.01")); // 末段前导零
    }

    #[test]
    fn ipv6_valid_samples() {
        assert!(is_valid_ipv6("::1"));
        assert!(is_valid_ipv6("::"));
        assert!(is_valid_ipv6("2001:db8::1"));
        assert!(is_valid_ipv6("fe80::1"));
        assert!(is_valid_ipv6("2001:0db8:0000:0000:0000:0000:0000:0001")); // 全写
        assert!(is_valid_ipv6("::ffff:192.168.1.1")); // IPv4-mapped
    }

    #[test]
    fn ipv6_invalid_samples() {
        assert!(!is_valid_ipv6("::g")); // 非法十六进制
        assert!(!is_valid_ipv6("2001:db8:::1")); // 多重缩写
        assert!(!is_valid_ipv6("2001:db8")); // 段数不足且无缩写
        assert!(!is_valid_ipv6("")); // 空串
        assert!(!is_valid_ipv6("gggg::1")); // 非法字符
    }
}
