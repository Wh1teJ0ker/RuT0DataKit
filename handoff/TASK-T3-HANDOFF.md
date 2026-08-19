task_id: T3
goal: |
  使出生日期格式和身份证首位为零提取开关在规则界面、前端 IPC 与 Rust 运行时逻辑中一致。
in_scope:
  - frontend/src/components/panels/ExtractPanel.jsx
  - frontend/src/components/panels/RulesPanel.jsx
  - frontend/src/components/panels/rule/
  - frontend/src/tauri/extract.js
  - src-tauri/src/commands/processor/extract_ops.rs
out_of_scope:
  - 身份证校验算法
  - 规则数据库 schema
  - CTF answer 与测试样例文件
acceptance_criteria:
  - 出生日期格式选择可保存、重置和传入规则运行预览。
  - 开启身份证开关时仅本次运行使用允许首位 0 的提取正则，不持久化规则。
  - IPC 参数在 JavaScript wrapper 和 Tauri command 之间名称、默认值一致，并有后端测试。
verification_commands:
  - cargo test -p ruT0-data-kit processor::extract_ops
  - pnpm --prefix frontend build
files_likely_to_change:
  - frontend/src/components/panels/ExtractPanel.jsx
  - frontend/src/components/panels/RulesPanel.jsx
  - frontend/src/components/panels/rule/
  - frontend/src/tauri/extract.js
  - src-tauri/src/commands/processor/extract_ops.rs
risks:
  - 运行时覆盖不可改变已保存的规则 pattern 或 params。
depends_on: []
status: planned
