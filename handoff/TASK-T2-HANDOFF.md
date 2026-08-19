task_id: T2
goal: |
  地址规则在后端和前端统一支持全中文校验及可选的号/室数字范围。
in_scope:
  - crates/core/src/processor/rules/extract_params.rs
  - crates/core/src/processor/rules/builtins.rs
  - crates/core/src/processor/rules/rule.rs
  - crates/core/src/processor/validators/address.rs
  - crates/core/src/processor/validators/mod.rs
  - frontend/src/components/panels/RulesPanel.jsx
  - frontend/src/components/panels/rule/
  - frontend/src/components/panels/validate/
  - frontend/src/components/panels/validateParams.js
  - frontend/src/utils/inlineValidators.js
out_of_scope:
  - SCHEMA_VERSION 与迁移
  - 非地址 validator 的行为
  - CTF answer 与测试样例文件
acceptance_criteria:
  - 旧 address 参数 JSON 反序列化为不限范围。
  - 含 ASCII 英文字母的地址不通过；地址关键词和范围校验正常。
  - 规则管理、行级校验契约和本地预览均传递 minHao/maxHao/minShi/maxShi。
verification_commands:
  - cargo test -p ruT0-data-kit-core processor::validators
  - pnpm --prefix frontend build
files_likely_to_change:
  - crates/core/src/processor/rules/
  - crates/core/src/processor/validators/
  - frontend/src/components/panels/
  - frontend/src/utils/inlineValidators.js
risks:
  - Rust serde 字段保持 camelCase 且默认值保证现有规则数据兼容。
depends_on: []
status: planned
