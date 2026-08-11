# v1.1.4 TASK-BOARD（R3 续轮：哈希函数 + DB 文件解析）

> 版本：v1.1.4
> 状态：in_progress（T73/T75 verified_complete + review_passed；T74 派 coder 进行中；T76 待派）
> 前置：v1.1.4 R2 续轮已 qa_passed（commit ba0b84f 合并 main）
> 主题：v1.1.4 R3 续轮 — 2 项增强（哈希函数加密 MD5/SHA1/SHA256 + 外部 DB 文件解析）

## 背景

用户在 v1.1.4 R2 qa_passed 后追加 2 个需求（仍属 v1.1.4，不升版本号）：

1. **哈希函数加密**：当前 CryptoPanel 只有 Base64 编解码。扩展加入 MD5 / SHA1 / SHA256 等哈希函数，对指定列做哈希变换（不可逆，输出 hex 小写）。
2. **DB 文件解析**：当前 `detect_format` 支持 csv/xlsx/json/jsonl/txt/log/sql/pcap/pcapng，但不支持直接打开 `.db`/`.sqlite`/`.sqlite3` 二进制 SQLite 数据库文件。新增 `DbReader` 数据源，打开外部 .db 文件并读取全部用户表数据。

用户明确指示："全部完成等待我对于v1.1.4版本的手工验证" — 即 R3 流水线走完（含 QA）后**不自动 finalize**（不 merge、不 tag），等待用户手工验证。

## 任务 DAG

```yaml
goal: |
  v1.1.4 R3 续轮：CryptoPanel 扩展哈希函数（MD5/SHA1/SHA256 列式变换，可撤销）+
  新增 DbReader 数据源（.db/.sqlite/.sqlite3 外部 SQLite 文件解析，多表联合 + __table 列）。

tasks:
  - id: T73
    title: 后端 — hash_column 命令（MD5/SHA1/SHA256）+ 依赖 + transform_column_cells 泛化 + 测试
    depends_on: []
    status: verified_complete  # commits 89d1aff+9e7bcd5; REVIEW review_passed 2026-08-11
    handoff: handoff/TASK-T73-HANDOFF.md
  - id: T74
    title: 前端 — CryptoPanel 哈希 UI + hashColumn IPC wrapper
    depends_on: [T73]
    status: in_progress  # T73 verified_complete, coder 派遣中
    handoff: handoff/TASK-T74-HANDOFF.md
  - id: T75
    title: 后端 — DbReader 数据源（.db/.sqlite/.sqlite3 外部 SQLite 文件解析）+ detect_format 分发 + 测试
    depends_on: []
    status: verified_complete  # commits 797088a+1b613a5; REVIEW review_passed 2026-08-11
    handoff: handoff/TASK-T75-HANDOFF.md
  - id: T76
    title: 文档 + E2E 验收 + Release QA R3 增量审计
    depends_on: [T73, T74, T75]
    status: pending  # 待 T74 verified_complete
    handoff: handoff/TASK-T76-HANDOFF.md

e2e_acceptance:
  - hash_column 命令对指定列做 MD5 哈希（hex 小写输出），已知向量 md5("hello")=5d41402abc4b2a76b9719d911017c592
  - hash_column 命令支持 SHA1 / SHA256（hex 小写输出），已知向量 sha1("hello")=aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d
  - hash_column 操作可撤销（list_undoable_operations 包含 hash_column kind，undo 后恢复原文）
  - CryptoPanel 可选算法（Base64 编/解码 + MD5/SHA1/SHA256）+ 执行后刷新数据 + undo 栈
  - detect_format 对 .db/.sqlite/.sqlite3 扩展名返回 DbReader（不返回 NotImplemented）
  - DbReader 打开外部 .db 文件，读取用户表数据（跳过 sqlite_% 内部表），返回 Dataset
  - DbReader 多表 .db 文件联合输出，__table 列标识来源表
  - cargo fmt/clippy/test 全绿 + pnpm build 全绿

e2e_verification:
  - cargo fmt --all
  - cargo clippy --all-targets --all-features -- -D warnings
  - cargo test --all
  - pnpm --prefix frontend build

release_qa:
  required: true
  report: docs/qa/versions/1.1.4/QA-审计报告.md
  audit_scope:
    - 需求覆盖
    - 端到端流程
    - 构建与测试
    - 代码质量
    - 安全与隐私
    - 数据与迁移
    - 依赖与配置
    - 文档一致性
```

## 安全约束（延续）

- SQL 全部 `?N` + `params![]` 绑定，禁拼接、format、f-string
  - DbReader 的表名不能参数绑定 → 用 `quote_identifier`（双引号转义内部双引号）+ 表名来源限定为 `sqlite_master`（受信系统表）
- 无凭据字面量
- Mimosa 完整审计未拿到结论前不宣称安全
- 不创建 tag（用户等待手工验证，不自动 finalize）
- 不自动 merge main（用户等待手工验证）
