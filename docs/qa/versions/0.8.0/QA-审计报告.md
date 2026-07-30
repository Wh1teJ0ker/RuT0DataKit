# v0.8.0 QA 审计报告

> 审计时间：2026-07-30
> 版本：v0.8.0（minor — 收尾发布）
> 审计结论：**qa_passed**

## 1. 功能验收

### T28-A Fix A — `SqlParseInput` 加 `#[serde(rename_all = "camelCase")]`（root cause）

- ✅ `crates/core/src/tools/sql_parse.rs:58-68` `SqlParseInput` 加 `#[serde(rename_all = "camelCase")]`
- ✅ 字段签名不变：`sql: String` + `response_body_size: Option<u64>` + `source_ip: Option<String>`；仅让 serde 反序列化时把前端 camelCase `responseBodySize` / `sourceIp` 正确映射到 snake_case 字段
- ✅ Root cause 验证（独立 serde 验证 `/tmp/serde_test_proj`，已复现）：
  - NoRename + camelCase input → `response_body_size: None, source_ip: None`（**静默丢弃**）
  - WithRename + camelCase input → `response_body_size: Some(862), source_ip: Some("1.1.1.1")` ✓
- ✅ 这解释了 v0.7.2 Rust 侧测试全绿但 GUI 从未跑通的真相：v0.7.2 单测用 struct 字面量 `SqlParseInput { sql, response_body_size: Some(862), source_ip: None }` 直构（snake_case），绕开了 serde，所以测试全绿但 IPC 路径（前端 JSON → Tauri → serde → struct）从未跑通
- ✅ 修复后链路打通：前端 `SqlParseTool.jsx::onParse` / `PreprocessView.jsx::handleBlindAutoExtract` 构造的 `{ sql, responseBodySize, sourceIp }` 经 Tauri IPC → serde → `response_body_size: Some(...)` → `probe.rs::extract_blind_probe_with_line` 走 `Some(_)` 分支生成探针 → schema 非空 → 还原链路打通

### T28-B Fix B — `detect_sql_blind_features` 移除 50 截断

- ✅ `src-tauri/src/commands/log.rs::detect_sql_blind_features` 删除 `|| samples.len() >= 50` 条件
- ✅ 现在扫描全量 row × cell，dedup 后全量传给 `parseSqlTool`
- ✅ 既有 7 个 log 单测全绿（截断移除不改既有行为，因为测试样本量 < 50；50 截断只在长 log 上制造 gap）
- ✅ IPC 体积可控：dedup 按 `(sql, body_size, source_ip)` 三元组，长 log 也只传唯一探针集

### 手机号自定义前缀（v0.6.8 修订已交付，本版本仅索引）

- ✅ v0.8.0 **未修改任何 phone 相关代码**
- ✅ 既有实现引用（commit `666b496`，v0.6.8 修订 `release_complete`）：
  - `crates/core/src/validators/phone.rs:34-49` `PhoneValidator::new(params)` 读 `params["prefixes"]`（YAML Sequence）；空/缺省 → `prefixes=None` → 默认 `starts_with('1')`（line 61）。**无 "52" 拼留**。
  - `frontend/src/components/RulesView.jsx:86-102` `VALIDATE_PARAM_META.phone` / `.pinfo_phone` 含 prefixes TextArea（placeholder `"138,159,734"`，label「前 1-3 位号段（逗号分隔，留空=默认 1 开头正常号码）」）。
  - `docs/versions/0.6.8/更新日志.md` + `docs/qa/versions/0.6.8/QA-审计报告.md` 已落档。
- ✅ 用户原话「默认的52是不正确的」：确认 "52" 只出现在历史注释里标记为废弃，运行时无 "52" 默认

## 2. 回归验收

### Core crate 测试

```
cargo test -p ruT0-data-kit-core --release
   Compiling ruT0-data-kit-core v0.8.0
...
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

含 sqli_* 8 个 + scan + yaml round-trip 等模块全绿。serde 属性不改 Rust 行为（只改 IPC 反序列化），core 单测用 struct 字面量直构不受影响。

### Tauri crate 测试

```
cd src-tauri && cargo test
   Compiling ruT0-data-kit v0.8.0
...
running 12 tests
test commands::extract::tests::package_extract_result_counts_dynamic_types ... ok
test commands::extract::tests::extract_text_without_rules_json_returns_empty ... ok
test commands::log::tests::detect_sql_blind_features_empty_rows ... ok
test commands::extract::tests::export_records_json_filters_columns_and_preserves_rows ... ok
test commands::extract::tests::export_records_json_filters_rows_by_indices ... ok
test commands::extract::tests::extract_text_with_rules_json_extracts_custom_type ... ok
test commands::log::tests::detect_sql_blind_features_decodes_url_encoded_cell ... ok
test commands::log::tests::detect_sql_blind_features_no_size_header_body_size_none ... ok
test commands::log::tests::detect_sql_blind_features_deduplicates ... ok
test commands::log::tests::detect_sql_blind_features_plain_cell_unchanged ... ok
test commands::log::tests::detect_sql_blind_features_size_dash_parses_none ... ok
test commands::log::tests::detect_sql_blind_features_carries_body_size_and_source_ip ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

7 个 log 测试 + 5 个 extract 测试全绿。50-cap 移除不影响测试（样本量 < 50）。

### 前端构建

```
npm --prefix frontend run build
vite v5.4.21 building for production...
✓ 3007 modules transformed.
dist/index.html                    0.31 kB │ gzip:   0.24 kB
dist/assets/index-Btfa90g4.js  1,102.43 kB │ gzip: 343.60 kB
✓ built in 2.44s
```

3007 modules, 0 error。仅 chunk size > 500kB 的既有 warning（非本版本引入）。

## 3. 构建验收

- ✅ `cargo build --release -p ruT0-data-kit-core` → 编译通过（仅 crate 名 snake_case 预存 warning，非本版本引入）
- ✅ `cd src-tauri && cargo build` → 编译通过
- ✅ `npm --prefix frontend run build` → 3007 modules, 0 error

## 4. 安全验收

- ✅ `SqlParseInput` 加 `#[serde(rename_all = "camelCase")]` 仅改 serde 反序列化字段映射，无网络/IO 新增
- ✅ `detect_sql_blind_features` 移除 50 截断，只增不减，纯本地字符串操作 + 正则匹配
- ✅ 手机前缀功能 v0.6.8 已交付，零代码改动
- ✅ 无网络/IO 新增
- ✅ `docs/00-需求文档.md §6`「不外发数据：全本地处理；规则与样本不上传」约束不变

## 5. 文档验收

- ✅ `docs/versions/0.8.0/更新日志.md` 落盘（含用户原话 + 背景 + Fix A/B 根因 + v0.6.8 索引 + 任务表 + DAG + 安全约束 + 向后兼容 + 不做）
- ✅ `docs/qa/versions/0.8.0/QA-审计报告.md` 落盘（本文件，5 维度）
- ✅ `docs/04-版本标准.md` 新增 v0.8.0 行
- ✅ `docs/02-技术设计文档.md` 新增 §2.21 v0.8.0
- ✅ `docs/00-需求文档.md` §界面 7 Tools 段补 v0.8.0 说明
- ✅ `docs/03-开发任务清单.md` 新增 v0.8.0 段
- ✅ `README.md` + `README_EN.md` header + 版本表更新
- ✅ `frontend/src/tauri.js` 注释更新（「上限 50」残留修正）
- ✅ 4 manifest + Cargo.lock 版本同步 0.7.4 → 0.8.0

## 结论

**qa_passed** — 5 维度 Release QA 全部通过。

- 功能验收：Fix A serde rename_all root cause 修复 + Fix B 50-cap 移除 + 手机前缀 v0.6.8 既有交付引用
- 回归验收：core 31 passed + Tauri 12 passed + 前端 0 error
- 构建验收：3 个 build 命令全通过
- 安全验收：纯本地字符串操作，§6 不变
- 文档验收：更新日志 + QA 报告 + 04/02/00/03 + README×2 + tauri.js + 4 manifest + Cargo.lock 全部落盘
