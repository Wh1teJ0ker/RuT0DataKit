# TASK-T12-5-REPORT — 前端 state.js 按领域切片 + action 常量化

implemented_changes:
  - frontend/src/state.js（重写）
    - 新增 `ACTION` 常量对象（54 个 entry），value 与现有组件 dispatch 字符串字面量逐字符一致（SET_FILE / SET_MASKED / SET_VALIDATE / ... / RESET）。
    - 拆 13 个领域子函数：fileDomain / maskDomain / validateDomain / logDomain / pcapDomain / rulesDomain / exportDomain / navDomain / toolsDomain / searchDomain / settingsDomain / extractDomain / uiDomain。每个领域函数只处理本领域 action，default 分支 `return state`。
    - 主 `appReducer` 改为顺序分派到各领域函数（`state = fileDomain(state, action); state = maskDomain(...); ...`），最后 return。
    - SET_FILE 的级联清空逻辑（maskedRows / maskedSummary / validateResult / maskOverrides / validateOverrides / selectedColumns / columnOrder / exportColumns / exportFormat / actionHint）集中保留在 fileDomain 内，不外移，避免改组件语义。
    - initialState 字段集合完全不变（35+ 字段全保留，注释原样保留）。
    - App.jsx 与所有组件未改动（仍用 `dispatch({ type: "SET_FILE", ... })` 字符串字面量）。
    - 单 useReducer 保留，App.jsx 的 `useReducer(appReducer, initialState)` 零改动。
  - diff stat vs HEAD: state.js +314 / -194，单文件改动。

verification_run:
  - cd frontend && npx vite build
  - grep -c 'case "' frontend/src/state.js
  - grep -rc "ACTION\." frontend/src/components/ frontend/src/App.jsx
  - git diff --name-only frontend/src/state.js
  - git diff --name-only frontend/src/components/ frontend/src/App.jsx（用于确认 components/App 的 git diff 是 v0.4.4 基线改动而非本次 T12-5 引入）

verification_results:
  - npx vite build: ✓ built in 2.34s（3005 modules transformed，dist/assets/index-BFChAeu2.js 1044.70 kB）。仅 chunk size >500kB 的常规 warning，无 error。
  - grep -c 'case "' frontend/src/state.js: 0（顶层 switch 已消除，改用 `switch (action.type) { case ACTION.X: }` 风格，无字面量 case）。
  - grep -rc "ACTION\." frontend/src/components/ frontend/src/App.jsx: 0 命中（组件/App.jsx 未引用 ACTION 常量，仍用字符串字面量 dispatch，符合「组件零改动」要求）。
  - git diff --name-only frontend/src/state.js: frontend/src/state.js（本次 T12-5 唯一改动文件）。
  - git diff --name-only frontend/src/components/ frontend/src/App.jsx: 列出多个文件，但均为 T12-1~T12-4 已完成、停留在工作树未提交的 v0.4.4 基线改动（App.jsx/ExtractView/MaskView/RulesView/Sidebar/ValidateView 等修改 + ActionsPanel/FieldInput/ParamField/PreviewTable/RuleDocPanel/RuleDrawer/RuleForm/RulesPanel 删除）。本次会话 T12-5 仅 Write 了 state.js，未触碰任何组件或 App.jsx。git status 输出与任务开始前状态一致。

docs_updated:
  - 无。state.js 为实现文件，无独立 docs/ 条目对应；行为与 dispatch 契约零变化，无需更新永久文档。

reported_status: verified_complete

scope_deviation:
  - none。严格按 HANDOFF「最小风险方案」：保留单 useReducer，内部按领域分组为子函数，主 reducer 分派；ACTION 常量化但字符串值不变；SET_FILE 级联清空留在 fileDomain 集中处理；未动 App.jsx / tauri.js / 任何 View 组件 / dispatch 调用；未拆 state/ 目录（单文件方案足够）。HANDOFF acceptance_criteria 中提到的「combineReducers 组合」方案在 risks 节已被标注为「最小风险方案」的对立面，明确要求优先采用单 useReducer + 领域子函数方案，故未引入 combineReducers，不构成越界。
