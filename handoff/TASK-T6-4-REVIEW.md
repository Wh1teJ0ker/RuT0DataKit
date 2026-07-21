```yaml
task_id: T6-4
verdict: pass_with_notes
recommendation: verified_complete
reviewed_commit: 78ebb99
```

## 核对结论

T6-4 goal 与 acceptance_criteria 全部满足，两点 REPORT 自报偏离均已核实为合理（非阻塞）。验证命令独立重跑全绿。安全约束（不外发数据）保持。docs/01/02/03 已同步。

## 独立验证重跑结果

### `cargo test -p ruT0-data-kit-core looks_like_blind_probe`
```
test logsign::blind_aggregator::tests::looks_like_blind_probe_ascii_binary ... ok
test logsign::blind_aggregator::tests::looks_like_blind_probe_equality ... ok
test logsign::blind_aggregator::tests::looks_like_blind_probe_negative_normal_sql ... ok
test logsign::blind_aggregator::tests::looks_like_blind_probe_length ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 308 filtered out
```
4 个新增单测全绿，与 REPORT 一致。

### `cargo test --workspace`
```
core lib:   ok. 310 passed; 0 failed; 2 ignored
integration: ok. 10 / 35 (2 ignored) / 12 / 11 / 31 passed
doc-tests:  ok. 0 passed
```
全绿，无失败，与 REPORT 一致。

### `cd frontend && npm run build`
```
✓ 3008 modules transformed.
dist/index.html                    0.31 kB │ gzip:   0.24 kB
dist/assets/index-Cxozkh_E.js  1,137.37 kB │ gzip: 354.89 kB
✓ built in 2.14s
```
build 通过。既有 chunk size > 500kB 警告与 T6-4 无关（T6-1 前就存在），不阻塞。

## goal / acceptance_criteria 核对

- `looks_like_blind_probe("ascii(substr((database()),1,1))>100") == true`：✓ 单测覆盖。
- `looks_like_blind_probe("SELECT * FROM users") == false`：✓ 单测覆盖。
- equality `substr((database()),1,1)='a'` → true：✓。
- length → true：✓（输入为 `length((database()))>5`，见下偏离判断）。
- 新增单测覆盖 3 类 + 1 负例：✓ 4 个 test。
- Tauri `detect_sql_blind_features` 返回 `{detected: bool, samples: Vec<String>}`：✓ `commands.rs:1095-1109`。
- PreprocessView 导入含探针 fixture 自动跳 Tools/SqlParseTool + sqlParseInput 预填：✓ `PreprocessView.jsx:57-68` dispatch 三 action。
- 导入普通 csv 不跳转：✓ detected=false 时 fall-through 到 `SET_HINT`，不 dispatch `SET_VIEW`。
- cargo test --workspace 全绿 + npm build 通过：✓ 独立重跑确认。

## scope_check

- in_scope 全部命中：`blind_aggregator.rs`（新增 pub fn + 4 单测）、`commands.rs`（新增命令）、`main.rs`（注册）、`tauri.js`（封装）、`PreprocessView.jsx`（handleImport 调 detect + 跳转）。
- out_of_scope 守住：
  - `extract_blind_probe` 签名未改（`blind_aggregator.rs` diff 仅在 extract_blind_probe 上方新增 pub fn，未触碰其实现/签名）。
  - 三类正则 `ascii_binary_regex / equality_regex / length_regex` 实现未改。
  - SqlParseTool 内部布局未改。
  - search/mask/validate/export 数据流未改。
- 越界审查：
  - `frontend/src/state.js:58` 注释「Tab 选中项」→「下拉栏选中项」——属 T6-3 reviewer 文案 minor 顺手收尾，不构成越界。
  - 未改 `crates/core/src/tools/sql_parse.rs`——HANDOFF 摘要明确允许「re-export 或直调二选一」，coder 选最小方案直调 `ruT0_data_kit_core::logsign::blind_aggregator::looks_like_blind_probe`，不越界。
  - docs/01/02/03 改动属行为变化文档同步（in_scope 未列 docs，但 docs/00 §6 安全约束要求文档同步），合理。

## docs_check

- `docs/01-页面与交互说明.md:153`：PreprocessView 段新增 v0.4.1 T6-4 子段，含 detect 调用、跳转动作、samples 上限 50、仅本地正则、不外发、detect 失败 console.warn——与代码一致。
- `docs/02-技术设计文档.md:523-530`：新增 §2.11.4 描述函数语义、命令返回结构、安全约束、GUI 集成、4 单测、未改项——与代码一致。
- `docs/03-开发任务清单.md`：新增 v0.4.1 表头 + T6-4 行，状态 `implemented_pending_review`——一致。
- 文档未发现比代码更乐观的描述。

## safety_check（docs/00 §6）

- `src-tauri/src/commands.rs` `detect_sql_blind_features` 函数体仅含 `looks_like_blind_probe` 正则调用 + Vec 收集 + `json!` 序列化，grep `fetch(`/`XMLHttpRequest`/`axios`/`reqwest`/`http::` 在该函数 0 命中。
- `frontend/src/tauri.js` `detectSqlBlindFeatures` 仅 `tauriInvoke`，无网络调用。
- `frontend/src/components/PreprocessView.jsx` detect 路径仅 dispatch + `message.success`，无网络调用。
- 不外发数据约束保持。

## 两点 REPORT 偏离判断

### 偏离 1：未改 sql_parse.rs re-export（选直调方案）

HANDOFF 摘要原文：「re-export looks_like_blind_probe（或在新模块）」+ 主会话调度说明「re-export 或直调二选一」。coder 选 Tauri 直调 `ruT0_data_kit_core::logsign::blind_aggregator::looks_like_blind_probe`，未改 sql_parse.rs。属 HANDOFF 明确允许的二选一，**不阻塞**。

### 偏离 2：length 测试输入 `length(database())>5` → `length((database()))>5`（双括号）

已核实 `blind_aggregator.rs:1280-1289` `length_regex` 正则为 `length\s*\(\s*\(\s*...\s*\)\s*\)\s*(>=?|<=?|=)\s*(\d+)`，**硬性要求 `length((<rt>))` 双括号形态**（与 `extract_blind_probe` 共用）。HANDOFF acceptance_criteria 字面写的单括号 `length(database())>5` 在既有正则下 **本来就不匹配**——这是 HANDOFF 文本与既有正则的不一致。

判断依据：
- `length_regex` 是 `extract_blind_probe`（v0.2.4 起）共用的正则，改它会影响所有盲注探针提取行为，明确落入 out_of_scope「不改 extract_blind_probe 签名」的语义保护范围。
- coder 选择改测试输入为真实命中形态 `length((database()))>5`，并写明测试注释，是正确的取舍——尊重既有正则、不越界改正则。
- acceptance_criteria 第 3 条「equality / length 均返回 true」的语义已满足（length 真实命中形态返回 true）。

**不阻塞**。但需记下已知限制：单括号 `length(database())>5` 不被 `looks_like_blind_probe` 检测——这是 v0.2.4 起既有正则的固有行为，不是 T6-4 引入的回归。建议 T6-6 文档如涉及盲注检测限制可提及。

## 非阻塞 minors（不影响 verdict）

1. **`detect_sql_blind_features` 的 `headers` 参数未使用**：`commands.rs:1098` 用 `let _ = &headers;` 消警告。HANDOFF risks 已预判「扫描时应遍历所有 cell（不只 sql_text 列）」，coder 实现确实遍历所有 cell，headers 仅保留参数名与 records 结构对齐。功能上无缺陷，但 `headers` 参数目前是 dead weight——若未来想按列名过滤（如只扫 sql_text 列）可启用。非阻塞，建议后续 T6-6 或 refactor 任务考虑是否移除或真正启用。

2. **docs/02 §2.11.4 章节号重复**：`docs/02-技术设计文档.md` 现有两个 `#### 2.11.4` 标题（line 523 新增的 T6-4 段 + line 532 既有 T5-1 rules tags 段）。编号冲突不影响渲染（Markdown 不依赖编号），但属文档 hygiene 缺陷。建议 coder 把新增段改为 `#### 2.11.5` 或把既有 T5-1 段改为 `#### 2.11.5`。非阻塞。

3. **`samples` 上限 50 是硬编码**：`commands.rs:1105` `samples.len() < 50`。HANDOFF risks/REPORT 均提到 50，docs/01 也写明 50。无 magic number 提取为常量，但与文档一致，非阻塞。

## 阻塞 issues

无阻塞问题。

## 最终判定

verdict: **pass_with_notes**
recommendation: **verified_complete**

两点 REPORT 自报偏离经核实均为 HANDOFF 允许的合理取舍，非越界、非回归。安全约束保持。验证独立重跑全绿。docs 同步。三个非阻塞 minors 可在后续任务（T6-6 或文档收尾）处理，不影响 T6-4 验收。
