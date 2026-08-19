task_id: T3
reported_by: coder (主会话直接验证)
reported_at: 2026-08-19
reported_status: verified_complete
summary: |
  出生日期格式持久化与身份证首位为零提取开关已实现：IPC 参数一致，
  身份证开关仅运行时覆盖不持久化，有后端测试覆盖。
implementation_details:
  - frontend/src/components/panels/ExtractPanel.jsx: 新增 idcardAllowLeadingZero
    Switch 组件，运行时覆盖正则。
  - frontend/src/components/panels/RulesPanel.jsx: birth-validate 格式编辑器
    (draftBirthFormats) + address-validate 参数编辑器 (draftAddressParams)。
  - frontend/src/components/panels/rule/RuleDetail.jsx: 接入 BirthParams +
    AddressParams 组件。
  - frontend/src/components/panels/rule/constants.js: BIRTH_FORMATS 常量。
  - frontend/src/tauri/extract.js: idcardAllowLeadingZero 参数传递。
  - src-tauri/src/commands/processor/extract_ops.rs: 运行时宽松正则覆盖 +
    测试 extract_validate_idcard_leading_zero_runtime_override。
verification_results:
  - cargo test -p ruT0-data-kit processor::extract_ops: 25/25 ok