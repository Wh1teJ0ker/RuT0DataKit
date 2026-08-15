//! 地址结构化校验（中文 + 地址关键词）。

/// 地址关键词表（v1.1.4 续轮 T70：结构化地址校验）。
///
/// 命中任意一个即视为含地址语义（与中文 ≥ 2 同时满足）。
pub const ADDR_KEYWORDS: &[&str] = &[
    "省", "市", "区", "县", "镇", "乡", "村", "路", "街", "道", "号", "室", "楼", "单元", "栋",
    "幢", "弄", "巷", "里", "组", "旗", "盟", "社区", "大厦", "小区", "花园",
];

/// 地址校验：结构化校验（中文 ≥ 2 + 包含地址关键词）。
///
/// v1.1.4 续轮 T70：放宽原严格正则（号1-1500+室101-999），改为结构化校验：
/// - 中文字符数 ≥ 2
/// - 包含至少一个地址关键词（见 [`ADDR_KEYWORDS`]）：省/市/区/县/镇/乡/村/
///   路/街/道/号/室/楼/单元/栋/幢/弄/巷/里/组/旗/盟/社区/大厦/小区/花园
/// - 数字不做范围限制
/// - 总长度 4-200
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::is_valid_address;
/// assert!(is_valid_address("北京市朝阳区建国路88号"));
/// assert!(is_valid_address("内蒙古自治区呼和浩特市玉泉区大南街街道1340号540室"));
/// assert!(!is_valid_address("hello world"));
/// assert!(!is_valid_address("张三"));
/// assert!(!is_valid_address(""));
/// ```
pub fn is_valid_address(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.chars().count() < 4 || trimmed.chars().count() > 200 {
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
    ADDR_KEYWORDS.iter().any(|kw| trimmed.contains(kw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_valid_samples() {
        // T70：结构化校验（中文 ≥ 2 + 地址关键词）
        // 用户给的正例
        assert!(is_valid_address(
            "内蒙古自治区呼和浩特市玉泉区大南街街道1340号540室"
        ));
        assert!(is_valid_address("北京市朝阳区建国路88号"));
        assert!(is_valid_address("北京市朝阳区1号101室"));
        // T70：原号超 1500 / 室超 999 → 现在有效（不限制数字范围）
        assert!(is_valid_address("北京市朝阳区1501号101室"));
        assert!(is_valid_address("北京市朝阳区1号1000室"));
        // T70：含号/室关键词 → 有效
        assert!(is_valid_address("1234号101室"));
        // 边界：长度=4（3 CJK + 1 digit，含「路」关键词）
        assert!(is_valid_address("北京路1"));
    }

    #[test]
    fn address_invalid_samples() {
        // T70：结构化校验失败场景
        // 无中文
        assert!(!is_valid_address("hello world"));
        // 中文 < 2（单字且无关键词）
        assert!(!is_valid_address("张"));
        // 无地址关键词（2 CJK 但非关键词）
        assert!(!is_valid_address("张三"));
        assert!(!is_valid_address("李四王五"));
        // 空串
        assert!(!is_valid_address(""));
        // 仅空格
        assert!(!is_valid_address("   "));
    }
}
