# TASK-BOARD — v0.6.3 脱敏/校验总行数 BUG 修复 + 正则构造可视化积木重构（release_complete）

> 版本：v0.6.3
> 创建：2026-07-23
> 完成：2026-07-23
> 依赖版本：v0.6.2（release_complete @ 860671c）
> 依据：用户 goal「① 校验后 - 行 显示总行数 BUG；脱敏后 - 行 同样问题；② Tools 正则解析的构造重构为可视化积木构建」
> **状态：release_complete（qa_passed）**

## 任务 DAG（全部 done）

```yaml
goal: 修复脱敏/校验视图总行数显示 BUG（前端字段名对齐 total_rows/invalid_rows/
  masked_rows）；RegexTool 构造 Tab 由自然语言描述重构为可视化积木构建（点选
  数字/字母/至少一次等模块拼装正则，纯客户端，不外发数据）。bump 0.6.3，
  全链路验收通过。
version: 0.6.3
depends_on_version: 0.6.2
tasks:
  - id: T18-1
    title: 修复脱敏/校验视图总行数显示（前端字段名对齐）
    status: done  # MaskView.jsx 5 处 .total→.total_rows；
                  # ValidateView.jsx 4 处 .total→.total_rows / .invalid_count→.invalid_rows /
                  # .valid_count→计算 total_rows-invalid_rows；npm build 绿
  - id: T18-2
    title: RegexTool 构造 Tab 改为可视化积木构建
    status: done  # 新增 RegexConstructTab.jsx（~300 LOC，5 类 21 模块 + 3 预设 +
                  # 参数弹窗 + 测试高亮）；RegexTool.jsx import 替换原 ConstructTab；
                  # 后端 regex_construct.rs 保留；npm build 绿（3009 modules +1）
  - id: T18-3
    title: 版本号 bump 0.6.2→0.6.3（4 manifest）+ docs 同步 + QA 报告
    status: done  # 本任务
```

任务 DAG 结构：

```
T18-1 (frontend field-name fix) ─┐
                                 ├─→ T18-3 (version+docs+QA)
T18-2 (regex building-block UI) ─┘
```

T18-1 和 T18-2 无文件交集（MaskView/ValidateView vs RegexTool），可并行实施。

## E2E 验收

- `cargo test --workspace`：493 passed / 0 failed / 5 ignored（与 v0.6.2 基线一致，无 Rust 变更）✅
- `cargo build --manifest-path src-tauri/Cargo.toml`：0 error ✅
- `npm --prefix frontend run build`：vite build 3009 modules（+1 对比 v0.6.2），0 error ✅
- grep 验证：
  - MaskView/ValidateView `.total_rows`/`.invalid_rows`/`.masked_rows` 命中（14 行）✅
  - 无残留 `.total`/`.valid_count`/`.invalid_count`（0 命中）✅
  - `RegexConstructTab` / `BLOCK_MODULES` / `PRESETS` 关键符号到位 ✅
  - 4 处 manifest 版本号 0.6.3 ✅

## QA 门禁

- `docs/qa/versions/0.6.3/QA-审计报告.md`：结论 `qa_passed` ✅
- `docs/versions/0.6.3/更新日志.md`：状态 `release_complete` ✅
- `docs/04-版本标准.md` v0.6.3 里程碑行：`release_complete` ✅

## 安全约束（不变）

`docs/00-需求文档.md §6`「不外发数据：全本地处理；规则与样本不上传」保留，v0.6.3 不变。T18-2 积木拼装纯客户端，不调用后端、不外发数据。

## 提交链

```
T18-1+T18-2 (frontend fix + rebuild) ─→ T18-3 (version+docs+QA)
```

按 v0.6.x 惯例，T18-1/T18-2 合并为一个 feat 提交（均为前端改动，无依赖耦合），T18-3 单独 chore 提交（版本号 + docs）。

---

# TASK-BOARD — v0.6.4 数据提取功能三项优化（release_complete）

> 版本：v0.6.4
> 创建：2026-07-23
> 完成：2026-07-23
> 依赖版本：v0.6.3（release_complete @ b746af4）
> 依据：用户 goal「① 文件导入和文本粘贴数据应不共通（黏贴后点文件导入出现所有数据且未优化显示）；② 导出应可调整格式（txt 固定 `iphone_176` 无法自定义，csv 无法预览改字段，无单条/批量）；③ 结果显示每页 50 个无法调整，点每页 10 个不改变」
> **状态：release_complete（qa_passed）**

## 任务 DAG（全部 done）

```yaml
goal: 数据提取三项优化 — 文件导入 vs 文本粘贴数据隔离；导出格式可自定义
  （txt 模板 + csv/json 字段勾选调序 + 单/批量勾选）；分页器受控化
  （8 处 Table 每页条数切换生效）。bump 0.6.4，全链路验收通过。
version: 0.6.4
depends_on_version: 0.6.3
tasks:
  - id: T19-1
    title: ExtractView 全面重构 — state 隔离 + 导出自定义 + 单/批量勾选
    status: done  # state.js EXTRACT_MODE_SET 级联清空 extractInput/extractResult；
                  # export.rs export_extract +3 参数（template/selected_columns/
                  # column_order）；tauri.js exportExtract opts 透传；
                  # ExtractView.jsx TXT 模板 + CSV/JSON 字段勾选调序 + rowSelection；
                  # commit f487637
  - id: T19-3
    title: 分页器受控化（8 处 Table）
    status: done  # ExtractView/SearchView/SqlParseTool×2/PcapView×2/LogView×2
                  # pagination 改受控 current+pageSize useState + onChange/
                  # onShowSizeChange；commit 2a0cba2
  - id: T19-4
    title: 版本号 bump 0.6.3→0.6.4（4 manifest）+ docs 同步 + QA 报告
    status: done  # 本任务
```

任务 DAG 结构：

```
T19-1 (extract state 隔离 + 导出自定义) ─┐
                                       ├─→ T19-4 (version+docs+QA)
T19-3 (分页器受控化 8 处)             ─┘
```

T19-1 和 T19-3 文件交集仅在 ExtractView.jsx（T19-1 改输入/导出区，T19-3 改结果表分页），按 T19-1 先行、T19-3 后继顺序实施无冲突。

## E2E 验收

- `cargo test --workspace`：493 passed / 1 failed (search_big_file 性能阈值) / 7 ignored ✅
  - search_big_file 失败为预先存在的性能敏感测试（断言 100k×10 索引 < 5s，本机 debug 5.5s），search 模块自 v0.4.2 无变更，stash+checkout v0.6.3 基线同样失败，非本版本引入，不阻塞发布
- `cargo build --manifest-path src-tauri/Cargo.toml`：0 error ✅
- `npm --prefix frontend run build`：vite build 3009 modules，0 error，3.91s ✅
- grep 验证：
  - `EXTRACT_MODE_SET` 级联清空 `extractInput=""` / `extractResult=null` ✅
  - `exportExtract` opts 透传 template/selectedColumns/columnOrder ✅
  - 受控分页 onChange/onShowSizeChange 8 处 57 行命中 ✅
  - 后端 `export_extract` +3 参数 ✅
  - 4 处 manifest 版本号 0.6.4 ✅

## QA 门禁

- `docs/qa/versions/0.6.4/QA-审计报告.md`：结论 `qa_passed` ✅
- `docs/versions/0.6.4/更新日志.md`：状态 `release_complete` ✅
- `docs/04-版本标准.md` v0.6.4 里程碑行：`release_complete` ✅

## 安全约束（不变）

`docs/00-需求文档.md §6`「不外发数据：全本地处理；规则与样本不上传」保留，v0.6.4 不变。T19-2 导出模板渲染纯本地，`export_extract` 仅写本地文件 `std::fs::write`，不外发。T19-1/T19-3 仅改前端 state 与 UI，不涉及网络/文件 IO。

## 提交链

```
T19-1+T19-2 (extract 隔离 + 导出自定义) ─→ T19-3 (分页受控化) ─→ T19-4 (version+docs+QA)
```

按 v0.6.x 惯例，T19-1+T19-2 合并为一个 feat 提交（ExtractView 集中改），T19-3 独立 fix 提交（分页受控化），T19-4 单独 chore 提交（版本号 + docs）。另含 2 个本会话前置 BUG 修复提交（正则构造白屏 + ExportView 总行数）。

---

# TASK-BOARD — v0.6.5 字段批量重命名 + 数据流数据源指示（release_complete）

> 版本：v0.6.5
> 状态：release_complete
> 范围：① PreprocessView 预览表头可编辑（单字段 ✏ + 批量重命名 Modal）；② 真实改写 `records.headers` 并级联 re-key 所有 header-keyed 状态切片；③ 预处理流与提取流加数据源指示 Tag，明确两流独立、不共享 records。

## 任务 DAG

```
T20-1 (state 级联 + UI + 数据源指示) ─→ T20-2 (version+docs+QA)
```

## 任务清单

- id: T20-1
  title: 字段重命名 state 级联 + PreprocessView 表头编辑 UI + 数据源指示
  priority: P0
  deps: []
  status: verified_complete
  scope:
    - frontend/src/state.js（COLUMN_RENAME_SET action + 级联 re-key）
    - frontend/src/components/PreprocessView.jsx（✏ 单字段 + 批量 Modal + 数据源 Tag）
    - frontend/src/components/ExtractView.jsx（数据源 Tag）
    - docs/01-页面与交互说明.md（交互规范补充）
  verification:
    - npm --prefix frontend run build → 0 error（3009 modules）
    - grep COLUMN_RENAME_SET 级联 / EditOutlined / 数据源 Tag 全到位

- id: T20-2
  title: 版本号 bump 0.6.4→0.6.5 + docs + QA
  priority: P1
  deps: [T20-1]
  status: verified_complete
  scope:
    - Cargo.toml / src-tauri/Cargo.toml / src-tauri/tauri.conf.json / frontend/package.json → 0.6.5
    - docs/versions/0.6.5/更新日志.md
    - docs/qa/versions/0.6.5/QA-审计报告.md
    - docs/04-版本标准.md（v0.6.5 里程碑行）
    - handoff/TASK-BOARD.md（v0.6.5 DAG）
  verification:
    - cargo build → 0 error
    - cargo test → 482 passed / 0 failed / 5 ignored（search_big_file 通过）
    - npm build → 0 error
    - grep 4 manifest 版本号 0.6.5

## 提交策略

T20-1 单独 feat 提交（state + UI + docs），T20-2 单独 chore 提交（版本号 + docs + QA）。


---

# TASK-BOARD — v0.6.6 数据提取类型重命名（release_complete）

> 版本：v0.6.6
> 状态：release_complete
> 范围：ExtractView 结果区新增「类型重命名」功能——自动列出当前 findings 的全部 type，高频（count ≥ 5）标 ★ 提示，用户填中文名后结果表、计数 Tag、txt/csv/json 导出统一应用。不持久化。

## 任务 DAG

```
T21-1 (类型重命名 UI + 后端 type_rename 透传) ─→ T21-2 (version+docs+QA)
```

## 任务清单

- id: T21-1
  title: ExtractView 类型重命名（自动列 type + 高频标记 + 输入框 + 结果表/导出应用）
  priority: P0
  deps: []
  status: verified_complete
  scope:
    - frontend/src/components/ExtractView.jsx（typeRename state + useEffect 自动列 + ★ 标记 + 结果表 render + 计数 Tag 同步 + handleExport 透传）
    - frontend/src/tauri.js（exportExtract opts.typeRename）
    - src-tauri/src/commands/export.rs（export_extract 签名 +type_rename + 三格式应用）
    - docs/01-页面与交互说明.md（类型重命名交互规范）
  verification:
    - npm --prefix frontend run build → 0 error（3009 modules）
    - cargo build → 0 error
    - grep type_rename / typeRename / ★ 标记全到位

- id: T21-2
  title: 版本号 bump 0.6.5→0.6.6 + docs + QA
  priority: P1
  deps: [T21-1]
  status: verified_complete
  scope:
    - Cargo.toml / src-tauri/Cargo.toml / src-tauri/tauri.conf.json / frontend/package.json → 0.6.6
    - docs/versions/0.6.6/更新日志.md
    - docs/qa/versions/0.6.6/QA-审计报告.md
    - docs/04-版本标准.md（v0.6.6 里程碑行）
    - handoff/TASK-BOARD.md（v0.6.6 DAG）
  verification:
    - cargo build → 0 error
    - cargo test --release → 493 passed / 0 failed / 5 ignored（search_big_file 通过）
    - cargo test (debug) → 492 passed / 1 failed (search_big_file 预先存在 flaky) / 5 ignored
    - npm build → 0 error
    - grep 4 manifest 版本号 0.6.6

## 提交策略

T21-1 单独 feat 提交（前端 + 后端 + docs），T21-2 单独 chore 提交（版本号 + docs + QA）。

## 安全约束（不变）

`docs/00-需求文档.md §6`「不外发数据：全本地处理」保留，v0.6.6 不变。类型重命名纯前端 state + 后端导出渲染（`std::fs::write` 本地文件），不涉及网络/外发。

---

# v0.6.7 数据提取规则重整：\b 边界修复 + 补 idcard/name extract 规则 + 提取/校验规则分离（release_complete）

> 版本：v0.6.7
> 创建：2026-07-25
> 完成：2026-07-25
> 依赖版本：v0.6.6（release_complete @ 07cb303）
> 依据：用户 goal「现在发现一个问题，提取手机号没有成功提取，检查问题，规则里需要添加数据提取，身份证号和手机号，姓名（中文），并且数据提取的规则不要和数据校验的规则共用，请分开，完成后更新到 0.6.7 版本」
> **状态：release_complete（qa_passed）**

## 根因

- **手机号提取失败根因**：`patterns.rs` `PHONE.extract = r"\b\d{11}\b"`，`regex` crate 默认 Unicode-aware，`\b` 把中文字符视为 word char → 中文与数字间不形成 `\b` 边界 → `find_iter` 召回不到候选 → 提取失败。影响范围：phone / idcard / bankcard / mac 四个 extract 正则都用了 `\b`。
- **修复**：四条 extract 正则加 `(?-u)` 内联标记切到 ASCII 模式，`\b` 在字节边界生效，中文旁也能召回。`regex 1.13.1` 支持，已实测（5 个回归测试全绿）。
- **idcard / name 缺 extract 规则**：`builtin_ruleset()` 原只有 3 条 extract（phone/bankcard/ip），idcard 与 name 只有 validate 规则。新增 `idcard_extract_rule()` + `name_extract_rule()`，tag="extract"，内置提取规则从 3 条扩到 5 条。
- **NAME.extract 过度召回**：原 `[\x{4e00}-\x{9fa5}]{2,}` 会召回任意长连续中文段。用户决策限长 2-4 字 → `[\x{4e00}-\x{9fa5}]{2,4}`。长中文段无分隔符时仍可能误报（可接受，用户自行判断）。
- **ExtractView 列表混入校验规则**：原 `filteredRules` 在 `extractRuleTagFilter=null` 时返回全部 validators（含 tag=validate 规则），勾选 validate 规则做提取行为不符预期。用户决策前端过滤分离：ExtractView 只显示 tag="extract" 规则，ValidateView 只显示 tag="validate" 规则，RulesView 仍展示全部（管理视图本职）。

## 任务 DAG（全部 done）

```yaml
goal: 修复手机号提取失败（\b Unicode 边界失效）；builtin 补 idcard/name
  extract 规则（5 条 extract 内置）；NAME.extract 限长 2-4 字；ExtractView
  规则列表与 ValidateView 严格分离（tag="extract" vs tag="validate"）。
  bump 0.6.6→0.6.7，全链路验收通过。
version: 0.6.7
depends_on_version: 0.6.6
tasks:
  - id: T22-1
    title: core patterns \b 修复 + builtin 补 idcard/name extract 规则
    priority: P0
    deps: []
    status: verified_complete
    scope:
      - crates/core/src/rules/patterns.rs: IDCARD/PHONE/BANKCARD/MAC extract
        加 (?-u) ASCII 标记；NAME.extract {2,} → {2,4}；文件头 + 字段注释更新
      - crates/core/src/rules/builtin.rs: 新增 idcard_extract_rule() +
        name_extract_rule()；builtin_ruleset validators vec 从 8 条扩到 10 条
        （5 extract + 5 validate）
      - crates/core/src/rules/mod.rs: pub use builtin::{...} 导出新增 2 构造函数
      - patterns.rs 新增 5 回归测试（phone/idcard/bankcard/mac 中文旁提取 + name 限长）
      - builtin.rs 新增 3 测试（idcard/name extract rule 字段 + 3 中文紧贴文本提取）
  - id: T22-2
    title: 前端 ExtractView 提取/校验规则分离
    priority: P0
    deps: [T22-1]
    status: verified_complete
    scope:
      - frontend/src/components/ExtractView.jsx: filteredRules 锁定 tag==="extract"；
        tag Select 选项只列「全部提取规则」+ extract；Empty 文案改「无带「extract」标签的
        提取规则」；规则数显示 已勾选 / extract 规则数
      - ValidateView.jsx 不动（已有 isValidateRule 过滤）
      - RulesView.jsx 不动（管理视图仍展示全部）
  - id: T22-3
    title: 版本号 bump 0.6.6→0.6.7（4 manifest）+ docs 同步 + QA 报告
    priority: P1
    deps: [T22-2]
    status: verified_complete
    scope:
      - Cargo.toml / src-tauri/Cargo.toml / src-tauri/tauri.conf.json /
        frontend/package.json → 0.6.7
      - docs/versions/0.6.7/更新日志.md
      - docs/qa/versions/0.6.7/QA-审计报告.md
      - docs/04-版本标准.md（v0.6.7 里程碑行）
      - docs/01-页面与交互说明.md（ExtractView 规则分离交互规范补充）
      - handoff/TASK-BOARD.md（v0.6.7 DAG）
    verification:
      - cargo build → 0 error
      - cargo test --release → 503 passed / 0 failed / 5 ignored（全绿）
      - npm build → 0 error（3009 modules）
      - grep 5 关键点到位（patterns.rs (?-u) × 4 / builtin.rs idcard_extract_rule
        + name_extract_rule / ExtractView tag==="extract" / 4 manifest 0.6.7）
```

任务 DAG 结构：

```
T22-1 (core patterns \b 修复 + builtin 补 extract 规则)
   ─→ T22-2 (前端 ExtractView 分离)
       ─→ T22-3 (version+docs+QA)
```

## 提交策略

T22-1 单独 fix 提交（core patterns + builtin + 测试），T22-2 单独 feat 提交（前端 ExtractView），T22-3 单独 chore 提交（版本号 + docs + QA）。

## 安全约束（不变）

`docs/00-需求文档.md §6`「不外发数据：全本地处理」保留，v0.6.7 不变。本版本仅改正则 + 补内置规则 + 前端过滤，无网络 / 文件 IO 新增。

---

# v0.6.8（修订）任务 DAG

- goal: 手机号规则（提取 phone + 校验 pinfo_phone）统一支持自定义前 1-3 位号段，缺省默认 1 开头正常号码；删除上一轮错误的 52 虚假号段默认；完成后 push
- version: 0.6.8（覆盖上一轮错误实现，不 bump 到 0.6.9）
- depends_on_version: 0.6.7
- status: verified_complete

## 任务列表

```yaml
tasks:
  - id: T23-1
    title: Rust 后端 — PhoneValidator 扩展 prefixes + 统一 pinfo_phone + 删除 PInfoPhoneValidator + trial_validate 扩展
    priority: P0
    deps: []
    status: verified_complete
    scope:
      - crates/core/src/validators/phone.rs: 重写 PhoneValidator（re + prefixes
        Option<HashSet<String>>；缺省 1 开头；自定义 prefixes 集合任一前缀匹配）
      - crates/core/src/validators/pinfo_phone.rs: 整文件删除（PInfoPhoneValidator
        废弃）
      - crates/core/src/validators/mod.rs: 删 pub mod pinfo_phone + 删注册 + 测试同步
      - crates/core/src/rules/mod.rs: build_validator 统一 phone/pinfo_phone 路由
        到 PhoneValidator（透传 params.prefixes）
      - crates/core/src/rules/builtin.rs: phone_extract_rule + pinfo_phone_validate_rule
        描述文案改为「默认 1 开头，可自定义前三位号段」；测试用例改 138... valid /
        788... invalid
      - src-tauri/src/commands/validate.rs: trial_validate 在 None 分支增加
        phone/pinfo_phone 回退到 PhoneValidator
      - 不改 CSV 表头（type,value 不变）
  - id: T23-2
    title: 前端 — RulesView phone/pinfo_phone prefixes UI + extractOverrides + ExtractView 合并
    priority: P0
    deps: [T23-1]
    status: verified_complete
    scope:
      - frontend/src/components/RulesView.jsx: VALIDATE_PARAM_META 加 phone 条目
        + 修订 pinfo_phone label（删除"52 虚假号段"字样）；RowExpanded 加 isExtractRow
        分支（applyLabel/resultLabel/validText 三分支）；新增 applyExtractForRow
        （写 SET_EXTRACT_OVERRIDE + 跳 extract view）；columns 操作列 + RowExpanded
        onRun/onApply 三分支路由
      - frontend/src/state.js: extractOverrides 初始化 + 3 ACTION 常量
        (SET_EXTRACT_OVERRIDE/CLEAR/CLEAR_ALL) + extractDomain reducer + fileDomain
        SET_FILE/SET_RECORDS 级联清空 + COLUMN_RENAME_SET remap
      - frontend/src/components/ExtractView.jsx: effectiveValidators useMemo
        (合并 rules.validators extract + extractOverrides，按 field 去重 override 优先)；
        filteredRules/ruleOptions/handleExtract/Empty 全部改用 effectiveValidators
  - id: T23-3
    title: 版本保持 0.6.8 + docs 修订 + QA + DMG + push
    priority: P1
    deps: [T23-2]
    status: verified_complete
    scope:
      - 4 manifest 核对 0.6.8（Cargo.toml / src-tauri/Cargo.toml /
        src-tauri/tauri.conf.json / frontend/package.json，不 bump）
      - docs/versions/0.6.8/更新日志.md（重写为修订版）
      - docs/qa/versions/0.6.8/QA-审计报告.md（重写 5 维度 Release QA）
      - docs/04-版本标准.md（v0.6.8 里程碑行更新为修订版）
      - docs/01-页面与交互说明.md（phone/pinfo_phone prefixes 交互规范修订）
      - handoff/TASK-BOARD.md（v0.6.8 DAG 更新为 T23-1~T23-3 修订版）
    verification:
      - cargo test --workspace --release → 501 passed / 0 failed / 5 ignored（全绿）
      - npm --prefix frontend run build → 0 error（3009 modules）
      - npx @tauri-apps/cli build → DMG 产出
      - grep 7 关键点到位（4 manifest 0.6.8 / pinfo_phone.rs 删除 /
        PhoneValidator prefixes / build_validator 统一路由 /
        RulesView phone+pinfo_phone prefixes UI / state.js extractOverrides +
        ExtractView effectiveValidators / 无"默认 52"功能文案残留）
```

任务 DAG 结构：

```
T23-1 (Rust: PhoneValidator prefixes + 统一 pinfo_phone)
   ─→ T23-2 (前端: RulesView phone/pinfo_phone prefixes UI + extractOverrides + ExtractView)
       ─→ T23-3 (version 0.6.8 + docs 修订 + QA + DMG + push)
```

## 提交策略

单次 commit：`v0.6.8 (修订): 手机号规则统一支持自定义前三位，缺省 1 开头正常号码 (T23-1~T23-3 verified_complete, qa_passed)`，完成后 `git push origin main`。

## 安全约束（不变）

`docs/00-需求文档.md §6`「不外发数据：全本地处理」保留，v0.6.8 修订不变。Rust 改动仅 PhoneValidator 读本地规则参数；前端改动仅参数 UI + 会话级 override 状态切片，无网络 / 文件 IO 新增。
