```yaml
task_id: T15-1
goal: |
  清理 fa7e8c6 重提交后遗留的代码质量问题：(1) 删除/消解
  `extract_col_name_from_def` 死代码 warning（sql_reader.rs）；(2) 清除
  代码层（crates/ + frontend/）的 PDF 绝对化措辞，替换为中性表述
  （「规范」「spec」「内置规则集」），保留技术语义。注释改动不动行为，不需新测试。
in_scope:
  - crates/core/src/readers/sql_reader.rs（死代码清理）
  - crates/core/src/validators/pinfo_phone.rs（doc-comment + 测试名 PDF 措辞）
  - crates/core/src/validators/phone.rs（doc-comment PDF 措辞）
  - crates/core/src/rules/builtin.rs（doc-comment spec 措辞中性化）
  - crates/core/src/rules/mod.rs（doc-comment 措辞，如有 PDF 引用）
  - frontend/src/components/RulesView.jsx（如有 PDF 引用）
out_of_scope:
  - docs/*（docs 允许保留 PDF 引用，本任务只清理代码层）
  - 任何行为变更（不改函数逻辑、不改正则、不改测试断言值）
  - 版本号 bump（T15-3 负责）
  - 前端 EncryptTool 接线（T15-2 负责）
acceptance_criteria:
  - cargo build --workspace 2>&1 | grep "extract_col_name_from_def" 返回 0 命中（死代码 warning 清除）
  - cargo build --workspace 2>&1 | grep "never used" 对 sql_reader.rs 内函数返回 0 命中
  - grep -rn "PDF" crates/ frontend/src/ 2>/dev/null | grep -v target/ | grep -v node_modules/ 返回 0 命中（代码层无 PDF 字样）
  - grep -rn "个人信息数据规范文档" crates/ frontend/src/ 2>/dev/null | grep -v target/ | grep -v node_modules/ 返回 0 命中
  - cargo test --workspace 全绿（基线 491，无新增失败）
  - cargo build --workspace 0 error（warning 数量不增，crate 名非 snake_case 是历史遗留不在本任务范围）
verification_commands:
  - cargo build --workspace 2>&1 | grep -E "warning|error" | head -20
  - grep -rn "PDF" crates/ frontend/src/ 2>/dev/null | grep -v target/ | grep -v node_modules/ | head
  - grep -rn "个人信息数据规范文档" crates/ frontend/src/ 2>/dev/null | grep -v target/ | grep -v node_modules/ | head
  - cargo test --workspace 2>&1 | grep "test result:" | head
files_likely_to_change:
  - crates/core/src/readers/sql_reader.rs
  - crates/core/src/validators/pinfo_phone.rs
  - crates/core/src/validators/phone.rs
  - crates/core/src/rules/builtin.rs
  - crates/core/src/rules/mod.rs
risks:
  - extract_col_name_from_def 与 extract_col_name_from_def_bytes 功能重复：
    前者是对后者 str 包装。确认无外部调用后删除前者；若仍有调用则改为
    直接调 bytes 版。删除前先 grep 全 workspace 确认调用点。
  - pinfo_phone.rs 测试函数名 `pdf_example_valid` 改名可能被误认为行为变更：
    只改函数名（`spec_example_valid`），测试体断言不动，cargo test 仍绿。
  - phone.rs doc-comment 「仅按 PDF spec 校验」改「仅按 spec 校验」是注释，
    不影响行为。但要注意不要改到正则字符串本身。
depends_on: [PRE]
status: planned
```

## 实施说明

### 1. 死代码清理：extract_col_name_from_def

`crates/core/src/readers/sql_reader.rs:282` 的 `extract_col_name_from_def(def: &str)`
是对 `:288` `extract_col_name_from_def_bytes(def.as_bytes())` 的 str 包装。
grep 确认：实际调用点（`:252`、`:261`）已直接用 bytes 版，str 版无人调用
→ `warning: function ... is never used`。

处理：删除 `extract_col_name_from_def` 函数（保留 bytes 版）。

### 2. PDF 绝对化措辞清理（代码层，不动 docs/）

当前命中点（fa7e8c6 重提交后状态）：
- `crates/core/src/validators/pinfo_phone.rs:4` doc-comment「严格按规范 spec」—— 已是中性，保留
- `crates/core/src/validators/pinfo_phone.rs:99` 测试名 `pdf_example_valid` → 改 `spec_example_valid`
- `crates/core/src/validators/pinfo_phone.rs:102-103` 注释「规范另一示例...与 spec 矛盾」—— 已中性，保留
- `crates/core/src/validators/phone.rs:4` 「仅按 PDF spec 校验」→「仅按 spec 校验」
- `crates/core/src/rules/builtin.rs` 各 doc-comment「spec：...」—— 已中性，保留

搜索命令：`grep -rn "PDF\|pdf\|个人信息数据规范文档" crates/ frontend/src/ | grep -v target/ | grep -v node_modules/`

### 3. 不动

- `docs/` 下允许保留 PDF 引用（产品文档，不是代码层）
- 测试断言值不动（不改 `78813630178` 等具体号码）
- 正则字符串不动
- 版本号不动（T15-3 负责）
