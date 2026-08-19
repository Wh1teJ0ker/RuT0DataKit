task_id: T4
goal: |
  修复工具面板缺失的 React 初始化声明，统一版本为 1.2.2，并完成全仓验证收口。
in_scope:
  - frontend/src/components/panels/ColumnOpsPanel.jsx
  - frontend/src/components/panels/CryptoPanel.jsx
  - Cargo.toml
  - frontend/package.json
  - src-tauri/tauri.conf.json
  - docs/versions/1.2.2/
out_of_scope:
  - 业务规则范围扩展
  - CTF answer 与测试样例文件
  - git tag 与 Release QA
acceptance_criteria:
  - 两个面板均创建 Form 实例和全部引用的 state。
  - 三个版本源都为 1.2.2。
  - 全部端到端验证命令通过。
verification_commands:
  - cargo test -p ruT0-data-kit-core
  - cargo test -p ruT0-data-kit
  - cargo check --workspace
  - pnpm --prefix frontend build
files_likely_to_change:
  - frontend/src/components/panels/ColumnOpsPanel.jsx
  - frontend/src/components/panels/CryptoPanel.jsx
  - Cargo.toml
  - frontend/package.json
  - src-tauri/tauri.conf.json
  - docs/versions/1.2.2/
risks:
  - 不得恢复用户已删除的测试样例或触碰 CTF answer 文件。
depends_on:
  - T1
  - T2
  - T3
status: planned
