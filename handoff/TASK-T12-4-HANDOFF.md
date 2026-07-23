# TASK-T12-4-HANDOFF — 建 rules/patterns.rs 集中正则表 + 消除 phone/ip 正则散布

```yaml
task_id: T12-4
goal: |
  新建 crates/core/src/rules/patterns.rs，集中定义 8 种 scope 的 extract 正则
  + validate 正则（成对），消除 scan/mod.rs::extract_pattern 与 validators/*.rs
  各自硬编码的双正则源。scan/extract_pattern 与 validators/phone.rs / ip.rs
  / mac.rs 等统一引用 patterns.rs 常量。行为零回归。
in_scope:
  - crates/core/src/rules/patterns.rs（NEW）
  - crates/core/src/rules/presets.rs（迁移已有 IP_REGEX 等到 patterns.rs 或反向合并）
  - crates/core/src/rules/mod.rs（加 pub mod patterns）
  - crates/core/src/scan/mod.rs（extract_pattern 改查 patterns.rs）
  - crates/core/src/validators/phone.rs（硬编码 ^1\d{10}$ 改引用 patterns.rs）
  - crates/core/src/validators/ip.rs（改引用 patterns.rs）
  - crates/core/src/validators/mac.rs（如有硬编码改引用）
  - crates/core/src/rules/operator.rs 区域（RegexWithGuard 守卫 phone 正则改引用）
out_of_scope:
  - 不得改 tools/ 下正则模板/解析（regex_template.rs / regex_explain.rs / regex_construct.rs 是正则工具示例，非业务正则源，保留硬编码示例）
  - 不得改 frontend/（UI placeholder 中的 ^1[3-9]\d{9}$ 是提示文案，非业务正则，保留）
  - 不得动 tests/ 测试断言中的正则字面量（测试是验证行为，不是正则源）
  - 不得改 builtin.rs 内置规则的 scope 映射（只改正则引用源）
acceptance_criteria:
  - patterns.rs 集中定义 8 种 scope 的 extract_pattern + validate_pattern 常量
  - scan/mod.rs::extract_pattern 改为查 patterns.rs 表（函数体仅 match scope → return 常量）
  - validators/phone.rs 不再有硬编码 r"^1\d{10}$"（改引用 patterns::PHONE_VALIDATE）
  - validators/ip.rs 不再有硬编码（改引用 patterns::IP_VALIDATE）
  - grep -rn "1\\\\d{10}" crates/core/src/{validators,scan,rules} 仅命中 patterns.rs 一处
  - cargo test --workspace 全绿（基线 445 passed）
  - cargo build --workspace 0 error
verification_commands:
  - cargo build --workspace
  - cargo test --workspace
  - grep -rn '1\\d{10}' crates/core/src/validators/phone.rs crates/core/src/scan/mod.rs crates/core/src/rules/operator.rs | grep -v patterns.rs | wc -l（预期 0）
files_likely_to_change:
  - crates/core/src/rules/patterns.rs
  - crates/core/src/rules/mod.rs
  - crates/core/src/scan/mod.rs
  - crates/core/src/validators/phone.rs
  - crates/core/src/validators/ip.rs
  - crates/core/src/rules/mask_op.rs（T12-3 产出）/ validate_op.rs
risks:
  - presets.rs 已有 9 个正则常量，patterns.rs 要么吸收 presets.rs（改所有引用）要么与 presets.rs 并存（patterns.rs 只放 scope 成对正则）。优先方案：patterns.rs 吸收 presets.rs，presets.rs 删除或改为 re-export 兼容层
  - IP_REGEX 在 presets.rs 已是 validate 形态（^...$），scan 用的是 extract 形态（\b...\b），两者不同 → patterns.rs 必须区分 EXTRACT vs VALIDATE 两套
depends_on: [T12-3]
status: planned
```

## 集中正则表设计

patterns.rs 应定义：

```rust
/// 按 scope 分组的正则对：extract（宽松召回候选）+ validate（严格过滤误报）。
/// 所有业务模块统一引用本表，消除散布硬编码。

pub mod scope {
    pub const IDCARD: ScopePatterns = ScopePatterns {
        extract: r"\b\d{17}[\dXx]\b",
        validate: r"^\d{17}[\dXx]$",
    };
    pub const PHONE: ScopePatterns = ScopePatterns {
        extract: r"\b\d{11}\b",
        validate: r"^1\d{10}$",
    };
    // ... bankcard / email / ip / mac / username / name
}

pub struct ScopePatterns {
    pub extract: &'static str,
    pub validate: &'static str,
}

pub fn extract_pattern(scope: &str) -> Option<&'static str> { ... }
pub fn validate_pattern(scope: &str) -> Option<&'static str> { ... }
```

scan/mod.rs 与 validators/*.rs 统一引用 `patterns::scope::PHONE.validate` 等。
