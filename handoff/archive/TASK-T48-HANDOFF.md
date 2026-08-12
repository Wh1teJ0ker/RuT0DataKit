# T48 — 通用模板脱敏规则（身份证/手机/出生日期/银行卡）HANDOFF

> 版本：v1.1.3
> 任务：T48
> 状态：verified_complete
> 依赖：无（T47 已完成）

## 1. 需求

在 v1.1.2 的 `SimpleMasker`（仅"保留首尾各 1 字符"）之上，引入**通用模板脱敏算子**（参考 v0.8.0 `TemplateOp`：keep_prefix / keep_suffix / mask_char / mask_min_len / min_len / max_len），并落地 4 条内置脱敏规则。测试数据按用户要求存放到 `tests/脱敏/`、`tests/提取/`、`tests/校验/` 三个文件夹。

### 4 条内置脱敏规则

| 规则 ID | 名称 | keep_prefix | keep_suffix | mask_min_len | min=max | 示例 |
|---|---|---|---|---|---|---|
| `idcard-mask` | 身份证号脱敏 | 6 | 4 | 8 | 18 | `110101199001011234` → `110101********1234` |
| `phone-mask` | 手机号脱敏 | 3 | 4 | 4 | 11 | `13812345678` → `138****5678` |
| `birthdate-mask` | 出生日期脱敏 | 8 | 0 | 2 | 10 | `1990-01-15` → `1990-01-**` |
| `bankcard-mask` | 银行卡号脱敏 | 4 | 4 | 1 | — | `6222021234567890123` → `6222***********0123` |

## 2. 改动文件清单

### Rust core（2 文件）
1. `crates/core/src/processor/rules.rs` — `TemplateParams` 结构 + `Rule.template` 字段 + 4 构造函数（`idcard_mask_rule`/`phone_mask_rule`/`birthdate_mask_rule`/`bankcard_mask_rule`）+ 3 name 规则补 `template:None` + 8 个单测
2. `crates/core/src/processor/masker.rs` — `mask()` template 分支 + `apply_template()` 函数 + mask_char 优先级 `replacement > template.mask_char > 默认 *` + 10 个单测

### Rust tauri（3 文件）
3. `src-tauri/src/db/schema.rs` — `SCHEMA_VERSION=4` + `rules` 表 DDL 加 `template TEXT` 列
4. `src-tauri/src/db/migrate.rs` — `migrate_v3_to_v4`（PRAGMA 检查 + ALTER 幂等）+ `migrate()` 链式分支 `Some(2) => v2→v3→v4` / `Some(3) => v3→v4` + 3 个迁移测试
5. `src-tauri/src/db/mod.rs` — `row_to_rule`（读 template 列 + serde_json 反序列化）+ `upsert_rule`（SQL + params 加 template）+ `list_rules`/`get_rule` SELECT 加 template + `seed_builtin_rules` 改 upsert-missing + 3 个测试更新（3→7 + 新增升级测试）

### 前端（1 文件）
6. `frontend/src/components/panels/MaskPanel.jsx` — `maskRules` state + 脱敏规则 `<Select>` 下拉 + `handleRuleChange` 联动 maskChar + `resolveMaskChar` 辅助函数

### 临时规则构造补 template:None（2 文件）
7. `src-tauri/src/commands/processor.rs` — temp-mask Rule 补 `template:None`
8. `crates/core/src/processor/validator.rs` — test helper Rule 补 `template:None`

### 测试数据（7 个新文件）
9-12. `tests/脱敏/{idcard,phone,birthdate,bankcard}.csv`
13. `tests/脱敏/README.md`
14. `tests/提取/sample.csv`
15. `tests/校验/sample.csv`

### 版本号（3 文件）
16. `Cargo.toml` workspace.package version → 1.1.3
17. `src-tauri/tauri.conf.json` version → 1.1.3
18. `frontend/package.json` version → 1.1.3

### 文档（3 文件）
19. `docs/versions/1.1.3/更新日志.md`（新建）
20. `docs/versions/1.1.3/RELEASE-NOTES.md`（新建）
21. `docs/02-技术设计文档.md`（版本覆盖说明 + §3.6 rules 表 template 列 + SCHEMA_VERSION=4 说明）

### 交接（本文件 + 报告）
22. `handoff/TASK-BOARD.md`（v1.1.3 T48）
23. `handoff/TASK-T48-HANDOFF.md`（本文件）
24. `handoff/TASK-T48-REPORT.md`

## 3. 关键设计决策

### 3.1 TemplateParams 结构（JSON 存储，不新增 5 个 typed 列）

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TemplateParams {
    pub keep_prefix: Option<usize>,   // 默认 0
    pub keep_suffix: Option<usize>,   // 默认 0
    pub mask_char: Option<char>,      // 默认 '*'
    pub mask_min_len: Option<usize>,  // 默认 1
    pub min_len: Option<usize>,       // 默认 None（不设下限 guard）
    pub max_len: Option<usize>,       // 默认 None（不设上限 guard）
}
```

`Rule.template: Option<TemplateParams>` 用 `#[serde(default, skip_serializing_if = "Option::is_none")]`，保证旧 JSON（name-mask 无 template）向后兼容。

### 3.2 mask_char 优先级：`replacement > template.mask_char > 默认 *`

- `replacement`（脱敏面板输入框）作**临时覆盖**：保留 `mask_column` 命令的 `replacement` 参数语义
- `template.mask_char` 作模板内置默认（4 条新规则均为 `*`）
- `*` 作兜底

### 3.3 SimpleMasker template 分支

`mask()` 在 `rule.kind == Mask` 分支内优先检查 `rule.template`：
- 有 template：走 keep_prefix/keep_suffix/mask_min_len + min_len/max_len guard + 重叠处理（head_end >= tail_start 时全掩码 mask_min_len 个字符）
- 无 template：走旧逻辑（保留首尾各 1），向后兼容

### 3.4 DB schema v3→v4

- `schema.rs`：`SCHEMA_VERSION=4`，`rules` 表 DDL 加 `template TEXT` 列
- `migrate.rs`：`migrate_v3_to_v4`（PRAGMA table_info 检查 + ALTER 幂等）；`migrate()` 链式分支 `Some(2) => v2→v3→v4` / `Some(3) => v3→v4`
- `db/mod.rs`：`row_to_rule` 读 template 列（index 6）→ serde_json::from_str；`upsert_rule` SQL + params 加 template；`list_rules`/`get_rule` SELECT 加 template

### 3.5 seed_builtin_rules 改 upsert-missing

从"只在 count==0 时 seed"改为"遍历内置规则集，对每条 id 不存在的规则 upsert"：
- v1.1.2 老用户升级 → 启动自动补 seed 4 条新脱敏规则
- 已存在规则（含用户修改过 pattern/replacement 的）不动，参数不丢

### 3.6 前端脱敏规则下拉

`MaskPanel.jsx` 新增 `maskRules` state（过滤 `kind === "mask"` 的全部规则）+ `<Select>` 下拉（5 条 mask 规则）。选中规则联动 maskChar（优先 `template.maskChar`，其次 `replacement`，再默认 `*`）。

## 4. 验收结果

- [x] `cargo fmt --check`：通过
- [x] `cargo clippy --workspace -- -D warnings`：通过
- [x] `cargo test --workspace`：全绿（含新增 16 个 masker 单测 + 3 个迁移测试 + 1 个 seed 升级测试）
- [x] `pnpm --prefix frontend build`：通过
- [x] 版本号 3 处一致 1.1.3

## 5. 安全约束

- DB 查询全部参数绑定（`params![]`），不拼接 SQL
- 测试数据为合成数据，不含真实凭据/隐私
- Mimosa 深度扫描需在 commit 前重跑完整审计（v1.1.2 兼容策略延续）
