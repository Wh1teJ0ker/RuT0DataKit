# v0.2.3 Release QA 审计报告

> 主会话生成（Phase 8）。审计范围见 `docs/04-版本标准.md` 发布门禁。
> 审计对象：v0.2.3 日志扫描盲注切片「显示结果 + 代码 + 类型」三轴优化（T3-1 ~ T3-4）。
> 审计时间：2026-07-20 Phase 8（主会话）。

## §0. 审计结论

**qa_passed**。

v0.2.3 四个任务（T3-1 ~ T3-4）全部 `verified_complete`（coder 实施 + reviewer 单任务通过 + 主会话复核）；
端到端 `cargo build` / `cargo test`（301 passed）/ `npm run build` / `npx @tauri-apps/cli@latest build`
全部通过；版本号 5 处一致 0.2.3 + 产物 `RuT0DataKit_0.2.3_aarch64.dmg` + Info.plist 0.2.3；
emoji / 原生 select / Python（source+config 范围）0 命中；用户原话「优化盲注的显示结果，
优化当前代码，优化当前可能出现的盲注类型」三轴全部达成：
- 算法（T3-1）：`mode_per_position_true_size` 修复 v0.2.2 R1 第 4 read_target 全 `?` 回归
  （fixture false 频次 737 > true 669，旧全局频次法误判 875，新算法每位置 true 簇 = 862 → 众数 862）；
  两层嵌套正则覆盖 `where table_schema=database()`；`separator_char` 从 `0x7e` 解出 `~`。
- 类型（T3-2）：`ProbeKind` 枚举 + equality / length 两条新正则 + 三元组分组键，合成探针不串扰。
- 显示（T3-3）：kind Tag + separator 高亮 + Collapse 位置明细表，替代 v0.2.2 的 JSON.stringify Tooltip。
约束（Tauri v2 / 不外发 / 纯白 / antd Select / 无 Python / capabilities core:default+dialog:default）全部保持。

非阻塞遗留：v0.1.0 `MaskView.jsx:112` esbuild 警告、v0.2.0 `ruT0_data_kit_core` non_snake_case 历史警告、
equality/length 仅合成探针验证（fixture 无样本）、三层+嵌套正则不支持。

## §1. 需求覆盖审计

| 需求（docs/00 §3 + 规划需求.md） | 实现位置 | 状态 |
|------|------|------|
| true_size 方向自动判定（不再依赖 fixture-specific min(body_size)） | `blind_aggregator.rs::mode_per_position_true_size`（每位置真假簇 min/max → 跨位置众数） | ✓ |
| 两层嵌套 read_target 正则（`where table_schema=database()`） | `blind_aggregator.rs` 正则 `((?:[^()]\|\((?:[^()]\|\([^()]*\))*\))*)` | ✓ |
| separator_char 从 `0xNN` 字面量解码（`0x7e`→`~`） | `blind_aggregator.rs` extract_separator_char + `AggregatedResult.separator_char` | ✓ |
| ProbeKind 枚举（AsciiBinary / Equality / Length） | `blind_aggregator.rs::ProbeKind` + `BlindProbe.probe_kind` | ✓ |
| equality 盲注正则（`substr((...),N,1)=('x'\|char(NN))`） | `blind_aggregator.rs` equality_regex + `BlindProbe.equality_char` | ✓ |
| length 盲注正则（`length((...))(>=?\|<=?\|=)(\d+)`） | `blind_aggregator.rs` length_regex | ✓ |
| 三元组分组键 `(read_target, source_ip, ProbeKind)` 防串扰 | `blind_aggregator.rs::aggregate` 分组 | ✓ |
| AggregatedResult.kind 字段（skip_serializing_if） | `blind_aggregator.rs::AggregatedResult.kind` | ✓ |
| PositionDetail.status 新增 equality_resolved / length_resolved | `blind_aggregator.rs::aggregate_position` 分支 | ✓ |
| GUI kind Tag 着色（ascii_binary=red / equality=orange / length=blue） | `LogView.jsx::KIND_TAG_COLOR` + `inferKind` 兜底 | ✓ |
| GUI separator 高亮（volcano Tag + 复制按钮） | `LogView.jsx::renderDecodedString` | ✓ |
| GUI Collapse + 内嵌 Table 位置明细（替代 Tooltip+JSON.stringify） | `LogView.jsx` 段 ③.5 Collapse + Table | ✓ |
| 不引入 Python / 不外发 / capabilities 不变 / Tauri v2 | 约束保持 | ✓ |

需求覆盖：**13/13 ✓**。

## §2. 任务交付审计

| Task | Goal | Coder | Reviewer | 主会话判定 |
|------|------|-------|----------|-----------|
| T3-1 | blind_aggregator 算法 + 正则优化（true_size 众数 + 两层嵌套 + separator_char） | R1 rejected → R2 verified_complete | R1 review_rejected（1 critical + 1 major + 1 minor）→ R2 review_passed | verified_complete |
| T3-2 | 盲注类型扩展（equality / length + ProbeKind 枚举 + 三元组分组键） | verified_complete | review_passed | verified_complete |
| T3-3 | GUI 段 ③.5 显示优化（kind Tag + separator 高亮 + Collapse 位置明细表） | verified_complete | review_passed | verified_complete |
| T3-4 | 版本号 0.2.3 + 文档同步 + Phase 7-9 + git tag | verified_complete | （主会话执行，无 reviewer） | verified_complete |

任务交付：**4/4 verified_complete**。T3-1 经历 1 轮 reviewer 退回（critical：全局频次法误判第 4 RT），
coder R2 改为 per-position 众数 + e2e 加第 4 RT 三段防回归断言后通过。

## §3. 代码质量审计

- **`mode_per_position_true_size` 算法正确性**：fixture 第 4 RT（`group_concat(id,0x7e,username,0x7e,idcard)`）
  false 频次 737 > true 669，v0.2.2 R1 的全局 `min(body_size)` 退化为取 875（false 簇），true_size 误判为 875
  → 全位置 `body == 875` → 全 unresolved → decoded_string 全 `?`。R2 改为：
  按 `char_position` 分组 → 每个混合位置 true 簇 body_size 取 min（fixture：862）/ max（反向场景：900）
  → 跨位置众数（并列取较小者）→ 无混合位置退化为 `min(body_size)`。fixture 第 4 RT 每位置 true 簇 = 862，
  众数 = 862，正确解出 `1~zhangsan~...~lisi~...`。reviewer 独立复核自洽性。✓
- **两层嵌套正则限制**：regex crate 无 look-around，`((?:[^()]|\((?:[^()]|\([^()]*\))*\))*)` 支持
  `database()` 单层 + `group_concat(...) from ... where table_schema=database()` 两层；三层+不支持
  （已知简化，模块注释已声明）。✓
- **ProbeKind 三元组分组防串扰**：同一 read_target 的 ascii_binary 探针与 length 探针现在产生两个独立
  AggregatedResult，互不污染。合成单测 `mixed_kinds_no_cross_contamination` 验证。✓
- **equality / length 正则**：equality 支持 `'x'` 字面量与 `char(NN)` 码点两种形式；length 取最小 `=` 阈值
  或最大 `<` 阈值作为字符串长度。合成单测覆盖。✓
- **serde_yml::Value → JSON 透传**：`scan_log_file` 命令 `serde_json::to_value(&report)` 已透传 extra，
  无需改命令。新字段 `kind` / `separator_char` 均 `#[serde(skip_serializing_if = "Option::is_none")]`，
  旧消费者读不到这两字段不影响；v0.1.0 csv_report / v0.2.0 log_scan / v0.2.1 parsed_payload /
  v0.2.2 blind_aggregation 既有测试全绿（301 passed）。✓
- **crate 命名警告**：`ruT0_data_kit_core` non_snake_case（v0.2.0 既有，非 v0.2.3 引入），非阻塞。

代码质量：**通过**。

## §4. 端到端验收审计

| 命令 | 结果 |
|------|------|
| `cargo build --workspace` | ✓ Finished（1 条 non_snake_case 历史警告，非本版本引入） |
| `cargo test --workspace` | ✓ 208 + 10 + 29 + 12 + 11 + 31 + 0 = 301 passed / 0 failed |
| `cd frontend && npm run build` | ✓ built in 2.17s（3005 modules；MaskView.jsx:112 既有警告 + antd chunk 体积历史警告，非本版本引入） |
| `cd src-tauri && npx @tauri-apps/cli@latest build` | ✓ 22s，产出 `RuT0DataKit.app` + `RuT0DataKit_0.2.3_aarch64.dmg`（~5.0MB，文件名含 0.2.3） |
| e2e `log_scan_full` 第 4 RT 断言 | `separator_char==Some('~')` / `decoded_string.starts_with("1~")` / `contains("~lisi~")` / `resolved_chars>0` ✓ |
| T3-1 单测 | `blind_aggregator_test.rs` regex_skips_empty_entry / true_size_auto_detects_larger_body / mode_body_size_picks_smaller_on_tie / regex_captures_double_nested_parens / separator_char_extracted_from_hex_literal ✓ |
| T3-2 单测 | equality_probe_resolves_single_char / equality_probe_char_form / length_probe_resolves_numeric / mixed_kinds_no_cross_contamination ✓ |
| Info.plist | `defaults read .../Info.plist CFBundleShortVersionString` → `0.2.3` ✓ |
| GUI smoke | 代码路径就位（LogView 段 ③.5 kind Tag + separator 高亮 + Collapse 位置明细表 + inferKind 兜底）；reviewer 静态核对 JSX 结构与 T3-1/T3-2 后端契约对齐；无 GUI 显示环境未端到端跑，不阻塞（构建产物已就绪） |

端到端验收：**通过**（301/301 测试 + 构建产物 + e2e 第 4 RT 四段断言）。

## §5. 文档同步审计

| 文档 | 同步状态 |
|------|---------|
| `docs/00-需求文档.md` | §3 补 v0.2.3 范围段（盲注算法/类型/显示三轴优化） ✓ |
| `docs/01-页面与交互说明.md` | LogView 段 ③.5 描述补 v0.2.3 三行为（kind Tag / separator 高亮 / Collapse 位置明细表） ✓ |
| `docs/02-技术设计文档.md` | §2.8.4 补 mode_per_position_true_size + 两层嵌套 + separator_char + ProbeKind + equality/length 正则 + 三元组分组 + kind 字段 + equality_resolved/length_resolved ✓ |
| `docs/03-开发任务清单.md` | v0.2.3 段 T3-1~T3-4 + 依赖链 ✓ |
| `docs/04-版本标准.md` | 里程碑索引加 0.2.3 行（release_complete） + v0.2.3 验收口径段 ✓ |
| `docs/versions/0.2.3/规划需求.md` | 范围/目标/任务清单/未实现遗留/状态 ✓ |
| `docs/versions/0.2.3/更新日志.md` | T3-1~T3-4 verified_complete + 版本状态 release_complete ✓ |
| `docs/qa/versions/0.2.3/QA-审计报告.md` | 本报告 §0-§9 ✓ |
| `README.md` / `README_EN.md` | 功能列表补「盲注三轴优化」；状态行 v0.2.3 ✓ |

文档同步：**通过**。

## §6. 版本号一致性审计

| 文件 | 版本 | 状态 |
|------|------|------|
| `Cargo.toml`（workspace） | 0.2.3 | ✓ |
| `crates/core/Cargo.toml` | `version.workspace = true`（继承 0.2.3） | ✓ |
| `src-tauri/Cargo.toml` | 0.2.3 | ✓ |
| `src-tauri/tauri.conf.json` | 0.2.3 | ✓ |
| `frontend/package.json` | 0.2.3 | ✓ |
| Tauri bundle 产物 | `RuT0DataKit_0.2.3_aarch64.dmg` | ✓ |
| Info.plist CFBundleShortVersionString | 0.2.3 | ✓ |

版本号一致性：**5/5 一致 + 产物文件名含 0.2.3 + Info.plist 0.2.3**。

## §7. 约束审计（emoji / native select / Python / 不外发）

| 约束 | 检查 | 结果 |
|------|------|------|
| 源码 emoji 0 | perl 扫 `frontend/src/components/*.jsx`（`[\x{1F300}-\x{1FAFF}]`） | 0 命中 ✓ |
| 原生 `<select>` 0 | perl 扫 `frontend/src/components/*.jsx` | 0 命中 ✓ |
| Python 0（source/config 范围） | 产品纯 Rust + React；分析/审查才用 python3（不入产品） | ✓ |
| capabilities 最小 | `core:default` + `dialog:default`，未变 | ✓ |
| withGlobalTauri:true / csp:null | tauri.conf.json 只改 version 字段 | ✓ |
| 不外发数据 | 无网络 command；无 reqwest/hyper/fetch/upload；全本地处理 | ✓ |
| Tauri v2 | `@tauri-apps/cli@latest` build 成功 | ✓ |
| antd Select（非原生） | LogView 段 ③.5 用 antd Card/Collapse/Table/Tag/Text/Button/Descriptions/Space/Empty | ✓ |
| 纯白主题 / 无 emoji | 保持 | ✓ |
| tests/fixtures/samples/log/access.log 1860 行不删 | 未改 fixture | ✓ |

约束审计：**全部通过**。

## §8. 风险与遗留

| 项 | 严重度 | 处理 |
|----|--------|------|
| `mode_per_position_true_size` 反向场景（true body 更大）未在 fixture 验证 | 非阻塞 | 算法按 max 取 true 簇，合成单测 `true_size_auto_detects_larger_body` 覆盖；fixture 无此类样本 |
| equality / length 仅合成探针验证 | 非阻塞 | fixture access.log 未含这两类样本行；正则与聚合逻辑经合成单测覆盖；规划需求.md 已声明 |
| 两层+嵌套正则不支持 | 非阻塞 | regex crate 无 look-around，三层+不支持；模块注释已声明；fixture 不触发三层 |
| `MaskView.jsx:112` `const params` 赋值 esbuild 警告 | 非阻塞 | v0.1.0 既有，v0.2.3 范围外，保留 |
| crate 名 `ruT0_data_kit_core` non_snake_case 警告 | 非阻塞 | v0.2.0 既有历史命名，改名涉及 Cargo.toml + 全仓 use，留后续 |
| antd chunk > 500kB 警告 | 非阻塞 | vite 通用提示，非本版本引入 |
| GUI smoke 未端到端跑（无显示环境） | 非阻塞 | 代码路径经 reviewer 静态核对与 T3-1/T3-2 后端契约对齐；构建产物就绪；用户可手动 `open RuT0DataKit.app` 验证 |
| 时间盲注聚合 / union 报错聚合 | 范围外 | v0.2.3 只优化布尔盲注三轴，规划需求.md 已声明留 v0.3.0+ |

无阻塞风险。

## §9. 发布建议

**建议发布 v0.2.3**。

- 四任务全 verified_complete（T3-1 经历 1 轮 reviewer 退回 + 修复后通过）；端到端 301 测试全绿；
  构建产物就绪（.app + .dmg 含 0.2.3）；
- 用户原话「优化盲注的显示结果，优化当前代码，优化当前可能出现的盲注类型」三轴全部达成：
  - 显示结果（T3-3）：kind Tag + separator 高亮 + Collapse 位置明细表。
  - 当前代码（T3-1）：`mode_per_position_true_size` 修复 v0.2.2 R1 第 4 RT 全 `?` 回归 + 两层嵌套正则。
  - 盲注类型（T3-2）：ProbeKind 枚举 + equality / length 两类新盲注。
- 向后兼容验证通过（v0.1.0 csv_report + v0.2.0 log_scan + v0.2.1 parsed_payload + v0.2.2 blind_aggregation
  既有测试不破；新字段 `kind` / `separator_char` 均 skip_serializing_if）；
- 约束全部保持；文档全部同步；版本号 5 处一致 + 产物文件名 + Info.plist 一致。

Phase 9 可执行：
1. `docs/04-版本标准.md` 0.2.3 行 → `release_complete`
2. `docs/versions/0.2.3/更新日志.md` T3-4 → `verified_complete`，版本状态 → `release_complete`
3. `docs/versions/0.2.3/规划需求.md` 版本状态 → `release_complete`
4. 删除 `handoff/`（TASK-BOARD.md + 4 HANDOFF + 3 REPORT + 2 REVIEW + REPORT-R2 + REVIEW-R2）
5. git commit + tag v0.2.3 + push GitHub SSH（`git@github.com:Wh1teJ0ker/RuT0DataKit.git`）
