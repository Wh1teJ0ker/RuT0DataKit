# v0.6.6 Release QA 审计报告

> 数据提取类型重命名版本发布前全局 QA 审计。依据 `orchestrator-workflow` 技能「发布前 QA 门禁」要求，覆盖需求覆盖、端到端流程、构建与测试、代码质量、文档一致性五维度。
>
> 本版本范围：ExtractView 结果区新增「类型重命名」功能——自动列出当前 findings 的全部 type，高频（count ≥ 5）标 ★ 提示，用户填中文名后结果表、计数 Tag、txt/csv/json 导出统一应用。

## §0 审计结论

`qa_passed` — 2 项任务（T21-1/T21-2）全部落地。类型重命名用户诉求满足（自动列出 + 高频 ★ 标记 + 结果表/计数 Tag/导出三处应用），cargo build 0 error，cargo test --release 全绿，npm build 绿（3009 modules），4 处 manifest 版本号同步 0.6.6，docs 一致，grep 验证四处关键点到位，无阻塞项。

## §1 任务对齐

| # | 任务 | 优先级 | 落地证据 | 状态 |
|---|------|--------|----------|------|
| 1 | T21-1 ExtractView 类型重命名（自动列 type + 高频标记 + 输入框 + 结果表/导出应用） | P0 | ExtractView.jsx `typeRename` state + useEffect 自动列 + ★ 标记 + 结果表 render + 计数 Tag 同步 + handleExport 透传；tauri.js opts.typeRename；export.rs `type_rename` 签名 + 三格式应用 | ✅ |
| 2 | T21-2 版本号 + docs | P1 | 4 manifest 0.6.6 + 更新日志 + QA 报告 + 04-版本标准里程碑行 | ✅ |

## §2 代码审计

### §2.1 T21-1 前端类型重命名 UI（ExtractView.jsx）

| 检查项 | 结果 |
|--------|------|
| `typeRename` useState（key=原 type，value=新名） | ✅（ExtractView.jsx:84） |
| `useEffect` 从 `extractResult.findings` 去重取全部 type | ✅（ExtractView.jsx:242-264） |
| 高频（count ≥ 5）排前 + ★ 标记（非强制） | ✅（sort + `hot ? "★ " : ""`） |
| 无结果时 `setTypeRename({})` 清空 | ✅（ExtractView.jsx:243-245） |
| 结果 Card 顶部加「类型重命名」区 | ✅ |
| 每个 type 一行：Tag（带 ★ + count）→ Input（placeholder 中文建议） | ✅ |
| placeholder 覆盖 8 已知 type（phone/idcard/name/bankcard/email/ip/mac/username）+ 兜底 | ✅ |
| 结果表 type 列 `render`：填了显示「中文名（原 type）」，未填显示原 type | ✅（displayType useCallback） |
| 顶部计数 Tag 同步显示「中文名（原 type）」 | ✅（countEntries.map 重命名 label） |
| `handleExport` 把非空 typeRename 透传给 `exportExtract` | ✅（Object.entries filter 非空） |
| 不持久化（用户决策） | ✅（无 localStorage 写入，每次新结果 useEffect 重建） |

### §2.2 T21-1 前端 tauri.js 透传

| 检查项 | 结果 |
|--------|------|
| `exportExtract` opts 增加 `typeRename` 字段 | ✅（tauri.js:308） |
| 缺省 `null`（`opts.typeRename ?? null`） | ✅ |
| 调用 `tauriInvoke("export_extract", { ..., typeRename })` | ✅ |

### §2.3 T21-1 后端 export.rs

| 检查项 | 结果 |
|--------|------|
| `export_extract` 签名 +`type_rename: Option<HashMap<String, String>>` | ✅（export.rs:193） |
| `rename = type_rename.unwrap_or_default()` 缺省空映射 | ✅ |
| `rename_type` 闭包：`rename.get(t).cloned().filter(!s.is_empty()).unwrap_or(t)` | ✅（命中即替换，空值/缺省回退原 type） |
| txt 分支：`{type}` 占位符替换为 `rename_type(&it.r#type)` | ✅ |
| csv/json 分支：type 列 `rename_type(&it.r#type)` | ✅ |
| 表头仍为 "type"（不改 header，仅改单元格值） | ✅ |
| 缺省/空映射回退原 type（向后兼容） | ✅ |

### §2.4 安全合规

- 类型重命名纯前端 state + 后端导出渲染（`std::fs::write` 本地文件），不涉及网络/外发。
- 与 `docs/00-需求文档.md §6`「不外发数据：全本地处理；规则与样本不上传」约束一致。
- 无新增网络调用 / 加密 / 外发逻辑。

### §2.5 scope 合规

- T21-1 改 `frontend/src/components/ExtractView.jsx` + `frontend/src/tauri.js` + `src-tauri/src/commands/export.rs` + `docs/01-页面与交互说明.md`。
- T21-2 改 4 manifest + docs。
- 未越界触及 core / mask pipeline / validate pipeline / 加密模块 / RulesView / RegexTool / SearchView / ExportView / PreprocessView（v0.6.5 字段重命名不破）。

## §3 测试审计

```
$ cargo test --release
test result: ok. 395 passed; 0 failed; 3 ignored   (lib unittests)
test result: ok. 10 passed; 0 failed; 0 ignored     (integration log_test)
test result: ok. 34 passed; 0 failed; 2 ignored     (integration log_scan_test, search_big_file 通过)
test result: ok. 12 passed; 0 failed; 0 ignored     (integration blind_aggregator_test)
test result: ok. 11 passed; 0 failed; 0 ignored     (integration logsign_test)
test result: ok. 31 passed; 0 failed; 0 ignored     (integration e2e)
test result: ok. 0 passed; 0 failed; 0 ignored      (Doc-tests)
```

release build 合计 **493 passed, 0 failed, 5 ignored**（全绿，含 `search_big_file`）。

```
$ cargo test  (debug)
test result: ok. 395 passed; 0 failed; 3 ignored
test result: ok. 10 passed; 0 failed; 0 ignored
test search_big_file ... FAILED  (6.28s, 阈值 5s)
test result: FAILED. 33 passed; 1 failed; 2 ignored
test result: ok. 12 passed; 0 failed; 0 ignored
test result: ok. 11 passed; 0 failed; 0 ignored
test result: ok. 31 passed; 0 failed; 0 ignored
```

debug build 合计 **492 passed, 1 failed (search_big_file), 5 ignored**。

**search_big_file 失败说明**：该测试断言 `SearchIndex::build` 对 100k 行 × 10 列索引构建 < 5s（e2e.rs:2302），本机 debug build 6.28s 超阈值。此为**预先存在的性能敏感测试**，与 v0.6.6 任何改动无关（search 模块自 v0.4.2 后无变更；v0.6.4/v0.6.5 QA 报告已记录）。release build 3.06s 远低于阈值。**不阻塞发布**。

本版本无 Rust 业务逻辑变更（export.rs 签名扩展不改行为契约），测试数与 v0.6.5 基线一致。

## §4 构建审计

| 构建目标 | 命令 | 结果 |
|----------|------|------|
| core lib + src-tauri | `cargo build` | 0 error，1 warning（历史遗留 crate 名 `ruT0_data_kit_core should have a snake case name`，与 v0.6.5 一致） |
| frontend | `npm --prefix frontend run build` | vite build 3009 modules（与 v0.6.5 一致），0 error，4.51s |

## §5 文档一致性

| 检查项 | 结果 |
|--------|------|
| 4 处 manifest 版本号 | 全部 0.6.6 ✅ |
| `docs/04-版本标准.md` 里程碑行 | v0.6.6 状态 `release_complete` ✅ |
| `docs/versions/0.6.6/更新日志.md` | 回填完毕 ✅ |
| QA 报告 | 本文件 ✅ |
| `handoff/TASK-BOARD.md` | v0.6.6 DAG 回填 ✅ |
| 安全约束 §6 | 「不外发数据：全本地处理」保留，v0.6.6 不变 ✅ |
| `docs/01-页面与交互说明.md` | ExtractView 类型重命名交互规范补充 ✅ |

## §6 grep 关键符号验证

```
=== grep 1: 后端 export_extract type_rename (export.rs) ===
193:    type_rename: Option<std::collections::HashMap<String, String>>,
200:    let rename = type_rename.unwrap_or_default();
201:    let rename_type = |t: &str| -> String {
209:                let t_val = rename_type(&it.r#type);
223:                                rename_type(&it.r#type)

=== grep 2: 前端 tauri.js typeRename 透传 ===
308:    typeRename: opts.typeRename ?? null,

=== grep 3: 前端 ExtractView typeRename state + useEffect + ★ 标记 ===
84:  const [typeRename, setTypeRename] = useState({});
242:  useEffect(() => {
256:      if ((ca >= 5) !== (cb >= 5)) return ca >= 5 ? -1 : 1;
267:  const displayType = useCallback(
286:      render: (_, r) => displayType(r.type),
+ 结果区「类型重命名」Card + ★ 标记 + Input

=== grep 4: 4 manifest 版本号 0.6.6 ===
Cargo.toml:8:version = "0.6.6"
src-tauri/Cargo.toml:3:version = "0.6.6"
src-tauri/tauri.conf.json:4:  "version": "0.6.6",
frontend/package.json:4:  "version": "0.6.6",
```

四处关键验证（后端 type_rename / 前端透传 / 前端 UI + ★ / 版本号）全部到位。

## §7 端到端流程验证

用户预期流程：

1. **提取 + 自动列出 type**：ExtractView → 粘贴文本 → 勾规则 → 点「开始提取」→ 结果出现 → 结果 Card 顶部自动列出所有 type（去重）+ 每个 type 一个 Input + 高频 type（count≥5）带 ★ ✅
2. **填中文名即时生效**：在 `name` 行 Input 填「名字」→ 结果表 type 列立即显示「名字（name）」+ 顶部计数 Tag 也显示「名字（name）」 ✅
3. **导出应用**：点「导出 CSV」→ 文件 type 列写「名字」（非 name） ✅
4. **txt 导出应用**：TXT 模板 `{type}_{value}` → 导出文件每行 `名字_xxx` ✅
5. **不持久化**：重新提取一批新数据 → typeRename 重建为新结果的 type 集合（旧的中文映射不残留） ✅
6. **向后兼容**：不填任何中文名 → 结果表/计数 Tag/导出全部显示原 type（与 v0.6.5 行为一致） ✅

## §8 阻塞项

无。全部任务落地，构建/test（release 全绿）/grep/docs 全绿。`search_big_file` debug build 失败为预先存在的环境敏感问题，非本版本引入，release build 通过，不阻塞发布。

## §9 结论

`qa_passed` — 可进入 `release_complete`。
