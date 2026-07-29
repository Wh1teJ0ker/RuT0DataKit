# v0.7.0 QA 审计报告

> 版本：0.7.0
> 范围：删除「正则解析」Tools 特性 + 新增「数据预处理列级 SQL 解析跳转」
> 审计日期：2026-07-25
> 审计人：主会话（automated）
> 结论：**qa_passed**

## 1. 功能验收

### 1.1 正则解析特性彻底删除（T24-1）

| 维度 | 检查项 | 结果 |
|------|--------|------|
| 前端组件 | `RegexTool.jsx` / `RegexConstructTab.jsx` 文件已删除 | ✅ |
| 前端入口 | Sidebar `tools.regex` 子项 + `handleClick` 分支 + `CodeOutlined` import 已删 | ✅ |
| 前端视图 | `ToolsView.jsx` title/render 分支简化为二元（encrypt/sql），无 regex 兜底渲染路径 | ✅ |
| 前端状态 | `state.js` 5 个 ACTION 常量 + 5 个 initialState 字段 + 5 个 reducer case 全部删除 | ✅ |
| 前端绑定 | `tauri.js` `explainRegex` / `regexConstruct` 函数 + 相关注释段删除 | ✅ |
| Tauri 命令 | `commands/tools.rs` `explain_regex` / `regex_construct` 命令 + imports 删除，仅余 `parse_sql_tool` | ✅ |
| Tauri 注册 | `main.rs` `tauri::generate_handler!` 删除两行 `commands::explain_regex` / `commands::regex_construct` | ✅ |
| Core 模块 | `tools/mod.rs` 删除 3 个 `pub mod` 声明 + 2 个 `pub use`；模块文档更新 | ✅ |
| Core 文件 | `regex_explain.rs` / `regex_construct.rs` / `regex_template.rs` 三个文件已删除 | ✅ |
| 边界保留 | `maskers/regex_*` / `search/` / `rules/patterns.rs` / `logsign/blind/` / `searchRegexInput` 全部保留 | ✅ |
| grep 残留 | `grep -rn "RegexTool\|explainRegex\|regexConstruct\|explain_regex\|regex_construct"` 在源码中仅命中 v0.7.0 注释说明 | ✅ |

### 1.2 PreprocessView 列级 SQL 解析跳转（T24-2）

| 维度 | 检查项 | 结果 |
|------|--------|------|
| imports | `ConsoleSqlOutlined` + `parseSqlTool` 新增 | ✅ |
| 表头按钮 | `previewColumns` Space 内新增 `<Button icon={<ConsoleSqlOutlined />} />`，在 ✏ 之前 | ✅ |
| handler | `handleColumnSqlParse(columnName)` 收集列全部非空值 → `parseSqlTool` → dispatch 4 个 action | ✅ |
| 跳转模式 | `SET_VIEW tools` + `SET_TOOLS_ACTIVE_TAB sql`，镜像 Sidebar.jsx:60-62 | ✅ |
| 预填 | `SET_SQL_PARSE_INPUT`（textarea 多行）+ `SET_SQL_PARSE_RESULT`（还原表数据） | ✅ |
| 空列边界 | `message.warning` 不跳转，不清空已有结果 | ✅ |
| 失败边界 | `message.error` 不清空已有结果 | ✅ |
| records 保留 | `SET_VIEW` 不清 records（与 Pattern B 一致） | ✅ |
| 无新 state | 复用既有 `sqlParseInput` / `sqlParseResult`，无新 action type | ✅ |
| SqlParseTool 不改 | 已读取 state 字段，预填后直接渲染 | ✅ |

### 1.3 版本 bump（T24-3）

| 文件 | 0.6.8 → 0.7.0 |
|------|---------------|
| `Cargo.toml:8` | ✅ |
| `src-tauri/Cargo.toml:3` | ✅ |
| `src-tauri/tauri.conf.json:4` | ✅ |
| `frontend/package.json:4` | ✅ |
| `Cargo.lock` | ✅（cargo build 自动同步） |

### 1.4 docs 全面清理（T24-3）

| 文档 | 检查项 | 结果 |
|------|--------|------|
| `docs/04-版本标准.md` | 新增 v0.7.0 里程碑行 | ✅ |
| `docs/versions/0.7.0/更新日志.md` | 新建，含 4 任务明细 + 安全约束 + 向后兼容 | ✅ |
| `docs/qa/versions/0.7.0/QA-审计报告.md` | 新建（本文件） | ✅ |
| `docs/01-页面与交互说明.md` | 删正则解析条目 + 新增列级 SQL 跳转说明 | ✅ |
| `docs/00-需求文档.md` | 删正则解析条目，§6 不变 | ✅ |
| `docs/02-技术设计文档.md` | 删 §2.10 tools 正则后端 + §2.12.5 regex_construct | ✅ |
| `docs/03-开发任务清单.md` | 删 T5-11 / T5-12 / T6-3 / T6-5 任务行 | ✅ |
| `README.md` | 删正则解析特性说明 + 新增 v0.7.0 列级 SQL 跳转 | ✅ |
| `README_EN.md` | 同步英文条目 | ✅ |

## 2. 回归验收

### 2.1 单元测试

```
$ cargo test -p ruT0-data-kit-core --release
test result: ok. 366 passed; 0 failed; 3 ignored
test result: ok. 10 passed; 0 failed; 0 ignored
test result: ok. 34 passed; 0 failed; 2 ignored
test result: ok. 12 passed; 0 failed; 0 ignored
test result: ok. 11 passed; 0 failed; 0 ignored
test result: ok. 31 passed; 0 failed; 0 ignored
test result: ok. 0 passed; 0 failed; 0 ignored
合计：464 passed, 0 failed, 5 ignored（全绿）
```

regex_explain / regex_construct 的单测随文件删除一并消失（约 40 个），其它测试不受影响。

### 2.2 Tauri 测试

```
$ cd src-tauri && cargo test --release
test result: ok. 5 passed; 0 failed; 0 ignored
```

### 2.3 前端构建

```
$ npm --prefix frontend run build
✓ 3007 modules transformed
dist/index.html                    0.31 kB │ gzip:   0.24 kB
dist/assets/index-Cvv9UMgn.js  1,101.37 kB │ gzip: 343.17 kB
✓ built in 2.11s
```

模块数从 v0.6.8 的 3009 降到 3007（删除 RegexTool.jsx + RegexConstructTab.jsx 两个文件）。

### 2.4 Rust 构建

```
$ cargo build --release
Finished `release` profile [optimized] target(s) in 3.49s

$ cd src-tauri && cargo build --release
Finished `release` profile [optimized] target(s) in 18.92s
```

仅 crate 名 snake_case 预存 warning（与 v0.6.8 一致，非本版本引入）。

### 2.5 其它视图不受影响

| 视图 | 验证 |
|------|------|
| 数据预处理 | 仅新增 SQL 按钮 + handler，原有 ✏ 重命名 / 批量重命名 / 跳转按钮组不动 | ✅ |
| 数据提取 | 不动 | ✅ |
| 规则管理 | 不动 | ✅ |
| 搜索 | `searchRegexInput` / `SEARCH_REGEX_*` 保留，搜索的正则模式不受影响 | ✅ |
| 数据脱敏 | `maskers/regex_*` 保留 | ✅ |
| 数据校验 | 不动 | ✅ |
| 数据导出 | 不动 | ✅ |
| Tools/Sql | `SqlParseTool.jsx` 不改，预填后即可渲染 | ✅ |
| Tools/Encrypt | `EncryptTool.jsx` 不动 | ✅ |
| 设置 | 不动 | ✅ |

## 3. 构建验收

- `cargo build --release`（workspace）通过
- `cargo build --release`（src-tauri）通过
- `npm --prefix frontend run build` 0 error
- DMG 产出：本轮跳过（用户未明确要求 DMG；E2E 已覆盖代码路径）

## 4. 安全验收

- **§6 安全约束不变**：全本地处理，不外发数据，规则与样本不上传
- 本版本无新增网络调用
- 本版本无新增文件 IO
- 删除特性：仅移除代码，不引入新风险
- 新增跳转：复用既有 v0.5.0 `parse_sql_tool` Tauri 命令（本地纯 SQL 文本解析，无 IO），仅前端新增按钮 + handler + dispatch
- 结论：**安全无回归**

## 5. 文档验收

- v0.7.0 更新日志就位：`docs/versions/0.7.0/更新日志.md`
- v0.7.0 QA 报告就位：`docs/qa/versions/0.7.0/QA-审计报告.md`（本文件）
- `docs/04-版本标准.md` 里程碑索引含 v0.7.0 行
- 当前文档（`docs/00~04`、`README.md`、`README_EN.md`）无正则解析残留
- grep 残留检查：历史版本日志（`docs/versions/0.4.0~0.6.x/`）保留正则解析历史描述可接受

## 6. 向后兼容

- 正则解析特性完全移除（用户明确要求，属破坏性变更，已在更新日志标注）
- 搜索的正则模式、masker 内部 regex、logsign blind regex、rules patterns.rs 全部保留
- PreprocessView 列级 SQL 跳转复用既有命令 + 既有 action，无新 state 字段
- v0.6.8 手机号 prefixes / v0.6.7 边界修复 / v0.6.6 类型重命名 / v0.6.5 字段批量重命名全部不破

## 7. 最终结论

| 维度 | 结论 |
|------|------|
| 功能 | ✅ 正则解析彻底删除；列级 SQL 跳转可用 |
| 回归 | ✅ 464 core + 5 tauri 测试全绿；前端 0 error；其它视图不受影响 |
| 构建 | ✅ Rust + 前端构建通过 |
| 安全 | ✅ §6 不变，无网络/IO 新增 |
| 文档 | ✅ v0.7.0 日志 + QA 报告就位，当前文档无残留 |

**Release QA 结论：qa_passed**

**版本状态：release_complete**

可执行 `git commit` + `git push origin main`。
