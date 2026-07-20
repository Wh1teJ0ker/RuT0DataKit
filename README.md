# RuT0DataKit

面向数据安全 CTF 场景的快速数据脱敏 / 解析工具集。核心用 Rust 实现，桌面端基于 Tauri v2，
全部处理在本地完成，不上传任何样本或规则。本仓库面向数据安全竞赛与红队场景中的
"敏感数据快速清洗 / 解析" 需求，不依赖 Python 运行时。

> **当前状态：v0.2.2 日志扫描盲注二分序列聚合还原（T2-12 ~ T2-14 已 verified_complete，T2-15 收尾完成待 Release QA）。** v0.2.2 在 v0.2.1 SQLi payload 单条语义解析基础上新增跨 entry 布尔盲注二分序列聚合还原：`blind_aggregator` 模块（`crates/core/src/logsign/blind_aggregator.rs`）对一批同源（同 `read_target` + 同 `source_ip`）的盲注二分探针按 `char_position` 聚类，基于 `LogEntry.size`（HTTP 响应 body 字节数）自动判定真假方向（`true_size = min(body_size)`，fixture-specific 假设），还原出被盲注读取的完整字符串（flag，如 `database()`=`"person"` / `table_name`=`"person_data"` / `column_name` 含 `id,username,password`）；自带嵌套正则 `(?:[^()]|\([^()]*\))*` 吃单层 `(...)`；`Report.extra` 由 `Value::Null` → `Value::Mapping({blind_aggregation: [...]})` 向后兼容；GUI LogView 新增「盲注聚合结果」卡片（antd inner Card + Text copyable 还原串 + 探针数 + 来源 IP + 位置明细 Tooltip + 空态 Empty）。v0.2.1 底座（parsed_payload 单条语义解析 + decoded 字段 + Finding.extra 透传）保持不变并向后兼容。版本状态约定见 `docs/04-版本标准.md`。

## 功能

| 版本 | 能力 | 状态 |
| --- | --- | --- |
| v0.1.0 | CSV / XLSX 表格数据脱敏 + 校验 + 导出 + 规则管理（四功能 GUI） | 已发布 v0.1.0 |
| v0.2.0 | 日志文件解析 + SQLi 攻击签名扫描 + 弱口令 / 敏感字段扫描 | 已发布 v0.2.0 |
| v0.2.1 | SQLi payload 语义解析（6 类）+ 字段还原（query/path/UA）+ `+` 解码 | 已发布 v0.2.1 |
| v0.2.2 | 日志扫描盲注二分序列自动聚合还原 flag + GUI 盲注聚合结果卡片 | 开发中（T2-12~T2-14 verified_complete，T2-15 收尾中） |
| v0.3.0（规划） | pcap 流量包敏感数据提取（依赖系统 tshark） | 规划中 |

v0.1.0 已落地：
- core pipeline：`detect_type` → `SourceReader` → `mask_pipeline` / `mask_pipeline_selected`（行选择，向后兼容）/ `mask_pipeline_columns`（列勾选） → `validate_pipeline`（校验） → `write_masked_csv` / `export_records_csv` / `export_records_xlsx`
- 7 种内置脱敏器 + 1 种通用 `custom` 脱敏器 + 4 种富规则脱敏器（`regex_replace` / `regex_extract` / `delete` / `replace`）
- 7 种内置校验器（v0.2.0 日志扫描复用）
- **抽象算子规则引擎**：`MaskOp`（Template/SplitTemplate/RegexReplace/ConstReplace）+ `ValidateOp`（Regex/Algorithm/RegexWithGuard）+ 预置库（6 脱敏别名 + 7 校验别名）+ `apply_mask_op`/`apply_validate_op` 试运行 API；新增脱敏类型 = 加 variant + 预置库一行映射。
- **规则管理单一列表 + Drawer + 动态试运行**：脱敏+校验合并单一列表，右上角「添加规则」Drawer，每条规则「试运行」由用户独立输入样例值（不依赖任何文件导入），完全脱离数据文件管理。
- **规则/映射双 state 分离**：`state.rules` 是全局规则库（RulesView 管理，独立于文件）；`state.maskOverrides` / `state.validateOverrides` 是会话级临时映射（MaskView/ValidateView 管理，导入文件时清空）。MaskView/ValidateView 选算子只写 override，不污染规则库；「应用」时合并 override + 全局规则组成临时 RuleSet，导出/校验流程不受影响。
- **数据脱敏/校验四段垂直 view**：原始数据 → 表头-规则映射 → 预览 → 操作面板；应用按钮触发预览；导出按钮跳转。
- **数据导出单一 Table 集成**：表头 checkbox + 上下移调序 + 单元格预览，CSV+XLSX 双格式。
- **四功能导航 GUI**（侧边栏 4 active view + 2 disabled；数据脱敏 / 数据校验 / 数据导出 / 规则管理）
  - **按列勾选脱敏**：表头列 checkbox + 「应用规则到勾选列」
  - **数据校验 GUI**：ValidateView 独立 view，逐 cell 合法性矩阵 + 「跳转到数据导出」
  - **数据导出列选 / 列序 / 格式**：CSV + XLSX（src-tauri `rust_xlsxwriter`）
  - **规则详细说明 + 编辑**：`MaskRule` / `FieldRule` 带 `description` 字段；RulesView 编辑（已移除 YAML 保存/加载，仅保留添加/编辑/删除）
  - **跨视图数据不丢**：`App.jsx` 顶层 `useReducer` 内存常驻，切 view 不重置数据
- **`MaskOp::Template` 边界 BUG 修复**：当 `keep_prefix + keep_suffix == 值长度`（中间段长度为 0）时，原实现错误地输出 `原值 + 脱敏星`（如 `李四海*`）；修复后按规格走重叠分支，仅输出 `mask_min_len` 个 `mask_char`（如 `*`）。同步修复旧 `CustomMask::mask`。
- 前端基于 React + Ant Design 5 + @ant-design/icons，纯白主题，无 emoji，下拉使用 antd Select 组件

v0.2.0 日志处理切片已落地：
- **CLF 日志解析**：`crates/core/src/log/`：`LogReader::read` + `LogEntry`（ip / time / method / path / query / status / user_agent 等字段）+ `parse_query` 双重 URL 解码（`url_decode_twice`，解码 `%XX` 但不解 `+`，pattern 一律按 decoded 形态编写）。fixture `tests/fixtures/samples/log/access.log` 共 1860 行（含 4 条 SQLi 变体样本）。
- **SQLi 攻击签名引擎**：`crates/core/src/logsign/`：`SignatureEngine` + `sqli_signatures.yaml`（6 类内置签名：`sqli_blind_binary` / `sqli_union` / `sqli_error_based` / `sqli_time_based` / `sqli_tautology` / `sqli_comment`）+ `context_anchor` 锚点抑制弱信号误报（如 `or` / `--` / `#` 单独命中不算）。
- **签名引擎装载 / 重载**：`load_builtin_signatures`（内嵌 YAML）+ `load_signatures_from_str`（运行时 YAML 字符串）+ `SignatureEngine::from_rules`（构造时合并 + `enabled` 开关）。
- **日志扫描 pipeline**：`crates/core/src/pipeline/log_scan.rs`：`scan_log` 组合 SignatureEngine + 弱口令 grep（`WEAK_KEYWORDS`：password / passwd / admin / 123456 / root / qwerty 等）+ `DefaultSensitiveScan`（复用 v0.1.0 `SensitiveScan` trait），产出统一 `Report(kind="log_scan")`；`summary` 含 `total_lines` / `sqli_hits` / `weak_password_hits` / `sensitive_hits` / `top_attack_ips`（按命中次数排序，截断到 5 个）。
- **GUI 日志入口激活**：src-tauri `scan_log` 命令；frontend 侧边栏「日志扫描」由 disabled 切 active；pcap 仍置灰（v0.3.0）。
- **可观测性**：日志扫描结果与脱敏共用统一 `Report` / `Finding` serde 结构，前端可序列化展示。

v0.2.1 日志处理切片语义增强已落地：
- **`parse_query` `+` 解码 + decoded 字段**：`parse_query` 按 form-urlencoded 标准 `+`→space 再双重 `%XX` 解码；`LogEntry` 新增 `decoded_path` / `decoded_query` / `decoded_ua` 字段，`parse_line` 一次性填充，供签名引擎与 GUI 直接消费已解码文本（v0.2.0 口径只解 `%XX` 不解 `+`，v0.2.1 起改为标准 form-urlencoded）。
- **SQLi payload 语义解析**：`crates/core/src/logsign/payload_parser.rs`：`parse_payload(decoded_text) -> Option<ParsedPayload>` 对命中后的 decoded payload 做 6 类结构化语义解析（blind_boolean / union / error / time / tautology / comment），提取 read_target / char_position / compared_ascii / comparator / union_columns / sleep_seconds + 人类可读 `summary`。
- **`SignatureHit.parsed_payload`**：`scan_log_entry` 产 hit 时对命中文本段调 `parse_payload` 填充，未解析到为 `None`。
- **`Finding.extra` 透传**：`Finding` 新增 `extra: Option<serde_yml::Value>`（`skip_serializing_if = "Option::is_none"`，向后兼容）；`scan_log` pipeline 把 sqli hit 的 `parsed_payload` 序列化透传进 `Finding.extra`，供前端展示「解析结果」/「读取目标」列；weak_password / sensitive / csv_mask 场景 `extra=None`。
- **6 类签名 pattern 扩变体**：`sqli_union` 兼容 `union all select`；`sqli_error_based` 兼容 `exp(~...)` / `floor(rand(0)*2)` 报错注入变体。
- **GUI LogView decoded 列 + 解析结果/读取目标列**：原始日志表新增 `decoded_path` / `decoded_query` / `decoded_ua` 列；findings 表新增「解析结果」（`ParsedPayload.summary`）+「读取目标」（`read_target`）列。

v0.2.2 日志扫描盲注二分序列聚合还原已落地：
- **blind_aggregator 模块**：`crates/core/src/logsign/blind_aggregator.rs`：`BlindAggregator::collect_from_entries` 跑自带正则抽 `ascii(substr((<read_target>),N,1))<cmp><thr>` 形态二分探针；按 `(read_target, source_ip)` 分组 + `char_position` 子分组；基于 `LogEntry.size`（HTTP 响应 body 字节数）自动聚类真假方向（`true_size = min(body_size)`，fixture-specific 假设：条件成立响应 body 更小）；首个 `beyond_end` 即停止拼接，还原出完整 flag 字符串（如 `database()`=`"person"` / `table_name`=`"person_data"` / `column_name` 含 `id,username,password`）。
- **BlindProbe / PositionDetail / AggregatedResult 结构**：`read_target` / `char_position` / `threshold` / `body_size` / `source_ip` / `line_no` / `decoded_char` / `ascii_val` / `true_size` / `probe_count` / `status`（`resolved` / `unresolved_all_true` / `beyond_end` / `insufficient_probes`）/ `decoded_string` / `resolved_chars` / `unresolved_chars` / `beyond_end_positions` / `position_details`。
- **自带嵌套正则**：regex crate 无 look-around，`read_target` 内层嵌套括号用 `(?:[^()]|\([^()]*\))*` 吃单层 `(...)`（如 `database()` / `group_concat(table_name)`）；多层嵌套不支持（已知简化）。
- **Report.extra.blind_aggregation 透传**：`scan_log` 末尾把 `Vec<AggregatedResult>` 序列化进 `Report.extra` 的 `Value::Mapping({blind_aggregation: [...]})`；无盲注探针时 extra 仍为含空数组的 Mapping；向后兼容（v0.2.1 `Finding.extra: Option<Value>` 既有行为不破）。
- **GUI LogView 盲注聚合结果卡片**：findings 表下方新增段 ③.5，读 `Report.extra.blind_aggregation` 渲染 antd inner Card + `Typography.Text` copyable 还原串（一键复制 flag）+ 探针数 + 来源 IP + 位置明细 Tooltip + 空态 `Empty`。

## 安装

### 构建依赖

- **Rust toolchain**（stable，推荐 1.75+）：通过 rustup 安装。
- **Tauri v2 CLI**：`cargo install tauri-cli --version "^2"` 或 `cargo tauri` 子命令（用于跑 `cargo tauri dev` / `cargo tauri build`）。
- **tshark**：仅 **v0.3.0**（pcap 解析）需要。v0.1.0 与 v0.2.0 **不需要** tshark。
  macOS：`brew install wireshark`；Debian/Ubuntu：`apt install tshark`。

### 构建源码

```sh
git clone <repo-url> RuT0DataKit
cd RuT0DataKit
cargo build --workspace
```

## 快速开始

### 跑核心 pipeline（测试）

```sh
cargo build --workspace
# core 单元 + 集成测试
cargo test --workspace
```

### 启动 GUI

> 前端基于 React + Vite + Ant Design，首次运行需先安装 npm 依赖（仅一次，联网拉取 react/antd/@ant-design/icons/vite）。

```sh
cd frontend && npm install && cd ..
cargo tauri dev
```

打包发布版（产出 `src-tauri/target/release/ruT0-data-kit`，dist 已内嵌）：

```sh
cd frontend && npm install && npm run build && cd ..
cargo build --manifest-path src-tauri/Cargo.toml --release
```

默认规则文件位于 `rules/default_mask.yaml`，可直接引用或拷贝改写；GUI 内可在"自定义规则"下加载本地 YAML。

## 规则编写

规则文件为 YAML，包含 `validators` 与 `maskers` 两条列表。每条规则指定目标 `field`、
脱敏器 / 校验器名，以及可选 `params`（仅 `custom` masker 与 `regex` validator 需要参数）。

参考示例：

- `rules/default_mask.yaml`：v0.1.0 内置脱敏规则，覆盖 `sample_mask.csv` 全部字段。
- `rules/custom_example.yaml`：演示 `custom` masker 的通用参数。

### 内置脱敏器（maskers）

| name | 规则 | 备注 |
| --- | --- | --- |
| `idcard_mask` | 前 6 + 后 4，中间 8 位 `*` | 18 位身份证；短于阈值原样返回 |
| `phone_mask` | 前 3 + 后 4，中间 4 位 `*` | 11 位手机号 |
| `bankcard_mask` | 前 6 + 后 4，中间 `*` | 银行卡号 |
| `email_mask` | 本地首末保留，中间 `*` × (n-2)；本地 1 字原样、2 字 `z*` | 域名不动 |
| `name_mask` | 2 字 `X*`；≥3 字 `X*…*Y` | 中文姓名 |
| `customer_id_mask` | 首字符保留，其余 `*` | |
| `custom` | `keep_prefix` + `keep_suffix` + `mask_char` + `mask_min_len` | 通用可配 |
| `regex_replace` | `pattern` 正则 + `replacement`（支持 `$1`/`$2` 捕获组） | 富规则；非法/缺失 pattern 原值返回 |
| `regex_extract` | `pattern` 正则提取首个匹配子串 | 富规则；无匹配原值返回 |
| `delete` | 对任意输入返回 `""` | 富规则；无参数 |
| `replace` | 对任意输入返回 `with` 字符串 | 富规则；缺 `with` 退化为空串 |

> v0.1.0 起，全部内置脱敏器收敛到 `MaskOp` 抽象算子（4 通用 variant + 6 预置别名）；预置别名清单见 `list_mask_op_types()`（src-tauri 命令）或 `docs/02-技术设计文档.md` §2.3。

### 内置校验器（validators）

| name | 用途 |
| --- | --- |
| `idcard` | 身份证号格式校验 |
| `phone` | 手机号格式校验 |
| `bankcard` | 银行卡号格式校验 |
| `email` | 邮箱格式校验 |
| `mac` | MAC 地址格式校验（v0.2.0 日志扫描用） |
| `username` | 用户名格式校验（v0.2.0 日志扫描用） |
| `name` | 姓名格式校验 |

> v0.1.0 起，全部内置校验器收敛到 `ValidateOp` 抽象算子（3 通用 variant + 7 预置别名）；预置别名清单见 `list_validate_op_types()`（src-tauri 命令）或 `docs/02-技术设计文档.md` §2.4。

v0.1.0 GUI 已通过「数据校验」view 暴露校验入口：选字段 + validator → 运行校验 → 非法单元格高亮 → 可跳转「数据导出」导出合法/非法子集；validator 同时供 v0.2.0 日志扫描 pipeline 复用。

### custom masker 参数示例

```yaml
maskers:
  - field: customer_id
    masker: custom
    params:
      keep_prefix: 2
      keep_suffix: 2
      mask_char: "#"
      mask_min_len: 4
```

应用结果（对 `12345678`）：`12####78`。

## 开发指南

### 仓库布局

```
RuT0DataKit/
├── crates/
│   └── core/          # ruT0-data-kit-core：pipeline / readers / rules / maskers / validators / report
├── src-tauri/         # Tauri v2 后端（GUI 入口）
├── frontend/          # Tauri 前端
├── rules/             # default_mask.yaml / custom_example.yaml
├── tests/             # fixtures（样本文件）
└── docs/              # 需求 / 技术设计 / 任务清单 / 版本标准
```

core 的集成测试位于 `crates/core/tests/e2e.rs`，用 `CARGO_MANIFEST_DIR` 回退两级定位
`tests/fixtures/samples` 与 `rules`。

### 常用命令

```sh
# 构建
cargo build --workspace
# 全量测试
cargo test --workspace
# 仅 E2E 集成测试
cargo test --test e2e
# 启动 GUI
cargo tauri dev
```

## 版本路线

| 版本 | 目标能力 | 当前状态 |
| --- | --- | --- |
| v0.1.0 | CSV / XLSX 脱敏 + 校验 + 导出 + 规则管理（四功能 GUI）+ Tauri GUI | 已发布 v0.1.0 |
| v0.2.0 | 日志文件解析 + SQLi 攻击签名扫描 + 弱口令 / 敏感字段扫描 | 已发布 v0.2.0 |
| v0.2.1 | SQLi payload 语义解析（6 类）+ 字段还原（query/path/UA）+ `+` 解码 | 已发布 v0.2.1 |
| v0.2.2 | 日志扫描盲注二分序列自动聚合还原 flag + GUI 盲注聚合结果卡片 | 开发中（T2-15 收尾中） |
| v0.3.0 | pcap 流量包敏感数据提取（依赖 tshark） | 规划中 |

版本判定标准见 `docs/04-版本标准.md`。

## 安全与隐私

- 全部数据在本地处理，不调用任何远程接口；样本与规则不上传。
- 不引入 Python 运行时依赖；核心与扩展均为 Rust 原生。
- 输出文件路径由用户在 GUI 中显式指定，工具不自行外发。
- pcap 解析（v0.3.0）依赖系统 `tshark`，仍在本机执行。

## 许可证

待定（license 待确认）。
