```yaml
task_id: T6-4
goal: |
  在 PreprocessView 导入完成后，扫描 records.rows 中所有看起来像 SQL 盲注探针
  的文本（ascii_binary / equality / length 三类正则）；若命中 ≥1 条，自动
  dispatch SET_VIEW("tools") + SET_TOOLS_ACTIVE_TAB("sql") + SET_SQL_PARSE_INPUT
  （把命中行拼成多行文本预填到 SqlParseTool 输入框）。

in_scope:
  - crates/core/src/logsign/blind_aggregator.rs    # 新增 pub fn looks_like_blind_probe(sql: &str) -> bool（复用既有正则，不依赖 response_body_size）
  - crates/core/src/tools/sql_parse.rs              # re-export looks_like_blind_probe（或在新模块）
  - src-tauri/src/commands.rs                       # 新增 #[tauri::command] detect_sql_blind_features(headers, rows) -> Value {detected, samples}
  - src-tauri/src/main.rs                           # 注册新命令到 invoke_handler
  - frontend/src/tauri.js                           # 新增 detectSqlBlindFeatures(headers, rows) 封装
  - frontend/src/components/PreprocessView.jsx       # handleImport 成功后调 detectSqlBlindFeatures；detected=true 时自动跳转

out_of_scope:
  - 不改 SqlParseTool 内部布局
  - 不改 extract_blind_probe 签名（保留 response_body_size 必填语义）
  - 不改 search/mask/validate/export 数据流（T6-1 负责）
  - 不外发数据：detect 仅本地正则匹配，不调用网络

acceptance_criteria:
  - 新增 core 函数 looks_like_blind_probe("ascii(substr((database()),1,1))>100") == true
  - looks_like_blind_probe("SELECT * FROM users") == false
  - looks_like_blind_probe 对 equality("substr((database()),1,1)='a'") / length("length(database())>5") 均返回 true
  - 新增单测覆盖上述 3 类 + 1 个负例
  - 新增 Tauri 命令 detect_sql_blind_features 返回 {detected: bool, samples: Vec<String>}
  - PreprocessView 导入含盲注探针的 sql fixture 后，自动跳到 Tools/SqlParseTool 且 sqlParseInput 预填命中行
  - PreprocessView 导入普通 csv（无盲注特征）不跳转，留在 preprocess view
  - cargo test --workspace 全绿；npm build 通过

verification_commands:
  - cargo test --workspace
  - cargo test -p ruT0-data-kit-core looks_like_blind_probe
  - cd frontend && npm run build

files_likely_to_change:
  - crates/core/src/logsign/blind_aggregator.rs
  - crates/core/src/tools/sql_parse.rs
  - src-tauri/src/commands.rs
  - src-tauri/src/main.rs
  - frontend/src/tauri.js
  - frontend/src/components/PreprocessView.jsx

risks:
  - records 来自 SqlReader 时 headers=[sql_text, statement_type]，盲注探针行 statement_type 多为 "select"；扫描时应遍历所有 cell（不只 sql_text 列），避免漏掉 json/log 源里偶现的 SQL 文本。
  - 跳转是「自动」行为，若用户导入的就是想留在预处理看的 sql，可能打扰；mitigate：仅在 detected=true 时跳转，detected=false 不动；且跳转后用户可手动切回。
  - looks_like_blind_probe 必须与 extract_blind_probe 用同一份正则（ascii_binary_regex / equality_regex / length_regex），避免检测/抽取不一致。

depends_on: [T6-1]
status: planned
```

## 上下文

**用户原话**：「如果在数据预处理的时候解析到了SQL盲注的特征，自动跳转到对应的工具页面」。

**既有能力**：
- `crates/core/src/logsign/blind_aggregator.rs` 已有 `extract_blind_probe(sql, response_body_size, source_ip)`，但要求 `response_body_size: Some(...)`，None 时直接返回空 Vec——不适合做「纯文本特征检测」。
- 三类正则 `ascii_binary_regex / equality_regex / length_regex` 已在 blind_aggregator.rs 内私有实现。
- `tools::sql_parse::parse_sqls` 是完整解析入口，太重（含聚合 + 数据库还原），不适合做轻量检测。

**本任务方案**：在 blind_aggregator.rs 新增 `pub fn looks_like_blind_probe(sql: &str) -> bool`，跑三类正则任一命中即 true，不依赖 response_body_size；Tauri 暴露 `detect_sql_blind_features(headers, rows)` 命令遍历 records 找命中行；PreprocessView handleImport 成功后调它，detected=true 时自动 SET_VIEW tools + SET_TOOLS_ACTIVE_TAB sql + SET_SQL_PARSE_INPUT 预填。

**安全约束**：不外发数据；全本地处理；规则与样本不上传。detect 命令仅本地正则匹配。
