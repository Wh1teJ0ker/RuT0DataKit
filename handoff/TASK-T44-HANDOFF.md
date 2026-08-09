```yaml
task_id: T44
goal: |
  v1.1.2 文档收口 + 全量验证 + Release QA 审计 → qa_passed。
in_scope:
  - docs/versions/1.1.2/规划需求.md（已存在，核对）
  - docs/versions/1.1.2/更新日志.md（已存在，补全实现状态）
  - docs/versions/1.1.2/RELEASE-NOTES.md（新建）
  - docs/qa/versions/1.1.2/QA-审计报告.md（新建，Release QA 审计 8 维度）
  - docs/02-技术设计文档.md（补充 Base64 列编解码 + .log 导入 + 设置按钮迁移段落）
  - handoff/TASK-BOARD.md（T37-T44 全部 verified_complete + E2E + qa_passed）
out_of_scope:
  - 不改功能代码
  - 不改版本号（T43 做）
acceptance_criteria:
  - cargo fmt --check + cargo clippy --workspace -- -D warnings + cargo test --workspace 全绿
  - pnpm --prefix frontend build 通过
  - docs/versions/1.1.2/ 三件套齐全且内容与实际实现一致
  - QA-审计报告 8 维度全部 pass + 结论 qa_passed
  - 02 设计文档含 v1.1.2 新增段落
verification_commands:
  - cargo fmt --check
  - cargo clippy --workspace -- -D warnings
  - cargo test --workspace
  - pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build
depends_on: [T43]
status: planned
```

## 实现指引

### 1. RELEASE-NOTES.md（docs/versions/1.1.2/RELEASE-NOTES.md）

参考 v1.1.1 格式（`# RuT0DataKit v1.1.2` + `## 新增` + `## 优化` + `## 下载` + `## 升级`）。

新增条目：
- 列编解码：Base64 编码/解码某列（可撤销）
- .log 文件导入：按行解析成单列
- tshark Windows 路径解析测试加固

优化条目：
- 设置按钮迁移到右上角 TopToolbar
- 设置界面冗余文案精简

### 2. QA-审计报告.md（docs/qa/versions/1.1.2/QA-审计报告.md）

8 维度审计（参考 docs/qa/versions/1.1.1/ 格式）：
1. 版本号一致性（T43 6 处）
2. 构建验证（cargo + pnpm）
3. 测试覆盖（T37 base64 ≥5 / T41 tshark ≥6 / T42 log ≥3）
4. 安全约束（SQL 参数绑定 + 凭据 + Mimosa 兼容策略）
5. 文档完整性（三件套 + 02 设计文档）
6. 端到端验收（E1-E7）
7. 依赖审计（base64 crate 0.22）
8. 回归检查（v1.1.1 功能不退化）

结论：`qa_passed` 或 `conditional_pass`（取决于 Mimosa 完整扫描结果）。

### 3. 更新日志.md 状态同步

T37-T44 全部 `verified_complete`，E2E 全绿，整体 `done_e2e`。

### 4. 02-技术设计文档.md 补充段落

新增 v1.1.2 小节：
- 列编解码：base64_column 命令（columns.rs）+ DbManager.base64_transform_column_cells
- .log 导入：LogReader（datasource/log.rs）+ detect_format 分发
- UI 迁移：设置按钮从 AiPanel 移至 TopToolbar 右端
- 设置文案：DbPathCard/AboutCard/TsharkPathCard 精简

### 5. TASK-BOARD.md 最终状态

T37-T44 全部 `verified_complete`，E2E 验收全绿，Release QA `qa_passed`。
