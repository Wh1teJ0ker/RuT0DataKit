# TASK-BOARD

> 版本：v1.1.4
> 状态：release_ready（文档对齐修复完成 T80-T87 全部 verified_complete + E2E 全绿 + Release QA R1-R4 审计通过；待 push origin main + tag v1.1.4）
> 历史 TASK-BOARD 快照：`handoff/archive/TASK-BOARD-v1.1.0.md` / `TASK-BOARD-v1.1.1.md` / `TASK-BOARD-v1.1.3-20260807.md`

## 仓库分支结构

- `main`：长期主干，包含所有功能分支合入历史（T5-T79），本批文档修复完成后推送 origin + tag v1.1.4
- `release/v0.8.0`：v0.8.0 发布轮基线
- 仅保留上述两个长期分支；不新开分支（用户指令）

## 文档对齐修复批次（T80-T87）

| 任务 | 标题 | 状态 | 交接文件 |
|------|------|------|----------|
| T80 | QA-审计报告.md 追加 R4 审计章节 + 修复顶部状态行 | verified_complete | archive/TASK-T80-*.md |
| T81 | README.md + README_EN.md 更新到 v1.1.4 | verified_complete | archive/TASK-T81-*.md |
| T82 | 04-版本标准.md v1.1.4 里程碑行补全 R3/R4 | verified_complete | archive/TASK-T82-*.md |
| T83 | 更新日志.md 顶部状态行 + 任务表补 T77-T79 | verified_complete | archive/TASK-T83-*.md |
| T84 | 03-开发任务清单.md 追加 v1.1.2-v1.1.4 摘要 | verified_complete | archive/TASK-T84-*.md |
| T85 | 创建 docs/versions/1.1.4/规划需求.md | verified_complete | archive/TASK-T85-*.md |
| T86 | 02-技术设计文档.md 顶部版本覆盖说明追加 R4 | verified_complete | archive/TASK-T86-*.md |
| T87 | TASK-BOARD.md line 10 T5-T76 → T5-T79 | verified_complete | (本文件) |

## E2E 验证结果

- `cargo fmt --all --check`：通过
- `cargo clippy --all-targets --all-features -- -D warnings`：通过
- `cargo test --all`：317 passed / 3 ignored / 0 failed（src-tauri 139 + core 165 + Doc-tests 13）
- `pnpm --prefix frontend build`：通过（3083 modules，2.63s）
- 版本号 4 处一致：1.1.4（Cargo.toml / tauri.conf.json / frontend/package.json / src-tauri Cargo.toml workspace=true）

## v1.1.4 全部任务清单

- **首轮（T67/T68/T69）**：校验统一 — 多规则校验双 Tab 落地
- **续轮 R2（T70/T71/T72）**：通用校验规则 + 地址 / 生日 / 前端参数 UI 统一
- **续轮 R3（T73-T76）**：哈希函数 + DB 文件解析
- **续轮 R4（T77-T79）**：特殊符号白名单 + 手机号前缀 UX 统一 + 名称排序
- **文档对齐修复（T80-T87）**：QA 报告 R4 章节 + README + 版本标准 + 更新日志 + 任务清单 + 规划需求 + 技术设计 + TASK-BOARD

## 安全约束（延续）

- SQL 全部 `?N` + `params![]` 绑定，禁拼接、format、f-string
  - DbReader 表名不能参数绑定 → `quote_identifier`（双引号转义）+ 来源限定为 `sqlite_master` 受信系统表
- 无凭据字面量
- Mimosa 完整审计未拿到结论前不宣称安全（静态分析基线沿用 v1.1.3 R1+R2 双轮 0 findings / 487 包 0 漏洞）
- 不新开分支（用户指令：后续无命令请不要随便新开分支）

## 后续约定

- 仅在收到明确命令后才推送 origin、创建 tag、发布新版本
- 当前批次完成后：push origin main + create tag v1.1.4
