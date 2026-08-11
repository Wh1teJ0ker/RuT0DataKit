# TASK-T68-REPORT

```yaml
implemented_changes:
  - src-tauri/src/commands/processor.rs：
      新增 CrossFieldConfig（camelCase，checkSex/sexColumn/checkBirth/birthColumn）
      与 MultiRuleValidation（column/ruleId/crossField?）结构体；
      新增 validate_multi_rules_to_two_sheets_inner(db, sheet_id, session_id,
      rules, phone_prefixes) -> Result<TwoSheetResult, String> 与
      #[tauri::command] 薄包装 validate_multi_rules_to_two_sheets；
      校验分发：phone-validate → is_valid_phone（整串 11 位+1 开头+前缀
      白名单），rule.params 非空 → validate_extracted（函数式 Username/Sex/
      Birth/IdCard/Address/PhonePrefix/Luhn/Ipv4/Ipv6 分发），rule.pattern
      非空 → 正则 is_match（如 name-validate），其他 → 默认通过；
      身份证跨字段联合校验（仅当 idcard-validate 规则带 crossField 配置
      且 idcard 本身有效）：normalize_gender(sex_col_val) vs idcard_gender
      (idcard_val)、is_valid_birth(birth_val) 且 idcard[6..14] == birth_val；
      整行分流（任一失败 → invalid + RowInvalidReason，全通过 → valid）；
      双 Tab 写出（{源sheet名}_校验通过/_校验失败，保留原列不新增）+
      log_operation("validate_multi_rules_to_two_sheets", ...)；
      复用 TwoSheetResult / RowInvalidReason / ParseResult 结构与
      validate_rows_to_two_sheets_inner 的双 Tab 写出模式。
  - src-tauri/src/commands/processor.rs tests mod：
      新增 setup_multi_rules_sheet 测试夹具 + 6 条集成测试：
      _all_rules_pass / _mixed_pass_fail / _idcard_cross_field_sex_mismatch
      / _idcard_cross_field_birth_mismatch / _regex_rule_validation
      / _partial_mapping；测试 use 语句补 CrossFieldConfig /
      MultiRuleValidation / validate_multi_rules_to_two_sheets_inner 导入。
  - src-tauri/src/lib.rs：generate_handler! 列表在
    validate_rows_to_two_sheets 后追加
    commands::processor::validate_multi_rules_to_two_sheets。
  - docs/02-技术设计文档.md：§4.6 处理命令表追加
    validate_multi_rules_to_two_sheets 行（签名、校验分发、跨字段、双 Tab、
    log_operation kind）+ MultiRuleValidation / CrossFieldConfig 结构签名。

verification_run:
  - cargo fmt --all
  - cargo clippy --all-targets --all-features -- -D warnings
  - cargo test --all

verification_results:
  - cargo fmt --all：通过（自动格式化 1 处）。
  - cargo clippy --all-targets --all-features -- -D warnings：初次编译报
    E0308（rr.rule.params.as_ref() == Some(ExtractParams::IdCard) 类型不匹配，
    Some 需要 &ExtractParams）；修正为 Some(&ExtractParams::IdCard) 后通过，
    Finished dev profile，零告警。
  - cargo test --all：全部通过。src-tauri (lib + tests) 129 + 0 + 0；
    core 152 + 0 + 3 ignored；doc-tests core 11 + 0；lib doc 0 + 0。
    新增 6 条 T68 测试全绿：
    validate_multi_rules_to_two_sheets_all_rules_pass / _mixed_pass_fail /
    _idcard_cross_field_sex_mismatch / _idcard_cross_field_birth_mismatch
    / _regex_rule_validation / _partial_mapping。

docs_updated:
  - docs/02-技术设计文档.md（§4.6 处理命令表 + 结构签名）

commit_summary:
  - feat(processor): add validate_multi_rules_to_two_sheets command (T68)
      （src-tauri/src/commands/processor.rs + src-tauri/src/lib.rs）
  - docs(design): register validate_multi_rules_to_two_sheets IPC contract (T68)
      （docs/02-技术设计文档.md）

reported_status:
  - verified_complete

scope_deviation:
  - docs/02-技术设计文档.md §4.6 追加新命令契约行 + 结构签名。
    HANDOFF in_scope 未列 docs/，但 04-coder-spec.md §4.5 要求「真实行为 /
    用法变化时同步更新 docs/」。新命令是新增 IPC 契约，属真实行为变化，
    据此同步设计文档；纯追加，未改既有命令描述。除此之外无越界改动。
```

## 关键文件路径

- 实现：/Users/joker/code/RuT0DataKit/src-tauri/src/commands/processor.rs
- 注册：/Users/joker/code/RuT0DataKit/src-tauri/src/lib.rs
- 文档：/Users/joker/code/RuT0DataKit/docs/02-技术设计文档.md
- 报告：/Users/joker/code/RuT0DataKit/handoff/TASK-T68-REPORT.md

## 实现要点

- 复用 `validate_rows_to_two_sheets_inner` 的双 Tab 写出闭包模式（`write_sheet`：表头 row_idx=0 + 数据行从 row_idx=1 起，列数 = headers.len()，保留原列不新增）。
- 校验分发按 HANDOFF §3 规格实现：`phone-validate` 走 `is_valid_phone`（不走 `validate_extracted` 的 PhonePrefix 分支，因后者只查前缀不查 11 位/1 开头，符合 risks 注明的约束）。
- 身份证跨字段：仅当 idcard-validate 规则带 `crossField` 配置且 idcard 本身校验通过时触发；性别比对走 `normalize_gender`（接受中英文）vs `idcard_gender`；出生日期比对要求 birth 是 8 位数字（`is_valid_birth`）后取 `idcard[6..14]` 切片。
- 每条规则的正则（`rule.pattern`）在循环外预编译一次（`ResolvedRule.re`），避免逐行重复编译。
- `log_operation` 的 params JSON 含 rules 列表（column + ruleId），不写入 before/after 快照（新 sheet 不进撤销栈，与 `validate_rows_to_two_sheets` / `extract_validate_to_new_sheet` 一致）。
