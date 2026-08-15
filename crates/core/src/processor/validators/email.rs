//! 邮箱地址校验（RFC 5321 简化）。

/// 邮箱地址校验：结构化校验（local@domain，RFC 5321 简化）。
///
/// v1.1.5 T81 新增。规则：
/// - trim 后非空，总长度 ≤ 254
/// - 含恰好 1 个 `@`
/// - local 部分非空、≤ 64 字符、仅允许 `[a-zA-Z0-9._%+-]`
/// - domain 部分非空、含至少 1 个 `.`、每段非空、仅允许 `[a-zA-Z0-9.-]`
///
/// # 示例
/// ```
/// use ruT0_data_kit_core::processor::validators::is_valid_email;
/// assert!(is_valid_email("user@example.com"));
/// assert!(is_valid_email("user.name@domain.co"));
/// assert!(is_valid_email("a@b.c"));
/// assert!(!is_valid_email("@b.com"));
/// assert!(!is_valid_email("a@"));
/// assert!(!is_valid_email("a@b"));
/// assert!(!is_valid_email("a b@c.com"));
/// assert!(!is_valid_email(""));
/// ```
pub fn is_valid_email(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.len() > 254 {
        return false;
    }
    let at_count = trimmed.matches('@').count();
    if at_count != 1 {
        return false;
    }
    let mut parts = trimmed.splitn(2, '@');
    let local = parts.next().unwrap_or("");
    let domain = parts.next().unwrap_or("");
    if local.is_empty() || local.len() > 64 {
        return false;
    }
    if domain.is_empty() || !domain.contains('.') {
        return false;
    }
    // local 部分仅允许 [a-zA-Z0-9._%+-]
    if !local
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'%' | b'+' | b'-'))
    {
        return false;
    }
    // domain 每段非空、仅允许 [a-zA-Z0-9.-]
    if domain
        .split('.')
        .any(|seg| seg.is_empty() || !seg.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_valid_samples() {
        assert!(is_valid_email("user@example.com"));
        assert!(is_valid_email("user.name@domain.co"));
        assert!(is_valid_email("a@b.c"));
    }

    #[test]
    fn email_invalid_samples() {
        assert!(!is_valid_email("@b.com"));
        assert!(!is_valid_email("a@"));
        assert!(!is_valid_email("a@b"));
        assert!(!is_valid_email("a b@c.com"));
        assert!(!is_valid_email(""));
    }
}
