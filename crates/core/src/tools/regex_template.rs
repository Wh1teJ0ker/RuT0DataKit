//! 正则模板生成器。
//!
//! 预置一组常见场景的正则骨架（email / phone_cn / idcard_cn / ipv4 / url /
//! sql_injection_bool / sql_injection_union / mac），供前端直接复用或基于
//! `params` 微调。模板返回的字符串始终可被 `regex::Regex::new` 编译通过。
//!
//! 设计：
//! - `TemplateMeta` 暴露 name / description / params_schema，前端据此渲染表单。
//! - `generate_regex(template_name, params)` 接收 `HashMap<String, String>`
//!   参数；模板自身决定是否使用参数，未提供的参数使用默认值。
//! - 不支持的模板名 / 不识别的参数 → `CoreError::InvalidInput`。

use crate::error::CoreError;
use std::collections::HashMap;

/// 模板元信息（供前端渲染参数表单）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct TemplateMeta {
    /// 模板名（如 `email`）。
    pub name: String,
    /// 模板用途说明。
    pub description: String,
    /// 参数 schema：参数名 → 说明（空 Vec 表示无参模板）。
    pub params_schema: Vec<TemplateParam>,
}

/// 单个模板参数描述。
#[derive(Debug, Clone, serde::Serialize)]
pub struct TemplateParam {
    /// 参数键名。
    pub key: String,
    /// 参数说明。
    pub description: String,
    /// 是否必填。
    pub required: bool,
    /// 默认值（若有）。
    pub default: Option<String>,
}

/// 列出所有预置模板。
pub fn list_regex_templates() -> Vec<TemplateMeta> {
    templates().into_iter().map(|(m, _)| m).collect()
}

/// 按模板名 + 参数生成正则字符串。
///
/// 生成后会用 `regex::Regex::new` 做一次校验；编译失败时返回
/// `CoreError::InvalidInput`（理论上不应触发，仅作兜底）。
pub fn generate_regex(
    template_name: &str,
    params: &HashMap<String, String>,
) -> Result<String, CoreError> {
    let list = templates();
    let entry = list
        .iter()
        .find(|(m, _)| m.name == template_name)
        .ok_or_else(|| {
            CoreError::InvalidInput(format!("unknown regex template: {template_name}"))
        })?;
    let generated = (entry.1)(params)?;
    // 兜底校验：模板自身不应产出非法正则，但若 params 引入非法字符，这里捕获。
    if let Err(e) = regex::Regex::new(&generated) {
        return Err(CoreError::InvalidInput(format!(
            "generated regex for template '{template_name}' is invalid: {e}"
        )));
    }
    Ok(generated)
}

// ─────────────────────────────────────────────────────────────────────
// 内置模板定义（运行时构建，避免 const 上下文里调用 String::from）
// ─────────────────────────────────────────────────────────────────────

type Generator = fn(&HashMap<String, String>) -> Result<String, CoreError>;

fn templates() -> Vec<(TemplateMeta, Generator)> {
    vec![
        (
            TemplateMeta {
                name: "email".into(),
                description: "通用电子邮件地址".into(),
                params_schema: vec![],
            },
            email_gen,
        ),
        (
            TemplateMeta {
                name: "phone_cn".into(),
                description: "中国大陆手机号（11 位，1 开头）".into(),
                params_schema: vec![],
            },
            phone_cn_gen,
        ),
        (
            TemplateMeta {
                name: "idcard_cn".into(),
                description: "中国大陆身份证号（18 位，末位可为 X）".into(),
                params_schema: vec![],
            },
            idcard_cn_gen,
        ),
        (
            TemplateMeta {
                name: "ipv4".into(),
                description: "IPv4 地址（0-255 四段）".into(),
                params_schema: vec![],
            },
            ipv4_gen,
        ),
        (
            TemplateMeta {
                name: "url".into(),
                description: "HTTP(S) URL".into(),
                params_schema: vec![TemplateParam {
                    key: "protocol".into(),
                    description: "是否限定协议（http|https|both），默认 both".into(),
                    required: false,
                    default: Some("both".into()),
                }],
            },
            url_gen,
        ),
        (
            TemplateMeta {
                name: "sql_injection_bool".into(),
                description: "布尔型 SQL 注入特征（OR 1=1 / AND 1=1 等）".into(),
                params_schema: vec![],
            },
            sqli_bool_gen,
        ),
        (
            TemplateMeta {
                name: "sql_injection_union".into(),
                description: "UNION 型 SQL 注入特征".into(),
                params_schema: vec![],
            },
            sqli_union_gen,
        ),
        (
            TemplateMeta {
                name: "mac".into(),
                description: "MAC 地址（冒号或横线分隔）".into(),
                params_schema: vec![],
            },
            mac_gen,
        ),
    ]
}

fn email_gen(_p: &HashMap<String, String>) -> Result<String, CoreError> {
    Ok(r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$".into())
}

fn phone_cn_gen(_p: &HashMap<String, String>) -> Result<String, CoreError> {
    Ok(r"^1[3-9]\d{9}$".into())
}

fn idcard_cn_gen(_p: &HashMap<String, String>) -> Result<String, CoreError> {
    Ok(r"^\d{17}[\dXx]$".into())
}

fn ipv4_gen(_p: &HashMap<String, String>) -> Result<String, CoreError> {
    // 每段 0-255 的常见正则：25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d
    Ok(
        r"^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$"
            .into(),
    )
}

fn url_gen(p: &HashMap<String, String>) -> Result<String, CoreError> {
    let proto = p.get("protocol").cloned().unwrap_or_else(|| "both".into());
    let scheme = match proto.as_str() {
        "http" => "http",
        "https" => "https",
        "both" | "" => "https?",
        other => {
            return Err(CoreError::InvalidInput(format!(
                "invalid url.protocol value: {other} (expected http|https|both)"
            )));
        }
    };
    Ok(format!("^{scheme}://[A-Za-z0-9.-]+(:\\d+)?(/[^\\s]*)?$"))
}

fn sqli_bool_gen(_p: &HashMap<String, String>) -> Result<String, CoreError> {
    // 匹配 " OR 1=1 / ' OR '1'='1 等，大小写不敏感用 (?i)
    Ok(r"(?i)\b(or|and)\b\s+\d+\s*=\s*\d+".into())
}

fn sqli_union_gen(_p: &HashMap<String, String>) -> Result<String, CoreError> {
    Ok(r"(?i)\bunion\b\s+\bselect\b".into())
}

fn mac_gen(_p: &HashMap<String, String>) -> Result<String, CoreError> {
    Ok(r"^([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$".into())
}

// ─────────────────────────────────────────────────────────────────────
// 单元测试
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    fn compiles(s: &str) -> bool {
        Regex::new(s).is_ok()
    }

    #[test]
    fn test_list_templates_contains_all() {
        let names: Vec<_> = list_regex_templates().into_iter().map(|m| m.name).collect();
        for expected in [
            "email",
            "phone_cn",
            "idcard_cn",
            "ipv4",
            "url",
            "sql_injection_bool",
            "sql_injection_union",
            "mac",
        ] {
            assert!(names.iter().any(|n| n == expected), "missing {expected}");
        }
        assert_eq!(names.len(), 8);
    }

    #[test]
    fn test_email_template_exact() {
        let p = HashMap::new();
        let r = generate_regex("email", &p).unwrap();
        assert_eq!(r, r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$");
        assert!(compiles(&r));
        // 实际可匹配
        let re = Regex::new(&r).unwrap();
        assert!(re.is_match("foo.bar+test@example.com"));
        assert!(!re.is_match("not-an-email"));
    }

    #[test]
    fn test_phone_cn_template() {
        let r = generate_regex("phone_cn", &HashMap::new()).unwrap();
        assert!(compiles(&r));
        let re = Regex::new(&r).unwrap();
        assert!(re.is_match("13812345678"));
        assert!(!re.is_match("12345678901")); // 第二位不在 3-9
        assert!(!re.is_match("1381234567")); // 长度不对
    }

    #[test]
    fn test_idcard_cn_template() {
        let r = generate_regex("idcard_cn", &HashMap::new()).unwrap();
        let re = Regex::new(&r).unwrap();
        assert!(re.is_match("11010119900307012X"));
        assert!(re.is_match("110101199003070123"));
        assert!(!re.is_match("1101011990030701")); // 太短
    }

    #[test]
    fn test_ipv4_template() {
        let r = generate_regex("ipv4", &HashMap::new()).unwrap();
        let re = Regex::new(&r).unwrap();
        assert!(re.is_match("192.168.1.1"));
        assert!(re.is_match("255.255.255.255"));
        assert!(re.is_match("0.0.0.0"));
        assert!(!re.is_match("256.1.1.1"));
        assert!(!re.is_match("1.2.3"));
    }

    #[test]
    fn test_url_template_default() {
        let r = generate_regex("url", &HashMap::new()).unwrap();
        let re = Regex::new(&r).unwrap();
        assert!(re.is_match("https://example.com/path"));
        assert!(re.is_match("http://example.com:8080/x"));
        assert!(!re.is_match("ftp://example.com"));
    }

    #[test]
    fn test_url_template_https_only() {
        let mut p = HashMap::new();
        p.insert("protocol".into(), "https".into());
        let r = generate_regex("url", &p).unwrap();
        let re = Regex::new(&r).unwrap();
        assert!(re.is_match("https://example.com"));
        assert!(!re.is_match("http://example.com"));
    }

    #[test]
    fn test_url_template_invalid_param() {
        let mut p = HashMap::new();
        p.insert("protocol".into(), "ftp".into());
        assert!(generate_regex("url", &p).is_err());
    }

    #[test]
    fn test_sqli_templates_compile() {
        let r = generate_regex("sql_injection_bool", &HashMap::new()).unwrap();
        assert!(compiles(&r));
        let re = Regex::new(&r).unwrap();
        assert!(re.is_match(" OR 1=1"));
        assert!(re.is_match("or 1 = 1"));
        assert!(!re.is_match("select 1"));
        let r = generate_regex("sql_injection_union", &HashMap::new()).unwrap();
        let re = Regex::new(&r).unwrap();
        assert!(re.is_match("UNION SELECT"));
        assert!(!re.is_match("select 1"));
    }

    #[test]
    fn test_mac_template() {
        let r = generate_regex("mac", &HashMap::new()).unwrap();
        let re = Regex::new(&r).unwrap();
        assert!(re.is_match("AA:BB:CC:DD:EE:FF"));
        assert!(re.is_match("aa-bb-cc-dd-ee-ff"));
        assert!(!re.is_match("AABBCCDDEEFF"));
        assert!(!re.is_match("AA:BB:CC:DD:EE"));
    }

    #[test]
    fn test_unknown_template_errors() {
        assert!(generate_regex("nope", &HashMap::new()).is_err());
    }
}
