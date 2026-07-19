//! 删除脱敏器：对任意输入返回空串 `""`。无参数。

use std::collections::HashMap;

use serde_yml::Value;

use crate::maskers::Masker;

/// 删除脱敏器。
#[derive(Clone, Debug, Default)]
pub struct DeleteMask;

impl DeleteMask {
    pub fn new(_params: HashMap<String, Value>) -> Self {
        Self
    }
}

impl Masker for DeleteMask {
    fn mask(&self, _value: &str) -> String {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_empty_for_any_input() {
        let m = DeleteMask::new(HashMap::new());
        assert_eq!(m.mask("anything"), "");
        assert_eq!(m.mask(""), "");
        assert_eq!(m.mask("中文测试"), "");
    }
}
