```yaml
task_id: T15-3
goal: |
  版本号 bump 0.5.0→0.6.0（5 manifest：workspace Cargo.toml + crates/core
  Cargo.toml + src-tauri/Cargo.toml + tauri.conf.json + frontend/package.json），
  同步 docs/versions/0.6.0/更新日志.md（任务状态 planned→done + 版本状态
  in_progress→release_complete 待 Phase 9）、docs/04-版本标准.md 里程碑
  索引行（状态 planned→release_complete 待 Phase 9）、生成
  docs/qa/versions/0.6.0/QA-审计报告.md（Phase 8 产出，本任务先建骨架）。
in_scope:
  - Cargo.toml（workspace version）
  - crates/core/Cargo.toml
  - src-tauri/Cargo.toml
  - src-tauri/tauri.conf.json
  - frontend/package.json
  - docs/versions/0.6.0/更新日志.md
  - docs/04-版本标准.md
  - docs/qa/versions/0.6.0/QA-审计报告.md
out_of_scope:
  - crates/core/src/*（T15-1 负责）
  - frontend/src/*（T15-2 负责）
  - src-tauri/src/*（不动 Rust 命令代码）
acceptance_criteria:
  - grep '"0.5.0"' frontend/package.json 返回 0 命中（已改为 0.6.0）
  - grep 'version = "0.6.0"' Cargo.toml 命中 workspace version
  - grep 'version = "0.6.0"' src-tauri/Cargo.toml 命中
  - grep '"version": "0.6.0"' src-tauri/tauri.conf.json 命中
  - docs/versions/0.6.0/更新日志.md 任务状态为 done ✅
  - docs/04-版本标准.md v0.6.0 行状态为 in_progress（Phase 9 改 release_complete）
  - docs/qa/versions/0.6.0/QA-审计报告.md 存在（骨架或完整，Phase 8 补全）
verification_commands:
  - grep -n 'version' Cargo.toml | head -3
  - grep -n 'version' src-tauri/Cargo.toml | head -3
  - grep -n '"version"' src-tauri/tauri.conf.json
  - grep -n '"version"' frontend/package.json
  - ls docs/versions/0.6.0/ docs/qa/versions/0.6.0/
files_likely_to_change:
  - Cargo.toml
  - crates/core/Cargo.toml
  - src-tauri/Cargo.toml
  - src-tauri/tauri.conf.json
  - frontend/package.json
  - docs/versions/0.6.0/更新日志.md
  - docs/04-版本标准.md
  - docs/qa/versions/0.6.0/QA-审计报告.md
risks:
  - 版本号 bump 5 处必须全部同步，漏一处会导致 manifest 不一致（Release QA 会拦截）
  - 更新日志状态在 Phase 9（release_complete）才最终改为 release_complete，本任务只改任务状态 done ✅ + 版本状态保持 in_progress
  - QA 报告本任务只建骨架（结论占位 pending），Phase 8 主会话补全审计内容 + 结论 qa_passed
  - crates/core/Cargo.toml 用 version.workspace = true 则不动；硬编码则改
depends_on: [T15-2]
status: planned
```

## 实施说明

### 1. 版本号 bump（5 manifest）

- `Cargo.toml`（workspace root）：`version = "0.5.0"` → `"0.6.0"`
- `crates/core/Cargo.toml`：若用 `version.workspace = true` 则不动；若硬编码则改
- `src-tauri/Cargo.toml`：`version = "0.5.0"` → `"0.6.0"`
- `src-tauri/tauri.conf.json`：`"version": "0.5.0"` → `"0.6.0"`
- `frontend/package.json`：`"version": "0.5.0"` → `"0.6.0"`

### 2. docs/versions/0.6.0/更新日志.md

现有内容是 T13-4 写的骨架（4 任务 planned）。
本版本实际做了 7 项（PRE 重提交 + T15-1/2/3），需重写更新日志：
- 任务表状态 planned → done ✅
- 版本状态保持 `in_progress`（Phase 9 改 release_complete）
- 变更明细回填实际改动（pinfo_phone / sql_reader / MaskColumnMapper+RulesView / encrypt 命令+trial_mask / EncryptTool 前端 / 代码优化 / 版本 bump）

### 3. docs/04-版本标准.md

v0.6.0 行已存在（T13-4），状态 `planned` → `in_progress`（Phase 9 改 release_complete）。
描述可能需更新以反映实际范围（不只是 encrypt，还有 pinfo_phone/sql_reader/RulesView）。

### 4. docs/qa/versions/0.6.0/QA-审计报告.md

建骨架（Phase 8 主会话补全）：

```markdown
# v0.6.0 QA 审计报告

> 版本：0.6.0
> 审计日期：2026-07-23（Phase 8 补全）
> 审计人：主会话
> 结论：pending（Phase 8 补全）
```
