task_id: T2
reported_by: coder (主会话直接验证)
reported_at: 2026-08-19
reported_status: verified_complete
summary: |
  地址校验全部实现：后端 is_valid_address 增加英文检查 + 号/室可选范围；
  serde 向后兼容 (#[serde(default)])；前端规则管理、行级校验、inline 预览
  均传递 minHao/maxHao/minShi/maxShi 参数。
implementation_details:
  - crates/core/src/processor/validators/address.rs: 签名扩展为 5 参数；
    extract_number_before_keyword 辅助函数；英文检查 + 号/室范围检查。
  - crates/core/src/processor/rules/extract_params.rs: Address 改为 struct variant。
  - crates/core/src/processor/rules/builtins.rs: 种子更新。
  - crates/core/src/processor/rules/rule.rs: 测试更新 + 向后兼容测试。
  - crates/core/src/processor/validators/mod.rs: dispatch 分支 + 测试更新。
  - frontend: AddressParams.jsx、RulesPanel、RuleDetail、RuleRowParams、
    contractBuilder、inlineValidators、validateParams 全部更新。
verification_results:
  - cargo test -p ruT0-data-kit-core processor::validators: 44/44 ok
  - cargo test -p ruT0-data-kit-core processor::rules: 41/41 ok
  - cargo test -p ruT0-data-kit-core --doc: 15/15 ok