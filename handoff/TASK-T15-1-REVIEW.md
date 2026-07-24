# TASK-T15-1 Review

```yaml
verdict: review_passed
```

## findings

无阻塞问题。

逐项核对（reviewer 独立复现，非引用 coder 报告）：

### goal 核对
- 死代码清理：`crates/core/src/readers/sql_reader.rs` 删除 `extract_col_name_from_def(def: &str)`（str 包装版），保留 `extract_col_name_from_def_bytes`。grep 确认全 workspace 仅剩 bytes 版定义与 `:252`/`:261` 两处调用，str 版无残留引用。doc-comment 合并为一条，保留「直接处理 `[u8]`，避免 `b as char` 拆散 UTF-8 多字节序列」语义，未失真。
- PDF 措辞清理：`pinfo_phone.rs:99` 测试名 `pdf_example_valid` → `spec_example_valid`，测试体断言值（`78813630178` 等）未动。`phone.rs:4` 当前为「仅按 spec 校验」，本任务未触碰该文件（diff 不含 phone.rs），已是中性状态，符合 acceptance。
- 注释/改名性质，无行为变更。

### acceptance_criteria 复现
1. `cargo build --workspace 2>&1 | grep "extract_col_name_from_def"` → 0 命中 ✅
2. `cargo build --workspace` 中 `never used` 计数 = 0（sql_reader 内函数无 never used 命中）✅
3. `grep -rn "PDF" crates/ frontend/src/`（排除 target/node_modules）→ 0 命中 ✅
   - 额外 `grep -rn "pdf\|Pdf\|pDF"` 同样 0 命中，无大小写遗漏。
4. `grep -rn "个人信息数据规范文档" crates/ frontend/src/` → 0 命中 ✅
5. `cargo test --workspace`：393+10+34+12+11+31+0 = 491 passed / 5 ignored / 0 failed，与基线 491 一致 ✅
6. `cargo build --workspace`：0 error，仅剩 2 条历史 warning（`crate name should have a snake case name` + `generated 1 warning`），warning 数量较基线 3→2 下降 ✅

### scope_check
未越界。仅改动 2 个文件，均在 `in_scope` 列表内：
- `crates/core/src/readers/sql_reader.rs`
- `crates/core/src/validators/pinfo_phone.rs`

`out_of_scope` 守护良好：
- `docs/` 未触碰 ✅
- 无函数逻辑/正则/断言值变更 ✅
- 无版本号 bump ✅
- 无前端 EncryptTool 接线 ✅
- 无无关重构或风格噪音 ✅

### docs_check
无需同步。本任务为死代码清理 + 注释/测试名中性化，无行为、用法、限制变化，`docs/` 按 out_of_scope 不动。`handoff/TASK-T15-1-HANDOFF.md` 与 `TASK-T15-1-REPORT.md` 均存在，未被 coder 自行删除。

### verification 核对
`verification_commands` 中列出的 4 类命令均由 reviewer 独立运行并复现通过，结果与 REPORT 一致，证据充分。

## defects

无。

## notes

- `phone.rs`、`rules/builtin.rs`、`rules/mod.rs`、`frontend/src/components/RulesView.jsx` 未改动：coder 报告称这些文件已是中性措辞，reviewer grep 确认 0 命中，与 handoff 实施说明「已中性，保留」一致，无遗漏。
- `cargo test` 中多出一条 `0 passed; 0 failed; 0 ignored` 的 test result 行（来自某个无测试的 crate），不影响 491 总数，与基线一致。
