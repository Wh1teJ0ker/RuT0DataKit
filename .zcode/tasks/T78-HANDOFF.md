# T78 交接 — 手机号前缀配置 UX 统一

## 任务

- **ID**: T78
- **标题**: 手机号前缀配置 UX 统一 — 校验每行内嵌 + 提取新增前缀输入
- **分支**: `feat/v1.1.4-r3-hash-dbparse`
- **提交**: `bf3511e`
- **状态**: implementation_complete，待 reviewer 验收

## 需求

用户反馈两个问题：

1. **手机号校验**：前缀白名单当前是 ValidatePanel 底部全局 Form.Item（所有规则共享），用户期望「在下拉选择手机号规则后填写」—— 即每行规则内嵌前缀输入，选中 phone-validate 时才展开。
2. **手机号提取**：ExtractPanel 完全没有前缀白名单输入 UI，phone-extract 只能依赖 DB 持久化的 `params.allowed_prefixes`，用户在提取时无法临时覆盖前缀。

## 改动清单

### 后端（src-tauri/src/commands/processor.rs）

1. `extract_validate_to_new_sheet_inner` 签名新增 `phone_prefixes: &[String]`（最后一个参数）
2. 在 phone-extract 候选校验循环（约 line 502-512）：
   - 新增 `is_phone_extract = rule.id == "phone-extract"`
   - 若 `is_phone_extract && !phone_prefixes.is_empty()`：构造 `ExtractParams::PhonePrefix { allowed_prefixes: phone_prefixes.to_vec() }` 调 `validate_extracted_with_params(&params_override, candidate)` 覆盖 DB rule.params
   - 否则保持原逻辑 `validate_extracted(rule, candidate)`
3. `extract_validate_to_new_sheet` Tauri 命令新增 `phone_prefixes: Vec<String>` 参数，透传 `&phone_prefixes` 到 inner
4. 10 个既有测试调用点补 `&[]` 参数（lines 1906/1953/1983/2016/2046/2071/2089/2108/2130/2174/2220 — 其中 1953 是新测试）
5. 新增测试 `extract_validate_phone_with_prefix_filter`：前缀 `["134"]` → 134 开头 valid / 159 开头 invalid

### 前端

**frontend/src/tauri.js**:
- `extractValidateToNewSheet` 新增 `phonePrefixes = []` 参数，传入 invoke
- JSDoc 补 `@param phonePrefixes` 说明

**frontend/src/components/panels/ValidatePanel.jsx**:
- 删除底部全局 `Form.Item name="phonePrefixes"`（原 lines 370-381）
- Form.List 每行新增 `shouldUpdate` 监听 ruleId，仅 `ruleId === "phone-validate"` 时展开 `Form.Item name={[name, "phonePrefixes"]}` 的 Select mode="tags"
- `handleValidate` 改为从所有 phone-validate 行收集 `r.phonePrefixes`，去重合并后传后端（替换原 `values.phonePrefixes`）
- 底部说明文案追加「手机号规则可在行内设置前缀白名单。」

**frontend/src/components/panels/ExtractPanel.jsx**:
- 新增 `isPhoneExtract` 判定：`selectedRuleIds.some(id => id === "phone-extract")`
- `isPhoneExtract` 时显示 `Form.Item name="phonePrefixes"` 的 Select mode="tags"（类似 isIdcardExtract 显示 genderCol）
- `handleExtractValidate` 收集 phonePrefixes（过滤三位纯数字）传 `extractValidateToNewSheet`

### 文档

- `docs/versions/1.1.4/更新日志.md`：追加「R4 续轮：手机号前缀配置 UX 统一（T77-T78）」章节 + 验收项 E111-E116
- `docs/02-技术设计文档.md`：§4.6 追加 T78 `extract_validate_to_new_sheet` phone_prefixes 参数说明

## E2E 验证

- `cargo fmt --all` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test --all` ✅（src-tauri 138 + core 165 + doc-tests 13 = 316 passed / 3 ignored / 0 failed）
- `pnpm --prefix frontend build` ✅（3083 modules transformed）

## 验收点

- **E113**: `extract_validate_to_new_sheet_inner` 新增 `phone_prefixes: &[String]` 参数，phone-extract 候选校验时非空覆盖 DB rule.params
- **E114**: `extract_validate_phone_with_prefix_filter` 测试通过（前缀 `["134"]` → 134 开头 valid / 159 开头 invalid）
- **E115**: ValidatePanel 选中 phone-validate 行时展开前缀白名单 Select（每行内嵌，非全局）；多行合并去重
- **E116**: ExtractPanel 选中 phone-extract 时显示前缀白名单 Select，运行时覆盖 DB 默认

## 安全约束（延续）

- SQL 全部参数绑定，禁拼接（后端未改 SQL）
- 无凭据字面量
- 不创建 tag（用户等待手工验证，不自动 finalize）
- 不自动 merge main（用户等待手工验证）
- Mimosa 未重新运行完整审计，不宣称项目安全
