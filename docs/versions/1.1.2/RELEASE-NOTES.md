# RuT0DataKit v1.1.2

> Git tag：`v1.1.2`（已推送，CI/CD 构建中）
> 状态：已发布（T37~T47 verified_complete；Release QA R5 完成；tag `v1.1.2` 已推送触发 tauri-action 三目标矩阵构建）
> 前置：v1.1.1 已 `qa_passed` 并发布 tag `v1.1.1`

## 这是什么

RuT0DataKit v1.1.2 在 v1.1.1（撤销/重做 + 列操作 + 搜索）之上，交付**列编解码（Base64，独立加解密面板）+ .log 文件导入（格式自动识别 + 结构化多列）+ 设置界面优化（全局每页行数可配置 + 文案精简 + 入口迁移）+ tshark 测试加固**四项增量，属 v1.1.x 系列的维护性增强版本。数据全程在本地处理，不外发；schema 不变（`SCHEMA_VERSION=3`），从 v1.1.1 升级无需迁移。

## 新增

- **列编解码 — Base64 编码/解码**：工具栏新增「加解密」能力按钮（`KeyOutlined` 图标），点击在左侧展开 `CryptoPanel`，作为所有加解密/哈希/编解码类操作的统一入口。当前承载 Base64 编解码：选择目标列与模式（编码 / 解码）后，对整列做 Base64 变换并就地写回。`encode` 用标准 Base64（`STANDARD` engine）把每行文本编码后写回；`decode` 把 Base64 串解码为原文写回，解码失败（非法串或非 UTF-8 字节）的行计入 `skipped` 不中断整体操作，空值行不参与变换。编码后再解码可恢复原文（往返一致）。该操作为就地变更，纳入撤销栈（`operations.kind="base64_column"`，before/after 快照仅含变更列 cells），可用撤销/重做恢复
- **.log 文件导入（格式自动识别 + 结构化多列）**：导入对话框新增 `.log` 文件类型过滤。`LogReader`（`crates/core/src/datasource/log.rs`）对文件前 10 条非空行做 probe，按正则命中数自动识别最匹配的日志格式，解析为结构化多列 + `raw_line` 列：
  - **Apache Combined Log Format** — 11 结构化列（`remote_host, ident, remote_user, time_local, request_method, request_url, request_protocol, status, body_bytes_sent, http_referer, http_user_agent`）+ `raw_line`；`request` 字段二次拆分为 method/url/protocol；`body_bytes_sent` 为 `-` 时归零
  - **Apache Common Log Format** — 9 结构化列（无 referer/user-agent）+ `raw_line`
  - **Syslog（RFC 3164）** — `timestamp, host, process, pid(opt), message` + `raw_line`
  - **通用应用日志** — `timestamp, level, thread(opt), message` + `raw_line`
  - **未识别格式** — 回退单列 `line`（与 v1.1.2 之前行为一致）

  正则经 `std::sync::OnceLock` 缓存，`raw_line` 始终保留原始全文；解析失败的行各结构化字段留空、`raw_line` 保留原文；空行保留为空串（不丢弃）。`detect_format` 工厂按扩展名 `.log` 分发到 `LogReader`，与既有 CSV/XLSX/JSON/JSONL/TXT/SQL/PCAP 七种格式并列
- **全局每页行数设置（PAGE_SIZE 持久化）**：设置页新增「每页行数」卡片，可选 20 / 50 / 100 / 200（默认 50）。保存后所有 Sheet 的分页、翻页、搜索翻页立即按新值生效（当前 Sheet 首页刷新，页码重置为 1），新建/导入的 Sheet 自动继承全局值。设置持久化到 `settings.json`（与 tshark 路径同文件），重启后保留；旧版 `settings.json`（无此字段）升级后自动回退 50，不报错
- **tshark 解析测试加固**：`crates/core/src/pcap/detect.rs` 新增 `candidate_paths_windows_paths_are_absolute` 测试（断言 Windows 候选路径以 `C:\` 开头且以 `tshark.exe` 结尾）+ `probe_tshark_nonexistent_returns_none` 测试（不存在路径返回 None 不 panic）；`reader.rs` 既有 tshark 输出解析测试保留。`candidate_paths()` 显式覆盖 macOS homebrew（apple silicon + intel）+ Wireshark.app bundle + Linux `/usr/bin` + Windows `Program Files` / `Program Files (x86)` 三平台七条候选路径

## 优化

- **设置按钮迁移到右上角**：设置入口从 AiPanel 底部迁移到 `TopToolbar` 右端（`<SettingOutlined />` 图标按钮，`flex: 1` 占位推到最右）。点击进入设置页（`currentView="settings"`），与原 AiPanel 入口行为一致；AiPanel 不再承载设置入口，职责更聚焦
- **设置界面文案精简**：移除 `DbPathCard` / `AboutCard` / `TsharkPathCard` 三张设置卡片中的冗余描述文案，保留核心字段与操作。DbPathCard 仍保持硬编码 DB 路径展示（动态 DB 路径命令推迟 v1.2+）；TsharkPathCard 的 tshark 路径覆盖与自动探测逻辑不变
- **加解密面板独立化**：Base64 编解码 UI 从「列操作」面板迁出，独立为工具栏「加解密」能力按钮（`crypto`，`KeyOutlined` 图标）+ 左侧 `CryptoPanel`。`CryptoPanel` 作为后续所有加解密/哈希/编解码类操作的统一入口（v1.1.2 先承载 Base64，预留扩展位）；`ColumnOpsPanel` 仅保留 JSON 解析。纯前端 UI 重组，后端 IPC/DB 方法不变

## 下载

> v1.1.2 Release 由 git tag `v1.1.2` 触发 `release.yml` 工作流，tauri-action 三目标矩阵构建（linux-x86_64 / macos-aarch64 / windows-x86_64），产物自动上传至 GitHub Release。

| 平台 | 安装包 | 校验 |
|---|---|---|
| macOS (Apple Silicon) | `RuT0DataKit_1.1.2_aarch64.dmg` / `.app.tar.gz` | 签名校验（updater 公钥） |
| Windows (x64) | `RuT0DataKit_1.1.2_x64-setup.exe` / `.msi.zip` | 签名校验 |
| Linux (x64) | `RuT0DataKit_1.1.2_amd64.AppImage` / `.deb` | 签名校验 |

> macOS Intel (x86_64) 不在本版构建矩阵中（v1.1.2 仅构建 aarch64-apple-darwin）。Intel Mac 用户可通过 Rosetta 运行 ARM 版本，或等待后续版本补齐 x86_64-apple-darwin 目标。

## 验证

- `cargo fmt --check`：通过
- `cargo clippy --workspace -- -D warnings`：通过
- `cargo test --workspace`：126 passed / 0 failed / 2 ignored（src-tauri lib 77 + core 49；2 ignored 为本机 tshark 探测/fixture 读取，CI 无 tshark 时跳过）
- `pnpm --prefix frontend install --frozen-lockfile`：通过（Lockfile is up to date）
- `pnpm --prefix frontend build`：通过（3081 modules transformed，2.58s）

## 已知限制

- Base64 列编解码仅支持标准 Base64（`STANDARD` engine），不支持 URL-safe / no-padding 等变体；解码失败的行跳过并计入 `skipped`，不报错中断
- `.log` 文件结构化解析覆盖四种主流格式（Apache Combined/Common、Syslog、通用应用日志），非主流格式回退单列 `line`；正则匹配基于行级 probe，不处理跨行多行日志（推迟 v1.2+）
- tshark 集成仍仅覆盖 HTTP 协议字段提取（DNS/TCP 等推迟 v1.3+）；本机无 tshark 时相关测试 `#[ignore]` 跳过，不阻塞 CI
- 设置按钮迁移后 AiPanel 不再承载设置入口；如用户依赖旧入口位置，需适应右上角新位置
- DbPathCard 仍为硬编码 DB 路径展示，动态 DB 路径命令推迟 v1.2+
- Mimosa 安全扫描已重跑完整审计（scan-2026-08-08T19-56-00.366Z-a958f772712b，deep 深度）：0 findings、487 个依赖包 0 漏洞、68/68 源文件全量解析成功；覆盖度 `partial`（调用图部分不完整，为动态派发方法学限制，非项目缺陷），`runStatus=inconclusive`。静态分析非运行时验证，**不宣称项目安全**，但无任何已识别 finding 阻碍发布
- commit/push 时 Mimosa 未得到完整扫描结论（library_source/library_source_unavailable、callgraph/callgraph_fact_partial），按兼容策略继续；建议后续重新运行完整审计

## 升级

v1.1.1 用户可直接升级：DB schema 不变（`SCHEMA_VERSION=3`，无新增表/列/索引），无需迁移，历史数据完整保留。`settings.json` 向后兼容（新增 `page_size` 字段用 `#[serde(default)]`，旧文件反序列化时自动取 `None` → 前端回退 50，无需手动处理）。启动后即可使用 Base64 列编解码、.log 结构化导入（格式自动识别 + 多列 + raw_line）、全局每页行数设置与优化后的设置界面；既有撤销/重做、列操作、搜索功能无回归。

---

完整更新日志：[`docs/versions/1.1.2/更新日志.md`](更新日志.md)
QA 审计报告：[`docs/qa/versions/1.1.2/QA-审计报告.md`](../../qa/versions/1.1.2/QA-审计报告.md)
