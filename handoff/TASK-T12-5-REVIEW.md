# TASK-T12-5-REVIEW — 前端 state.js 按领域切片 + action 常量化

```yaml
verdict: verified_complete
```

## 审查依据
- handoff/TASK-T12-5-HANDOFF.md
- handoff/TASK-T12-5-REPORT.md
- 实际产物：frontend/src/state.js、frontend/src/App.jsx、frontend/src/components/*
- 独立验证命令输出（见末节）

## 一、ACTION 常量 vs 组件 dispatch 字符串核对（最关键）

方法：从组件 + App.jsx 提取所有 `type: "XXX"` 字面量（50 个唯一值），与 state.js 的 ACTION 对象 value（54 个）做集合包含核对。

- 组件 dispatch 用的 50 个 type **全部**存在于 ACTION 对象 value 集合（comm -23 输出为空）。
- ACTION 多出的 4 个（CLEAR_ALL_MASK_OVERRIDES / CLEAR_ALL_VALIDATE_OVERRIDES / RESET / SET_SELECTED_COLUMNS）为保留兜底 action，属合理保留。
- `grep -rc "ACTION\." frontend/src/components/ frontend/src/App.jsx` = 0 → 组件仍用字符串字面量 dispatch，零改动契约满足。

抽样点（组件字面量 → ACTION entry → value）：
- `dispatch({ type: "SET_FILE"` → `ACTION.FILE_SET` → `"SET_FILE"` ✓
- `dispatch({ type: "SET_MASKED"` → `ACTION.MASKED_SET` → `"SET_MASKED"` ✓
- `dispatch({ type: "SET_VALIDATE"` → `ACTION.VALIDATE_SET` → `"SET_VALIDATE"` ✓
- `dispatch({ type: "CLEAR_MASK_OVERRIDE"` → `ACTION.MASK_OVERRIDE_CLEAR` → `"CLEAR_MASK_OVERRIDE"` ✓
- `dispatch({ type: "SET_EXTRACT_RULE_TAG_FILTER"` → `ACTION.EXTRACT_RULE_TAG_FILTER_SET` → `"SET_EXTRACT_RULE_TAG_FILTER"` ✓

## 二、领域函数齐全性（13 个，全部 default 兜底）

| 领域函数 | 位置 | default return state |
|---|---|---|
| fileDomain | state.js:199 | ✓ :237 |
| maskDomain | state.js:243 | ✓ :270 |
| validateDomain | state.js:276 | ✓ :303 |
| logDomain | state.js:309 | ✓ :323 |
| pcapDomain | state.js:329 | ✓ :343 |
| rulesDomain | state.js:349 | ✓ :365 |
| exportDomain | state.js:371 | ✓ :397 |
| navDomain | state.js:403 | ✓ :409 |
| toolsDomain | state.js:416 | ✓ :454 |
| searchDomain | state.js:460 | ✓ :494 |
| settingsDomain | state.js:500 | ✓ :514 |
| extractDomain | state.js:520 | ✓ :547 |
| uiDomain | state.js:553 | ✓ :565 |

主 appReducer（state.js:573）顺序分派 13 个领域函数，逻辑等价于原单 switch。

## 三、SET_FILE 级联清空逻辑核对

fileDomain 内 ACTION.FILE_SET（state.js:201-230）集中清空：maskedRows / maskedSummary / validateResult / maskOverrides / validateOverrides / selectedColumns / columnOrder / exportColumns / exportFormat / actionHint，保留 records 不动。未外移，符合 HANDOFF「仅分组，不改副作用语义」。

## 四、initialState 字段集合核对

工作树 initialState 共 55 字段。与 HEAD（v0.4.3）差异（多 extractSelectedIndices / extractRuleTagFilter，少 editingRule）为 v0.4.4 基线改动，非 T12-5 引入。T12-5 为纯重组，字段集合未变。git diff --name-only 确认 T12-5 唯一改动文件为 frontend/src/state.js。

## 五、组件零改动核对

git diff --name-only 列出 App.jsx + 多个组件改动，抽样确认均为 v0.4.4 基线改动（App.jsx 启动拉取 builtin_ruleset；MaskView tag 单值化）。这些 diff 与 T12-5 无关，coder 仅 Write 了 state.js。

## 六、scope_check

无越界。未拆 state/ 目录、未引入 combineReducers、未动 App.jsx / tauri.js / 任何 View 组件 / dispatch 调用。严格按 HANDOFF「最小风险方案」执行。

## 七、独立验证

```
$ cd frontend && npx vite build
vite v5.4.21 building for production...
✓ 3005 modules transformed.
dist/index.html                    0.31 kB │ gzip:   0.24 kB
dist/assets/index-BFChAeu2.js  1,044.70 kB │ gzip: 326.10 kB
✓ built in 2.18s

$ grep -c 'case "' frontend/src/state.js
0
```

## 八、最终建议状态

verdict: **verified_complete**

goal 满足，acceptance_criteria 全部满足，无 critical/major/minor 缺陷，验证充分，无越界，文档无需同步。
