# TASK-T12-6-REPORT — 抽 ColumnRuleMapper 公共组件消除 MaskView/ValidateView 重复

implemented_changes:
  - 新建 `frontend/src/components/ColumnRuleMapper.jsx`（253 LOC）：封装「表头-规则映射 Table + 规则下拉 + 摘要列 + 清空按钮」结构。内部封装 `summarizeRule(r)`（从两 View 抽出，签名不变）、`resolveRule` / `resolveOptionValue` / `onChangeRule` / mappingColumns。组件纯展示，不直接 dispatch，通过 `onSetOverride(header, rule)` / `onClearOverride(header)` / `onGotoRules()` 回调让调用方 dispatch。
  - `frontend/src/components/MaskView.jsx`（500 → 323 LOC）：
    - 删除段②重复代码（mappingData / ruleOptions / resolveMask / resolveMaskOptionValue / onChangeRule / mappingColumns）及本地 `summarizeRule` 定义。
    - 保留 `NO_MASK_SENTINEL = "__no_mask__"` 常量（通过 sentinel prop 传入公共组件，组件内不硬编码哨兵）。
    - 保留 `isMaskRule` + `taggedMaskers`（按 tag="mask" 筛选后作为 rules 数组传入公共组件，组件内不再按 tag 筛选）。
    - 段②改为 `<ColumnRuleMapper ... />`，回调内 dispatch `SET_MASK_OVERRIDE` / `CLEAR_MASK_OVERRIDE` / `SET_VIEW`，action.type 字符串字面量保持不变（未改用 ACTION. 常量，符合主会话指示）。
    - 移除不再需要的 antd 导入（Select / Tooltip / DeleteOutlined）。
    - 段①（含 onlyFiltered 开关）、段③预览、段④ onApply / effectiveMaskers 全部保留不变。
  - `frontend/src/components/ValidateView.jsx`（478 → 294 LOC）：
    - 删除段②重复代码及本地 `summarizeRule` 定义。
    - 保留 `NO_VALIDATE_SENTINEL = "__no_validate__"` 常量。
    - 保留 `isValidateRule` + `taggedValidators`。
    - 段②改为 `<ColumnRuleMapper ... />`，回调内 dispatch `SET_VALIDATE_OVERRIDE` / `CLEAR_VALIDATE_OVERRIDE` / `SET_VIEW`。
    - 移除不再需要的 antd 导入（Select / Tooltip / Alert / DeleteOutlined）。
    - 段①、段③预览（红底高亮 valid_matrix 逻辑）、段④ onRun / effectiveValidators 全部保留不变。

verification_run:
  - `cd frontend && npx vite build`
  - `wc -l frontend/src/components/MaskView.jsx frontend/src/components/ValidateView.jsx frontend/src/components/ColumnRuleMapper.jsx`
  - `grep -c "summarizeRule" frontend/src/components/MaskView.jsx frontend/src/components/ValidateView.jsx`
  - `grep -n "summarizeRule" frontend/src/components/MaskView.jsx frontend/src/components/ValidateView.jsx`（确认仅剩注释引用）

verification_results:
  - vite build：成功。输出 `dist/index-ClqBlf24.js 1,043.10 kB`，`✓ built in 2.05s`。仅有 chunk 大小警告（既有，与本次改动无关）。
  - wc -l：
    - MaskView.jsx = 323 LOC（< 350 ✓）
    - ValidateView.jsx = 294 LOC（< 350 ✓）
    - ColumnRuleMapper.jsx = 253 LOC
  - grep -c "summarizeRule"：MaskView=1, ValidateView=1。grep -n 确认两处均为 T12-6 抽取说明注释（`v0.5.0 T12-6：「不脱敏」哨兵、summarizeRule、段②映射表抽到 ColumnRuleMapper。`），无实际调用。符合验收要求「预期 0 或仅 import 引用」。

docs_updated:
  - 未更新 docs/。本次只抽取 UI 组件、不改对外行为或用法，无永久产品文档需要同步。

reported_status:
  - verified_complete

scope_deviation:
  - none。仅改 ColumnRuleMapper.jsx(新) / MaskView.jsx / ValidateView.jsx；未碰 state.js / tauri.js / 其他 View；未改 dispatch action.type；未改段①/③/④试运行与结果渲染逻辑。

补充说明：
- 公共组件 API 与 HANDOFF 设计一致，采用「调用方按 tag 筛选后传 rules 数组」的解耦方案（组件内不再按 tag 筛选）。
- 哨兵通过 `sentinel` / `sentinelLabel` props 传入（NO_MASK_SENTINEL / NO_VALIDATE_SENTINEL 分别保留在各自 View），组件内不硬编码哨兵值。
- MaskView/ValidateView 仍使用字符串字面量 dispatch（`dispatch({ type: "SET_MASK_OVERRIDE", ... })`），未改为 ACTION. 常量，符合主会话「保持与现有组件一致」的指示。
- 运行时回归（选规则→应用→脱敏 / 选规则→运行→红底高亮）的逻辑链路（effectiveMaskers / effectiveValidators 合并 override 优先 + 全局兜底）保留在各自 View 内，行为零回归。
- 未运行手动端到端验证（无 Tauri 运行环境），仅通过 build + LOC + grep 静态验收。如需运行时回归，需 reviewer 在 dev 环境手测。
