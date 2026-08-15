//! Luhn 算法校验（银行卡号）。

/// Luhn 算法校验（银行卡号）。
///
/// 标准实现：从右往左，对偶数位（1-based，右起第 2/4/... 位）数字 ×2，
/// 若乘积 > 9 则数位相加，最后所有数字求和，总和能被 10 整除则有效。
///
/// 非 ASCII 数字字符直接判否（提取正则已保证纯数字，此处兜底）。
/// 空串 / 单字符 → `false`（Luhn 至少需要 2 位）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::luhn_check;
/// assert!(luhn_check("6222021234567890128"));   // 通过 Luhn（校验位 8）
/// assert!(!luhn_check("6222021234567890123"));  // 未通过 Luhn（校验位 3，总和 75）
/// ```
///
/// 注：需求原始示例 `6222021234567890123`（声称通过）与标准 Luhn 不符
/// （总和 75，%10≠0），故有效样例改为校验位 8（`...0128`，总和 80）。
pub fn luhn_check(s: &str) -> bool {
    if s.len() < 2 {
        return false;
    }
    let digits: Vec<u8> = s.bytes().filter(|b| b.is_ascii_digit()).collect();
    if digits.len() != s.len() {
        return false;
    }
    let mut sum: u32 = 0;
    let mut odd = true; // 右起第 1 位 = 奇数位（不 ×2）
    for &b in digits.iter().rev() {
        let mut d = (b - b'0') as u32;
        if !odd {
            d *= 2;
            if d > 9 {
                d = d / 10 + d % 10;
            }
        }
        sum += d;
        odd = !odd;
    }
    sum.is_multiple_of(10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luhn_valid_samples() {
        // 用户给的有效样例（标准 Luhn 校验位：`6222021234567890128` 总和 80，%10==0）
        assert!(luhn_check("6222021234567890128"));
        // 常见测试卡号（Visa / Mastercard / Amex 经典 Luhn 通过号）
        assert!(luhn_check("4111111111111111")); // Visa 测试号
        assert!(luhn_check("5500000000000004")); // Mastercard 测试号
        assert!(luhn_check("378282246310005")); // Amex 测试号
    }

    #[test]
    fn luhn_invalid_samples() {
        // 用户给的无效样例（`6222021234567890123` 总和 75，%10≠0）
        assert!(!luhn_check("6222021234567890123"));
        // 末位错一位
        assert!(!luhn_check("4111111111111112"));
    }

    #[test]
    fn luhn_edge_cases() {
        assert!(!luhn_check("")); // 空串
        assert!(!luhn_check("7")); // 单字符
        assert!(!luhn_check("12a4")); // 含非数字
                                      // 最短合法 Luhn：00 → 0+0=0；18 → 8 + (1×2=2) = 10 → 都 %10==0
        assert!(luhn_check("00"));
        assert!(luhn_check("18"));
        // 12 → 2 + (1×2=2) = 4 ≠ 0 → 无效
        assert!(!luhn_check("12"));
    }
}
