# RuT0DataKit v1.1.4

> Git tag：`v1.1.4`（待推送）
> 状态：开发中
> 前置：v1.1.3 已发布 tag `v1.1.3`

## 概要

v1.1.4 重做校验模块为**统一校验页面**：用户可动态添加任意多条「目标列 + 校验规则」组合，一个「校验」按钮即把通过 / 失败的行分流到两个新 Tab。身份证规则可勾选跨字段比对性别 / 出生日期。

首轮（T67/T68/T69）完成统一校验页面与后端规则系统扩展；续轮（T70/T71/T72）在此基础上增强 4 项能力：通用校验规则（Generic 变体 + 字符类 / 长度范围参数覆盖）、地址校验放宽为结构化校验（中文 + 地址关键词）、出生日期校验支持分隔符格式（`clean_birth` 归一化）、前端参数 UI 统一化。

## 改动

### 续轮：通用校验 + 地址放宽 + 生日清理 + 前端参数 UI（T70/T71/T72）

**后端（T70）**：

- `ExtractParams` 新增 `Generic` 变体（`allow_digits` / `allow_letters` / `allow_special` + `min_len` / `max_len`），新增 `generic-validate` 内置规则（`with_defaults()` 16 → 17 条）
- 新增 `is_valid_generic` 函数式校验器（字符类白名单 + 长度范围）；`is_valid_birth` 改为先 `clean_birth` 清理分隔符（`-` / `/` / `.` / 空格）再校验 8 位有效日期；`is_valid_address` 放宽为结构化校验（中文 ≥ 2 + 地址关键词）
- `validate_extracted` 抽取为 `validate_extracted_with_params(params, value)`（不依赖 Rule，便于覆盖参数）；`MultiRuleValidation` 新增 `params_override` 字段（前端可下发参数覆盖 DB 默认值，`#[serde(default)]` 向后兼容）
- 跨字段 birth 比对（`validate_rows_to_two_sheets_inner` + `validate_multi_rules_to_two_sheets_inner`）改用 `clean_birth` 归一化，支持 `1949-12-31` 等分隔符格式

**前端（T71/T72）**：见 T71/T72 任务，统一化校验规则参数 UI。

### 首轮：统一校验页面（T67/T68/T69）

**问题**：v1.1.3 把行级多字段校验（T57）做成了独立能力（TopToolbar 单独按钮「行级校验」+ SidePanel 单独面板入口），与单列校验割裂；v1.1.4 初稿曾以 antd `Tabs` 把「单列校验」与「行级校验」并列入 `ValidatePanel`，但仍需用户在两个 Tab 间手动切换、且行级校验固定 7 字段、无法自由组合规则。

**方案**：把 `ValidatePanel` 完全重写为单一表单（无 Tabs/Segmented 切换）：

- 用 antd `Form.List` 实现动态规则行增删，每行 = 目标列 Select + 校验规则 Select + 删除按钮
- 校验规则 options 来自 `listRules()` 异步加载、`filter kind === "validate"`（含 T67 的 7 条 + T70 新增的 `generic-validate` 共 8 条）
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
- 参数：`sheetId / sessionId / rules: Array<{column, ruleId, crossField?, paramsOverride?}> / phonePrefixes?: string[]`
- 返回：`{ validSheet: ParseResult, invalidSheet: ParseResult, invalidReasons: Array<{sourceRow, field, reason}> }`
- `frontend/src/tauri.js` 新增 `validateMultiRulesToTwoSheets` wrapper

## 不变项

- 既有 `validate_column` / `validate_rows_to_two_sheets` IPC 与 wrapper 保留（向后兼容）；`validate_rows_to_two_sheets` 命令签名不变（内部 birth 比对改用 `clean_birth`）
- `extract_validate_to_new_sheet_inner` 不改（`validate_extracted` wrapper 签名保持）
- `App.jsx` 路由不变（validate 走默认 SidePanel 分支）
- DB schema / capabilities/default.json 不变（SCHEMA_VERSION=5，`params` 列复用）
- `TopToolbar.jsx` / `SidePanel.jsx` 沿用 v1.1.4 初稿已移除 rowValidate 入口的状态
- v1.1.3 已发布文档与 GitHub Release 正文不改

## 验收

- `ValidatePanel.jsx` 是单一表单（无 Tabs/Segmented 切换），顶部可动态添加多条规则行
- 每条规则行：目标列 Select + 校验规则 Select + 删除按钮
- 校验规则 options 包含 8 条 validate 规则（含 `generic-validate`）
- 当选中规则是 `idcard-validate` 时展开跨字段配置
- 底部有手机号前缀白名单 `Select mode="tags"`
- 一个「校验」按钮 → 双 Tab 落地 + 汇总消息
- `cargo fmt --all` / `cargo clippy --all-targets --all-features -- -D warnings` / `cargo test --all` 全绿
- `pnpm --prefix frontend build` 通过

## 安全说明

- SQL 全部参数化绑定，禁拼接（后端未改，保持）
- 无凭据字面量
- Mimosa 完整审计未拿到结论前不宣称安全；本版未重新运行完整审计

## 详细文档

- 更新日志：[`docs/versions/1.1.4/更新日志.md`](./更新日志.md)
- v1.1.3 基线：[`docs/versions/1.1.3/RELEASE-NOTES.md`](../1.1.3/RELEASE-NOTES.md)
