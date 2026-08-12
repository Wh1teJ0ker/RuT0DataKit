# T50 — 清理遗留规则 + 移除前端「目标列」显示 REPORT

> 版本：v1.1.3
> 任务：T50
> 状态：verified_complete
> 依赖：T49（已 verified_complete）

## 1. 完成内容

### 1.1 Rust tauri（1 文件）

**`src-tauri/src/db/mod.rs`**：
- `seed_builtin_rules` 末尾新增 `self.cleanup_deprecated_rules()?;` 调用
- 新增私有方法 `cleanup_deprecated_rules`：
  ```rust
  fn cleanup_deprecated_rules(&self) -> Result<(), DbError> {
      let conn = self.conn.lock().expect("db mutex poisoned");
      for id in &["idcard-mask", "phone-mask", "birthdate-mask", "bankcard-mask"] {
          conn.execute("DELETE FROM rules WHERE id = ?1", params![id])?;
      }
      Ok(())
  }
  ```
- 新增测试 `cleanup_deprecated_rules_removes_legacy_ids`：
  - 手动 upsert 4 条废弃 id 的 Rule（模拟 T48 老 DB）
  - 调 `seed_builtin_rules`
  - 断言 4 条废弃 id 已 `get_rule(...).is_none()`
  - 断言 `count_rules() == 4`（3 name + general-mask）
  - 断言 4 条内置规则仍在

### 1.2 前端（1 文件）

**`frontend/src/components/panels/RulesPanel.jsx`**：
- 删除规则信息卡片中「目标列」Row block（6 行）：
  ```jsx
  // 已删除
  <Col span={6}><Text type="secondary">目标列</Text></Col>
  <Col span={18}><Text code>{selected.field || "（不限定）"}</Text></Col>
  ```
- 后端 `Rule.field` 字段、DB `field` 列、IPC 序列化均不变

### 1.3 文档（3 文件）

- `docs/versions/1.1.3/更新日志.md`：追加 T50 进度行 + E1（80→83 单测）+ E3（不再显示「目标列」行）+ 关键设计决策新增 T50 条目 + 已知边界更新（遗留 id 自动清理 + 前端移除目标列显示）
- `handoff/TASK-BOARD.md`：状态改 T50 done_e2e + DAG 追加 T50 节点 + 任务清单追加 T50 行 + E1/E3 更新
- `handoff/TASK-T50-HANDOFF.md` + `handoff/TASK-T50-REPORT.md`（本文件）

## 2. 验收清单

| 验收项 | 结果 | 备注 |
|---|---|---|
| `cargo fmt --check` | ✅ 通过 | |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ 通过 | |
| `cargo test --workspace` | ✅ 全绿 | 83 个单测（新增 `cleanup_deprecated_rules_removes_legacy_ids`） |
| `pnpm --prefix frontend build` | ✅ 通过 | |
| 4 条废弃 id 被清理 | ✅ | `cleanup_deprecated_rules_removes_legacy_ids` 单测验证 |
| 前端规则信息卡片不再显示「目标列」行 | ✅ | RulesPanel.jsx 删除 6 行 |
| 后端 Rule.field 字段不变 | ✅ | DB schema / IPC 契约 / Rule 结构均不动 |
| 4 条内置规则不受影响 | ✅ | 单测断言 name-validate/name-mask/name-extract/general-mask 仍在 |

## 3. 已知边界

- `cleanup_deprecated_rules` 只删 4 个已知废弃 id（硬编码），不影响未来可能的用户自定义规则
- 前端移除「目标列」显示行，但后端 `Rule.field` 字段仍保留（name 规则的 `field: "name"` 用于目标列绑定语义，只是不在规则信息卡片展示）
- Mimosa 深度扫描需在 commit 前重跑完整审计（v1.1.2 兼容策略延续）

## 4. 安全约束遵守

- DB 查询全部参数绑定（`params![]`），不拼接 SQL — ✅ `cleanup_deprecated_rules` 用 `?1` + `params![id]`
- 只删 4 个已知废弃 id，不删其他规则 — ✅ 硬编码列表
- Mimosa 深度扫描需在 commit 前重跑完整审计 — ⏳ 待 commit 前执行
