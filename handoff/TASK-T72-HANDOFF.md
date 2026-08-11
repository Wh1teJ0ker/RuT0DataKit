# TASK-T72-HANDOFF

```yaml
task_id: T72
goal: |
  v1.1.4 续轮文档同步 + E2E 验收 + Release QA 增量审计：
  (1) 更新 更新日志.md / RELEASE-NOTES.md / 02-技术设计文档.md / TASK-BOARD.md；
  (2) 跑全量 E2E（cargo fmt/clippy/test + pnpm build）；
  (3) 更新 QA-审计报告.md 为覆盖 T70~T72 的增量审计。

depends_on: [T71]
status: planned
```

## in_scope

### 1. `docs/versions/1.1.4/更新日志.md`

- 状态行更新：追加 T70/T71/T72 进度
- 任务表追加 T70/T71/T72 行
- 新增「续轮：通用校验 + 地址放宽 + 生日清理 + 前端统一化」章节
- 验收项 E85~E96
- 设计决策追加：
  - 通用校验：ExtractParams::Generic 变体（字符类白名单 + 长度范围）
  - 地址校验放宽：结构化校验（中文≥2 + 地址关键词，不限制数字范围）
  - 出生日期清理：clean_birth 过滤非数字 → 8 位 + 日期有效性
  - params_override 契约：MultiRuleValidation 新增 paramsOverride 字段，ValidatePanel 每行可覆盖 DB 默认 params

### 2. `docs/versions/1.1.4/RELEASE-NOTES.md`

追加续轮功能：
- 通用校验规则（可选字符类 + 长度限制）
- 地址校验放宽为结构化校验
- 出生日期校验支持分隔符格式（自动清理）
- RulesPanel/ValidatePanel 参数 UI 统一化

### 3. `docs/02-技术设计文档.md`

- ExtractParams 枚举说明追加 Generic 变体
- MultiRuleValidation 结构体说明追加 paramsOverride 字段
- is_valid_address 说明更新（结构化校验）
- is_valid_birth 说明更新（clean_birth + 日期有效性）

### 4. `handoff/TASK-BOARD.md`

- T70/T71/T72 状态更新为 verified_complete / done_e2e / qa_passed

### 5. `docs/qa/versions/1.1.4/QA-审计报告.md`

- 追加「续轮 T70~T72 增量审计」章节
- 覆盖 8 维度增量审计
- 结论更新为 qa_passed（续轮）

## out_of_scope

- 不改代码（T70/T71 负责）
- 不创建 git tag（v1.1.4 续轮 Release QA 通过前不打）

## 验收项

- **E97**：`cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all` 全绿
- **E98**：`pnpm --prefix frontend build` 全绿
- **E99**：版本号 4 处一致 1.1.4
- **E100**：QA 报告结论 qa_passed（续轮）

## verification_commands

```sh
cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all
pnpm --prefix frontend build
```

## files_likely_to_change

- `docs/versions/1.1.4/更新日志.md`
- `docs/versions/1.1.4/RELEASE-NOTES.md`
- `docs/02-技术设计文档.md`
- `handoff/TASK-BOARD.md`
- `docs/qa/versions/1.1.4/QA-审计报告.md`

## risks

- QA 报告需准确反映续轮变更范围（增量审计，非全量重审）
- 文档状态需与实际代码一致

## 安全约束

- 无凭据字面量
- 不创建 tag
- Mimosa 完整审计未拿到结论前不宣称安全
