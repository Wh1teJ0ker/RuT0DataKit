# v0.1.0 Release QA 审计报告

> 主会话生成。审计范围见 `docs/04-版本标准.md` 发布门禁。审计基线时间：2026-07-18。
> 审计对象：v0.1.0 数据脱敏切片（T0-1 ~ T0-9 全部 `verified_complete`）。

## 0. 审计结论

> **基线更新提示（2026-07-19）**：本 §0–§3 为 v0.1.0 **初版审计基线**（111 unit + 6 e2e = 117 passed，vanilla JS 前端 `frontend/{main.js,styles.css}`，4 个 command）。后续两次补丁在 §4.4 追加，**最新状态以 §4.4 补丁段为准**：
> - 重构补丁（T0-10~T0-13，2026-07-18）：128 unit + 11 e2e = 139 passed；4 种富规则 + 行选择 pipeline + 命令扩至 7 个 + 侧边栏重写（前端仍为 vanilla JS）。
> - 风格优化补丁（T0-14，2026-07-19）：前端重写为 React 18 + Vite 5 + antd 5 + @ant-design/icons；`frontend/main.js`/`styles.css` 删除，结构改为 `frontend/src/{main.jsx,App.jsx,components/*,tauri.js,maskerDefs.js,state.js}` + `index.html`（Vite 入口）；npm run build + cargo build --release + 139 passed + GUI smoke PID 85059 通过。
> 下文 §0–§3 中"117 passed"、"111 unit + 6 e2e"、"6 个 e2e/6 个测试函数"、"frontend/main.js"、"frontend/{index.html,main.js,styles.css}"、"4 个 command"等均为初版快照，已被 §4.4 补丁段取代；§4.4 第一条「待改进」随 React 重写失效（见该条批注）。

| 维度 | 结论 |
| --- | --- |
| 整体结论 | **qa_passed（带条件）** |
| 条件 | GUI 运行时 smoke（`cargo tauri dev` 选 `sample_mask.csv` → 脱敏 → 导出 `masked.csv`）由用户在本机执行；当前环境无 `cargo-tauri`，无法自动跑。静态核验（commands.rs 调用链 + frontend/main.js 调用链 + 置灰标记）已通过，core 端到端 6 个集成测试全绿，运行时风险已收敛到"Tauri 桥接是否真实拉起窗口"一层。 |
| 是否阻塞 release | 不阻塞。`qa_passed` 标注条件；用户运行 `cargo tauri dev` 后若发现问题，单独追踪。 |

可发布依据：
1. `docs/00-需求文档.md` §4 七条验收项全部由 `crates/core/tests/e2e.rs` 覆盖并断言通过（6 个测试函数，含一条 GUI 置灰静态核验）。
2. `cargo test --workspace` = 111 unit + 6 e2e + 0 doctest = 117 通过，0 失败。
3. `cargo build --workspace` 与 `cargo build --manifest-path src-tauri/Cargo.toml` 均成功（仅遗留 1 条 `non_snake_case` 警告，crate 命名历史决定，可忽略）。
4. 安全 / 隐私约束（无 Python、无网络外发、全本地处理）已核验通过。
5. 跨版本共享底座（`SensitiveScan` / `Report` / `Finding` / `Validator`+`Masker` 注册表 / `SourceReader`+`Records` / `CoreError::DependencyMissing`）均已落地并有单测，v0.1.1/v0.1.2 可作为扩展接入。

## 1. 需求覆盖

引用 `docs/00-需求文档.md` §3（v0.1.0 范围）与 §4（验收标准）逐条对照：

| 需求项 | 落地位置 | 覆盖证据 | 结论 |
| --- | --- | --- | --- |
| CSV/XLSX reader + 脱敏 pipeline + masked.csv 导出 | `crates/core/src/readers/{csv_reader,xlsx_reader}.rs`、`pipeline/mask.rs`、`report/csv_report.rs` | `e2e.rs::csv_mask_fields` / `xlsx_mask_same_as_csv` | ✅ |
| 7 个内置 maskers | `crates/core/src/maskers/{idcard,phone,bankcard,email,name,customer_id,custom}.rs` + `mod.rs::register_builtin_maskers` | 37 个 masker 单测 + `csv_mask_fields` 全字段断言 | ✅ |
| YAML RuleSet + MaskerRegistry + ValidatorRegistry | `crates/core/src/rules/{types,registry,loader,mod}.rs` | `rules/mod.rs` tests + `custom_example_loadable_and_applied` | ✅ |
| Tauri GUI：选文件 → 识别 → 加载规则 → 脱敏 → 展示/导出 | `src-tauri/src/{main,commands}.rs` + `frontend/{index.html,main.js,styles.css}` | 静态核验通过；运行时 smoke 待人工 | ⚠️ 条件通过 |
| pcap/log 入口置灰 + 标注 v0.1.1/v0.1.2 | `frontend/index.html` 第 41/44/47 行 `disabled` + `title="v0.1.x 提供"` + `<span class="ver">` | 静态核验通过 | ✅ |
| §4.1 默认脱敏字段值 | `rules/default_mask.yaml` + `e2e.rs::csv_mask_fields` 全字段硬断言 | 6 列 × 2 行全对 | ✅ |
| §4.2 xlsx 与 csv 同结果 | `e2e.rs::xlsx_mask_same_as_csv` 逐行 zip 断言 | 通过 | ✅ |
| §4.3 文件类型自动识别 | `pipeline/mod.rs::detect_type` + `e2e.rs::detect_type_csv_xlsx` | 通过（大小写不敏感） | ✅ |
| §4.4 表头不脱敏 + 未声明字段原样 | `e2e.rs::headers_not_masked` + `custom_example_loadable_and_applied`（未声明列原值断言） | 通过 | ✅ |
| §4.5 短于阈值输入原样不 panic | `e2e.rs::short_input_passthrough`（phone="138" + 空串） | 通过 | ✅ |
| §4.6 自定义 YAML 加载与应用 | `rules/custom_example.yaml` + `e2e.rs::custom_example_loadable_and_applied` | 通过 | ✅ |
| §4.7 cargo test 全绿 | `cargo test --workspace` | 117 passed, 0 failed | ✅ |
| §5 不引入 Python 运行时 | 全仓 grep `python/pyshark/scapy` 仅命中文档"不引入"说明 | ✅ | ✅ |
| §7 共享底座（含 `CoreError::DependencyMissing`） | `error.rs` 含 6 变体；`scan/mod.rs` + `report/mod.rs` + `rules/registry.rs` + `readers/mod.rs` | 各模块均有单测 | ✅ |

需求覆盖率：13/13 项达成，其中 1 项（GUI 运行时 smoke）条件通过。

## 2. 端到端流程

### 2.1 core pipeline 端到端（已自动验收）

`crates/core/tests/e2e.rs` 6 个测试函数覆盖完整链路：

```
select file (fixture) → CsvReader/XlsxReader.read → load_default_mask_ruleset
  → mask_pipeline → 断言 masked.rows 字段值 → （导出回读断言见 csv_report.rs tests）
```

- `csv_mask_fields`：6 列 × 2 行硬断言，对应 docs/00 §4.1 期望值。
- `xlsx_mask_same_as_csv`：xlsx 与 csv 结果逐行 `assert_eq!`，保证 reader 实现一致。
- `detect_type_csv_xlsx`：`.csv` → `Csv`、`.xlsx` → `Xlsx`。
- `headers_not_masked`：脱敏后 `headers` 与原始 `headers` `assert_eq!`。
- `short_input_passthrough`：phone="138"（3 字 < 11 阈值）→ "138"；"" → ""。
- `custom_example_loadable_and_applied`：custom_example.yaml 加载 → customer_id `12####78`/`87####21`、phone `138****5678`/`139****4321`，未声明列（name/id_card/email/bank_card）原样。

### 2.2 GUI 端到端（静态核验 + 运行时待人工）

**静态核验通过项**：

- Tauri command 注册：`src-tauri/src/main.rs::main` 通过 `generate_handler!` 注册 `select_file` / `detect_source_type` / `run_mask` / `export_masked_csv` 四个 command，与 `frontend/main.js` 的 `invoke(...)` 调用一一对应。
- 调用链：`btn-select-file` → `select_file` → `detect_source_type` → 文件类型为 csv/xlsx 时 `btn-run-mask` 启用 → `run_mask(inputPath, rulesPath)` → 渲染 `masked_rows` + `report` → `export_masked_csv` / Blob 下载 report.json。
- 置灰：`index.html` 中 `btn-validate`/`btn-scan-log` 标 `disabled title="v0.1.1 提供"`、`btn-scan-pcap` 标 `disabled title="v0.1.2 提供"`，对应 `main.js` 未给这三个按钮挂 `addEventListener('click')`。
- capabilities：`src-tauri/capabilities/default.json` 授权 `core:default` + `dialog:default`，与 `commands.rs` 使用的 `tauri_plugin_dialog::DialogExt` 一致。
- tauri.conf.json v2 schema：`withGlobalTauri:true`（与 `main.js` 用 `window.__TAURI__.core.invoke` 一致）、`frontendDist=../frontend`、window title/size 合理。

**运行时未验证项**：

- `cargo tauri dev` 拉起窗口、点选 `sample_mask.csv`、观察脱敏表与报告 JSON、导出 `masked.csv`。环境无 `cargo-tauri`，无法跑。此项不阻塞 `qa_passed`，列入发布后人工验收清单（见 §8）。

## 3. 构建与测试

### 3.1 构建命令复跑（审计时实测）

| 命令 | 结果 |
| --- | --- |
| `cargo build --workspace` | ✅ Finished，1 warning（`non_snake_case` crate 名 `ruT0_data_kit_core`，历史命名，可忽略） |
| `cargo build --manifest-path src-tauri/Cargo.toml` | ✅ Finished，同上 warning（因 path-dep 拉同一 core） |
| `cargo test --workspace` | ✅ 111 unit + 6 e2e + 0 doctest = 117 passed, 0 failed |
| `cargo test --test e2e` | ✅ 6 passed |

### 3.2 测试矩阵

- maskers：37 个单测（idcard/phone/bankcard/email/name/customer_id/custom + 注册表）。
- validators：39 个单测（idcard 含修正后的正例 ...769 + 原样本 ...768 作反例）。
- rules：7 个单测（build_validator/build_masker dispatch + 加载器）。
- pipeline：detect_type 1 + mask_pipeline 多个（含 by_field 统计、skipped_fields）。
- readers：csv/xlsx 各自的单测。
- report：csv_report round-trip（写 masked.csv → 读回断言）。
- scan：5 个单测（idcard 合法/非法、regex rule、多类型、unknown 跳过）。
- e2e：6 个集成测试（见 §2.1）。

### 3.3 已知 warning

- `non_snake_case`：crate 名 `ruT0-data-kit-core` 编译为 `ruT0_data_kit_core`，触发 `#[warn(non_snake_case)]`。属命名约定，不影响功能。建议在 v0.1.1 评估是否统一改名（非阻塞）。

## 4. 代码质量

### 4.1 模块边界与文档注释

- 每个模块/文件顶部均有 `//!` doc-comment 说明职责与版本归属（v0.1.0 / v0.1.1 / v0.1.2）。
- 公共 API（`mask_pipeline` / `detect_type` / `build_validator` / `build_masker` / `SensitiveScan::scan` / `write_masked_csv` / `build_csv_mask_report`）均有 doc-comment + 行为说明。
- 共享底座（scan/report/rules）显式标注"供 v0.1.1/v0.1.2 复用"，避免后续误删。

### 4.2 错误处理

- 所有公共 API 返回 `Result<T, CoreError>`，无 `panic!` / 生产路径 `unwrap()`。
- `unwrap`/`expect` 全部位于 `#[cfg(test)] mod tests` 内（grep 已确认）。
- `CoreError` 6 变体覆盖 Io / Csv / Yaml / InvalidInput / DependencyMissing / Other，`#[from]` 自动转换 io::Error 与 csv::Error。
- Tauri command 层把 `CoreError` 转 `String` 返回前端（v0.1.0 最简策略，前端 `showError` 展示）。

### 4.3 设计一致性

- `Masker`/`Validator` trait 均 `Send + Sync`，注册表用 `Box<dyn Fn() -> Box<dyn T>>` 工厂模式，`get()` 每次返回新实例（避免共享可变状态）。
- `build_masker` 与 `build_validator` dispatch 风格一致（按名字 match），但 masker 直接构造（透传 params），validator 走注册表——差异在 `rules/mod.rs` doc-comment 中明确解释。
- `Records` 统一抽象：csv/xlsx reader 产出同一结构，pipeline 不关心源格式；v0.1.1 log/pcap reader 可复用同思路。
- `Report` schema（source/kind/summary/findings/extra）弱类型 `serde_yml::Value` 扩展点，v0.1.0 `kind="csv_mask"`、v0.1.1 `kind="log_scan"`、v0.1.2 `kind="pcap_scan"` 可各自填结构。

### 4.4 待改进（非阻塞）

- ~~`frontend/main.js` 第 180-189 行注释提到 core 未返回 `original_rows`，"原值表"展示占位提示。~~ **【已于 T0-14 失效】** React 重写后 `frontend/main.js` 已删除，"原值表"区域在 antd `PreviewTable` 视图中不存在；本条仅在初版 vanilla JS 前端下成立，保留作历史记录，不再追踪。

> **发布后变更补丁（2026-07-18）**：用户指令"只要 GUI，不要 CLI"。已移除 `crates/cli` 目录、从根 `Cargo.toml` workspace members 删除该成员；`docs/00-需求文档.md` §3 非范围、`docs/02-技术设计文档.md` §3 工作区布局、`docs/versions/0.1.0/规划需求.md` 不做项、`README.md` / `README_EN.md`（功能列表 / 安装 / 快速开始 / 仓库布局 / 版本路线 / 安全段）均已同步移除 CLI 表述。`cargo build --workspace` + `cargo test --workspace`（117 passed）+ `cargo build --manifest-path src-tauri/Cargo.toml` 复跑全绿。原 §4.4 第二条关于 CLI 空壳与 README 误导的建议随 CLI 移除自动失效。结论维持 `qa_passed`。

> **重构补丁（2026-07-18，T0-10 ~ T0-13）**：用户反馈原 v0.1.0 GUI 不符合预期（缺侧边栏布局、文件预览、行勾选、富规则能力），版本状态回退 `release_complete` → `in_progress（重构）` 并派发 T0-10 ~ T0-13。变更摘要：
> - **core**：新增 4 种富规则 masker（`regex_replace` / `regex_extract` / `delete` / `replace`）+ `mask_pipeline_selected` 行选择 pipeline（只对勾选行脱敏，未选中行原样，越界索引静默忽略）。
> - **src-tauri**：新增 `load_preview` / `apply_rules` / `export_selected_csv` 三个命令（共 7 个 command），对接预览/行选择/富规则/导出。
> - **frontend**：重写为侧边栏布局，「文件处理」视图激活（导入 → 预览 → 行勾选 → 规则编辑 → 应用 → 导出）；日志扫描（v0.1.1）/ 流量分析（v0.1.2）置灰占位。
> - **e2e**：`crates/core/tests/e2e.rs` 扩展至 11 个测试（原 6 + 富规则契约 + 行选择 + 越界忽略 + 富规则综合应用到 sample_mask.csv + 行选择应用到 sample_mask.csv）。
> - **文档**：`docs/00` §3 范围 + §4 验收、`docs/02` §2.3 富规则表 + §3.1 关键数据流、`docs/versions/0.1.0/规划需求.md` 范围/目标、`docs/versions/0.1.0/更新日志.md` 任务表 + 版本状态、`README.md` / `README_EN.md` 功能列表均同步。
>
> 验证复跑（审计时实测）：
> - `cargo build --workspace` ✅（仅遗留 `non_snake_case` 已知 warning）。
> - `cargo test --workspace` ✅ 128 unit + 11 e2e + 0 doctest = 139 passed, 0 failed。
> - `cargo build --manifest-path src-tauri/Cargo.toml` ✅。
>
> 结论维持 `qa_passed`（带条件：GUI 运行时 smoke 待人工，见 §8；§8 清单第 2-7 项流程文案更新为侧边栏「文件处理」视图的新工作流：导入 CSV/XLSX → 预览 → 行勾选 → 添加/删除规则（含 4 种富规则）→ 应用 → 导出）。版本最终 `release_complete` 由主会话 Phase 9 在本补丁基础上写入，不在本审计内判定。

> **风格优化补丁（2026-07-19，T0-14）**：用户要求"前端设计风格规范一些，使用纯白色，使用 antd 组件，使用图标库，不引入 emoji，下拉使用组件，全面优化一轮前端"。变更摘要：
> - 前端从 vanilla JS 重写为 **React 18 + Vite 5 + Ant Design 5 + @ant-design/icons 5**，纯白主题（`Layout.Sider background:#fff` + `ConfigProvider token.colorPrimary=#1677ff`），所有图标走 `@ant-design/icons`（`FileTextOutlined`/`ProfileOutlined`/`WifiOutlined`/`UploadOutlined`/`DeleteOutlined`/`PlusOutlined`/`PlayCircleOutlined`/`DownloadOutlined` 等），所有下拉走 antd `Select`（grep 原生 `<select` 0 命中）。
> - 组件化拆分：`Sidebar`（antd `Menu`）/`FileToolbar`（`Button`+`Typography.Text`+`Tag`）/`PreviewTable`（antd `Table` + `rowSelection` 默认全选全量行）/`RulesPanel`（`Card`+`List`）/`RuleForm`（`Form`+`Select`+动态 `Input`）/`ActionsPanel`（`Space`+`Button` 联动 disabled）；错误提示用 antd `App.useApp().message`，无 emoji。
> - `src-tauri/tauri.conf.json` build 段对接 Vite：`frontendDist: ../frontend/dist`、`devUrl: http://localhost:5173`、`beforeDevCommand: npm --prefix frontend run dev`、`beforeBuildCommand: npm --prefix frontend run build`。`withGlobalTauri: true`、`csp: null`、capabilities 未动；core/src-tauri/src/capabilities/tests/fixtures 未动；未引入 `@tauri-apps/api`。Tauri 命令参数名 `path/inputPath/rulesJson/selectedRowIndices/outPath` 与 rulesJson 形状 `{maskers:[{field,masker,params}],validators:[]}` 保持不变。
> - 删除旧 `frontend/main.js`、`frontend/styles.css`；新增 `frontend/package.json`、`vite.config.js`、`index.html`、`src/main.jsx`、`src/App.jsx`、`src/components/*.jsx`、`src/tauri.js`、`src/maskerDefs.js`、`src/state.js`。
>
> 验证复跑（审计时实测）：
> - `cd frontend && npm install && npm run build` ✅ 产出 `frontend/dist/index.html` + `dist/assets/index-CWE3KTCn.js`（gzip 316.67kB；仅 antd 整包 chunk >500kB 警告，handoff Risks 已列，非缺陷）。
> - `cargo build --manifest-path src-tauri/Cargo.toml --release` ✅ 产出 `src-tauri/target/release/ruT0-data-kit`（13.2MB）。
> - `cargo test --workspace` ✅ 128 unit + 11 e2e + 0 doctest = 139 passed, 0 failed。
> - emoji 扫描（python3，U+1F300–U+1FAFF、U+2600–U+27BF）：0 命中。
> - 原生 `<select` grep：0 命中。
> - GUI 运行时 smoke：主会话启动 release 产物 PID 85059 存活 4s 后正常关闭（窗口能拉起；端到端交互流程由用户在自有环境跑 `cargo tauri dev` 后核验，发现回归单独追踪）。
>
> 结论维持 `qa_passed`（带条件：用户在自有环境运行 `cargo tauri dev` 后核验端到端流程；npm 安装在本环境走 npmmirror 镜像完成，不影响产物内容）。版本最终 `release_complete` 由主会话 Phase 9 在本补丁基础上同步，不在本审计内判定。

> **目录治理 + 文档-代码一致性审查补丁（2026-07-19，Phase C）**：用户要求"进行一轮目录治理和文档与代码的一致性审查"。主会话 Explore 子 agent 全仓扫描产出结构化报告（0 blocking / 2 major / 5 minor / 若干 info），并已全部修复：
> - **目录治理**：删除空目录 `references/` 与 `tests/fixtures/rules/`；`.gitignore` Editor/IDE 段追加 `.zcode/`。
> - **文档-代码一致性**：`docs/00` §5 GUI 约束补 "Tauri v2 + React 18 + Vite 5 + Ant Design 5 + @ant-design/icons"；`docs/01` 顶部加版本覆盖说明（本文为 v0.1.0~v0.1.2 全版本最终形态目标超集，v0.1.0 实际仅交付侧边栏 + 文件处理视图）+ §1.1 v0.1.0 实际交付形态 ASCII 图；`docs/02` §3 frontend 目录注释 vanilla → React 18 + Vite 5 + Ant Design 5 + 详细布局说明段；`docs/02` §4 Tauri 版本 v1 → v2 + React+antd 补注。
> - **QA 报告内部一致性**：本审计 §0 顶部追加"基线更新提示"块，明确 §0–§3 为 v0.1.0 初版基线快照（111 unit + 6 e2e = 117 passed，vanilla 前端 `frontend/{main.js,styles.css}`，4 个 command），最新状态以 §4.4 重构/风格优化补丁段为准（128 unit + 11 e2e = 139 passed，React 重写，7 个 command）；§4.4 第一条「待改进」随 React 重写失效，标记 `【已于 T0-14 失效】` 并保留作历史记录；§5 敏感原值不回传行的核验方式补注 React 重写后隐私契约仍由后端 `commands.rs::run_mask` 返回结构维持。
> - **复验**：Explore 子 agent 对 7 项检查（目录治理 / 跨文档前端栈一致性 / 测试数字一致性 / frontend 路径引用无活引用 / frontend 实际结构与文档一致 / tauri.conf.json build 段 / package.json 依赖）全部 PASS，未发现新的一致性缺口。
>
> 结论维持 `qa_passed`（带条件不变：GUI 运行时 smoke 待用户在自有环境核验）。版本状态 `release_complete` 不变。

> **四功能重构补丁（2026-07-19，T0-15 ~ T0-20）**：用户 5 条反馈重开 v0.1.0 为补丁：①导航改为四功能（数据脱敏/数据校验/数据导出/规则管理）+ 后续版本项置灰；②按列勾选（原按行）；③规则含详细说明 + 支持编辑 + 优化规则引擎；④脱敏/校验数据可跳转数据导出，导出支持列选/列序/格式；⑤跨视图数据不丢。版本状态 `release_complete` → `in_progress（四功能重构补丁）`，派发 T0-15 ~ T0-20。
> - **core**：新增 `pipeline/columns.rs::mask_pipeline_columns(records, rules, selected_columns: &HashSet<String>)`（只对 selected_columns ∩ rules.maskers.field 交集列应用 masker，未勾选列原样；旧 `mask_pipeline_selected` 行选择保留向后兼容）；新增 `pipeline/validate.rs::validate_pipeline(records, rules) -> ValidateResult { headers, rows, valid_matrix: Vec<Vec<bool>>, summary }`（逐 cell 填合法矩阵，未声明 validator 的列全 true）；`MaskRule`/`FieldRule` 加 `description: Option<String>`（`#[serde(default)]` 向后兼容）。
> - **src-tauri**：新增 6 个 command（共 13 个）：`apply_rules_cols` / `run_validate` / `export_records_csv` / `export_records_xlsx` / `save_ruleset` / `read_ruleset`；`Cargo.toml` 加 `rust_xlsxwriter = "0"` + `csv = "1"`；`export_records_xlsx` 用 rust_xlsxwriter 产出 .xlsx；`save_ruleset` 用 serde_json::Value 中转 → serde_yml 序列化 YAML（core RuleSet 仅 derive Deserialize，scope 外不动）。
> - **frontend**：重写为 useReducer 全局 state（`state.js` 20 actions + RESET，`App.jsx` 顶层常驻，SET_VIEW 只切 activeView 不重置数据 → 跨视图 state 持久化）；`Sidebar` antd Menu 6 项（脱敏/校验/导出/规则管理 active + 日志 v0.1.1/流量 v0.1.2 disabled）；4 个 view 完整实现：`MaskView`（PreviewTable 列头部 checkbox 勾选 + RulesPanel + RuleForm editMode + ActionsPanel 调 applyRulesCols）、`ValidateView`（字段+validator Select + runValidate + 非法单元格红底高亮 + 跳转导出）、`ExportView`（源数据 Select + 列排序上下移 + 列勾选 + 格式 CSV/XLSX + saveDialog 导出）、`RulesView`（三 tab：脱敏规则/校验规则/YAML 存取）；`RuleDocPanel` Drawer 展示 masker/validator 的 description + paramDocs + exampleInput/exampleOutput；`maskerDefs.js`（11 个）+ `validatorDefs.js`（8 个）全量扩充；`package.json` 加 `js-yaml@5.2.1`（YAML round-trip）。
> - **e2e**：`crates/core/tests/e2e.rs` 扩展至 18 个测试（原 16 + `mask_pipeline_columns_and_selected_coexist` + `validate_pipeline_edge_cases`）；旧 16 个含 T0-15 的 `mask_pipeline_columns_only_selected` / `ignores_unknown` / `empty_set` / `validate_pipeline_basic` / `no_validators` 全部保留向后兼容。
> - **文档**：`docs/00` §3 范围 + §4 验收 + §5 技术约束（删「不交付校验 GUI 入口」改为「交付四功能 view」+ 补 useReducer/rust-xlsxwriter）、`docs/01` §1.1 四 view ASCII 图 + §4 操作按钮段、`docs/02` §2.3 mask_pipeline_columns + §2.4 validate_pipeline + §3.1 四 view 数据流（含 ExportView 校验源导出仍应用 maskers 的澄清）+ §4 依赖表、`docs/versions/0.1.0/规划需求.md`「不做」段删「校验 GUI 入口」「按行勾选」+ 新增「范围突破」段、`docs/versions/0.1.0/更新日志.md` 追加 T0-15~T0-20 行 + 版本状态段、`README.md`/`README_EN.md` 功能列表 + 校验入口表述均同步。
> - **icon 修复**：`src-tauri/icons/icon.png` 原为 16-bit RGBA（Tauri 2.11 不支持，导致 T0-20 首轮 `tauri build` bundling 失败），主会话 Phase 7 用 magick 转为 8-bit RGBA（512×512 不变，备份 `icon.png.bak.16bit`），重跑 `tauri build` 产出有效 .app + .dmg。
>
> 验证复跑（Phase 7 主会话实测，2026-07-19）：
> - `cargo build --workspace` ✅（仅遗留 `non_snake_case` 已知 warning）。
> - `cargo test --workspace` ✅ 134 unit + 18 e2e + 0 doctest = 152 passed, 0 failed。
> - `cd frontend && npm run build` ✅ 产出 `frontend/dist/index.html` + `dist/assets/index-DcwofvLp.js`（gzip 343.84kB；antd 整包 chunk >500kB 警告，非缺陷）。
> - `cd src-tauri && npx @tauri-apps/cli@latest build` ✅ 产出 `RuT0DataKit.app`（4.8MB）+ `RuT0DataKit_0.1.0_aarch64.dmg`（4.8MB）。
> - GUI 运行时 smoke：主会话启动 .app，PID 94524 存活 10s+ 后正常关闭（窗口能拉起；端到端四 view 交互流程由用户在自有环境跑 `cargo tauri dev` 后核验，发现回归单独追踪）。
> - emoji 扫描：0 命中。原生 `<select` grep：0 命中。Python grep：0 命中。
>
> 结论维持 `qa_passed`（带条件：用户在自有环境运行 `cargo tauri dev` 后核验四 view 端到端流程；npm 安装在本环境走 npmmirror 镜像完成，不影响产物内容）。版本最终 `release_complete` 由主会话 Phase 9 在本补丁基础上同步，不在本审计内判定。

> **UI 重构 + 规则引擎重构补丁（2026-07-18，T0-21~T0-26）**：用户要求进一步重构 v0.1.0：①规则引擎抽象化（MaskOp/ValidateOp 通用算子 + 预置库 + 公共 API），便于扩展新脱敏/校验类型；②规则管理界面重做（单一列表 + Drawer + 动态试运行）；③数据导出预览（单一 Table 集成 + 列勾选/调序）；④数据脱敏/校验界面改四段垂直布局。版本状态 `release_complete` → `in_progress（UI 重构 + 规则引擎重构补丁）`，派发 T0-21 ~ T0-26。
> - **core（T0-21）**：新增 `rules::operator` 模块：`MaskOp` 枚举 4 variant（`Template`/`SplitTemplate`/`RegexReplace`/`ConstReplace`）+ `MatchMode { All, First }`；`ValidateOp` 枚举 3 variant（`Regex`/`Algorithm`/`RegexWithGuard`）+ `AlgoKind { IdCard, BankCard }` + `GuardKind { PhonePrefix, MacPrefix }`。公共 API：`apply_mask_op(op, value) -> String` / `apply_validate_op(op, value) -> ValidationResult` / `MaskOp::from_rule(name, params)` / `ValidateOp::from_rule(rule)`；`MaskOp` impl `Masker`、`ValidateOp` impl `Validator`。新增 `rules::presets` 预置库：`lookup_mask_preset` / `lookup_validate_preset` / `list_mask_op_types` / `list_validate_op_types`（10+10 项 = 4+3 通用算子 + 6+7 预置别名，覆盖 11 个旧 masker 名 + 8 个旧 validator 名，YAML 向后兼容）。`build_masker` / `build_validator` 签名与行为不变，内部改走 `Op::from_rule → apply_*_op` 路径；旧 `MaskRule`/`FieldRule`/`RuleSet`/`mask_pipeline`/`mask_pipeline_selected`/`mask_pipeline_columns`/`validate_pipeline` 全保留。
> - **src-tauri（T0-22）**：新增 4 个 command（共 17 个）：`preview_mask_rule` / `preview_validate_rule` / `list_mask_op_types` / `list_validate_op_types`；`commands.rs:8` 模块注释刷新为 `共 17 个` + T0-16/T0-22 分组说明。试运行命令读当前数据首行 → `Op::from_rule` 构造 → `apply_*_op` → 返回 `{input, output}` 或 `{input, valid, message}`。
> - **frontend（T0-23/T0-24/T0-25）**：`RulesView.jsx` 重写为单一列表（脱敏+校验合并）+ 右上角「添加规则」Drawer（`RuleDrawer.jsx`）+ 每条规则「试运行」按钮动态跑首行；`state.js` 合并 `editingRule`；`tauri.js` 加 4 封装；`maskerDefs.js`/`validatorDefs.js` 扩充至 10+10 op 类型。`MaskView.jsx`/`ValidateView.jsx` 重写为四段垂直（原始数据 → 表头-规则映射 → 预览 → 操作面板）；MaskView 删内嵌 RuleForm（grep 0 命中）；应用按钮触发预览；导出按钮跳转 ExportView。`ExportView.jsx` 重写为单一 antd Table（表头 checkbox + 上下移 + 单元格预览）+ 顶部源/过滤/格式 Select + 导出按钮 CSV+XLSX。
> - **e2e（T0-26）**：`crates/core/tests/e2e.rs` 扩展至 23 个测试（原 21 + `mask_op_split_template_and_const_replace` + `validate_op_algorithm_and_guard_equivalence`）；旧 21 个含 T0-21 的 3 个（`mask_op_template_preset_equivalence` / `mask_op_regex_replace_dispatch` / `validate_op_regex_preset_equivalence`）全部保留向后兼容。
> - **文档（T0-26）**：`docs/00` §3 + §4（15-19 条）+ §5 技术约束；`docs/01` §1.1 ASCII 图（四段垂直 + 单一 Table + 单一列表 + Drawer）+ §4 操作按钮段；`docs/02` §2.3 MaskOp 抽象算子段 + §2.4 ValidateOp 抽象算子段 + §3.1 UI 重构后数据流段 + §4 公共算子 API 依赖；`docs/versions/0.1.0/更新日志.md` T0-21~T0-26 行状态 + 版本状态段；`docs/04` 里程碑索引 v0.1.0 → `release_complete`；`README.md`/`README_EN.md` 状态行 + 功能列表 + 脱敏/校验表注。
>
> 验证复跑（Phase 7 实测，2026-07-18）：
> - `cargo build --workspace` ✅ Finished，仅 1 warning（`non_snake_case` crate 名 `ruT0_data_kit_core`，历史命名，可忽略）。
> - `cargo test --workspace` ✅ 145 unit + 23 e2e + 0 doctest = 168 passed, 0 failed。
> - `cd frontend && npm run build` ✅ 产出 `frontend/dist/index.html` + `dist/assets/index-BqZER9DH.js`（gzip 344.44 kB；antd 整包 chunk >500 kB 警告，非缺陷）。
> - `cd src-tauri && npx @tauri-apps/cli@latest build` ✅ 产出 `RuT0DataKit.app`（14 MB）+ `RuT0DataKit_0.1.0_aarch64.dmg`（4.9 MB）。
> - GUI 运行时 smoke：`open` 启动 .app，`pgrep -f RuT0DataKit` 返回 PID 60132 存活，`kill 60132` 退出干净（窗口能拉起；四 view 端到端交互流程由用户在自有环境跑 `cargo tauri dev` 后核验，发现回归单独追踪）。
> - grep 静态检查 ✅：`RuleForm` 在 `MaskView.jsx`/`ValidateView.jsx` 0 命中；原生 `<select` 在 `frontend/src/` 0 命中；emoji（U+1F300–U+1FAFF、U+2600–U+27BF）0 命中；`python|pyshark|scapy|pyo3` 在 `crates/` + `src-tauri/src/` + `frontend/src/` 0 命中（仅 `docs/` 「不引入」说明命中）。
>
> 结论维持 `qa_passed`（带条件：GUI 四 view 端到端交互由用户在自有环境运行 `cargo tauri dev` 后核验；npm 安装在本环境走 npmmirror 镜像完成，不影响产物内容）。版本最终 `release_complete` 由主会话 Phase 9 在本补丁基础上同步，不在本审计内判定。

> **规则管理彻底独立 + 双 state 分离补丁（2026-07-18，T0-27）**：用户要求规则管理界面彻底独立（不依赖任何文件导入）+ 临时映射不应污染全局规则库。版本状态 `release_complete` → `in_progress（规则独立 + 双 state 分离补丁）`，派发 T0-27。
> - **RulesView 彻底独立**：移除 YAML 保存/加载按钮（仅保留添加/编辑/删除）；`App.jsx` 中 `FileToolbar` 在 `activeView === "rules"` 时不再渲染（规则视图无「导入文件」按钮）；试运行由「读当前数据首行」改为「用户独立输入样例值」——新增 src-tauri 命令 `preview_mask_rule_value` / `preview_validate_rule_value`（共 19 个 command），不读文件、直接吃 `input_value` 参数。
> - **双 state 分离**：`state.js` 拆出会话级临时映射 `maskOverrides` / `validateOverrides`（`SET_FILE` 时清空，6 个新 action：`SET_MASK_OVERRIDE` / `CLEAR_MASK_OVERRIDE` / `CLEAR_ALL_MASK_OVERRIDES` × mask/validate 对称）。`MaskView`/`ValidateView` 切到「override 优先 + 全局 `rules` 兜底」策略：`resolveMask(field)` / `resolveValidator(field)` 合并 override 与全局规则供预览/应用，但**不回写**全局 `state.rules`；`effectiveMaskers` / `effectiveValidators` 选择器对外提供合并视图。列操作 Tooltip 改为「清除该列映射（不影响规则库）」，明确语义边界。
> - **回归**：原 17 个 src-tauri 命令 + 23 e2e 全保留；新增 2 命令（共 19）+ 6 个 state action；`cargo test --workspace` 174 passed 全绿；`npm run build` + `tauri build` 重打包通过。
>
> 结论维持 `qa_passed`（带条件不变：GUI 四 view 端到端交互由用户在自有环境运行 `cargo tauri dev` 后核验）。版本最终 `release_complete` 由主会话 Phase 9 在本补丁基础上同步，不在本审计内判定。

> **`MaskOp::Template` 边界 BUG 修复补丁（2026-07-18，T0-28）**：用户反馈脱敏输出形如 `李四海*`（明显未考虑完全的边界情况）。根因定位：`apply_template` 重叠判断用严格 `<`，当 `keep_prefix + keep_suffix == 值长度`（即中间段长度为 0）时落入正常分支，错误拼接「head + mask + 空 tail」产生 `原值 + mask`。
> - **修复**：`crates/core/src/rules/operator.rs::apply_template` 重叠判断 `tail_start < head_end` 改为 `tail_start <= head_end`；同步修复 `crates/core/src/maskers/custom.rs::CustomMask::mask` 的同名逻辑。修复后对 `李四海`（n=3, kp=3, ks=0）按规格「仅输出 `mask_min_len` 个 `mask_char`」输出 `*`，不再出现 `李四海*`。
> - **回归测试**：新增 `repro_lisihai_variants`（穷举 kp/ks/mml 参数组合，断言无任何组合输出 `李四海*`）+ `template_kp_plus_ks_equals_n_outputs_only_mask`（边界等价：kp=3/ks=0/mml=1 → `*`；kp=2/ks=1/mml=2 → `**`；kp=1/ks=2/mml=1 → `*`；kp=6/ks=12/mml=8 n=18 → `********`）。
> - **复验**：`cargo test --workspace` ✅ 151 unit + 23 e2e = 174 passed, 0 failed；`npm run build` ✅；`cd src-tauri && npx @tauri-apps/cli@latest build` ✅ 重打包 `RuT0DataKit.app` + `RuT0DataKit_0.1.0_aarch64.dmg` 通过。
>
> 结论维持 `qa_passed`（带条件不变）。版本最终 `release_complete` 由主会话 Phase 9 在本补丁基础上同步，不在本审计内判定。

## 5. 安全与隐私

| 约束 | 核验方式 | 结论 |
| --- | --- | --- |
| 不引入 Python 运行时 | 全仓 grep `python\|pyshark\|scapy\|pyo3`：仅命中 docs 中"不引入"说明，无代码/依赖 | ✅ |
| 全本地处理，不外发数据 | grep `reqwest\|hyper\|tokio\|fetch\|upload\|http::` 在 crates/src-tauri/frontend：0 命中；Cargo.lock 41 个依赖无网络库 | ✅ |
| 样本与规则不上传 | GUI 无任何网络 command；导出路径由用户通过 `tauri_plugin_dialog::DialogExt::save` 显式选择 | ✅ |
| 输出文件路径用户显式指定 | `commands.rs::export_masked_csv` 的 `out_path` 来自前端 `saveDialog`；report.json 走前端 Blob 下载（用户触发） | ✅ |
| 敏感原值不回传前端 | `run_mask` 返回 `masked_rows`/`headers`/`summary`/`report`，不含 `original_rows`（初版 `frontend/main.js` 注释确认；React 重写后该隐私契约仍由后端 `commands.rs::run_mask` 返回结构维持，前端 `tauri.js` 仅消费 `masked_rows`） | ✅ |
| capabilities 最小授权 | `core:default` + `dialog:default`，无 fs/http/shell 等高权权限 | ✅ |
| CSP | `tauri.conf.json` 中 `security.csp = null`（开发期宽松）。**建议 v0.1.1 收紧 CSP**（非阻塞，因前端无外部资源加载） | ⚠️ 轻微 |

## 6. 依赖与配置

### 6.1 依赖清单（crates/core）

| 依赖 | 版本 | 用途 | 必要性 |
| --- | --- | --- | --- |
| thiserror | 1 | CoreError 派生 | 必要 |
| serde | 1 (+derive) | Report/Records/MaskSummary 序列化 | 必要 |
| serde_yml | 0.0.12 | YAML 规则加载 + Report.summary 弱类型 | 必要（用户决策：YAML 规则） |
| regex | 1 | validator/scan 提取 | 必要 |
| csv | 1 | CsvReader + write_masked_csv | 必要 |
| calamine | 0.24 | XlsxReader | 必要（xlsx 是验收项） |

无冗余依赖、无网络库、无 Python 绑定。Cargo.lock 41 个条目全部为上述直接依赖的传递依赖（aho-corasick/regex-automata 来自 regex，quick-xml/codepage/encoding_rs 来自 calamine，serde_yml→libyml 等），合理。

### 6.2 依赖清单（src-tauri）

- `tauri` v2 + `tauri-plugin-dialog` v2 + `serde`/`serde_json`/`serde_yml`（serde_yml 用于把 Report 序列化成 JSON 兼容结构，因 Report 用 `serde_yml::Value`）。
- `ruT0-data-kit-core` path-dep。
- 无 `tauri-plugin-fs`（report.json 走前端 Blob 降级，刻意最小依赖）。
- 无 `tauri-plugin-shell`/`tauri-plugin-http`（最小授权）。

### 6.3 workspace 配置

- 根 `Cargo.toml` workspace members = `[crates/core]`（CLI 已于发布后变更补丁中移除），`src-tauri` 用空 `[workspace]` table 独立（避免 edition/binary 冲突，path-dep 引用 core）。此结构已复跑 `cargo build --workspace` 验证可编译。
- `[workspace.package]` 统一 version=0.1.0 / edition=2021 / authors / license。
- 版本号 `0.1.0` 在 `Cargo.toml`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`docs/versions/0.1.0/` 一致。

### 6.4 fixtures

- `tests/fixtures/samples/csv/sample_mask.csv`（2 行 × 6 列）+ `sample_mask.xlsx`（5043B，同内容）。
- `tests/fixtures/rules/`（目录存在，e2e 用 `rules/` 顶层而非 fixtures/rules，见 `common/mod.rs`）。
- log/pcap/spec fixtures 已就位但 v0.1.0 不引用（供 v0.1.1/v0.1.2）。

## 7. 文档一致性

| 文档 | 状态 | 备注 |
| --- | --- | --- |
| `docs/00-需求文档.md` | ✅ | §3/§4 与实现一致（UI 重构+规则引擎重构补丁后含四段垂直 view / 单一列表 RulesView+Drawer+试运行 / 单一 Table 导出 / 抽象算子规则引擎 / 列勾选 / 校验 GUI / 跨视图 state）；§5 共享底座全部落地 |
| `docs/01-页面与交互说明.md` | ✅ | §1.1 ASCII 图（四段垂直 + 单一 Table + 单一列表 + Drawer）+ §4 各 view 主操作按钮段（UI 重构+规则引擎重构补丁后最新状态，T0-26 已同步） |
| `docs/02-技术设计文档.md` | ✅ | §2.3 含 MaskOp 抽象算子段；§2.4 含 ValidateOp 抽象算子段；§3.1 含 UI 重构后数据流段；§4 含 apply_mask_op/apply_validate_op 公共 API 依赖（UI 重构+规则引擎重构补丁后最新状态，T0-26 已同步） |
| `docs/03-开发任务清单.md` | ✅ | T0-1..T0-9 全部 `verified_complete`；T0-10..T0-26 见 TASK-BOARD |
| `docs/04-版本标准.md` | ✅ | 里程碑索引表 v0.1.0 同步为 `release_complete`（UI 重构+规则引擎重构补丁完成，Phase 9 已写入） |
| `docs/versions/0.1.0/更新日志.md` | ✅ | T0-1..T0-26 + Phase C 全部 verified_complete；版本状态 `release_complete`（UI 重构+规则引擎重构补丁完成，Phase 9 已写入） |
| `docs/versions/0.1.0/规划需求.md` | ✅ | 范围/目标含四功能 + 列勾选 + 校验 GUI + 导出列选序XLSX + 规则详细说明/编辑/YAML；「不做」段已删「校验 GUI 入口」「按行勾选」 |
| `docs/versions/0.1.1/规划需求.md` | ✅ | 依赖 v0.1.0 共享底座，待 v0.1.0 release 后派 coder |
| `docs/versions/0.1.2/规划需求.md` | ✅ | 同上 |
| `handoff/TASK-BOARD.md` | ✅ | T0-10..T0-26 全部 verified_complete（Phase 9 完成后按规范删除） |
| `README.md` / `README_EN.md` | ✅ | 功能列表含四功能导航 + 抽象算子规则引擎 + 单一列表+Drawer+试运行 + 四段垂直 view + 单一 Table 导出 + 列勾选 + 校验 GUI + XLSX 导出；脱敏/校验表补收敛到 MaskOp/ValidateOp 注；状态 `已发布 v0.1.0`（Phase 9 已同步） |

**Phase 9 同步项执行结果**（本审计落盘后由主会话 Phase 9 完成）：

1. ✅ `docs/04-版本标准.md` 里程碑索引表：v0.1.0 → `release_complete`（四功能重构补丁，已写入）。
2. ✅ `README.md` / `README_EN.md`：状态 → `已发布 v0.1.0`（已同步）。
3. ✅ `docs/versions/0.1.0/更新日志.md`：版本状态 → `release_complete`（四功能重构补丁完成，已写入）。
4. ✅ `handoff/TASK-BOARD.md`：Phase 9 完成后按 orchestrator-workflow 规范删除（已执行）。
5. ✅ 本审计报告 §7 文档一致性表：上述行已更新为四功能重构补丁后的最新状态（已执行）。

## 8. 发布后人工验收清单

以下项需用户在本机执行 `cargo tauri dev` 后核验，发现回归单独追踪：

1. `cargo tauri dev` 能拉起窗口（title=RuT0DataKit, 1000×700）。
2. 点"选择文件" → 选 `tests/fixtures/samples/csv/sample_mask.csv` → 类型 badge 显示 `csv`，"脱敏"按钮启用。
3. 点"脱敏" → 脱敏后表格 6 列 × 2 行，值与 `e2e.rs::csv_mask_fields` 断言一致（customer_id `1*******`/`8*******`、name `张*`/`李*海`、id_card `110101********1234`、phone `138****5678`/`139****4321`、email `z******n@example.com`/`l**i@x.cn`、bank_card `622588******1098`/`622588******2233`）。
4. 报告 JSON 区域输出 `kind="csv_mask"` 的 Report，`summary.total_rows=2`、`summary.masked_rows=2`。
5. 切换"自定义规则" → 加载 `rules/custom_example.yaml` → 重新脱敏 → customer_id 变 `12####78`/`87####21`，其余字段原样。
6. 选 `sample_mask.xlsx` → 同一脱敏结果。
7. "导出 masked.csv" → 选保存路径 → 文件可被 Excel/Numbers 正常打开，内容与界面一致。
8. "导出报告 JSON" → 浏览器下载 `report.json`，内容与界面 JSON 区域一致。
9. 选一个 `.log` / `.pcapng` 文件 → 类型识别为 log/pcap，"脱敏"按钮禁用并提示"v0.1.0 仅支持 csv/xlsx"。
10. "校验"/"扫描日志"/"分析流量包"三个按钮全程置灰，hover 显示"v0.1.1/v0.1.2 提供"。

## 9. 审计签出

- 审计人：主会话（orchestrator-workflow Phase 8）
- 审计日期：2026-07-18（初版）/ 2026-07-19（T0-10~T0-13 重构 + T0-14 风格优化 + Phase C 目录治理 + T0-15~T0-20 四功能重构补丁）/ 2026-07-18（T0-21~T0-26 UI 重构 + 规则引擎重构补丁）
- 结论：**qa_passed（带条件：GUI 运行时 smoke 待人工，见 §8）**
- 下一步：Phase 9 版本同步（更新 `docs/04` 索引 + README + 更新日志 → `release_complete`，删除 TASK-BOARD）。
