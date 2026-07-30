# TASK-T9-HANDOFF — 双语 README 收口 + docs 一致性 + QA 审计

> 本 HANDOFF 与最新代码基线（T5 verified_complete @ 4aae991，含 T4-hotfix edb3ad6）对齐。T1~T8 全部 `verified_complete`，进入文档收口 + Release QA 审计阶段。

## task_id
T9

## depends_on
T1、T2、T3、T4（含 T4-hotfix）、T5、T6、T7、T8 全部 `verified_complete`。

## goal
完成 v1.0.0 文档体系最终收口与 Release QA 审计前置准备：
1. 更新 `README.md` / `README_EN.md`：去掉「规划阶段」免责声明，落地与当前实现一致的 Quick Start + 基本验证 + 项目结构说明。
2. 同步 `docs/versions/1.0.0/更新日志.md`：T9 行状态 → `verified_complete`；版本摘要补 T4-hotfix 与 T5 实际依赖版本（calamine 0.26）。
3. 预填 `docs/qa/versions/1.0.0/QA-审计报告.md`：把 5 维度表里能在静态审计阶段确定的项填为 `pass`/`fail` 并附证据（命令 + 文件:行号），GUI 项（E3~E8 需 `cargo tauri dev`）标 `pending_e2e`（留 Phase 7 主会话补）。**结论保持 `planned`/`pending`，不擅自推 `qa_passed`**——QA 审计最终结论由主会话 Phase 8 判定。
4. 校验 docs 间无术语 / 范围冲突（grep 扫描）。

## scope（in_scope 文件，仅这些可改）

### 文档（主战场）
- `README.md`
- `README_EN.md`
- `docs/versions/1.0.0/更新日志.md`（仅 T9 行 + 版本摘要微调，勿改其它任务行的事实）
- `docs/qa/versions/1.0.0/QA-审计报告.md`（预填静态可判定项，GUI 项标 pending_e2e）
- `docs/04-版本标准.md`（仅若 v1.0.0 里程碑索引状态需从 planned 调整，否则不动）

### handoff
- `handoff/TASK-BOARD.md`（T9 行状态 → `verified_complete`，待主会话最终判定后；coder 先标 `implemented_not_verified`）

## out_of_scope（禁止改动）

- **任何代码**：`crates/`、`src-tauri/src/`、`frontend/src/` 一律不动。
- **T1~T8 的 docs 事实**：`docs/00-需求文档.md`、`docs/01-页面与交互说明.md`、`docs/02-技术设计文档.md`、`docs/03-开发任务清单.md` 的验收标准/范围条款不改（如发现与实现冲突，在 REPORT 的 `docs_conflict_findings` 记录，由主会话裁决）。
- `docs/versions/1.0.0/规划需求.md`、`docs/versions/1.0.0/updater-密钥.md` 不改。
- `Cargo.toml` / `Cargo.lock` / `package.json` / `tauri.conf.json` 不改。
- 不改 QA 报告的最终结论字段（结论由主会话 Phase 8 定）。

## 实现要点

### 1. README.md / README_EN.md 收口

当前两份 README 含「v1.0.0 尚处规划阶段，部分命令可能因工程未初始化而失败」免责声明——这与 T1~T8 已 verified_complete 的事实冲突，需更新：

- 「当前状态」段：版本 v1.0.0 框架阶段，状态改为「开发中（任务 T1~T8 已验证完成，待端到端 + QA）」；去掉「规划阶段」措辞。
- Quick Start：保留现有三步（clone → pnpm install → cargo tauri dev），**实测确认命令可复现**后再写「已验证可复现」。coder 须实际跑一遍 `pnpm --prefix frontend install` + `cargo check --workspace` + `pnpm --prefix frontend build` 确认通过，把「部分命令可能失败」那句删掉。
- 基本验证段：去掉「随实现进度陆续可用」免责声明，改为「已验证通过」并附实际命令输出摘要（PASS 即可，不必贴长日志）。
- 新增「项目结构」段（简版）：根 `Cargo.toml`（workspace）→ `crates/core`（纯逻辑引擎，v1.0.0 骨架）+ `src-tauri`（Tauri 命令层 + SQLite + updater）+ `frontend`（React shell）。一句话三层架构 + 单向依赖（frontend → src-tauri → crates/core）。
- 新增「v1.0.0 能力边界」简表：已交付（四区布局 / Sheet/Tab / antd Table / CSV·XLSX 导入 / SQLite 持久 / updater 检查 / AI 占位 / 设置页）vs 推迟 v1.1+（脱敏 / 校验 / 提取 / 规则 / 搜索 / Tools / PCAP / 真实 AI）。
- License 行：仓库当前无 LICENSE 文件，保留「待添加」即可（不伪造）。
- 中英文保持事实一致。

### 2. 更新日志收口

- T9 行：状态 `planned` → `verified_complete`（coder 标 `implemented_not_verified`，主会话判后改）。
- 版本摘要段：补一句 T4-hotfix（分页语义修复）+ T5 实际用 calamine 0.26（非 0.27）。
- 不改 T1~T8 各行的事实表述（已由各任务 commit 固化）。

### 3. QA 审计报告预填

按 5 维度逐项判定，**静态可判定的填 `pass`/`fail` + 证据，需 GUI 的标 `pending_e2e`**：

- **功能完整性**（§2 表 11 项）：
  - #1 cargo check + cargo tauri dev：cargo check 可静态验证 → `pass`（附命令）；cargo tauri dev 需 GUI → `pending_e2e`。
  - #2~#5（四区布局 / 能力按钮 / Sheet Tab / antd Table）：需 GUI → `pending_e2e`。但可用 grep 验证组件存在性作为辅证（如 `frontend/src/components/layout/TopToolbar.jsx` 存在、panels/ 4 文件存在、SheetTabs.jsx / DataTable.jsx 存在）→ 在证据列附 grep 结果，状态仍 `pending_e2e`。
  - #6 SQLite 5 表 3 索引：读 `src-tauri/src/db/schema.rs`（或 SCHEMA_DDL 常量）静态核验表/索引定义齐全 → `pass`（附文件:行号）；`schema_version` 写入由单测 `new_creates_tables_and_schema_version` 覆盖 → `pass`。
  - #7 导入 1000 行 CSV 分页：cargo test 覆盖 datasource + db 分页（含 T4-hotfix 多列单测）→ `pass`（附测试名）；实际 1000 行 GUI 导入 → `pending_e2e`。
  - #8 重启历史 Session：后端 `get_session`/`list_sessions` 单测覆盖 → `pass`；前端历史入口未实现（T5 REPORT 已说明留后续）→ 在证据列如实标注，状态 `pending_e2e`（或 `partial`，由 coder 判断，但勿判 `fail`——属 v1.0.0 已知边界）。
  - #9 operations 表 import 记录：`log_operation` 单测 + import_file 调用 grep → `pass`（附 commands.rs:204 + db test `log_operation_inserts_row`）。
  - #10 check_update 无网降级：T6 代码 `check_update` catch 错误返回 available=false（grep commands.rs）→ `pass`（附行号）；签名验签需 GUI/网络 → `pending_e2e`。
  - #11 aiSuggest v1.1+ 文案：commands.rs `ai_suggest` 返回 Err 字符串 grep + AiPanel.jsx catch 降级 grep → `pass`（附行号）。
- **回归与端到端**（§3）：全部 `pending_e2e`（需 GUI 主流程）。
- **构建与产物**（§4）：
  - 四目标矩阵构建 / checksums / updater JSON：`pending_release`（需 CI/Release 流程，非本阶段）。
  - 版本一致性：grep 4 处版本号（Cargo.toml / src-tauri/Cargo.toml / tauri.conf.json / package.json）均为 1.0.0 → `pass`（附 grep 输出）。
- **安全**（§5）：
  - 全本地处理：架构单向依赖 + 无网络调用（除 updater）grep → `pass`（附架构说明）。
  - updater 仅拉元数据+签名产物：T6 代码 + updater-密钥.md 文档 → `pass`。
  - 私钥不落盘仓库：`.gitignore` 含 `*.key` + grep 仓库无 `.key` 文件 → `pass`（附命令）。
  - 嵌套 struct camelCase：grep `rename_all` 命中全部嵌套 struct（Cell/SessionSummary/SheetSummary/SessionDetail/UpdateStatus/AiContext/AiSuggestion/ImportResult/PageData）→ `pass`（附 grep）。
  - 签名验签失败拒绝 / endpoints HTTPS：`pending_e2e`（需实际 updater JSON 测试）。
- **文档**（§6）：
  - 双语 README 存在互链 → `pass`。
  - Quick Start 可复现：coder 实跑后 → `pass`（附命令输出摘要）。
  - 更新日志与 TASK-BOARD 一致：两份文件 T1~T9 状态对齐核验 → `pass`（附对照）。
  - 04-版本标准里程碑索引：核验 v1.0.0 行状态 → `pass` 或标注待调整。
  - 无术语冲突 / 无 v1.1+ 误写成 v1.0.0：grep `v1.0.0` + `v1.1+` 扫描 docs → `pass`（附扫描结果）。
  - README 无 `{{PLACEHOLDER}}` 残留：grep → `pass`。
- **结论字段（§8）**：保持 `planned` 或写「静态审计预填完成，待 Phase 7 E2E + Phase 8 最终判定」——**不写 `qa_passed`**。

### 4. docs 冲突扫描

- grep 扫描 docs/ 下 `v1.0.0` 与 `v1.1+` / `v1.2+` 出现处，确认无「v1.1+ 能力被误写成 v1.0.0 已交付」。
- 若发现 02-技术设计文档.md 的 IPC 契约清单（§4 列了 set_selection/reorder_columns/rename_column/list_sessions/open_session/get_setting/set_setting）与当前实现（只实现了 import_file/get_sheet_data/check_update/install_update/ai_suggest/invoke_ai_op 共 6 个命令）不符——这是已知边界（02 文档写的是完整契约蓝图，v1.0.0 只落地导入+updater+AI 占位子集），在 REPORT `docs_conflict_findings` 记录，**不改 02 文档**（属 T9 out_of_scope），由主会话裁决是否在 02 文档补「v1.0.0 落地子集」注释。

## 交付物

- 更新后的 `README.md` / `README_EN.md`
- 更新后的 `docs/versions/1.0.0/更新日志.md`（T9 行 + 摘要微调）
- 预填后的 `docs/qa/versions/1.0.0/QA-审计报告.md`（静态项 pass，GUI 项 pending_e2e，结论未定）
- `handoff/TASK-BOARD.md`（T9 → `implemented_not_verified`）
- `handoff/TASK-T9-REPORT.md`

## acceptance_criteria

1. README 中英文均去掉「规划阶段/命令可能失败」免责声明，Quick Start 标注已验证可复现，含项目结构段 + v1.0.0 能力边界简表。
2. 更新日志 T9 行状态更新，版本摘要含 T4-hotfix + calamine 0.26 说明。
3. QA 报告 5 维度表：静态可判定项均填 `pass`+证据（文件:行号 / 命令 / 测试名），GUI 项标 `pending_e2e`，结论字段未擅自推 `qa_passed`。
4. `cargo check --workspace` + `cargo test --workspace` + `pnpm --prefix frontend build` 全 PASS（coder 实跑确认 README 命令可复现）。
5. grep 核验：4 处版本号均 1.0.0；嵌套 struct rename_all 全覆盖；README 无 `{{PLACEHOLDER}}`；仓库无 `.key` 文件。
6. docs 冲突扫描完成，冲突项（若有）记入 REPORT `docs_conflict_findings`，不改 out_of_scope 文档。
7. 不改任何代码文件（`crates/` / `src-tauri/src/` / `frontend/src/`）。
8. TASK-BOARD T9 → `implemented_not_verified`。

## verification_commands

```sh
cd /Users/joker/Code/RuT0DataKit
# 1. 确认代码未被改动（应为空）
git diff --stat -- crates/ src-tauri/src/ frontend/src/
# 2. 构建测试（README 可复现性）
cargo check --workspace
cargo test --workspace
pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build
# 3. 版本号一致性
grep -n 'version' Cargo.toml src-tauri/Cargo.toml src-tauri/tauri.conf.json frontend/package.json | grep '1.0.0'
# 4. camelCase 覆盖
grep -n 'rename_all' src-tauri/src/db/mod.rs src-tauri/src/commands.rs
# 5. 无占位符残留
grep -rn '{{PLACEHOLDER}}' README.md README_EN.md docs/ || echo "no placeholder"
# 6. 无私钥落盘
find . -name '*.key' -not -path './target/*' -not -path './node_modules/*' 2>/dev/null || echo "no key file"
# 7. docs 冲突扫描
grep -rn 'v1\.0\.0' docs/ | grep -i '已交付\|已实现\|delivered' || echo "no v1.0.0 overclaim"
```

## reporting

完成后写 `handoff/TASK-T9-REPORT.md`，含：implemented_changes / verification_run / verification_results / docs_conflict_findings（若有）/ acceptance_criteria 证据映射 / reported_status（`implemented_not_verified`）。最终 `verified_complete` 由主会话判定。
