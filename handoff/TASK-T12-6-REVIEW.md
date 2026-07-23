# TASK-T12-6-REVIEW — 抽 ColumnRuleMapper 公共组件

```yaml
verdict: verified_complete
```

## 审查依据
- handoff/TASK-T12-6-HANDOFF.md
- handoff/TASK-T12-6-REPORT.md
- 实际产物：frontend/src/components/ColumnRuleMapper.jsx、MaskView.jsx、ValidateView.jsx、state.js
- 独立验证命令输出（见末节）

## 一、段②抽取完整性

ColumnRuleMapper.jsx 封装「表头列表 + 规则下拉 + 摘要列 + 清空按钮」全套，含 mappingData / ruleOptions / resolveRule / resolveOptionValue / onChangeRule / mappingColumns。

- 下拉源：rulesArr（调用方按 tag 筛选后传入）+ 哨兵项置首。与原 MaskView taggedMaskers / ValidateView taggedValidators 等价。
- 选中：onChangeRule → onSetOverride(header, {...rule})，rule 用 spread 保留 field/scope/tag/params/description。与原 {...rule} 一致。
- 清空：哨兵 → onClearOverride(header)；操作列 DeleteOutlined → onClearOverride(row.header)。与原 CLEAR_*_OVERRIDE 一致。
- 摘要：summarizeRule(existing) 或哨兵 label。与原一致。
- 无规则 Alert + Empty 引导：noRuleTitle/noRuleDesc/hasRecords props 化，等价。

MaskView.jsx:233-253 与 ValidateView.jsx:208-228 段②均改为 `<ColumnRuleMapper .../>`，无残留旧 mappingColumns/ruleOptions 代码。

## 二、段①/③/④保留核对

MaskView：
- 段① onlyFiltered 开关 + filteredRowIndices 过滤 + effectiveRows/rawData 保留（:59-86, :186-230）。
- 段③ 预览（previewData/previewColumns/SET_MASKED 渲染）保留（:100-118, :256-293）。
- 段④ onApply + effectiveMaskers（override 优先 + 全局兜底合并）保留（:124-175, :296-319）。

ValidateView：
- 段① 原始数据表保留（:53-70, :180-205）。
- 段③ 红底高亮 `valid_matrix?.[rowIdx]?.[colIdx] !== false` + `background: #fff1f0` 完整保留（:99-122, :249-256）。
- 段④ onRun + effectiveValidators 保留（:141-174, :267-290）。

## 三、summarizeRule 抽取

ColumnRuleMapper.jsx:19-26 定义 summarizeRule，签名 `${scope} → ${field} (params)` 与原两 View 一致。grep 确认 MaskView/ValidateView 仅剩 T12-6 说明注释引用，无本地定义、无实际调用。

## 四、哨兵 props 化

- NO_MASK_SENTINEL="__no_mask__" 保留在 MaskView.jsx:26，通过 sentinel prop 传入（:237）。
- NO_VALIDATE_SENTINEL="__no_validate__" 保留在 ValidateView.jsx:24，通过 sentinel prop 传入（:212）。
- ColumnRuleMapper 内不硬编码哨兵。

## 五、dispatch 回调核对

组件通过 onSetOverride/onClearOverride/onGotoRules 回调，不直接 dispatch。调用方 dispatch 用字符串字面量：
- MaskView.jsx:245 SET_MASK_OVERRIDE、:248 CLEAR_MASK_OVERRIDE、:251 SET_VIEW。
- ValidateView.jsx:220 SET_VALIDATE_OVERRIDE、:223 CLEAR_VALIDATE_OVERRIDE、:226 SET_VIEW。

state.js ACTION 常量值与上述字符串逐一对应，reducer case 通过 ACTION.* 匹配，等价。

## 六、独立验证

```
$ wc -l
   MaskView.jsx = 323 (< 350 ✓)
   ValidateView.jsx = 294 (< 350 ✓)
   ColumnRuleMapper.jsx = 253

$ cd frontend && npx vite build
✓ built in 2.06s, dist/index-ClqBlf24.js 1,043.10 kB
（仅既有 chunk 大小 warning，无 error）

$ grep -c "summarizeRule" MaskView.jsx ValidateView.jsx
   各 1 处，均为注释，无实际调用。
```

## 七、行为零回归静态审查

下拉源、选中逻辑、清空逻辑、摘要渲染、兜底合并（effectiveMaskers/effectiveValidators）均与原实现等价。

一处正向修正（非缺陷）：原 HEAD MaskView resolveMaskOptionValue 用 r.masker === existing.masker，v0.4.4 规则引擎重构已删 masker/validator 字段改为 scope，原匹配在 v0.4.4 schema 下恒为 undefined === undefined = true。新 ColumnRuleMapper.jsx:95-103 改用 r.scope === existing.scope，与 v0.4.4 schema 一致，匹配更精确。仅影响下拉「当前选中项」高亮定位，不影响 override 写入/清空/兜底合并。属抽取时顺带修正的 schema 适配，非回归。

## 八、scope_check

无越界。仅改 ColumnRuleMapper.jsx(新) / MaskView.jsx / ValidateView.jsx。未碰 state.js / tauri.js / 其他 View。未改 dispatch action.type 字符串值。未改段①/③/④试运行与结果渲染逻辑。

## 九、docs_check

无需同步。纯 UI 组件内部抽取，不改对外行为/用法/限制。

## 十、最终建议状态

**verified_complete**。无 critical/major/minor 缺陷，无越界，验证充分。

运行时端到端回归（Tauri 环境手测）按 REPORT 说明未执行，建议在版本 QA 阶段补做：
1. MaskView 选规则 → 应用 → 看脱敏结果
2. ValidateView 选规则 → 运行 → 看红底高亮
