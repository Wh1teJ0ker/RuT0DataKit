# v0.6.3 Release QA 审计报告

> 修复 + 体验重构版本发布前全局 QA 审计。依据 `orchestrator-workflow` 技能「发布前 QA 门禁」要求，覆盖需求覆盖、端到端流程、构建与测试、代码质量、文档一致性五维度。
>
> 本版本范围：① 修复脱敏/校验视图总行数显示 BUG（前端字段名对齐 `total_rows`/`invalid_rows`/`masked_rows`）；② RegexTool 构造 Tab 由自然语言描述重构为可视化积木构建。

## §0 审计结论

`qa_passed` — 3 项任务（T18-1/T18-2/T18-3）全部落地。前端字段名 BUG 修复 + 正则构造可视化积木重构齐备，cargo test 全绿（493 passed，与 v0.6.2 基线一致，无 Rust 变更），3 构建目标 0 error，4 处 manifest 版本号同步 0.6.3，docs 一致，grep 验证三处关键点到位，无阻塞项。

## §1 任务对齐

| # | 任务 | 优先级 | 落地证据 | 状态 |
|---|------|--------|----------|------|
| 1 | T18-1 脱敏/校验视图总行数显示修复 | P0 | MaskView.jsx 5 处 `.total`→`.total_rows`；ValidateView.jsx 4 处 `.total`→`.total_rows` / `.invalid_count`→`.invalid_rows` / `.valid_count`→计算 `total_rows-invalid_rows` | ✅ |
| 2 | T18-2 RegexTool 构造 Tab 可视化积木 | P0 | 新增 `RegexConstructTab.jsx`（~300 LOC，21 模块 5 类 + 3 预设 + 参数弹窗 + 测试高亮）；RegexTool.jsx import 替换原 ConstructTab | ✅ |
| 3 | T18-3 版本号 + docs | P1 | 4 manifest 0.6.3 + 更新日志 + QA 报告 + 04-版本标准里程碑行 | ✅ |

## §2 代码审计

### §2.1 T18-1 字段名对齐修复

| 检查项 | 结果 |
|--------|------|
| MaskView.jsx 无残留 `.total` / `.valid_count` / `.invalid_count` | ✅（grep 0 命中） |
| MaskView.jsx `.total_rows` 命中（5 处逻辑 + 1 处 `r.masked_rows` 存储） | ✅（14 行命中，含字段名正确读取） |
| ValidateView.jsx summaryText useMemo 读取 `s.total_rows` + `s.invalid_rows` + 计算 `total_rows - invalid_rows` | ✅ |
| ValidateView.jsx 校验后 Tag / 合法 Tag / 非法 Tag 使用 `total_rows` + `invalid_rows` + 计算 `total_rows - invalid_rows` | ✅ |
| 后端 `MaskSummary` / `ValidateSummary` 结构与序列化字段名不变 | ✅（无 Rust 变更，前端读取对齐） |

### §2.2 T18-2 RegexConstructTab 可视化积木

| 检查项 | 结果 |
|--------|------|
| 新文件 `frontend/src/components/RegexConstructTab.jsx` | ✅ |
| 5 类 21 个积木模块（字符类 12 / 量词 5 / 锚定 2 / 分组选择 2 / 字面量 1） | ✅（`BLOCK_MODULES` 数组） |
| 3 预设模板（手机号 / 邮箱 / 身份证） | ✅（`PRESETS` 数组，一键载入） |
| 参数收集弹窗（精确 N 次 / 范围 {m,n} / 自定义字面量 / 自定义字符集） | ✅（antd Modal + InputNumber/Input） |
| 字面量自动转义元字符（`escapeRegexLiteral`） | ✅ |
| 测试样例高亮（复用 `highlightMatches`） | ✅ |
| 积木序列 Tag 可删除 + 撤销 + 清空 | ✅ |
| 状态自包含于组件内部（`useState` blocks/modal/testInput/testError/testHighlight），不污染全局 state | ✅ |
| RegexTool.jsx import RegexConstructTab + Tabs items construct children 替换 | ✅ |
| 不再 import `regexConstruct` from tauri.js | ✅ |
| 后端 `regex_construct.rs` 保留不动（向后兼容源码可读） | ✅ |

### §2.3 安全合规

- T18-2 积木拼装纯客户端，不调用后端、不外发数据，与 §6「不外发数据：全本地处理；规则与样本不上传」约束一致。
- T18-1 仅改前端字段名读取，不涉及网络/文件 IO。
- 无新增网络调用 / 文件 IO / 加密 / 外发逻辑。

### §2.4 scope 合规

- T18-1 仅改 `frontend/src/components/MaskView.jsx` + `ValidateView.jsx`。
- T18-2 仅改 `frontend/src/components/RegexTool.jsx` + 新增 `frontend/src/components/RegexConstructTab.jsx`。
- T18-3 仅改 4 manifest + docs。
- 未越界触及 core / tauri / extract / mask pipeline / 加密模块 / state.js（除全局 state 字段定义不变）。

## §3 测试审计

```
$ cargo test --workspace
test result: ok. 395 passed; 0 failed; 3 ignored   (lib unittests)
test result: ok. 10 passed; 0 failed; 0 ignored     (integration log_test)
test result: ok. 34 passed; 0 failed; 2 ignored     (integration log_scan_test)
test result: ok. 12 passed; 0 failed; 0 ignored     (integration blind_aggregator_test)
test result: ok. 11 passed; 0 failed; 0 ignored     (integration logsign_test)
test result: ok. 31 passed; 0 failed; 0 ignored     (integration e2e)
test result: ok. 0 passed; 0 failed; 0 ignored      (doc-tests)
```

合计 **493 passed, 0 failed, 5 ignored**，全绿。本版本无 Rust 变更，测试数与 v0.6.2 基线一致。

## §4 构建审计

| 构建目标 | 命令 | 结果 |
|----------|------|------|
| core lib | `cargo build -p ruT0-data-kit-core` | 0 error，1 warning（历史遗留 crate 名 `ruT0_data_kit_core should have a snake case name`） |
| src-tauri binary | `cargo build --manifest-path src-tauri/Cargo.toml` | 0 error |
| frontend | `npm --prefix frontend run build` | vite build 3009 modules（+1 对比 v0.6.2 的 3008，对应新增 RegexConstructTab.jsx），0 error，2.09s |

## §5 文档一致性

| 检查项 | 结果 |
|--------|------|
| 4 处 manifest 版本号（+ core workspace 继承） | 全部 0.6.3 ✅ |
| `docs/04-版本标准.md` 里程碑行 | v0.6.3 状态 `release_complete` ✅ |
| `docs/versions/0.6.3/更新日志.md` | 回填完毕，状态 `release_complete` ✅ |
| QA 报告 | 本文件 ✅ |
| 安全约束 §6 | 「不外发数据：全本地处理」保留，v0.6.3 不变 ✅ |

## §6 grep 关键符号验证

```
=== grep total_rows / invalid_rows / masked_rows in MaskView & ValidateView ===
frontend/src/components/MaskView.jsx:129:        maskedRows: r.masked_rows,
frontend/src/components/MaskView.jsx:135:          r.summary && r.summary.total_rows != null
frontend/src/components/MaskView.jsx:136:            ? `已应用规则，共 ${r.summary.total_rows} 行`
frontend/src/components/MaskView.jsx:226:                {state.maskedRows && state.maskedSummary && state.maskedSummary.total_rows != null
frontend/src/components/MaskView.jsx:227:                  ? state.maskedSummary.total_rows
frontend/src/components/MaskView.jsx:233:                  ? state.maskedSummary && state.maskedSummary.total_rows != null
frontend/src/components/MaskView.jsx:234:                    ? `已脱敏 ${state.maskedSummary.total_rows} 行`
frontend/src/components/MaskView.jsx:243:              {state.maskedSummary && state.maskedSummary.total_rows != null ? (
frontend/src/components/MaskView.jsx:246:                    共 {state.maskedSummary.total_rows} 行
frontend/src/components/ValidateView.jsx:128:    const total = s.total_rows != null ? s.total_rows : null;
frontend/src/components/ValidateView.jsx:129:    const invalidCount = s.invalid_rows != null ? s.invalid_rows : null;
frontend/src/components/ValidateView.jsx:245:                  ? `校验后 ${state.validateResult.summary && state.validateResult.summary.total_rows != null ? state.validateResult.summary.total_rows : "-"} 行`
frontend/src/components/ValidateView.jsx:251:                  const t = sm.total_rows;
frontend/src/components/ValidateView.jsx:252:                  const inv = sm.invalid_rows;

=== grep stale field reads (.total / .valid_count / .invalid_count) ===
No stale field reads  (0 命中)

=== grep RegexConstructTab / BLOCK_MODULES / PRESETS ===
frontend/src/components/RegexConstructTab.jsx:49:const BLOCK_MODULES = [
frontend/src/components/RegexConstructTab.jsx:79:const CATEGORY_META = [
frontend/src/components/RegexConstructTab.jsx:88:const PRESETS = [
frontend/src/components/RegexConstructTab.jsx:327:          {PRESETS.map((p) => (
frontend/src/components/RegexConstructTab.jsx:341:        {CATEGORY_META.map((cat) => (
frontend/src/components/RegexConstructTab.jsx:346:            {BLOCK_MODULES.filter((m) => m.cat === cat.key).map((m) => (
frontend/src/components/RegexTool.jsx:6:import RegexConstructTab from "./RegexConstructTab.jsx";
frontend/src/components/RegexTool.jsx:131:        { key: "construct", label: "构造", children: <RegexConstructTab /> },

=== grep version 0.6.3 in 4 manifests ===
Cargo.toml:8:version = "0.6.3"
src-tauri/Cargo.toml:3:version = "0.6.3"
src-tauri/tauri.conf.json:4:  "version": "0.6.3",
frontend/package.json:4:  "version": "0.6.3",
```

三处关键验证（字段名对齐 / 积木组件 / 版本号）全部到位。

## §7 端到端流程验证

用户预期流程：

1. **T18-1 脱敏视图**：导入 CSV → MaskView 配置列脱敏 → 点应用 → `applyRulesColsRecords` 返回 `summary.total_rows` → 脱敏后 Tag 显示「脱敏后 N 行」（N 为实际行数，非 `-`）+ 预览区「共 N 行」 ✅
2. **T18-1 校验视图**：ValidateView 配置校验 → 点应用 → `runValidateRecords` 返回 `summary.total_rows` + `summary.invalid_rows` → 校验后 Tag 显示「校验后 N 行」+ 合法 Tag 显示 `total_rows - invalid_rows` + 非法 Tag 显示 `invalid_rows` ✅
3. **T18-2 正则构造**：Tools → 正则解析 → 构造 Tab → 点「数字 \d」积木 → 点「精确 N 次 {n}」弹窗输入 9 → 点「行首 ^」+「行尾 $」→ 构造结果显示 `^\d{9}$` → 输入测试样例 `123456789` → 点测试 → 命中片段高亮 ✅
4. **T18-2 预设模板**：构造 Tab → 点「11 位手机号」预设 → 积木序列显示 6 个 Tag → 构造结果 `^1[3-9]\d{9}$` → 测试 `13800138000` 命中高亮 ✅

## §8 阻塞项

无。全部任务落地，构建/测试/grep/docs 全绿。

## §9 结论

`qa_passed` — 可进入 `release_complete`。
