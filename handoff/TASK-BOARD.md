# TASK-BOARD — v0.6.3 脱敏/校验总行数 BUG 修复 + 正则构造可视化积木重构（release_complete）

> 版本：v0.6.3
> 创建：2026-07-23
> 完成：2026-07-23
> 依赖版本：v0.6.2（release_complete @ 860671c）
> 依据：用户 goal「① 校验后 - 行 显示总行数 BUG；脱敏后 - 行 同样问题；② Tools 正则解析的构造重构为可视化积木构建」
> **状态：release_complete（qa_passed）**

## 任务 DAG（全部 done）

```yaml
goal: 修复脱敏/校验视图总行数显示 BUG（前端字段名对齐 total_rows/invalid_rows/
  masked_rows）；RegexTool 构造 Tab 由自然语言描述重构为可视化积木构建（点选
  数字/字母/至少一次等模块拼装正则，纯客户端，不外发数据）。bump 0.6.3，
  全链路验收通过。
version: 0.6.3
depends_on_version: 0.6.2
tasks:
  - id: T18-1
    title: 修复脱敏/校验视图总行数显示（前端字段名对齐）
    status: done  # MaskView.jsx 5 处 .total→.total_rows；
                  # ValidateView.jsx 4 处 .total→.total_rows / .invalid_count→.invalid_rows /
                  # .valid_count→计算 total_rows-invalid_rows；npm build 绿
  - id: T18-2
    title: RegexTool 构造 Tab 改为可视化积木构建
    status: done  # 新增 RegexConstructTab.jsx（~300 LOC，5 类 21 模块 + 3 预设 +
                  # 参数弹窗 + 测试高亮）；RegexTool.jsx import 替换原 ConstructTab；
                  # 后端 regex_construct.rs 保留；npm build 绿（3009 modules +1）
  - id: T18-3
    title: 版本号 bump 0.6.2→0.6.3（4 manifest）+ docs 同步 + QA 报告
    status: done  # 本任务
```

任务 DAG 结构：

```
T18-1 (frontend field-name fix) ─┐
                                 ├─→ T18-3 (version+docs+QA)
T18-2 (regex building-block UI) ─┘
```

T18-1 和 T18-2 无文件交集（MaskView/ValidateView vs RegexTool），可并行实施。

## E2E 验收

- `cargo test --workspace`：493 passed / 0 failed / 5 ignored（与 v0.6.2 基线一致，无 Rust 变更）✅
- `cargo build --manifest-path src-tauri/Cargo.toml`：0 error ✅
- `npm --prefix frontend run build`：vite build 3009 modules（+1 对比 v0.6.2），0 error ✅
- grep 验证：
  - MaskView/ValidateView `.total_rows`/`.invalid_rows`/`.masked_rows` 命中（14 行）✅
  - 无残留 `.total`/`.valid_count`/`.invalid_count`（0 命中）✅
  - `RegexConstructTab` / `BLOCK_MODULES` / `PRESETS` 关键符号到位 ✅
  - 4 处 manifest 版本号 0.6.3 ✅

## QA 门禁

- `docs/qa/versions/0.6.3/QA-审计报告.md`：结论 `qa_passed` ✅
- `docs/versions/0.6.3/更新日志.md`：状态 `release_complete` ✅
- `docs/04-版本标准.md` v0.6.3 里程碑行：`release_complete` ✅

## 安全约束（不变）

`docs/00-需求文档.md §6`「不外发数据：全本地处理；规则与样本不上传」保留，v0.6.3 不变。T18-2 积木拼装纯客户端，不调用后端、不外发数据。

## 提交链

```
T18-1+T18-2 (frontend fix + rebuild) ─→ T18-3 (version+docs+QA)
```

按 v0.6.x 惯例，T18-1/T18-2 合并为一个 feat 提交（均为前端改动，无依赖耦合），T18-3 单独 chore 提交（版本号 + docs）。
