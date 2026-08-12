# T78 Reviewer 审查报告

## verdict: review_passed

无阻塞问题。

## 审查依据

- HANDOFF 文件位于 `.zcode/tasks/T78-HANDOFF.md`（存在且内容完整）。
- REPORT 文件缺失（流程缺口，见下方 minor）。本次审查以 HANDOFF 的 goal/scope/acceptance 为判断基准，结合 `git show bf3511e` 实际 diff、仓库现状与独立 E2E 复跑结果交叉验证。
- 改动范围与 HANDOFF 描述一致：7 个文件，单一逻辑目的（手机号前缀配置 UX 统一：校验每行内嵌 + 提取新增前缀输入），无夹带无关改动。

## 6 项审查范围

### 1. Goal 核对 — 满足

两个需求均完整落地：

**需求 1：手机号校验每行内嵌前缀**
- `ValidatePanel.jsx:357-382` Form.List 每行新增 `shouldUpdate` 监听 ruleId，仅 `phone-validate` 时展开 `Form.Item name={[name, "phonePrefixes"]}` 的 Select mode="tags"
- `ValidatePanel.jsx:395-397` 删除底部全局 `Form.Item name="phonePrefixes"`（原 lines 370-381 已移除，diff 确认）
- `ValidatePanel.jsx:127-135` `handleValidate` 从所有 phone-validate 行 `flatMap(r?.phonePrefixes)` 收集 → `Set` 去重 → 三位纯数字过滤，正确
- `ValidatePanel.jsx:412` 底部文案追加「手机号规则可在行内设置前缀白名单。」

**需求 2：手机号提取新增前缀输入**
- `ExtractPanel.jsx:58` `isPhoneExtract = selectedRuleIds.some(id => id === "phone-extract")`
- `ExtractPanel.jsx:229-243` `isPhoneExtract` 时显示 `Form.Item name="phonePrefixes"` Select mode="tags"
- `ExtractPanel.jsx:155-160` `handleExtractValidate` 收集 phonePrefixes（三位纯数字过滤）传 `extractValidateToNewSheet`
- `tauri.js:238` `extractValidateToNewSheet` 新增 `phonePrefixes = []` 参数，invoke payload 含 `phonePrefixes`

### 2. Scope 核对 — none

改动严格限定在 T78 描述的范围内：
- 后端：`extract_validate_to_new_sheet_inner` + `extract_validate_to_new_sheet` 签名 + phone-extract 候选校验循环 + 测试调用点补参 + 新测试
- 前端：tauri.js IPC + ValidatePanel.jsx 行内嵌 + ExtractPanel.jsx 新增输入
- 文档：更新日志 + 技术设计文档同步

无越界改动，无夹带无关重构/格式化噪音。T77-REVIEW.md 文件被一并纳入 commit 是因为前序任务遗留，属合理范围。

### 3. Verification 核对 — 满足

独立复跑四项关键命令全绿：

```
cargo fmt --all -- --check ✓ (无输出)
cargo clippy --all-targets --all-features -- -D warnings ✓ (Finished)
cargo test --all ✓
  - src-tauri: 138 passed / 0 failed / 0 ignored
  - core: 165 passed / 0 failed / 3 ignored
  - doc-tests: 13 passed / 0 failed
  - extract_validate_phone_with_prefix_filter ... ok
pnpm --prefix frontend build ✓ (built in 2.72s)
```

测试覆盖到位：
- `processor.rs:1944-1970` `extract_validate_phone_with_prefix_filter` 验证前缀 `["134"]` → 134 开头 valid / 159 开头 invalid，且 `parse_result.row_count == 1`（只写有效候选到新 Tab），覆盖前缀过滤语义
- `processor.rs` 10 个既有测试调用点全部补 `&[]` 参数（lines 1912/1989/2022/2052/2093/2108/2137/2181/2231），编译通过

### 4. 文档核对 — synced

- `docs/02-技术设计文档.md:375` §4.6 追加 T78 `extract_validate_to_new_sheet` phone_prefixes 参数说明（运行时覆盖语义、空时回落 DB 规则）
- `docs/versions/1.1.4/更新日志.md:385-466` 追加「R4 续轮：手机号前缀配置 UX 统一（T77-T78）」章节，含背景、设计决策、改动清单、验收项 E111-E116、不变项、安全约束

文档与代码语义一致，无文档比代码更乐观的情况。

### 5. commit 核对 — 满足

- Conventional Commits 格式：`feat(processor): T78 手机号前缀配置 UX 统一 — 校验每行内嵌 + 提取新增前缀输入`
- 单一逻辑目的（手机号前缀配置 UX 统一）
- commit message 详尽列出后端/前端/文档/E2E 各项改动
- 无夹带无关格式化/无关重构噪音（T77-REVIEW.md 纳入是前序任务遗留补盘，合理）

### 6. 风险核对 — 无关键回归风险

- **后端覆盖逻辑正确**：`processor.rs:507-517` `is_phone_extract = rule.id == "phone-extract"` 判定精确（不影响 phone-validate / 其他规则）；`!phone_prefixes.is_empty()` 时构造 `ExtractParams::PhonePrefix { allowed_prefixes: phone_prefixes.to_vec() }` 调 `validate_extracted_with_params` 覆盖 DB rule.params；空时保持 `validate_extracted(rule, candidate)` 原逻辑。覆盖是运行时临时覆盖，不持久化，符合设计。
- **前端去重合并正确**：`ValidatePanel.jsx:129-135` `Array.from(new Set(...))` 去重 + 三位纯数字过滤，多行合并语义正确。
- **前端过滤一致**：ExtractPanel 与 ValidatePanel 均用 `/^\d{3}$/` 过滤，语义一致。
- **serde 向后兼容**：`ExtractParams::PhonePrefix` 为既有变体，无 schema 变更，无破坏性。
- **边界**：`maxTagCount` 从原全局的 3 改为 5（ValidatePanel:379 / ExtractPanel:242），一致，非缺陷。
- **SQL**：本任务未改 SQL，无拼接风险。
- **凭据**：无凭据字面量。

## defects

```yaml
defects:
  - severity: minor
    file: .zcode/tasks/T78-REPORT.md (缺失)
    issue: .zcode/tasks/ 下不存在 T78-REPORT.md（handoff/ 目录下也无）
    impact: 流程上 reviewer 无法按 spec 5.2 读取 REPORT 文件核对 coder 自验结果；本次靠独立复跑 E2E 命令替代 REPORT 验证
    fix: 后续任务请按 orchestrator-workflow 规范落盘 REPORT 文件到 .zcode/tasks/ 或 handoff/ 目录
scope_check: none
docs_check: synced
```

## 附加观察（非缺陷，记录供主会话参考）

- **行级前缀 vs 全局语义变化**：原全局 phonePrefixes 对所有 phone-validate 行共享同一白名单；改为每行内嵌后，每行可独立配置不同前缀集，`handleValidate` 合并去重后传后端。这是用户期望的语义增强，文档已说明，不判为缺陷。
- **ExtractPanel 前缀仍是全局字段**：ExtractPanel 只有一个 phone-extract 规则选择（不像 ValidatePanel 多行），因此前缀输入用单一 `Form.Item name="phonePrefixes"` 是合理的，无需每行内嵌。
- **测试仅覆盖单规则场景**：`extract_validate_phone_with_prefix_filter` 只测了单 phone-extract 规则 + 单前缀 `["134"]`。多规则混合（如 phone-extract + idcard-extract 同时选中）+ 多前缀的边界场景未覆盖。这是测试覆盖度的优化空间，非阻塞缺陷（核心逻辑已被单测验证）。

## E2E 验证状态（独立复跑）

```
cargo fmt --all -- --check ✓
cargo clippy --all-targets --all-features -- -D warnings ✓
cargo test --all ✓ (src-tauri 138 + core 165 + doc 13 = 316 passed / 3 ignored / 0 failed)
pnpm --prefix frontend build ✓ (built in 2.72s)
```

## 安全约束

- SQL 全部参数绑定，无拼接 ✓（本任务未改 SQL）
- 无凭据字面量 ✓
- 不创建 tag ✓（`git tag --list` 无 v1.1.4 tag）
- 不自动 merge main ✓（仍在 `feat/v1.1.4-r3-hash-dbparse` 分支）
- Mimosa 未重新运行完整审计，不宣称项目安全
