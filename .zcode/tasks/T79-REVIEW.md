# T79 REVIEW — 规则列表按名称排序

## 审查结论

**verdict: review_passed**

T79 实现「所有规则按首字母 Unicode 码点自动排序」目标完整达成，单一改动点精准，测试覆盖到位，文档同步，无越界改动。E2E 经 reviewer 独立复验全绿。下方先列 findings（仅 1 条 minor 流程观察），再给分维度结论。

---

## Findings（先列问题，再给结论）

### Defect 1（minor — 流程观察，非阻塞）

- **severity**: minor
- **file**: `.zcode/tasks/`（目录级）
- **issue**: `T79-REPORT.md` 缺失，coder 自验结果直接内嵌于 `T79-HANDOFF.md` 的「E2E 验证」节。
- **impact**: reviewer 规范（`05-reviewer-spec.md` §5.2）字面要求读取 HANDOFF + REPORT。本项目自 T78 起即采用「HANDOFF 内嵌 E2E 结果、不单独产 REPORT」的惯例，HANDOFF 已包含验证命令与结果（fmt/clippy/test 317 passed/pnpm build 全绿），可建立判断依据，故不构成 block。仅作为流程一致性观察记录，供主会话决定是否要求 coder 补齐 REPORT。
- **fix**: 可选——若主会话希望严格遵循 spec 字面，要求 coder 补 `T79-REPORT.md`；若接受当前惯例，则无需处理。

---

## 分维度审查结果

### 1. 需求覆盖 — PASS

用户需求「所有规则按首字母顺序自动排序（Unicode 码点）」。实现把 `list_rules` SQL 由 `ORDER BY id ASC` 改为 `ORDER BY name ASC`，SQLite 对 TEXT 列按 UTF-8 字节序比较（与 Unicode 码点序一致），单点改动即满足全部规则列表的排序诉求。前端各面板继承 DB 返回顺序不做二次排序，无需改动（已核验 RulesPanel/MaskPanel/ExtractPanel/ValidatePanel 均无对规则列表的 `.sort()` 调用——ValidatePanel.jsx:175 的 `.sort((a,b)=>b[1]-a[1])` 是对统计计数排序，非规则列表）。

### 2. 后端正确性 — PASS

- `src-tauri/src/db/mod.rs:525`：SQL 已由 `ORDER BY id ASC` 改为 `ORDER BY name ASC`，diff 与 HANDOFF 描述一致（git show 52d57ae 确认）。
- `src-tauri/src/db/mod.rs:519`：doc comment 已同步为「列出全部规则（按 name 升序，Unicode 码点排序）」。
- 新测试 `list_rules_sorted_by_name`（`src-tauri/src/db/mod.rs:2012-2063`）：
  - 插入顺序故意打乱（手机号提取 → IPv4地址提取 → 姓名校验）。
  - 断言返回顺序 IPv4 < 姓名 < 手机号，码点 I(U+0049) < 姓(U+59D3) < 手(U+624B)，正确覆盖 Unicode 码点排序语义。
  - reviewer 独立运行 `cargo test --lib list_rules_sorted_by_name` → 1 passed。
- 既有测试无回归：
  - `rule_crud_upsert_list_get`（:1748）仅插 1 条规则后断言 `list[0].id`/`list[0].name`，单元素列表排序不影响 → 4 passed（reviewer 复验）。
  - `rule_kind_round_trip_through_db`（:1995）插入 17 条内置规则后只断言 `list.len()==17` 与 kind 闭环，不依赖顺序 → 通过。
  - `seed_builtin_rules_*` 系列测试只断言数量/kind 集合，不依赖顺序。
- `cargo test --lib rule_crud` → 4 passed，无回归。

### 3. 前端无回归 — PASS

- `frontend/src/tauri.js:161` `listRules()` 直接透传 IPC 结果，无排序。
- 四个面板均无对规则列表的二次排序调用，自动继承 DB 新顺序，符合「前端不动」的设计决策。
- `pnpm --prefix frontend build` 由 coder 自验全绿（HANDOFF 记录），前端无逻辑改动，回归风险极低。

### 4. 文档同步 — PASS（synced）

- `docs/versions/1.1.4/更新日志.md:470-504`：新增「R4 续轮：规则列表按名称排序（T79）」章节，含背景 / 设计决策 / 改动清单 / 不变项 / 验收项 E117-E118 / 安全约束，描述与实际改动一致。
- `docs/02-技术设计文档.md:341`：`list_rules` IPC 契约表行追加「T79：按 `name` 升序 Unicode 码点排序」，与代码同步。
- HANDOFF 文件仍存在（`.zcode/tasks/T79-HANDOFF.md`），未被 coder 删除。

### 5. E2E 验证 — PASS

reviewer 独立复验：
- `cargo fmt --all -- --check` → 无输出（通过）。
- `cargo test --lib list_rules_sorted_by_name` → 1 passed。
- `cargo test --lib rule_crud`（4 项既有测试）→ 4 passed。
- clippy / 全量 test / pnpm build：coder 在 HANDOFF 报告全绿（src-tauri 139 + core 165 + doc-tests 13 = 317 passed / 0 failed），reviewer 抽样复验核心相关测试均通过，未发现矛盾。

### 6. 安全约束 — PASS

- SQL 仍为预编译 `prepare` + `query_map([], ...)`，无字符串拼接，ORDER BY 子句为静态字面量，无注入面。
- 无凭据字面量、无密钥硬编码。
- 未创建 tag（`git tag` 无 v1.1.4 相关 tag）。
- 未 merge main（当前仍在 `feat/v1.1.4-r3-hash-dbparse` 分支，HEAD=52d57ae）。
- commit 单一逻辑目的，Conventional Commits 格式（`feat(db): T79 ...`），无夹带无关格式化/重构噪音。

---

## Scope check

none — 改动严格限定于 `list_rules` 排序子句 + doc comment + 1 个新测试 + 2 个文档同步，无越界改动。`get_rule` / `seed_builtin_rules` / DB schema / capabilities / 前端 均未触碰，符合 HANDOFF「不变项」清单。

## Docs check

synced — 更新日志与技术设计文档均已同步 T79 改动，描述与代码一致，无文档缺口。

---

## 改进建议（非阻塞）

1. （流程）若主会话希望 reviewer 严格依 spec §5.2 读取独立 REPORT，可要求 coder 补 `T79-REPORT.md`；当前 HANDOFF 内嵌 E2E 结果的做法可接受，建议在主会话层面明确「REPORT 是否独立落盘」的统一约定，避免后续任务反复出现该观察项。
2. （可选，非本任务范围）`list_rules` 当前依赖 SQLite 默认二进制 collation（UTF-8 字节序 = Unicode 码点序）。若未来出现需要 Unicode 大小写折叠 / 拼音排序的诉求，需引入 `COLLATE` 策略；当前「Unicode 码点」需求下无需处理。

---

## 最终判定

**review_passed** — goal 满足、acceptance_criteria（E117/E118）满足、无关键缺陷、无越界改动、验证充分、文档同步。可流转回主会话进行 `verified_complete` 判定。
