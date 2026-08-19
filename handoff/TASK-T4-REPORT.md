task_id: T4
reported_by: coder (主会话直接验证)
reported_at: 2026-08-19
reported_status: verified_complete
summary: |
  修复两个工具面板缺失的 Form/state 声明（ColumnOpsPanel + CryptoPanel），
  统一三个版本源为 1.2.2，全仓构建与测试验收通过。
implementation_details:
  - frontend/src/components/panels/ColumnOpsPanel.jsx: + parseForm/transformForm
    (Form.useForm) + parsing/transforming (useState)。
  - frontend/src/components/panels/CryptoPanel.jsx: + form (Form.useForm) +
    running/hashCase (useState)。
  - Cargo.toml / frontend/package.json / src-tauri/tauri.conf.json 版本 1.2.1 → 1.2.2。
verification_results:
  - pnpm --prefix frontend build: ✓ 3124 modules, built in 2.80s
  - cargo check --workspace: ✓ Finished dev profile, 0 error
  - cargo test -p ruT0-data-kit-core: 188 passed + 1 integration + 15 doc-tests, 0 failed
  - cargo test -p ruT0-data-kit: 141 passed, 0 failed