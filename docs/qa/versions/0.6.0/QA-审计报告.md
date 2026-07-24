# v0.6.0 Release QA 审计报告

> 新功能版本发布前全局 QA 审计。依据 `orchestrator-workflow` 技能「发布前 QA 门禁」要求，覆盖需求覆盖、端到端流程、构建与测试、代码质量、文档一致性五维度。
>
> 本版本范围：Tools 下新增加密/解密器（AES-CBC / Base64 / Hex 单值 + 指定列批量，纯本地），并附带代码重整（大杂烩 commit 拆分、死代码清理、PDF 绝对化措辞中性化）。

## §0 审计结论

`qa_passed` — 4 项功能任务（T13-1~T13-4）+ 3 项重整任务（T15-1~T15-3）全部落地，加密/解密核心模块 + Tauri 命令 + 前端 EncryptTool 接线齐备，cargo test 全绿（491 passed），3 构建目标 0 error，死代码 warning 清零，5 处 manifest 版本号同步 0.6.0，docs 一致，无阻塞项。

## §1 任务对齐

| # | 任务 | 优先级 | 落地证据 | 状态 |
|---|------|--------|----------|------|
| 1 | T13-1 core tools/encrypt.rs 加密模块 | P0 | `crates/core/src/tools/encrypt.rs` 落地：AES-CBC + Base64 + Hex + `parse_algo`/`parse_key` + 单测 | ✅ |
| 2 | T13-2 Tauri encrypt 命令 | P0 | `src-tauri/src/commands/encrypt.rs` 4 命令 + `commands/mod.rs` + `main.rs` invoke_handler 注册 | ✅ |
| 3 | T13-3/T15-2 前端 EncryptTool 接线 | P0 | `EncryptTool.jsx`（NEW）+ `tauri.js` 4 wrapper + `state.js` 7 ACTION/case/field + `Sidebar.jsx` tools.encrypt 子项 + `ToolsView.jsx` encrypt 分支；npm build 绿 | ✅ |
| 4 | T13-4 docs 骨架 | P1 | `docs/versions/0.6.0/{规划需求,更新日志}.md` + `04-版本标准.md` 里程碑行 | ✅ |
| 5 | T15-1 死代码 + PDF 措辞清理 | P1 | `extract_col_name_from_def` 删除（warning 3→2）；`grep PDF crates/ frontend/src/` 0 命中 | ✅ |
| 6 | T15-2 补齐 EncryptTool 接线 | P0 | reviewer pass（acceptance 9 条全 pass） | ✅ |
| 7 | T15-3 版本号 + docs 同步 | P1 | 5 manifest 0.6.0 + 更新日志回填 + QA 报告 | ✅ |

## §2 代码审计

### §2.1 核心加密模块（`crates/core/src/tools/encrypt.rs`）

| 检查项 | 结果 |
|--------|------|
| AES-CBC PKCS7 padding 正确性 | `aes` crate `cbc::Encryptor`/`Decryptor`，固定 IV `0x00..0x10`，单测覆盖 16/24/32 字节密钥 ✅ |
| Base64 互逆 | `base64::engine::general_purpose::STANDARD`，加解密互逆单测 ✅ |
| Hex 互逆 | `hex::encode`/`decode`，互逆单测 ✅ |
| 非法 algo / 密钥长度错误路径 | `parse_algo` 返回 `Err`，`parse_key` 校验 16/24/32，单测覆盖 ✅ |
| 无网络调用 | `grep -rn 'reqwest\|http\|net' crates/core/src/tools/` 0 命中 ✅ |

### §2.2 Tauri 命令层（`src-tauri/src/commands/encrypt.rs`）

| 检查项 | 结果 |
|--------|------|
| 4 命令签名 | `encrypt_text`/`decrypt_text`/`encrypt_columns`/`decrypt_columns` 对齐前端 invoke 名 ✅ |
| 参数 camelCase | 前端 `tauri.js` wrapper 参数名与命令参数名逐字对齐 ✅ |
| 批量未选列原样透传 | `selectedColumns` 为列名集合，未命中列 rows 原样保留 ✅ |
| 错误统一 String 化 | `Result<_, String>`，前端 Alert 展示 ✅ |
| main.rs 注册 | `invoke_handler` 含 4 encrypt 命令 ✅ |

### §2.3 前端 EncryptTool（`EncryptTool.jsx` + 接线）

| 检查项 | 结果 |
|--------|------|
| 三段结构 | 配置（算法 Select + 模式 Radio + Key Input.Password 仅 AES）+ 单值试运行 + 批量列预览 ✅ |
| AES Key 空校验 | `ensureKey()` → `message.warning`，单值 + 批量两路径前置阻断 ✅ |
| 批量预览前 50 行 | `ENCRYPT_PREVIEW_ROW_LIMIT=50`，`scroll.y=360` ✅ |
| state.js SET_RECORDS 级联未破坏 | 新增 case 全在 toolsDomain，fileDomain `RECORDS_SET` 级联清空逻辑保留 ✅ |
| Sidebar/ToolsView 接线 | `tools.encrypt` 子项 + handleClick 分支 + title 三元 + 渲染分支 ✅ |
| npm build | vite build 3008 modules，0 error，仅 chunk size 警告（既有，非本版本引入） ✅ |

### §2.4 死代码与措辞清理（T15-1）

| 检查项 | 结果 |
|--------|------|
| `extract_col_name_from_def` 死代码删除 | `cargo build` 无 `never used` 对 sql_reader 命中 ✅ |
| warning 数下降 | 3 → 2（仅剩 crate 名非 snake_case 历史遗留，out of scope） ✅ |
| 代码层 PDF 措辞中性化 | `grep "PDF" crates/ frontend/src/` 0 命中；`grep "个人信息数据规范文档" crates/ frontend/src/` 0 命中 ✅ |

### §2.5 commit 重整

- 原 `fa7e8c6` 大杂烩 commit（pinfo_phone + sql_reader + MaskColumnMapper/RulesView + encrypt 命令 + Cargo.lock/README 混合）已通过 `git reset --soft` + 选择性 `git add` 拆为 6 个职责单一的 Conventional Commits：
  - `c43235e` chore(handoff): TASK-BOARD 重写
  - `a2e9f09` feat(rules): pinfo_phone 参数化校验器（T9-8）
  - `5b1af51` feat(readers): sql_reader 结构化 dump 还原（T9-9）
  - `cd41b18` refactor(gui): MaskColumnMapper + RulesView Table + state 级联（T12-7）
  - `17ae025` feat(commands): encrypt/decrypt tauri 命令（T13-2）
  - `74dc849` chore: Cargo.lock + README 同步
- 后续追加 T15-1（`52adcff`）、T15-2（`97bfbac`）、T15-3 版本号 + docs 提交。
- 历史线性、可读，每个 commit 单一职责。

## §3 测试审计

```
$ cargo test --workspace
test result: ok. 393 passed; 0 failed; 5 ignored   (lib unittests)
test result: ok. 10 passed; 0 failed; 0 ignored     (integration log_test)
test result: ok. 34 passed; 0 failed; 2 ignored     (integration log_scan_test)
test result: ok. 12 passed; 0 failed; 0 ignored     (integration blind_aggregator_test)
test result: ok. 11 passed; 0 failed; 0 ignored     (integration logsign_test)
test result: ok. 31 passed; 0 failed; 0 ignored     (integration e2e)
```

合计 **491 passed, 0 failed, 7 ignored**，全绿。相比 v0.5.0 基线 450 → 491（+41，含 encrypt 模块单测 + pinfo_phone 参数化 + sql_reader 结构化 dump + RulesView/state 重整相关测试）。

## §4 构建审计

| 构建目标 | 命令 | 结果 |
|----------|------|------|
| core lib | `cargo build -p ruT0-data-kit-core` | 0 error，2 warning（历史遗留 crate 名） |
| src-tauri binary | `cargo build -p ruT0-data-kit` | 0 error |
| frontend | `npm --prefix frontend run build` | vite build 3008 modules，0 error |

## §5 文档一致性

| 检查项 | 结果 |
|--------|------|
| 5 处 manifest 版本号 | 全部 0.6.0 ✅ |
| `docs/04-版本标准.md` 里程碑行 | v0.6.0 状态 `release_complete` ✅ |
| `docs/versions/0.6.0/更新日志.md` | 回填完毕，状态 `release_complete` ✅ |
| `docs/versions/0.6.0/规划需求.md` | 存在 ✅ |
| QA 报告 | 本文件 ✅ |
| 安全约束 §6 | 「不外发数据：全本地处理」保留，v0.6.0 不变 ✅ |

## §6 安全审计

- 加密/解密全本地：`grep -rn 'reqwest\|http::\|net::' crates/core/src/tools/ src-tauri/src/commands/encrypt.rs` 0 命中。
- AES-CBC 固定 IV：本工具定位为本地数据脱敏辅助工具，非生产级密文传输；IV 固定便于单值试运行可复现，密钥不落盘、不记录、不上传。
- 密钥前端 `Input.Password` 输入，不持久化到 state 之外（切 view 不重置但刷新即失）。

## §7 阻塞项

无。

## §8 结论

`qa_passed` — v0.6.0 全部任务落地，测试 / 构建 / 文档 / 安全五维度通过，可标记 `release_complete`。
