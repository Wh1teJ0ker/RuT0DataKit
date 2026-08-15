# RuT0DataKit v1.1.2

## Update

- **列编解码 — Base64 编码/解码**：工具栏新增「加解密」能力按钮（`KeyOutlined` 图标），左侧展开 `CryptoPanel`，作为所有加解密/哈希/编解码类操作的统一入口。当前承载 Base64 编解码：选择目标列与模式（编码 / 解码）后对整列变换并就地写回。`encode` 用标准 Base64（`STANDARD` engine）；`decode` 解码失败（非法串或非 UTF-8 字节）的行计入 `skipped` 不中断整体操作，空值行不参与变换。编码后再解码可恢复原文（往返一致）。该操作为就地变更，纳入撤销栈（`operations.kind="base64_column"`，before/after 快照仅含变更列 cells），可用撤销/重做恢复
- **.log 文件导入（格式自动识别 + 结构化多列）**：导入对话框新增 `.log` 文件类型过滤。`LogReader` 对文件前 10 条非空行做 probe，按正则命中数自动识别最匹配的日志格式，解析为结构化多列 + `raw_line` 列：Apache Combined Log Format（11 列）/ Apache Common Log Format（9 列）/ Syslog RFC 3164 / 通用应用日志 / 未识别格式回退单列 `line`。正则经 `std::sync::OnceLock` 缓存，`raw_line` 始终保留原始全文；空行保留为空串（不丢弃）
- **全局每页行数设置（PAGE_SIZE 持久化）**：设置页新增「每页行数」卡片，可选 20 / 50 / 100 / 200（默认 50）。保存后所有 Sheet 的分页、翻页、搜索翻页立即按新值生效（当前 Sheet 首页刷新，页码重置为 1），新建/导入的 Sheet 自动继承全局值。设置持久化到 `settings.json`，重启后保留；旧版 `settings.json`（无此字段）升级后自动回退 50，不报错
- **tshark 解析测试加固**：`pcap/detect.rs` 新增 Windows 候选路径绝对路径断言 + 不存在路径返回 None 不 panic 测试；`candidate_paths()` 显式覆盖 macOS homebrew（apple silicon + intel）+ Wireshark.app bundle + Linux `/usr/bin` + Windows `Program Files` 三平台七条候选路径
- **设置按钮迁移到右上角**：设置入口从 AiPanel 底部迁移到 `TopToolbar` 右端（`<SettingOutlined />` 图标按钮，`flex: 1` 占位推到最右）。AiPanel 不再承载设置入口，职责更聚焦
- **设置界面文案精简**：移除 `DbPathCard` / `AboutCard` / `TsharkPathCard` 三张设置卡片中的冗余描述文案，保留核心字段与操作
- **加解密面板独立化**：Base64 编解码 UI 从「列操作」面板迁出，独立为工具栏「加解密」能力按钮（`crypto`，`KeyOutlined` 图标）+ 左侧 `CryptoPanel`，作为后续所有加解密/哈希/编解码类操作的统一入口；`ColumnOpsPanel` 仅保留 JSON 解析。纯前端 UI 重组，后端 IPC/DB 方法不变
- 版本号 1.1.1 → 1.1.2

## Fix

- 修复 `DataTable` 搜索翻页时 `PAGE_SIZE` 使用不一致（L241/269 硬编码常量 → 改用 `sheet.pageSize || PAGE_SIZE`，与翻页/分页路径对齐）
- 修正 `release.yml` releaseBody 中 RELEASE-NOTES 路径 `1.1.1` → `1.1.2`

## 下载

| 平台 | 文件 |
|---|---|
| Windows x64 | RuT0DataKit_1.1.2_x64-setup.exe / RuT0DataKit_1.1.2_x64_en-US.msi |
| macOS arm64 | RuT0DataKit_1.1.2_aarch64.dmg / RuT0DataKit_aarch64.app.tar.gz |
| Linux x64 | RuT0DataKit_1.1.2_amd64.deb / RuT0DataKit-1.1.2-1.x86_64.rpm / RuT0DataKit_1.1.2_amd64.AppImage |

每个安装包均附带 .sig 签名文件，供 updater 验签。macOS Intel (x86_64) 不在本版构建矩阵中（仅 aarch64-apple-darwin），Intel Mac 用户可通过 Rosetta 运行 ARM 版本。

## 升级

v1.1.1 用户可直接升级：DB schema 不变（`SCHEMA_VERSION=3`，无新增表/列/索引），无需迁移，历史数据完整保留。`settings.json` 向后兼容（新增 `page_size` 字段用 `#[serde(default)]`，旧文件反序列化时自动取 `None` → 前端回退 50，无需手动处理）。启动后即可使用 Base64 列编解码、.log 结构化导入、全局每页行数设置与优化后的设置界面；既有撤销/重做、列操作、搜索功能无回归。

完整技术文档见 docs/versions/1.1.2/。
