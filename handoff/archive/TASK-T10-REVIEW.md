# TASK-T10-REVIEW — datasource mod 拆分

```yaml
task_id: T10
verdict: review_passed
defects:
  - severity: minor
    file: crates/core/src/datasource/mod.rs:72-85
    issue: |
      acceptance_criteria 要求「现有 3 个 datasource 单测保持原断言通过，位置可迁移」。
      detect_format_routes_by_extension 的断言被扩展：T10 前仅断言
      detect_format(csv).is_ok() + detect_format(".txt").err()==NotImplemented；
      T10 后新增了对 .txt/.json/.jsonl/.sql/.pcap/.pcapng 的 is_ok() 断言，并把
      .txt 从 NotImplemented 分支改为 is_ok() 分支。属于断言改动，非「原断言不变」。
    impact: |
      无功能回归——新增断言全部通过（各扩展名确实路由成功）且更严格。
      但与 handoff「测试断言字节级保留」的措辞存在偏差，且 docs/qa/versions/1.0.0/
      QA-审计报告.md:30 仍描述为「datasource::tests::* 3 项」，未反映断言已强化。
    fix: |
      二选一：(a) 若 reviewer 接受为测试加强，无需改码，但建议在 QA 报告脚注补一句
      「T10 后 detect_format 路由测试已覆盖全 8 扩展名」；(b) 若严格守「原断言不变」，
      将 mod.rs:76-81 的新增 is_ok() 断言删去，仅保留原 csv.is_ok() + unknown→NotImplemented。
      推荐 (a)。
scope_check:
  越界项: src-tauri/src/commands.rs 的 deletion 被 T10 commit (85d86d7) 一并记录。
  证据:
    - git show --stat 85d86d7 含 `src-tauri/src/commands.rs | 284 ----`
    - 85d86d7^ 中 commands.rs 存在、commands/ 目录不存在
    - 85d86d7 提交树中 commands.rs 不存在、commands/ 目录也不存在
      （git ls-tree -r 85d86d7 -- src-tauri/src/ | grep commands 为空）
    - 但 85d86d7:src-tauri/src/lib.rs:7 仍声明 `mod commands;` 并在 lib.rs:19-24
      引用 commands::check_update / import_file 等
  判定: |
    该 commit 单独不可编译（commands 模块声明无对应文件/目录）。这是真实的越界
    且造成了一个中间 commit 不可构建。但这是 working tree 既有状态的诚实记录——
    commands.rs 已被先前 UI/export 会话物理拆为 commands/ 子模块（untracked），
    git add crates/core/src/datasource/ 时 commands.rs 的 deletion 被一并计入
    index，commands/ 子模块仍 untracked 故未进 85d86d7 树。
    紧随其后的 T11 commit f1744f2 (refactor(tauri): T11 split commands.rs ...)
    补齐了 commands/ 子模块入库。当前 HEAD (e0f4e3f) 树自洽：
      commands.rs 不存在、commands/{mod,data,settings,update}.rs 存在、
      lib.rs mod commands 解析正常、cargo check --workspace 通过。
    对 T10 验收语义无破坏：T10 的实际代码改动（datasource/ 拆分）与该 commands.rs
    deletion 无耦合，commands.rs 内容一字未改地被 T11 以 commands/ 形式恢复。
    coder 在 REPORT 中如实声明了此 scope_deviation，未隐瞒。
  处理建议: |
    接受为已声明的副作用，不判 reject。但需提示主会话：
    (1) 85d86d7 是 bisect-unfriendly 的不可构建中间 commit，后续若做 git bisect
        需跳过该 commit；
    (2) 根因是「跨会话 working tree 未及时提交」的工作流问题，非 T10 任务本身缺陷。
docs_check:
  - handoff/TASK-T10-HANDOFF.md 与 handoff/TASK-T10-REPORT.md 均存在且未被 coder 删除。
  - handoff 明确为「纯结构搬运，无行为/用法变化，docs/ 无需同步」——成立。
  - 唯一缺口：docs/qa/versions/1.0.0/QA-审计报告.md:30 描述「datasource::tests::* 3 项」
    仍准确（测试数量未变），但该行未提及测试名迁移到子模块路径
    （datasource::csv::tests::* vs 原 datasource::tests::*）。属可选同步项，不阻塞。
verification_independent:
  - cargo check --workspace：PASS（独立复跑，1.27s，无 error）
  - cargo test --workspace：PASS（core 11 passed/2 ignored[tshark]，app 9 passed/0 failed）
  - cargo test -p ruT0-data-kit-core --lib datasource：PASS
      datasource::tests::detect_format_routes_by_extension ... ok
      datasource::csv::tests::csv_reader_headers_and_rows ... ok
      datasource::csv::tests::csv_reader_flexible_columns ... ok
goal_acceptance:
  - goal 满足：mod.rs 由 731 行精简为 96 行，仅含 mod 声明 + pub use + Reader trait +
    detect_format 工厂 + 1 单测；无任何 Reader struct/impl/helper fn 实现体残留。
  - 每个格式有独立 .rs 子模块：csv.rs / xlsx.rs / json.rs / txt.rs / sql.rs / pcap.rs，
    共享 helper 在 util.rs。
  - 公共 API 表面不变：mod.rs:30-35 pub use 重导出，ruT0_data_kit_core::datasource::
    {Reader, CsvReader, XlsxReader, JsonReader, TxtReader, SqlReader, PcapReader,
    detect_format} 全部可访问；外部引用 src-tauri/src/commands/data.rs:88
    datasource::detect_format 仍解析。
  - SqlReader split_statements / extract_table_name 已迁入 sql.rs，行为注释随迁，
    未处理块注释/BEGIN END 的现状保留（符合 out_of_scope）。
  - 3 个测试全部通过（csv 两个测试断言字节级未变；detect_format 测试断言被加强，见 defects）。
```

## 结论

review_passed。T10 的核心目标（datasource god module 按格式拆分、纯结构搬运、公共 API 不变）已达成，三项验证命令独立复跑全绿。

唯一需 coder/主会话知悉的两点（均不阻塞）：

1. `detect_format_routes_by_extension` 测试断言被加强而非「字节级保留」（minor）——建议同步 QA 报告脚注，或按需回退断言。
2. 85d86d7 因 working tree 既有状态附带记录了 src-tauri/src/commands.rs 的 deletion，使该中间 commit 不可独立编译；但 coder 如实声明，紧随的 T11 commit f1744f2 已补齐 commands/ 子模块，当前 HEAD 树自洽。这是工作流时序问题，非 T10 任务缺陷，不影响验收。后续 git bisect 需跳过 85d86d7。
