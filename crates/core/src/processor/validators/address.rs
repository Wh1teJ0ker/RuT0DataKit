//! 地址结构化校验（全中文 + 地址关键词 + 可选号/室范围）。

/// 地址关键词表（v1.1.4 续轮 T70：结构化地址校验）。
///
/// 命中任意一个即视为含地址语义（与中文 ≥ 2 同时满足）。
pub const ADDR_KEYWORDS: &[&str] = &[
    "省", "市", "区", "县", "镇", "乡", "村", "路", "街", "道", "号", "室", "楼", "单元", "栋",
    "幢", "弄", "巷", "里", "组", "旗", "盟", "社区", "大厦", "小区", "花园",
];

/// 从 `s` 中提取关键词 `keyword` 前紧邻的数字。
///
/// 例如 `extract_number_before_keyword("5189号375室", "号")` → `Some(5189)`。
/// 找不到关键词或关键词前无数字 → `None`。
fn extract_number_before_keyword(s: &str, keyword: &str) -> Option<u32> {
    let pos = s.rfind(keyword)?;
    let before = &s[..pos];
    // 从 before 末尾向前扫描连续数字。
    let digits: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

/// 地址校验：结构化校验（全中文无英文字母 + 地址关键词 + 可选号/室范围）。
///
/// v1.1.4 T70：放宽原严格正则（号1-1500+室101-999），改为结构化校验：
/// - 中文字符数 ≥ 2
/// - 包含至少一个地址关键词（见 [`ADDR_KEYWORDS`]）
/// - **v1.2.2：不含英文字母**（`[a-zA-Z]`），全中文字符
/// - **v1.2.2：可选号/室数字范围**（`min_hao`/`max_hao`/`min_shi`/`max_shi` 为 `Some` 时检查）
/// - 总长度 4-200
///
/// `min_hao`/`max_hao`/`min_shi`/`max_shi` 全 `None` = 不检查数字范围（向后兼容）。
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::is_valid_address;
/// assert!(is_valid_address("北京市朝阳区建国路88号", None, None, None, None));
/// assert!(is_valid_address("内蒙古自治区呼和浩特市玉泉区大南街街道1340号540室", None, None, None, None));
/// assert!(!is_valid_address("hello world", None, None, None, None));
/// assert!(!is_valid_address("张三", None, None, None, None));
/// assert!(!is_valid_address("", None, None, None, None));
/// // v1.2.2：含英文字母 → 不通过
/// assert!(!is_valid_address("吉林省长春T朝阳区前进街道4342号1323室", None, None, None, None));
/// // v1.2.2：号范围校验
/// assert!(is_valid_address("北京市朝阳区88号101室", Some(1), Some(1500), Some(101), Some(999)));
/// assert!(!is_valid_address("北京市朝阳区5189号375室", Some(1), Some(1500), None, None));
/// ```
pub fn is_valid_address(
    s: &str,
    min_hao: Option<u32>,
    max_hao: Option<u32>,
    min_shi: Option<u32>,
    max_shi: Option<u32>,
) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.chars().count() < 4 || trimmed.chars().count() > 200 {
        return false;
    }
    // v1.2.2：不含英文字母（全中文地址）。
    if trimmed.chars().any(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    // 中文字符数（基本区 U+4E00..U+9FA5）
    let cjk_count = trimmed
        .chars()
        .filter(|&c| ('\u{4e00}'..='\u{9fa5}').contains(&c))
        .count();
    if cjk_count < 2 {
        return false;
    }
    // 包含至少一个地址关键词
    if !ADDR_KEYWORDS.iter().any(|kw| trimmed.contains(kw)) {
        return false;
    }
    // v1.2.2：可选号范围校验。
    if let Some(num) = extract_number_before_keyword(trimmed, "号") {
        if let Some(min) = min_hao {
            if num < min {
                return false;
            }
        }
        if let Some(max) = max_hao {
            if num > max {
                return false;
            }
        }
    }
    // v1.2.2：可选室范围校验。
    if let Some(num) = extract_number_before_keyword(trimmed, "室") {
        if let Some(min) = min_shi {
            if num < min {
                return false;
            }
        }
        if let Some(max) = max_shi {
            if num > max {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_valid_samples() {
        // T70：结构化校验（中文 ≥ 2 + 地址关键词）
        // 用户给的正例
        assert!(is_valid_address(
            "内蒙古自治区呼和浩特市玉泉区大南街街道1340号540室",
            None, None, None, None,
        ));
        assert!(is_valid_address("北京市朝阳区建国路88号", None, None, None, None));
        assert!(is_valid_address("北京市朝阳区1号101室", None, None, None, None));
        // 边界：长度=4（3 CJK + 1 digit，含「路」关键词）
        assert!(is_valid_address("北京路1", None, None, None, None));
        // 号/室在范围内
        assert!(is_valid_address(
            "重庆市江津区几江街道260号228室",
            Some(1), Some(1500), Some(101), Some(999),
        ));
    }

    #[test]
    fn address_invalid_samples() {
        // T70：结构化校验失败场景
        // 无中文
        assert!(!is_valid_address("hello world", None, None, None, None));
        // 中文 < 2（单字且无关键词）
        assert!(!is_valid_address("张", None, None, None, None));
        // 无地址关键词（2 CJK 但非关键词）
        assert!(!is_valid_address("张三", None, None, None, None));
        assert!(!is_valid_address("李四王五", None, None, None, None));
        // 空串
        assert!(!is_valid_address("", None, None, None, None));
        // 仅空格
        assert!(!is_valid_address("   ", None, None, None, None));
    }

    #[test]
    fn address_english_letter_rejected() {
        // v1.2.2：含英文字母 → 不通过
        assert!(!is_valid_address(
            "内蒙古自治区呼和O特市托克托县古城镇1319号139室",
            None, None, None, None,
        ));
        assert!(!is_valid_address(
            "吉林省长春T朝阳区前进街道4342号1323室",
            None, None, None, None,
        ));
    }

    #[test]
    fn address_hao_shi_range_check() {
        // 号在范围内 → 通过
        assert!(is_valid_address(
            "重庆市江津区几江街道260号228室",
            Some(1), Some(1500), Some(101), Some(999),
        ));
        // 号超出范围 → 不通过
        assert!(!is_valid_address(
            "天津市河西区下瓦房街道5189号375室",
            Some(1), Some(1500), None, None,
        ));
        // 室超出范围 → 不通过
        assert!(!is_valid_address(
            "北京市朝阳区1号1000室",
            None, None, Some(101), Some(999),
        ));
        // 号/室都在范围外 → 不通过
        assert!(!is_valid_address(
            "吉林省长春T朝阳区前进街道4342号1323室",
            Some(1), Some(1500), Some(101), Some(999),
        ));
        // 无号/室关键词 → 范围校验跳过（不影响）
        assert!(is_valid_address(
            "北京市朝阳区建国路88号",
            Some(1), Some(1500), Some(101), Some(999),
        ));
    }
}
