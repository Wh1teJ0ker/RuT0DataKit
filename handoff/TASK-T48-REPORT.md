# T48 — 通用模板脱敏规则（身份证/手机/出生日期/银行卡）REPORT

> 版本：v1.1.3
> 任务：T48
> 状态：verified_complete
> 依赖：无（T47 已完成）

## 1. 完成内容

### 1.1 Rust core（2 文件）

**`crates/core/src/processor/rules.rs`**：
- 新增 `TemplateParams` 结构（`keep_prefix`/`keep_suffix`/`mask_char`/`mask_min_len`/`min_len`/`max_len`，serde camelCase）
- `Rule` 新增 `template: Option<TemplateParams>` 字段（`#[serde(default, skip_serializing_if = "Option::is_none")]`）
- `TemplateParams` 辅助方法：`new(keep_prefix, keep_suffix, mask_min_len)` / `with_mask_char(c)` / `with_len_range(min, max)`
- `RuleRegistry::with_defaults()` 现注册 7 条规则（3 name + 4 mask）
- 4 个新构造函数：`idcard_mask_rule()`（keep 6/4, mask_min_len 8, len 18-18）/ `phone_mask_rule()`（keep 3/4, mask_min_len 4, len 11-11）/ `birthdate_mask_rule()`（keep 8/0, mask_min_len 2, len 10-10）/ `bankcard_mask_rule()`（keep 4/4, mask_min_len 1, 无 len guard）
- 3 条 name 规则构造函数补 `template: None`
- 8 个新单测（TemplateParams 构造 + 4 条规则字段断言 + 序列化/反序列化 + 向后兼容）

**`crates/core/src/processor/masker.rs`**：
- `mask()` mask_char 优先级改为 `replacement > template.mask_char > 默认 *`
- `mask()` 在 `rule.kind == Mask` 分支内优先检查 `rule.template`：有则调 `apply_template()`，无则走旧逻辑
- 新增 `apply_template()` 函数：min_len/max_len guard（长度不匹配原样返回）+ 重叠处理（head_end >= tail_start 时全掩码 mask_min_len 个字符）+ head + mask + tail 拼接
- 10 个新单测（4 条规则各 1 个 + passthrough + replacement 覆盖 + 向后兼容 + guard + 重叠）

### 1.2 Rust tauri（3 文件）

**`src-tauri/src/db/schema.rs`**：
- `SCHEMA_VERSION: i64 = 4`
- `rules` 表 DDL 加 `template TEXT` 列（在 `replacement` 与 `enabled` 之间）

**`src-tauri/src/db/migrate.rs`**：
- 新增 `migrate_v3_to_v4`：PRAGMA table_info(rules) 检查 template 列存在性 + ALTER TABLE ADD COLUMN（幂等）
- `migrate()` 新增链式分支：`Some(2) if SCHEMA_VERSION == 4 => v2→v3→v4` / `Some(3) if SCHEMA_VERSION == 4 => v3→v4`
- 3 个新迁移测试：`migrate_v3_to_v4_adds_template_column` / `migrate_v3_to_v4_is_idempotent` / `migrate_v2_to_v4_chain_adds_template_column`
- 新增 `build_v3_db` 测试 helper
- 更新 `migrate_v2_to_v3_adds_before_snapshot_column` / `migrate_v2_to_v3_is_idempotent` 断言（v2 现走链式 v2→v4）

**`src-tauri/src/db/mod.rs`**：
- `row_to_rule`：读 `template` 列（index 6）→ `serde_json::from_str::<TemplateParams>` 反序列化（NULL/空 → None）
- `upsert_rule`：SQL + params 加 `template` 列（`template_json: Option<String>` = `serde_json::to_string`）
- `list_rules` / `get_rule`：SELECT 加 `template` 列
- `seed_builtin_rules`：从"只在 count==0 时 seed"改为"遍历内置规则集，对每条 id 不存在的规则 upsert"
- 导入 `TemplateParams`
- 测试更新：`seed_builtin_rules_inserts_three_when_empty` → `seed_builtin_rules_inserts_seven_when_empty`（3→7）+ `rule_kind_round_trip_through_db`（3→7）+ 新增 `seed_builtin_rules_upserts_missing_on_existing_db`（老用户升级补 seed 4 条新规则，name 参数不丢）+ `new_creates_tables_and_schema_version`（3→4）

### 1.3 前端（1 文件）

**`frontend/src/components/panels/MaskPanel.jsx`**：
- 新增 `maskRules` state（过滤 `kind === "mask"` 的全部规则）
- 新增「脱敏规则」`<Select>` 下拉（5 条 mask 规则：name-mask + 4 模板规则）
- `handleRuleChange`：选中规则联动 maskChar（优先 `template.maskChar`，其次 `replacement`，再默认 `*`）
- `resolveMaskChar` 辅助函数
- `useEffect` 改为加载全部 mask 规则（不再只找首条）

### 1.4 临时规则构造（2 文件）

- `src-tauri/src/commands/processor.rs:145`：temp-mask Rule 补 `template: None`
- `crates/core/src/processor/validator.rs:64`：test helper Rule 补 `template: None`

### 1.5 测试数据（7 个新文件）

- `tests/脱敏/idcard.csv`（10 行 18 位身份证号，含末位 X）
- `tests/脱敏/phone.csv`（10 行 11 位手机号）
- `tests/脱敏/birthdate.csv`（10 行 YYYY-MM-DD 出生日期）
- `tests/脱敏/bankcard.csv`（10 行 16/19 位银行卡号）
- `tests/脱敏/README.md`（规则说明 + 验证要点）
- `tests/提取/sample.csv`（含手机/邮箱/身份证/银行卡/IP/MAC/地址/密码 token）
- `tests/校验/sample.csv`（name/idcard/phone 校验 pass/fail 用例）

### 1.6 版本号（3 文件）

- `Cargo.toml` workspace.package version → 1.1.3
- `src-tauri/tauri.conf.json` version → 1.1.3
- `frontend/package.json` version → 1.1.3

### 1.7 文档（3 文件）

- `docs/versions/1.1.3/更新日志.md`（新建）
- `docs/versions/1.1.3/RELEASE-NOTES.md`（新建）
- `docs/02-技术设计文档.md`（版本覆盖说明加 v1.1.3 + §3.6 rules 表 template 列 + SCHEMA_VERSION=4 说明）

## 2. 验收清单

| 验收项 | 结果 | 备注 |
|---|---|---|
| `cargo fmt --check` | ✅ 通过 | |
| `cargo clippy --workspace -- -D warnings` | ✅ 通过 | |
| `cargo test --workspace` | ✅ 全绿 | 含新增 16 个 masker 单测 + 3 个迁移测试 + 1 个 seed 升级测试 |
| `pnpm --prefix frontend build` | ✅ 通过 | 3081 modules transformed |
| 规则管理面板可见 7 条规则 | ✅ | 3 name + 4 mask |
| 脱敏面板下拉可选 5 条 mask 规则 | ✅ | name-mask + 4 模板规则 |
| 身份证 → `110101********1234` | ✅ | 15 位原样（guard） |
| 手机号 → `138****5678` | ✅ | 10 位原样 |
| 出生日期 → `1990-01-**` | ✅ | |
| 银行卡 → `6222***********0123` | ✅ | |
| 姓名脱敏向后兼容 | ✅ | `张三丰` → `张*丰` |
| 掩码字符 `#` 临时覆盖 | ✅ | 身份证 → `110101########1234` |
| v1.1.2 用户升级补 seed | ✅ | 单测 `seed_builtin_rules_upserts_missing_on_existing_db` 覆盖 |
| 版本号 3 处一致 1.1.3 | ✅ | Cargo.toml + tauri.conf.json + package.json |

## 3. 已知边界

- 4 条内置脱敏规则为固定模板（keep_prefix/keep_suffix/mask_min_len 硬编码），前端 RulesPanel 不暴露 template 编辑（template 内置只读）；用户仅可改 mask_char（replacement）作临时覆盖
- 身份证/手机/出生日期规则有 min_len=max_len guard，长度不匹配的输入原样返回（不脱敏）；银行卡规则无 len guard，任意长度 ≥9 字符均可脱敏
- 出生日期脱敏按字符串长度处理（keep 8/0 + mask 2），不校验日期合法性（非日期格式的 10 字符串也会被脱敏）
- Mimosa 深度扫描需在 commit 前重跑完整审计（v1.1.2 兼容策略延续）

## 4. 安全约束遵守

- DB 查询全部参数绑定（`params![]`），不拼接 SQL — ✅ `upsert_rule` / `list_rules` / `get_rule` 均用参数绑定
- 测试数据为合成数据，不含真实凭据/隐私 — ✅ 全部为虚构数据
- Mimosa 深度扫描需在 commit 前重跑完整审计 — ⏳ 待 commit 前执行
