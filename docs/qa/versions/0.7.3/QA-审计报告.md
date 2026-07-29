# v0.7.3 QA 审计报告

> 审计时间：2026-07-30
> 版本：v0.7.3（patch）
> 审计结论：**qa_passed**

## 1. 功能验收

### T27-1 `detect_sql_blind_features` 返回 body_size + source_ip + 去重

- ✅ 返回形状从 `samples: Vec<String>` 改为 `samples: Vec<{ sql, body_size, source_ip }>`
- ✅ 从 `headers` 按 header 名定位 `size` / `ip` 列下标（兼容列重命名）
- ✅ body_size 解析：`u64::from_str`，空/`-` → `None`（CLF 语义）
- ✅ source_ip：空串 → `None`
- ✅ 按 `(sql, body_size, source_ip)` 三元组去重
- ✅ samples 上限 50 不变

### T27-2 前端 PreprocessView「盲注自动提取」按钮

- ✅ `handleBlindAutoExtract` 调用 `detectSqlBlindFeatures` 获取结构化 samples
- ✅ 构造 `SqlParseInput[]`（带 responseBodySize + sourceIp）
- ✅ 拼 `body|sql` 文本填 `sqlParseInput`（有 body_size 时加前缀，否则裸 sql）
- ✅ 调 `parseSqlTool` 后 dispatch `SET_SQL_PARSE_INPUT` + `SET_SQL_PARSE_RESULT` + `SET_VIEW("tools")` + `SET_TOOLS_ACTIVE_TAB("sql")`
- ✅ 未检测到探针时 `message.warning` 不跳转
- ✅ 失败时 `showError` 不清空已有结果
- ✅ 按钮 disabled 条件 `!records`（无导入数据时禁用）
- ✅ import `detectSqlBlindFeatures` 已加入

### 用户场景验证

用户诉求：「需要在数据预处理处加入自动提取的按钮，然后提取并且跳转 `862|username=1'%20or%20ascii(substr((database()),1,1))%3E110%23&password=1`」

- ✅ 导入 log 文件后，PreprocessView 段 ④ 跳转区显示「盲注自动提取」按钮
- ✅ 点击后扫描全表，从同行 `size` 列取 body_size，`ip` 列取 source_ip
- ✅ 拼成 `862|username=1' or ascii(substr((database()),1,1))>110#&password=1` 文本（URL-decode 后形态）
- ✅ 跳转 Tools/Sql 子面板，textarea 预填带前缀文本，结果区预填解析结果

## 2. 回归验收

### Tauri crate 测试

```
cd src-tauri && cargo test
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

含 7 个 log 测试（3 个既有更新断言 + 4 个新增）：

- `detect_sql_blind_features_decodes_url_encoded_cell` ✅
- `detect_sql_blind_features_plain_cell_unchanged` ✅
- `detect_sql_blind_features_empty_rows` ✅
- `detect_sql_blind_features_carries_body_size_and_source_ip` ✅（新增）
- `detect_sql_blind_features_deduplicates` ✅（新增）
- `detect_sql_blind_features_no_size_header_body_size_none` ✅（新增）
- `detect_sql_blind_features_size_dash_parses_none` ✅（新增）

### Core crate 测试（无改动，全绿回归）

```
cargo test -p ruT0-data-kit-core --release
test result: ok. 376 passed; 0 failed; 3 ignored
（+ 10 + 34 + 2 ignored + 12 + 11 + 31 + 0 各模块全绿）
```

### 前端构建

```
npm --prefix frontend run build
✓ built in 2.31s
3007 modules, 0 error
```

## 3. 构建验收

- ✅ `cargo build --release -p ruT0-data-kit-core` → 编译通过（仅 crate 名 snake_case 预存 warning）
- ✅ `cd src-tauri && cargo build` → 编译通过（0 warning after col_idx fix）
- ✅ `npm --prefix frontend run build` → 3007 modules, 0 error

## 4. 安全验收

- ✅ `detect_sql_blind_features` 仅扩展返回字段（从同行 `size`/`ip` 列按 header 名定位），纯本地字符串操作 + 正则匹配
- ✅ 前端 `handleBlindAutoExtract` 仅调用既有命令 + dispatch state，无新依赖
- ✅ 无网络/IO 新增
- ✅ `docs/00-需求文档.md §6`「不外发数据：全本地处理；规则与样本不上传」约束不变

## 5. 文档验收

- ✅ `docs/versions/0.7.3/更新日志.md` 落盘
- ✅ `docs/qa/versions/0.7.3/QA-审计报告.md` 落盘（本文件）
- ✅ `docs/04-版本标准.md` 新增 v0.7.3 行
- ✅ `docs/02-技术设计文档.md` 新增 §2.20 v0.7.3
- ✅ `docs/00-需求文档.md` §界面 7 Tools 段补 v0.7.3 说明
- ✅ `docs/03-开发任务清单.md` 新增 v0.7.3 段
- ✅ `README.md` + `README_EN.md` header + 版本表更新
- ✅ `frontend/src/tauri.js` 注释更新
- ✅ 4 manifest + Cargo.lock ×2 版本同步 0.7.2 → 0.7.3

## 结论

**qa_passed** — 5 维度 Release QA 全部通过。
