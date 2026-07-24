# TASK-T15-1 Report

## implemented_changes

- `crates/core/src/readers/sql_reader.rs`
  - 删除死代码 `extract_col_name_from_def(def: &str)`（对 bytes 版的 str 包装，无调用点）。
  - 将原 bytes 版函数上方 doc-comment 合并整理：原两条 doc（一条属于被删的 str 版、一条属于 bytes 版）合并为一条直接描述 bytes 版的注释，保留「直接处理 `[u8]`，避免 `b as char` 把 UTF-8 多字节序列拆散导致乱码」的语义。
- `crates/core/src/validators/pinfo_phone.rs`
  - 测试函数名 `pdf_example_valid` → `spec_example_valid`（仅改名，测试体断言值不动）。
- 未改动 `crates/core/src/validators/phone.rs`、`crates/core/src/rules/builtin.rs`、`crates/core/src/rules/mod.rs`、`frontend/src/components/RulesView.jsx`：实测 grep 这些文件对 `PDF` / `pdf` / `个人信息数据规范文档` 均无命中，已是中性措辞，无需改动（handoff 实施说明已明确「已中性，保留」）。

## verification_run

- `cargo build --workspace 2>&1 | grep -E "warning|error" | head -20`
- `cargo build --workspace 2>&1 | grep "extract_col_name_from_def"`
- `cargo build --workspace 2>&1 | grep "never used" | grep sql_reader`
- `grep -rn "PDF" crates/ frontend/src/ 2>/dev/null | grep -v target/ | grep -v node_modules/ | head`
- `grep -rn "个人信息数据规范文档" crates/ frontend/src/ 2>/dev/null | grep -v target/ | grep -v node_modules/ | head`
- `cargo test --workspace 2>&1 | grep "test result:" | head`

## verification_results

- `cargo build ... | grep warning|error`：仅剩 2 条历史遗留 warning（`crate name should have a snake case name` + `generated 1 warning`），无 error，无 `extract_col_name_from_def` 命中，无 `never used` 对 sql_reader 命中。死代码 warning 已清除。
- `grep "PDF" crates/ frontend/src/`：0 命中（代码层无 PDF 字样）。
- `grep "个人信息数据规范文档" crates/ frontend/src/`：0 命中。
- `cargo test --workspace`：全绿，合计 491 passed（393+10+34+12+11+31）/ 5 ignored / 0 failed，与基线 491 一致。
- `cargo build`：0 error，warning 数量从 3 降至 2（仅剩 crate 名非 snake_case 历史遗留，按 handoff 不在本任务范围）。

## docs_updated

- 未更新。本任务 out_of_scope 明确不动 `docs/`，且无行为/用法变化。

## reported_status

verified_complete

## scope_deviation

none
