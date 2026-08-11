# TASK-T76-HANDOFF

```yaml
task_id: T76
goal: |
  v1.1.4 R3 续轮收尾：同步 5 项文档（更新日志 / RELEASE-NOTES / 02-技术设计文档 /
  TASK-BOARD / QA-审计报告），跑端到端验证（cargo fmt/clippy/test + pnpm build + 版本号 4 处一致），
  执行 Release QA R3 增量审计（8 维度），结论 qa_passed（R3 续轮）。

in_scope:
  - docs/versions/1.1.4/更新日志.md（追加 R3 续轮章节 + E101-E110 + 状态行更新）
  - docs/versions/1.1.4/RELEASE-NOTES.md（追加 R3 功能段）
  - docs/02-技术设计文档.md（同步 hash_column IPC 契约 + DbReader 数据源 + 依赖变更）
  - handoff/TASK-BOARD.md（T73-T76 status verified_complete + 状态行 qa_passed R3）
  - docs/qa/versions/1.1.4/QA-审计报告.md（追加 §16 R3 增量审计 + §17 证据索引 + qa_passed R3）

out_of_scope:
  - 不改代码（T73/T74/T75 负责实现）
  - 不创建 tag（用户等待手工验证，不自动 finalize）
  - 不合并 main（用户等待手工验证）
  - 不改 04-版本标准.md 里程碑表 v1.1.4 行状态（保持 qa_passed；release_complete 待用户手工验证后另议）

acceptance_criteria:
  - 更新日志.md 状态行 → qa_passed（首轮 + R2 续轮 + R3 续轮）
  - 更新日志.md 进度表追加 T73/T74/T75/T76 行（verified_complete / E101-E110）
  - 更新日志.md 新增「R3 续轮：哈希函数 + DB 文件解析」章节（背景 + 2 项设计决策 + 验收项 E101-E110）
  - RELEASE-NOTES.md 状态 → qa_passed（R3 续轮）
  - RELEASE-NOTES.md 追加 R3 功能段（哈希函数 MD5/SHA1/SHA256 + DB 文件解析）
  - 02-技术设计文档.md 同步 hash_column IPC 契约 + HashAlgorithm enum + DbReader 数据源 + 新增依赖（md-5/sha1/sha2/hex）
  - TASK-BOARD.md T73-T76 status 全 verified_complete + 状态行 qa_passed（R3 续轮）
  - QA-审计报告.md 追加 §16 R3 增量审计（8 维度：DAG 完整性 / 验收项覆盖 E101-E110 / 代码审查闭环 / 验证命令全绿 / 文档同步 / 安全与隐私 / 向后兼容 / 版本号一致性）
  - QA-审计报告.md 追加 §17 R3 修复证据索引
  - QA-审计报告.md 结论 qa_passed（R3 续轮）
  - cargo fmt --all --check exit 0
  - cargo clippy --all-targets --all-features -- -D warnings exit 0
  - cargo test --all 全绿（src-tauri + core + Doc-tests）
  - pnpm --prefix frontend build exit 0
  - 版本号 4 处一致 1.1.4（Cargo.toml / tauri.conf.json / frontend/package.json / frontend/src/constants.js）

verification_commands:
  - cargo fmt --all
  - cargo clippy --all-targets --all-features -- -D warnings
  - cargo test --all
  - pnpm --prefix frontend build

files_likely_to_change:
  - docs/versions/1.1.4/更新日志.md
  - docs/versions/1.1.4/RELEASE-NOTES.md
  - docs/02-技术设计文档.md
  - handoff/TASK-BOARD.md
  - docs/qa/versions/1.1.4/QA-审计报告.md

risks:
  - R3 新增 4 个 crate 依赖（md-5/sha1/sha2/hex），需在 QA 报告「依赖与配置」维度说明无安全顾虑
  - DbReader 表名不能参数绑定，需在 QA 报告「安全与隐私」维度说明 quote_identifier 转义 + sqlite_master 受信来源
  - Mimosa R3 未重新运行，沿用 v1.1.3 R1+R2 双轮 0 findings 基线（v1.1.4 R3 改动为 codec + datasource 功能性增强，未改 DB schema / CSP / 权限）
  - 用户等待手工验证，不自动 finalize（不 merge / 不 tag / 不改 04-版本标准.md 里程碑状态）

depends_on: [T73, T74, T75]
status: planned
```

## 实现指引

### 更新日志.md R3 续轮章节模板

```markdown
## R3 续轮：哈希函数 + DB 文件解析（T73-T76）

### 背景

用户在 v1.1.4 R2 qa_passed 后追加 2 个需求（仍属 v1.1.4）：
1. 哈希函数加密：CryptoPanel 扩展 MD5/SHA1/SHA256 列式哈希变换
2. DB 文件解析：detect_format 新增 .db/.sqlite/.sqlite3 外部 SQLite 文件读取

### 设计决策

1. **哈希逻辑放在 src-tauri（不放 crates/core）**：沿用 base64 先例，crates/core 不引入 hash 依赖，保持 core 轻量。复用 `base64_transform_column_cells` 闭包式列变换 DB 方法（泛型 F: Fn(&str)->Option<String>，与函数名无关），无需重命名。
2. **DbReader 表名用 quote_identifier 转义**：SQLite 表名不能参数绑定，表名来源限定为 sqlite_master（受信系统表），用双引号转义防御性处理。

### 验收项

- E101：hash_column MD5 已知向量 md5("hello")="5d41402abc4b2a76b9719d911017c592"
- E102：hash_column SHA1 已知向量 sha1("hello")="aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
- E103：hash_column SHA256 已知向量 sha256("hello")="2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
- E104：hash_column 可撤销（list_undoable_operations 包含 hash_column，undo 恢复原文）
- E105：CryptoPanel 可选算法（Base64 编/解码 + MD5/SHA1/SHA256）+ 执行后刷新数据 + undo 栈
- E106：detect_format 对 .db/.sqlite/.sqlite3 返回 DbReader
- E107：DbReader 单表 .db 文件读取（跳过 sqlite_% 内部表）
- E108：DbReader 多表 .db 文件联合输出（__table 列标识来源）
- E109：cargo fmt/clippy/test 全绿
- E110：pnpm build 全绿 + 版本号 4 处一致 1.1.4
```

### QA-审计报告.md §16 R3 增量审计模板

8 维度结构沿用 R2 §14（DAG 完整性 / 验收项覆盖 / 代码审查闭环 / 验证命令全绿 / 文档同步 / 安全与隐私 / 向后兼容 / 版本号一致性），覆盖范围改为 T73-T76 + E101-E110。

### 安全约束（延续）

- 不创建 tag（用户等待手工验证）
- 不合并 main（用户等待手工验证）
- Mimosa R3 未重新运行，沿用 v1.1.3 R1+R2 双轮 0 findings 基线
- SQL 全参数绑定（DbReader 表名例外：quote_identifier 转义 + sqlite_master 受信来源，已在风险维度说明）
- 无凭据字面量

### 完成后动作

T76 verified_complete 后：
1. 不自动 merge main（用户等待手工验证）
2. 不创建 tag（用户等待手工验证）
3. 报告用户：R3 续轮全部完成，等待手工验证
4. 保留 handoff/TASK-BOARD.md（用户手工验证通过后再清理 + finalize）
