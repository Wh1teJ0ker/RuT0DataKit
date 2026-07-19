//! 替换脱敏器：对任意输入返回 `with` 字符串。
//!
//! 参数：
//! - `with`（String，必填）：替换为的目标字符串。缺失时退化为空串。

use std::collections::HashMap;

use serde_yml::Value;

use crate::maskers::Masker;

/// 替换脱敏器。
#[derive(Clone, Debug)]
pub struct ReplaceMask {
    with: String,
}

impl ReplaceMask {
    pub fn new(params: HashMap<String, Value>) -> Self {
        let with = params
            .get("with")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Self { with }
    }
}

impl Masker for ReplaceMask {
    fn mask(&self, _value: &str) -> String {
        self.with.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(p: &[(&str, &str)]) -> HashMap<String, Value> {
        p.iter()
            .map(|(k, v)| (k.to_string(), Value::String((*v).to_string())))
            .collect()
    }

    #[test]
    fn spec_example_replace_with_redacted() {
        let p = params(&[("with", "REDACTED")]);
        let m = ReplaceMask::new(p);
        assert_eq!(m.mask("anything"), "REDACTED");
        assert_eq!(m.mask(""), "REDACTED");
    }

    #[test]
    fn missing_with_defaults_empty() {
        let m = ReplaceMask::new(HashMap::new());
        assert_eq!(m.mask("anything"), "");
    }
}
