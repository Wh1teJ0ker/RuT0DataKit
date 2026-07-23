# TASK-T12-6-HANDOFF — 抽 ColumnRuleMapper 公共组件消除 MaskView/ValidateView 重复

```yaml
task_id: T12-6
goal: |
  抽取 frontend/src/components/ColumnRuleMapper.jsx 公共组件，封装 MaskView 与
  ValidateView 高度重复的「表头-规则映射 Table + 规则下拉 + 摘要列 + 清空按钮」
  结构，两 View 改为消费公共组件并传入 tag/override 操作回调。行为零回归。
in_scope:
  - frontend/src/components/ColumnRuleMapper.jsx（NEW，公共组件）
  - frontend/src/components/MaskView.jsx（改用 ColumnRuleMapper）
  - frontend/src/components/ValidateView.jsx（改用 ColumnRuleMapper）
out_of_scope:
  - 不得改 state.js（T12-5 已处理）
  - 不得改 tauri.js
  - 不得改其他 View 组件
  - 不得改 MaskView/ValidateView 的试运行逻辑、结果渲染逻辑（仅抽段②映射表）
  - 不得改 dispatch action 类型
acceptance_criteria:
  - ColumnRuleMapper.jsx 封装：表头列表 → 每行一个规则下拉（按 tag 筛选 state.rules）+ 摘要显示 + 清空按钮
  - MaskView 段②改 <ColumnRuleMapper tag="mask" overrides={state.maskOverrides} onSet={(h,r)=>dispatch SET_MASK_OVERRIDE} onClear={(h)=>dispatch CLEAR_MASK_OVERRIDE} />
  - ValidateView 段②改 <ColumnRuleMapper tag="validate" overrides={state.validateOverrides} onSet/onClear 对应 validate override action />
  - summarizeRule / NO_MASK_SENTINEL / NO_VALIDATE_SENTINEL 抽到公共组件或 props 化
  - MaskView.jsx / ValidateView.jsx 各自 LOC 下降（目标 < 350）
  - cd frontend && npx vite build 成功
  - 手动验证：MaskView 选规则 → 应用 → 看脱敏结果；ValidateView 选规则 → 运行 → 看红底高亮，主流程不破
verification_commands:
  - cd frontend && npx vite build
  - wc -l frontend/src/components/MaskView.jsx frontend/src/components/ValidateView.jsx（目标各 < 350）
files_likely_to_change:
  - frontend/src/components/ColumnRuleMapper.jsx
  - frontend/src/components/MaskView.jsx
  - frontend/src/components/ValidateView.jsx
risks:
  - MaskView 段②有「仅显示搜索命中行」开关（onlyFiltered）影响 rawData，ValidateView 没有 → 公共组件只抽段②映射表，不抽段①原始数据表
  - summarizeRule 两边重复实现，抽到公共组件内
  - 哨兵常量 NO_MASK_SENTINEL / NO_VALIDATE_SENTINEL 不同，组件用 props 传入或统一为一个 __none__ 哨兵
depends_on: [T12-5]
status: planned
```

## 公共组件 API 设计

```jsx
<ColumnRuleMapper
  headers={headers}
  rules={state.rules.maskers}        // 或 state.rules.validators
  tag="mask"                          // 筛选规则 tag
  overrides={state.maskOverrides}     // 当前映射 { [header]: rule }
  onSetOverride={(header, rule) => dispatch({ type: ACTION.MASK_OVERRIDE_SET, header, rule })}
  onClearOverride={(header) => dispatch({ type: ACTION.MASK_OVERRIDE_CLEAR, header })}
  sentinel="__no_mask__"              // 「不应用」选项哨兵
/>
```

组件内部：
- 渲染 antd Table，每行一个表头
- 每行一个 Select：选项 = rules.filter(r => r.tag === tag) + 哨兵「不应用」
- 选中规则 → onSetOverride(header, rule)；选哨兵 → onClearOverride(header)
- 摘要列显示 summarizeRule(overrides[header]) 或「—」
