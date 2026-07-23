# TASK-T12-5-HANDOFF — 前端 state.js 按领域切片 + action 常量化

```yaml
task_id: T12-5
goal: |
  将 frontend/src/state.js（385 LOC / 29 action 全平铺）按业务领域切片为
  多个独立 reducer（file/mask/validate/log/pcap/rules/search/tools/extract/
  settings/export/nav/ui），用 combineReducers 组合；action type 抽常量对象。
  现有组件 dispatch 调用签名不变（action.type 字符串值不变）。
in_scope:
  - frontend/src/state.js（重构）
  - frontend/src/state/ 目录（NEW，可选：若拆多文件）
  - frontend/src/App.jsx（useReducer 初始化改 combineReducers）
out_of_scope:
  - 不得改任何组件的 dispatch 调用（action.type 字符串值不变，组件无需改）
  - 不得动 tauri.js 或任何 View 组件内部逻辑
  - 不得改 initialState 字段集合（只是分组管理）
  - 不得删任何字段或 action（纯重组，不删功能）
acceptance_criteria:
  - state.js 或 state/ 目录下有按领域分组的 reducer
  - 29 个 action type 抽为常量（如 const ACTION = { FILE_SET: "SET_FILE", ... }）
  - 组件 dispatch({ type: "SET_FILE", ... }) 调用零改动（字符串值不变）
  - initialState 字段集合不变（35+ 字段全保留）
  - SET_FILE 内的级联清空逻辑保留（不外移，避免改组件）—— 仅分组，不改副作用语义
  - cd frontend && npx vite build 成功
  - 前端运行时行为零回归（手动验证 MaskView/LogView/ExtractView 主流程）
verification_commands:
  - cd frontend && npx vite build
  - grep -c "case \"" frontend/src/state.js（预期 0 或大幅减少，改用常量）
files_likely_to_change:
  - frontend/src/state.js
  - frontend/src/App.jsx
risks:
  - useReducer 无原生 combineReducers，需手写 combineReducers 或用 useReducer 多实例
  - SET_FILE 跨领域清空（maskedRows/validateResult/maskOverrides/validateOverrides），若按领域切片 reducer，SET_FILE 需被多 reducer 监听 → 用「跨切片 action」模式（每个领域 reducer 对 SET_FILE 各自返回对应清空状态）
  - 优先方案：保留单 useReducer，但把 switch 按领域分组为子函数（fileReducer / maskReducer ...），主 reducer 分派。action 常量化但字符串值不变。这是最小风险方案
depends_on: []
status: planned
```

## 领域切片设计（最小风险方案）

保留单 `useReducer`，但内部按领域分组：

```js
// state.js
export const ACTION = {
  FILE_SET: "SET_FILE",
  MASKED_SET: "SET_MASKED",
  MASK_OVERRIDE_SET: "SET_MASK_OVERRIDE",
  // ... 29 个常量
};

const fileDomain = (state, action) => { /* SET_FILE / SET_RECORDS */ };
const maskDomain = (state, action) => { /* SET_MASKED / SET_MASK_OVERRIDE / CLEAR_* */ };
const validateDomain = (state, action) => { /* ... */ };
// ... 11 个领域函数

export function appReducer(state, action) {
  state = fileDomain(state, action);
  state = maskDomain(state, action);
  state = validateDomain(state, action);
  // ... 每个领域函数对不相关 action 返回 state 原样
  return state;
}
```

组件 `dispatch({ type: ACTION.FILE_SET, ... })` 调用不变（字符串值 "SET_FILE" 保持）。
