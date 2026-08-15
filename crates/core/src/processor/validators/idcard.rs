//! 身份证号校验（GB 11643-1999 校验码 + 性别推断）。

/// 身份证号校验码系数（GB 11643-1999），第一位到第十七位依次乘。
const IDCARD_WEIGHTS: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];

/// 身份证号校验码查表（余数 0..=10 → 校验码）。
const IDCARD_CHECK_CODES: [char; 11] = ['1', '0', 'X', '9', '8', '7', '6', '5', '4', '3', '2'];

/// 身份证号严格校验（GB 11643-1999 校验码算法）。
///
/// 规则：18 位，前 17 位纯 ASCII 数字，第 18 位为数字或 'X'/'x'。
/// 前 17 位分别乘 [`IDCARD_WEIGHTS`]，加权和 mod 11，查
/// [`IDCARD_CHECK_CODES`] 得第 18 位校验码，匹配则有效。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::is_valid_idcard;
/// assert!(is_valid_idcard("11010519491231002X"));  // 校验码 X
/// assert!(is_valid_idcard("110105194912310038"));  // 校验码 8
/// assert!(is_valid_idcard("11010519491231002x"));  // 小写 x
/// assert!(!is_valid_idcard("110105194912310021")); // 校验码错
/// assert!(!is_valid_idcard("12345"));              // 长度不足
/// assert!(!is_valid_idcard("11010519491231002A"));// 非法末位
/// ```
pub fn is_valid_idcard(s: &str) -> bool {
    if s.len() != 18 {
        return false;
    }
    let bytes = s.as_bytes();
    // 前 17 位必须是 ASCII 数字
    let digits: Vec<u32> = match bytes[..17]
        .iter()
        .map(|b| (b - b'0') as u32)
        .collect::<Vec<u32>>()
    {
        ds if bytes[..17].iter().all(|b| b.is_ascii_digit()) => ds,
        _ => return false,
    };
    // 第 18 位：数字或 'X'/'x'（大写化后比对）
    let last = bytes[17];
    if !last.is_ascii_digit() && last != b'X' && last != b'x' {
        return false;
    }
    let expected = if last.is_ascii_digit() {
        last as char
    } else {
        'X'
    };
    // 加权求和 mod 11 → 查表
    let sum: u32 = digits
        .iter()
        .zip(IDCARD_WEIGHTS.iter())
        .map(|(d, w)| d * w)
        .sum();
    let check = IDCARD_CHECK_CODES[(sum % 11) as usize];
    check == expected
}

/// 从身份证号第 17 位（顺序码末位，0-indexed 16）推断性别。
///
/// 奇数 → `'男'`，偶数 → `'女'`。输入非合法身份证前 17 位（长度不足或
/// 第 17 位非数字）返回 `None`。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::idcard_gender;
/// assert_eq!(idcard_gender("110105194912310038"), Some('男')); // 第 17 位=3 奇
/// assert_eq!(idcard_gender("11010519491231002X"), Some('女')); // 第 17 位=2 偶
/// assert_eq!(idcard_gender("123"), None);                       // 长度不足
/// ```
pub fn idcard_gender(s: &str) -> Option<char> {
    let bytes = s.as_bytes();
    if bytes.len() < 17 || !bytes[16].is_ascii_digit() {
        return None;
    }
    let d = (bytes[16] - b'0') % 2;
    if d == 1 {
        Some('男')
    } else {
        Some('女')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idcard_valid_samples() {
        // 校验码 X（余数 2），性别女（第 17 位 2 偶）
        assert!(is_valid_idcard("11010519491231002X"));
        // 校验码 8（余数 4），性别男（第 17 位 3 奇）
        assert!(is_valid_idcard("110105194912310038"));
        // 小写 x
        assert!(is_valid_idcard("11010519491231002x"));
    }

    #[test]
    fn idcard_invalid_samples() {
        // 校验码错（应为 X，给 1）
        assert!(!is_valid_idcard("110105194912310021"));
        // 长度不足
        assert!(!is_valid_idcard("12345"));
        // 长度超
        assert!(!is_valid_idcard("11010519491231002X1"));
        // 前 17 位含非数字
        assert!(!is_valid_idcard("11010519491231002A"));
        // 空串
        assert!(!is_valid_idcard(""));
    }

    #[test]
    fn idcard_gender_inference() {
        // 第 17 位（0-indexed 16）= 3 奇 → 男
        assert_eq!(idcard_gender("110105194912310038"), Some('男'));
        // 第 17 位 = 2 偶 → 女
        assert_eq!(idcard_gender("11010519491231002X"), Some('女'));
        // 第 17 位 = 0 偶 → 女
        assert_eq!(idcard_gender("110105194912310000"), Some('女'));
        // 第 17 位 = 9 奇 → 男
        assert_eq!(idcard_gender("110105194912310090"), Some('男'));
        // 长度不足 → None
        assert_eq!(idcard_gender("123"), None);
        // 第 17 位非数字 → None
        assert_eq!(idcard_gender("1101051949123100X8"), None);
    }
}
