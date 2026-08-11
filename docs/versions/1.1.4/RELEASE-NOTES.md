# RuT0DataKit v1.1.4

> Git tag：`v1.1.4`（待推送）
> 状态：开发中
> 前置：v1.1.3 已发布 tag `v1.1.3`

## 概要

v1.1.4 重做校验模块为**统一校验页面**：用户可动态添加任意多条「目标列 + 校验规则」组合，一个「校验」按钮即把通过 / 失败的行分流到两个新 Tab。身份证规则可勾选跨字段比对性别 / 出生日期。本版由 T67（后端规则系统扩展）+ T68（新命令 `validate_multi_rules_to_two_sheets`）+ T69（前端统一校验页面）三个任务组成。

## 改动

### 统一校验页面（T69 前端）

**问题**：v1.1.3 把行级多字段校验（T57）做成了独立能力（TopToolbar 单独按钮「行级校验」+ SidePanel 单独面板入口），与单列校验割裂；v1.1.4 初稿曾以 antd `Tabs` 把「单列校验」与「行级校验」并列入 `ValidatePanel`，但仍需用户在两个 Tab 间手动切换、且行级校验固定 7 字段、无法自由组合规则。

**方案**：把 `ValidatePanel` 完全重写为单一表单（无 Tabs/Segmented 切换）：

- 用 antd `Form.List` 实现动态规则行增删，每行 = 目标列 Select + 校验规则 Select + 删除按钮
- 校验规则 options 来自 `listRules()` 异步加载、`filter kind === "validate"`（T67 的 7 条规则）
- 当选中规则是 `idcard-validate` 时，用 `Form.Item shouldUpdate` 条件渲染跨字段配置：勾选「对比性别一致性」+ Select 性别列；勾选「对比出生日期一致性」+ Select 出生日期列
- 底部手机号前缀白名单 `Select mode="tags"`（可选，全局应用于 `phone-validate` 规则）
- 一个「校验」按钮 → 调 `validateMultiRulesToTwoSheets` → 通过 / 失败行分别写入两个新 Tab（`{源sheet名}_校验通过` / `{源sheet名}_校验失败`）
- 汇总消息：通过 / 失败行数 + top 失败原因

### 后端规则系统扩展（T67）

- `ExtractParams` 从单一形态扩展为 4 个变体（mask / extract / validate / 自定义）
- 新增 6 条 `kind=validate` 规则（函数式 + 正则），加上既有的 `name-validate`，共 7 条 validate 规则：username-validate / sex-validate / birth-validate / idcard-validate / phone-validate / address-validate / name-validate
- 规则分发逻辑泛化：按 `ruleId` 查表 → 路由到对应校验函数
- DB seed 写入上述 7 条 validate 规则；`idcard-validate` 规则支持 crossField

### 新命令（T68）

- 新增 IPC 命令 `validate_multi_rules_to_two_sheets`，承接多规则列式校验
- 参数：`sheetId / sessionId / rules: Array<{column, ruleId, crossField?}> / phonePrefixes?: string[]`
- 返回：`{ validSheet: ParseResult, invalidSheet: ParseResult, invalidReasons: Array<{sourceRow, field, reason}> }`
- `frontend/src/tauri.js` 新增 `validateMultiRulesToTwoSheets` wrapper

## 不变项

- 既有 `validate_column` / `validate_rows_to_two_sheets` IPC 与 wrapper 保留（向后兼容）
- `App.jsx` 路由不变（validate 走默认 SidePanel 分支）
- DB schema / capabilities/default.json 不变
- `TopToolbar.jsx` / `SidePanel.jsx` 沿用 v1.1.4 初稿已移除 rowValidate 入口的状态
- v1.1.3 已发布文档与 GitHub Release 正文不改

## 验收

- `ValidatePanel.jsx` 是单一表单（无 Tabs/Segmented 切换），顶部可动态添加多条规则行
- 每条规则行：目标列 Select + 校验规则 Select + 删除按钮
- 校验规则 options 包含 T67 的 7 条 validate 规则
- 当选中规则是 `idcard-validate` 时展开跨字段配置
- 底部有手机号前缀白名单 `Select mode="tags"`
- 一个「校验」按钮 → 双 Tab 落地 + 汇总消息
- `pnpm --prefix frontend build` 通过

## 安全说明

- SQL 全部参数化绑定，禁拼接（后端未改，保持）
- 无凭据字面量
- Mimosa 完整审计未拿到结论前不宣称安全；本版未重新运行完整审计

## 详细文档

- 更新日志：[`docs/versions/1.1.4/更新日志.md`](./更新日志.md)
- v1.1.3 基线：[`docs/versions/1.1.3/RELEASE-NOTES.md`](../1.1.3/RELEASE-NOTES.md)
