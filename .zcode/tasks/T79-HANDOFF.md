# T79 交接 — 规则列表按名称排序

## 任务

- **ID**: T79
- **标题**: 所有规则按照首字母顺序自动排序（Unicode 码点）
- **分支**: `feat/v1.1.4-r3-hash-dbparse`
- **提交**: `52d57ae`
- **状态**: implementation_complete，待 reviewer 验收

## 需求

用户要求「所有规则按照首字母顺序自动排序」。经 AskUserQuestion 确认排序方式为「Unicode 码点排序」（非拼音、非 id）。

## 改动清单

### 后端（src-tauri/src/db/mod.rs）

1. `list_rules` 方法 SQL `ORDER BY id ASC` → `ORDER BY name ASC`（line 525）
2. doc comment 同步「按 name 升序（Unicode 码点排序）」
3. 新增测试 `list_rules_sorted_by_name`：插入 3 条规则（name="手机号提取"/"IPv4地址提取"/"姓名校验"），断言返回顺序为 IPv4 < 姓名 < 手机号（码点 I<U+0049> < 姓<U+59D3> < 手<U+624B>）

### 文档

- `docs/versions/1.1.4/更新日志.md`：追加「R4 续轮：规则列表按名称排序（T79）」章节 + 验收项 E117-E118
- `docs/02-技术设计文档.md`：`list_rules` IPC 契约表行追加「T79：按 name 升序 Unicode 码点排序」

### 不变项

- 不改前端任何文件（前端不做排序，继承 DB 返回顺序）
- 不改 `seed_builtin_rules` 注册顺序
- 不改 `get_rule`（单行查询）
- 不改 DB schema / capabilities/default.json

## E2E 验证

- `cargo fmt --all` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test --all` ✅（src-tauri 139 + core 165 + doc-tests 13 = 317 passed / 3 ignored / 0 failed）
- `pnpm --prefix frontend build` ✅

## 验收点

- **E117**: `list_rules` 返回结果按 `name` 字段 Unicode 码点升序排列（`list_rules_sorted_by_name` 测试通过）
- **E118**: E2E 全绿

## 安全约束（延续）

- SQL 全部参数绑定，禁拼接（本任务只改 ORDER BY 子句，无新 SQL 拼接）
- 无凭据字面量
- 不创建 tag（用户等待手工验证，不自动 finalize）
- 不自动 merge main（用户等待手工验证）
