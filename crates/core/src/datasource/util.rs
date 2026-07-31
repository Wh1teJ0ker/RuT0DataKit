//! 数据源读取器共享 helper。

use calamine::Data;

/// `calamine::Data` 转字符串。
pub fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::DateTimeIso(dt) => dt.to_string(),
        Data::DurationIso(d) => d.to_string(),
        Data::DateTime(_dt) => format!("{:?}", cell),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => {
            // 整数浮点避免尾部 `.0`。
            if f.fract() == 0.0 && f.is_finite() {
                format!("{}", *f as i64)
            } else {
                format!("{}", f)
            }
        }
        Data::Bool(b) => b.to_string(),
        Data::Error(err) => format!("{:?}", err),
    }
}

/// `serde_json::Value` → 字符串。数组和对象序列化保留原始结构。
pub fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => v.to_string(),
    }
}
