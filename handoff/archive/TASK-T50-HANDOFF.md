# T50 — 清理遗留规则 + 移除前端「目标列」显示 HANDOFF

> 版本：v1.1.3
> 任务：T50
> 状态：verified_complete
> 依赖：T49（已 verified_complete）

## 1. 需求

用户要求：
1. **删除多余的规则** — v1.1.3 T48 曾落地的 4 条独立脱敏规则 id（`idcard-mask`/`phone-mask`/`birthdate-mask`/`bankcard-mask`）在 T49 收敛为 `general-mask` 预设后，需从已有用户 DB 中清理。当前 `seed_builtin_rules` 只 upsert 缺失规则，不删除遗留规则。
2. **所有规则信息的目标列（不限定）全部删除，仅删除前端显示** — 移除 `RulesPanel.jsx` 规则信息卡片中的「目标列」行，后端 `Rule.field` 字段不动。

## 2. 改动文件清单

### Rust tauri（1 文件）
1. `src-tauri/src/db/mod.rs` — `seed_builtin_rules` 末尾新增 `cleanup_deprecated_rules()` 调用 + 新增私有方法 `cleanup_deprecated_rules`（`DELETE FROM rules WHERE id = ?1` + `params![id]`，参数绑定，遍历 4 个废弃 id）+ 新测试 `cleanup_deprecated_rules_removes_legacy_ids`

### 前端（1 文件）
2. `frontend/src/components/panels/RulesPanel.jsx` — 删除规则信息卡片中「目标列」Row block（6 行：`<Col span={6}>目标列</Col>` + `<Col span={18}><Text code>{selected.field || "（不限定）"}</Text></Col>`）

### 文档（3 文件）
3. `docs/versions/1.1.3/更新日志.md` — 追加 T50 进度行 + 更新 E1/E3 验收 + 关键设计决策新增 T50 条目 + 已知边界更新
4. `handoff/TASK-BOARD.md` — 追加 T50 任务行 + DAG + E1/E3 更新
5. `handoff/TASK-T50-HANDOFF.md`（本文件）+ `handoff/TASK-T50-REPORT.md`

## 3. 关键设计决策

### 3.1 cleanup_deprecated_rules：seed 末尾清理遗留 id

`seed_builtin_rules` 在 upsert 缺失规则之后，调用 `cleanup_deprecated_rules()` 删除 4 个已知废弃 id：

```rust
fn cleanup_deprecated_rules(&self) -> Result<(), DbError> {
    let conn = self.conn.lock().expect("db mutex poisoned");
    for id in &["idcard-mask", "phone-mask", "birthdate-mask", "bankcard-mask"] {
        conn.execute("DELETE FROM rules WHERE id = ?1", params![id])?;
    }
    Ok(())
}
```

- **幂等**：id 不存在时 `DELETE` 影响 0 行，不报错
- **参数绑定**：`?1` + `params![id]`，禁止字符串拼接 SQL
- **放在 upsert 之后**：保证 `general-mask` 先补 seed，再清理旧 id
- **只删已知废弃 id**：硬编码 4 个 id，不影响未来可能的用户自定义规则

### 3.2 前端移除「目标列」显示行

`RulesPanel.jsx` 规则信息卡片删除「目标列」Row block。后端 `Rule.field` 字段、DB `field` 列、IPC 序列化均不动 — `field` 仍用于 name 规则的目标列绑定语义（如 `name-validate` 的 `field: "name"`），只是不在规则信息卡片展示。

## 4. 验收结果

- [x] `cargo fmt --check`：通过
- [x] `cargo clippy --workspace --all-targets -- -D warnings`：通过
- [x] `cargo test --workspace`：全绿（83 个单测，含新增 `cleanup_deprecated_rules_removes_legacy_ids`）
- [x] `pnpm --prefix frontend build`：通过

## 5. 安全约束

- DB 查询全部参数绑定（`params![]`），不拼接 SQL — `cleanup_deprecated_rules` 用 `?1` + `params![id]`
- 只删 4 个已知废弃 id，不删其他规则
- Mimosa 深度扫描需在 commit 前重跑完整审计（v1.1.2 兼容策略延续）
