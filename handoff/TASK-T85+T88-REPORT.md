---
task_id: T85+T88
title: 前端改动（先校验再脱敏 UI + 生日多格式勾选 UI）
---

```yaml
implemented_changes:
  - frontend/src/tauri.js
      - T85：maskColumn wrapper 追加 4 个可选参数 validateRuleId / invalidText /
        paramsOverride / phonePrefixes。全部 `?? null`，向后兼容；invoke 透传给
        后端 mask_column IPC。更新了 JSDoc 注释。
  - frontend/src/components/panels/MaskPanel.jsx
      - T85：导入追加 Checkbox / Empty；state 新增 validateRules / validateEnabled /
        validateRuleId / invalidText(默认 "INVALID") / phonePrefixesInput。
      - listRules() 加载时同时过滤 validate-kind 规则；首条作为默认 validateRuleId。
      - UI：在脱敏规则配置下方、执行按钮上方新增「先校验再脱敏」Checkbox + 展开区
        （校验规则 Select + 无效输出文本 Input + phone-validate 时额外前缀白名单
        Select tags 模式）。
      - handleRun：按 validateEnabled 组装四个参数透传 maskColumn；
        phone-validate 时 phonePrefixes 按三位数字过滤；paramsOverride 当前透传 null。
      - handleReset：同步重置校验开关与参数。
  - frontend/src/components/panels/ValidatePanel.jsx
      - T88：新增 BIRTH_FORMATS 常量（4 个选项：yyyymmdd / yyyy-mm-dd / yyyy-mm-dd /
        yyyy.mm.dd）。
      - UI：通过 Form.Item shouldUpdate 条件渲染——仅当某行 ruleId ===
        "birth-validate" 时展开 Checkbox.Group（字段名 [name, "birthFormats"]），
        位于 phone-validate 展开块之后。
      - handleValidate：组装 multiRules 时，若 r.ruleId === "birth-validate" 且
        r.birthFormats?.length 非空，构造 item.paramsOverride = { validator: "birth",
        formats: r.birthFormats }；全不选 = 不传 paramsOverride（向后兼容）。
      - 底部 Text 提示追加「生日规则可在行内勾选要校验的格式」。
  - docs/02-技术设计文档.md
      - 追加 v1.1.5 前端增量段（T85/T88），描述 maskColumn wrapper 扩展、MaskPanel
        先校验再脱敏 UI、ValidatePanel birth-validate 格式勾选与 paramsOverride 组装。
verification_run:
  - pnpm --prefix frontend build
verification_results:
  - pnpm --prefix frontend build：通过。vite v5.0.0，3083 modules transformed，
    built in 2.27s。仅 chunk 大小告警（既有，非本任务引入）。
    dist/assets/index-CZB41KCS.js 1,200.51 kB │ gzip: 380.96 kB。
docs_updated:
  - docs/02-技术设计文档.md（追加 v1.1.5 前端增量段）
commit_summary:
  - none（任务约束：不创建 git 分支，不 commit）
reported_status:
  - verified_complete
scope_deviation:
  - none
```
