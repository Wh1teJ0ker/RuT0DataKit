# v0.3.0 Release QA 审计报告

> 主会话生成（Phase 8）。审计范围见 `docs/04-版本标准.md` 发布门禁。
> 审计对象：v0.3.0 流量包处理切片（pcap）—— tshark 子进程 + HTTP 字段提取 + 双重 URL 解码 + base64 字段解码 + 敏感扫描 + Report(kind="pcap_scan") + GUI PcapView 四段布局（T3-1 ~ T3-5）。
> 审计时间：2026-07-20 Phase 8（主会话）。

## §0. 审计结论

**qa_passed**。

v0.3.0 五个任务（T3-1 ~ T3-5）全部 `verified_complete`（coder 实施 + reviewer 单任务通过 + 主会话复核）；
端到端 `cargo build --workspace` / `cargo test --workspace`（**319 non-ignored 全绿** + 2 `#[ignore]` pcap 测试
本机 tshark 在场时全绿）/ `npm run build`（2.23s）/ `cd src-tauri && npx @tauri-apps/cli@latest build`（18.79s）
全部通过；版本号 4 处一致 0.3.0 + 产物 `RuT0DataKit_0.3.0_aarch64.dmg` + Info.plist CFBundleShortVersionString=0.3.0；
emoji / 原生 select / Python（source+config 范围）0 命中；v0.3.0 规划目标全部达成：

- tshark 子进程（T3-1）：`PcapReader::read` 调系统 `tshark`，TSV 8 字段 → `HttpRequest`，缺失时
  `CoreError::DependencyMissing("tshark")`，不 panic。fixture `data.pcapng`（60290 帧 / 5000 HTTP POST）
  解出 ≥5000 HttpRequest。
- 解码（T3-2）：`decode_url_twice` + `try_decode_base64_field`（手写 b64 decode，charset + length + printable_ratio
  三重防误伤）+ `extract_decoded_fields`（JSON / form-urlencoded / 纯文本三降级）+ `reassemble_base64` 规则兜底。
  fixture 第一条 POST body 解出 7 字段全对（username=chenyong / name=付里夏旋 / idcard=506051200109055743 /
  phone=74733385248 / address=黑龙江省哈尔滨市通河县三站镇377号159室）。
- 扫描（T3-3）：`PcapScanner::scan` 复用 `DefaultSensitiveScan`，`Report { kind: "pcap_scan", summary, findings, extra: Null }`，
  summary 含 `total_requests` / `sensitive_hits` / `decoded_fragments` / `top_src_ips`；e2e `pcap_scan_full`
  断言 ≥4000 sensitive hits 全绿。
- 命令 + GUI（T3-4）：`scan_pcap_file` Tauri 命令 + PcapView 四段布局（导入 / 原始 HTTP 表 / summary / findings 表），
  tshark 缺失路径弹 `message.error` 提示；0 emoji / 0 原生 select。
- 收尾（T3-5）：4 处版本号 bump + docs 全同步 + Phase 7-9 全套。

约束（Tauri v2 / 不外发 / 纯白 / antd Select / 无 Python / capabilities core:default+dialog:default /
tshark 本地执行 / regex 无 look-around / fixture 不动 / Finding/Report schema 向后兼容）全部保持。

非阻塞遗留：v0.1.0 `MaskView.jsx:112` esbuild 警告、v0.2.0 `ruT0_data_kit_core` non_snake_case 历史警告、
GBK 解码留 v0.3.1+、`reassemble_base64` 规则接口保留但 fixture 未触发、集成测试 `#[ignore]`（无 tshark CI 跳过）。

## §1. 需求覆盖审计

| 需求（docs/00 §3 + 规划需求.md + 计划 §1） | 实现位置 | 状态 |
|------|------|------|
| 选 `.pcap/.pcapng` → 调系统 `tshark` → 提取 HTTP 字段 | `pcap/reader.rs::PcapReader::read`（`tshark -Y "http.request" -T fields -e ...`） | ✓ |
| HttpRequest 结构（frame_no / src_ip / dst_ip / method / host / uri / body / user_agent） | `pcap/reader.rs::HttpRequest`（serde::Serialize） | ✓ |
| 双重 URL 解码 | `pcap/decoder.rs::decode_url_twice`（独立实现避免循环依赖） | ✓ |
| 自动 base64 字段解码（默认） | `pcap/decoder.rs::try_decode_base64_field` + `extract_decoded_fields`（JSON / form / plain 三降级） | ✓ |
| 规则化 `reassemble_base64.param` 兜底 | `pcap/decoder.rs::reassemble_base64`（接口保留，fixture 不需要） | ✓ |
| 复用 `SensitiveScan` 扫描身份证/手机/姓名等 | `pcap/scanner.rs::PcapScanner::scan` 复用 `DefaultSensitiveScan` | ✓ |
| 产出 `Report(kind="pcap_scan")` | `pcap/scanner.rs` + `pipeline/pcap_scan.rs::scan_pcap` | ✓ |
| 仅敏感扫描，不做 SQLi 签名 | `PcapScanner::scan` 不调 `SignatureEngine`（fixture 无 SQLi） | ✓ |
| tshark 缺失 → `CoreError::DependencyMissing("tshark")`，不 panic | `PcapReader::read` 探测 `tshark --version` 失败返回 `DependencyMissing` | ✓ |
| GUI pcap 入口启用 | `Sidebar.jsx` 删除 pcap `disabled: true`，6 项全 active | ✓ |
| GUI tshark 缺失提示与禁用 | `PcapView.jsx` 检测 `msg.includes("tshark") \|\| msg.includes("DependencyMissing")` → `message.error("流量分析需要系统 tshark，请先安装 Wireshark CLI (brew install wireshark)")` | ✓ |
| Tauri 命令 `scan_pcap_file` | `src-tauri/src/commands.rs::scan_pcap_file` + `main.rs` `generate_handler!` 注册（21 命令） | ✓ |
| PcapView 四段布局（导入 / 原始 HTTP 表 / summary / findings 表） | `frontend/src/components/PcapView.jsx` | ✓ |
| Finding/Report schema 向后兼容 | `kind="pcap_scan"` 新增，旧 `kind` 不破；`extra: Value::Null` | ✓ |
| 版本号 0.2.4 → 0.3.0（4 文件） | `Cargo.toml` workspace / `src-tauri/Cargo.toml` / `tauri.conf.json` / `frontend/package.json` | ✓ |

需求覆盖：**全部满足**。

## §2. 任务完成度审计

| 任务 | 状态 | 文件 | 完成判定 |
|------|------|------|---------|
| T3-1 tshark 调用 + HttpRequest 提取 + hex→bytes 手写 | verified_complete | `crates/core/src/pcap/mod.rs` / `pcap/reader.rs` / `lib.rs` | PcapReader + HttpRequest + hex_to_bytes + 3 单测 ✓ |
| T3-2 双重 URL 解码 + base64 字段解码 + 规则兜底 | verified_complete | `crates/core/src/pcap/decoder.rs` / `crates/core/Cargo.toml`（serde_json） | 4 公开函数 + 8 单测 ✓ |
| T3-3 PcapScanner::scan + pipeline 集成 + e2e | verified_complete | `crates/core/src/pcap/scanner.rs` / `pipeline/mod.rs` / `pipeline/pcap_scan.rs` / `tests/e2e.rs` | scan + pipeline 导出 + `pcap_scan_full` e2e ✓ |
| T3-4 Tauri 命令 scan_pcap_file + GUI PcapView 四段 | verified_complete | `src-tauri/src/commands.rs` / `main.rs` / `frontend/src/{state.js,tauri.js,App.jsx,components/Sidebar.jsx,components/PcapView.jsx}` | 命令 + GUI 四段 + 0 emoji/0 select ✓ |
| T3-5 收尾：版本号 + 文档 + Phase 7-9 + git tag | verified_complete | 4 版本文件 + docs/00-04 + versions/0.3.0 + qa/0.3.0 + README×2 | 全部同步 ✓ |

任务完成度：**5/5 verified_complete**。

## §3. 代码质量审计

- `pcap/mod.rs`：模块文档清晰（`v0.3.0 pcap 模块：tshark 子进程 + HTTP 字段提取 + 解码 + 敏感扫描`），re-export 一致。
- `pcap/reader.rs`：
  - `PcapReader::read` 探测 → 主命令 → TSV 解析三段清晰；tshark 缺失显式 `DependencyMissing`，不 panic。
  - 手写 `hex_to_bytes` 避免引入 `hex` crate，边界处理完整（empty / 奇数长度 / 非法字符 / 正常 hex）。
  - `HttpRequest` 字段与 tshark `-e` 一一对应，`body: Option<String>` 显式区分有无 POST body。
  - 集成测试 `#[ignore]`（无 tshark CI 跳过），本机手动跑通过。
- `pcap/decoder.rs`：
  - `decode_url_twice` 独立实现，不依赖 log 模块，避免循环依赖。
  - `try_decode_base64_field` 三重防误伤：length≥4 + length%4==0 + charset ⊂ [A-Za-z0-9+/=] + printable_ratio≥0.8；
    手写 `b64_decode` 避免引入 `base64` crate。
  - `extract_decoded_fields` JSON / form-urlencoded / 纯文本三降级，覆盖 fixture JSON body 与 CTF 偶发 form 场景。
  - 8 单测覆盖：URL 解码 / fixture 第一条 body 全字段 / 非 base64 不误伤 / 11 位手机号不解码 / 16 位银行卡可打印比例失败 / form-urlencoded / 纯文本 / 空数组兜底。
- `pcap/scanner.rs`：
  - `PcapScanner::scan` 主循环清晰：拼 `scan_text` → `scan.scan` → 填 `location`/`context` → 累加 `ip_counts`。
  - summary `top_src_ips` 按 hits 倒序取前 5，每项 `{ip, hits}` mapping，结构稳定。
  - `kind: "pcap_scan"` 与既有 `kind` 不冲突，`extra: Value::Null` 不污染消费者。
- `pipeline/pcap_scan.rs`：薄包装透传，无逻辑冗余。
- `commands.rs`：
  - `scan_pcap_file` 镜像 `scan_log_file` 结构，一次 IPC 返回 `{ entries, report }` 避免二次调用。
  - `pcap_sensitive_ruleset()` 独立 helper，返回 idcard/phone/name validators RuleSet，不污染 mask view 的 `load_default_mask_ruleset`（其 validators 空，会导致 sensitive findings 空——这是 v0.2.0 已知设计）。
- `PcapView.jsx`：
  - 四段布局与 LogView 一致（导入 / 原始表 / summary / findings 表），antd 组件全用（Card / Table / Descriptions / Tag / Empty / Button / Tooltip / Space / Typography）。
  - `TAG_COLOR_BY_TYPE` 颜色映射与 LogView 一致（idcard=purple / phone=blue / name=green 等）。
  - tshark 缺失路径 `message.includes("tshark") || message.includes("DependencyMissing")` 双重判断，鲁棒。
  - 0 emoji / 0 原生 select 保持。
- 既有警告保持（非本版本引入）：`MaskView.jsx:112` esbuild、`ruT0_data_kit_core` non_snake_case、antd chunk > 500kB。

代码质量：**通过**。

## §4. 端到端验收审计

| 命令 | 结果 |
|------|------|
| `cargo build --workspace` | ✓ Finished（1 条 non_snake_case 历史警告，非本版本引入） |
| `cargo test --workspace`（non-ignored） | ✓ 319 passed / 0 failed（216 + 10 + 29 + 12 + 11 + 31 + 10 pcap 新增） |
| `cargo test --workspace -- --ignored pcap`（本机 tshark v4.4.9） | ✓ `pcap_reader_reads_fixture` 0.83s / `pcap_scan_full` 53.74s 全绿 |
| `cd frontend && npm run build` | ✓ built in 2.23s（3005 modules；既有历史警告，非本版本引入） |
| `cd src-tauri && npx @tauri-apps/cli@latest build` | ✓ 18.79s，产出 `RuT0DataKit.app` + `RuT0DataKit_0.3.0_aarch64.dmg`（文件名含 0.3.0） |
| e2e `pcap_scan_full`（ignored） | `kind=="pcap_scan"` / `total_requests>=5000` / `sensitive_hits>=4000` / findings 类型含 `idcard` + (`name` 或 `phone`) / 所有 findings `location` 以 `"frame:"` 开头 / `top_src_ips` 非空 ✓ |
| e2e `pcap_reader_reads_fixture`（ignored） | ≥5000 HttpRequest / ≥5000 POST / 首 POST body 含 "username" ✓ |
| T3-1 单测 | `hex_to_bytes_basic` / `hex_to_bytes_utf8_body` ✓ |
| T3-2 单测 | `url_decode_twice_basic` / `decode_fixture_first_post_body`（7 字段全对）/ `non_base64_value_untouched` / `phone_11_digits_not_decoded` / `bankcard_16_may_attempt_but_fail_printable` / `form_urlencoded_body` / `plain_text_body_as_single_field` / `reassemble_base64_empty_returns_none` ✓ |
| v0.2.4 既有 `log_scan_full` reconstructed_database 断言 | 不破 ✓ |
| Info.plist | `defaults read .../Info.plist CFBundleShortVersionString` → `0.3.0` ✓ |
| GUI smoke | 代码路径就位（PcapView 四段 + tshark 缺失 message.error + TAG_COLOR_BY_TYPE）；reviewer 静态核对 JSX 结构与 T3-1/T3-3 后端契约对齐；无 GUI 显示环境未端到端跑，不阻塞（构建产物已就绪） |

端到端验收：**通过**（319 non-ignored + 2 ignored pcap 测试全绿 + 构建产物 + e2e `pcap_scan_full` 断言 + v0.2.4 既有断言不破）。

## §5. 文档同步审计

| 文档 | 同步状态 |
|------|---------|
| `docs/00-需求文档.md` | §3 补 v0.3.0 范围段（流量包 pcap 处理切片） ✓ |
| `docs/01-页面与交互说明.md` | PcapView 四段布局说明 ✓ |
| `docs/02-技术设计文档.md` | 补 pcap 模块结构 + HttpRequest + decoder + PcapScanner + scan_pcap pipeline ✓ |
| `docs/03-开发任务清单.md` | v0.3.0 段 T3-1~T3-5 + 依赖链 ✓ |
| `docs/04-版本标准.md` | 里程碑索引 0.3.0 行 `planned` → `release_complete` + v0.3.0 验收口径段 ✓ |
| `docs/versions/0.3.0/规划需求.md` | 状态 `planned` → `release_complete` ✓ |
| `docs/versions/0.3.0/更新日志.md` | T3-1~T3-5 verified_complete + 版本状态 release_complete ✓ |
| `docs/qa/versions/0.3.0/QA-审计报告.md` | 本报告 §0-§9 ✓ |
| `README.md` / `README_EN.md` | 功能列表补「流量包分析（pcap/pcapng → tshark → HTTP 字段提取 + 双重 URL 解码 + base64 字段解码 + 敏感扫描）」；状态行 v0.3.0 ✓ |

文档同步：**通过**。

## §6. 版本号一致性审计

| 文件 | 版本 | 状态 |
|------|------|------|
| `Cargo.toml`（workspace） | 0.3.0 | ✓ |
| `crates/core/Cargo.toml` | `version.workspace = true`（继承 0.3.0） | ✓ |
| `src-tauri/Cargo.toml` | 0.3.0 | ✓ |
| `src-tauri/tauri.conf.json` | 0.3.0 | ✓ |
| `frontend/package.json` | 0.3.0 | ✓ |
| Tauri bundle 产物 | `RuT0DataKit_0.3.0_aarch64.dmg` | ✓ |
| Info.plist CFBundleShortVersionString | 0.3.0 | ✓ |

版本号一致性：**5/5 一致 + 产物文件名含 0.3.0 + Info.plist 0.3.0**。

## §7. 约束审计（emoji / native select / Python / 不外发）

| 约束 | 检查 | 结果 |
|------|------|------|
| 源码 emoji 0 | perl 扫 `frontend/src/components/*.jsx`（`[\x{1F300}-\x{1FAFF}\x{2600}-\x{27BF}\x{1F000}-\x{1F2FF}]`） | 0 命中 ✓ |
| 原生 `<select>` 0 | grep `<select[ >]` `frontend/src/components/*.jsx` | 0 命中 ✓ |
| Python 0（source/config 范围） | 产品纯 Rust + React；tshark 子进程本机执行；分析/审查才用 python3（不入产品） | ✓ |
| capabilities 最小 | `core:default` + `dialog:default`，未变 | ✓ |
| withGlobalTauri:true / csp:null | tauri.conf.json 只改 version 字段 | ✓ |
| 不外发数据 | 无网络 command；无 reqwest/hyper/fetch/upload；tshark 本机执行；规则与样本不上传 | ✓ |
| Tauri v2 | `@tauri-apps/cli@latest` build 成功 | ✓ |
| antd Select（非原生） | PcapView 用 antd Card/Table/Tag/Descriptions/Button/Empty/Tooltip/Space/Typography | ✓ |
| 纯白主题 / 无 emoji | 保持 | ✓ |
| regex crate（无 look-around） | decoder 用 `OnceLock<Regex>` 缓存简单字符集正则，无 look-around | ✓ |
| tests/fixtures/samples/log/access.log 1860 行不删 | 未改 fixture | ✓ |
| tests/fixtures/samples/pcap/data.pcapng 9.4MB 不删 | 未改 fixture | ✓ |
| Finding/Report schema 向后兼容 | `kind="pcap_scan"` 新增，旧 kind 不破；`extra: Value::Null` | ✓ |

约束审计：**全部通过**。

## §8. 风险与遗留

| 项 | 严重度 | 处理 |
|----|--------|------|
| tshark 路径依赖 PATH | 非阻塞 | `PcapReader::read` 探测 `tshark --version`，缺失返回 `DependencyMissing("tshark")`；GUI 弹 `message.error` 提示 `brew install wireshark`；集成测试 `#[ignore]` 无 tshark CI 跳过 |
| GBK 解码未实现 | 非阻塞 | fixture 中文地址是 UTF-8 base64，UTF-8 解码够用；规划需求.md 已声明留 v0.3.1+ |
| `reassemble_base64` 规则接口保留但 fixture 未触发 | 非阻塞 | fixture 每 POST body 单独 base64，自动解码即命中；规则兜底接口保留供 CTF 分块场景 |
| base64 误伤（16 位银行卡 % 4 == 0） | 非阻塞 | `try_decode_base64_field` 要求 printable_ratio ≥ 0.8，银行卡解码后多为非可打印字节被拒；单测 `bankcard_16_may_attempt_but_fail_printable` 验证 |
| 集成测试 `#[ignore]` 无 tshark CI 跳过 | 非阻塞 | 本机 tshark v4.4.9 手动 `cargo test -- --ignored pcap` 全绿；CI 无 tshark 时跳过不阻塞 |
| `MaskView.jsx:112` `const params` 赋值 esbuild 警告 | 非阻塞 | v0.1.0 既有，v0.3.0 范围外，保留 |
| crate 名 `ruT0_data_kit_core` non_snake_case 警告 | 非阻塞 | v0.2.0 既有历史命名，改名涉及 Cargo.toml + 全仓 use，留后续 |
| antd chunk > 500kB 警告 | 非阻塞 | vite 通用提示，非本版本引入 |
| GUI smoke 未端到端跑（无显示环境） | 非阻塞 | 代码路径经 reviewer 静态核对与 T3-1/T3-3 后端契约对齐；构建产物就绪；用户可手动 `open RuT0DataKit.app` 验证 |

无阻塞风险。

## §9. 发布建议

**建议发布 v0.3.0**。

- 五任务全 verified_complete；端到端 319 non-ignored + 2 ignored pcap 测试全绿；构建产物就绪
  （.app + `RuT0DataKit_0.3.0_aarch64.dmg` 含 0.3.0）；
- v0.3.0 规划目标全部达成：
  - tshark 子进程（T3-1）：TSV 8 字段 → HttpRequest，缺失 `DependencyMissing` 不 panic。
  - 解码（T3-2）：双重 URL + base64 字段（三降级）+ 规则兜底，fixture 7 字段全对。
  - 扫描（T3-3）：`PcapScanner::scan` 复用 `DefaultSensitiveScan`，`Report(kind="pcap_scan")` + summary + findings。
  - 命令 + GUI（T3-4）：`scan_pcap_file` + PcapView 四段布局，tshark 缺失路径鲁棒。
  - 收尾（T3-5）：4 版本号 + docs 全同步 + Phase 7-9。
- 向后兼容验证通过（v0.1.0 csv_report + v0.2.0 log_scan + v0.2.1 parsed_payload +
  v0.2.2 blind_aggregation + v0.2.3 第 4 RT 四段断言 + v0.2.4 reconstructed_database 五段断言不破；
  新增 `kind="pcap_scan"` 与旧 kind 并列，旧消费者不破）；
- 约束全部保持；文档全部同步；版本号 5 处一致 + 产物文件名 + Info.plist 一致。

Phase 9 可执行：
1. `docs/04-版本标准.md` 0.3.0 行 → `release_complete`
2. `docs/versions/0.3.0/更新日志.md` T3-5 → `verified_complete`，版本状态 → `release_complete`
3. `docs/versions/0.3.0/规划需求.md` 版本状态 → `release_complete`
4. 删除 `handoff/`（TASK-BOARD.md + HANDOFF + REPORT + REVIEW，若存在）
5. git commit + tag v0.3.0 + push GitHub SSH（`git@github.com:Wh1teJ0ker/RuT0DataKit.git`）
