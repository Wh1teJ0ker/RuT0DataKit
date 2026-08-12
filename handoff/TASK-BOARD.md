# TASK-BOARD

> 版本：v1.1.4
> 状态：branch_cleanup（分支整合完成；所有历史交接文件已归档至 `handoff/archive/`；仅保留 2 个长期分支：`main` + `release/v0.8.0`）
> 状态：待用户手工验证（不 push、不 tag）
> 历史 TASK-BOARD 快照：`handoff/archive/TASK-BOARD-v1.1.0.md` / `TASK-BOARD-v1.1.1.md` / `TASK-BOARD-v1.1.3-20260807.md`

## 仓库分支结构（整合后）

- `main`：长期主干，包含所有功能分支合入历史（T5-T76），等待用户手工验证后推送 origin
- `release/v0.8.0`：T3 v0.8.0 发布轮基线
- 仅保留上述两个长期分支；`origin` 远端同步 origin/main, origin/release/v0.8.0

## 分支整合历史（按时间倒序）

| 任务批次 | 描述 | 交接文件位置 | 状态 |
|---------|------|-------------|------|
| v1.1.4 | 校验统一 + 加密 + DB 文件解析 + 手机前缀 + 名称排序 | archive/ | 完成 |
| T73 | 哈希函数加密（MD5/SHA1/SHA256）| archive/TASK-T73-*.md | verified_complete |
| T74 | CryptoPanel 哈希 UI + IPC wrapper | archive/TASK-T74-*.md | verified_complete |
| T75 | DbReader 数据源（.db/.sqlite/.sqlite3）| archive/TASK-T75-*.md | verified_complete |
| T76 | 文档 + E2E + QA R3 | archive/TASK-T76-*.md | verified_complete |
| T77 | generic-validate 特殊符号白名单 | (handoff 整理) | verified_complete |
| T78 | 手机号前缀配置 UX 统一 | (handoff 整理) | verified_complete |
| T79 | 规则列表按 name Unicode 码点排序 | (handoff 整理) | verified_complete |

## v1.1.4 全部任务清单

- **首轮（T67/T68/T69）**：校验统一 — 多规则校验双 Tab 落地
- **续轮 R2（T70/T71/T72）**：通用校验规则 + 地址 / 生日 / 前端参数 UI 统一
- **续轮 R3（T73-T76）**：哈希函数 + DB 文件解析
- **续轮 R4（T77-T79）**：特殊符号白名单 + 手机号前缀 UX 统一 + 名称排序

## 安全约束（延续）

- SQL 全部 `?N` + `params![]` 绑定，禁拼接、format、f-string
  - DbReader 表名不能参数绑定 → `quote_identifier`（双引号转义）+ 来源限定为 `sqlite_master` 受信系统表
- 无凭据字面量
- Mimosa 完整审计未拿到结论前不宣称安全
- 不创建 tag（用户等待手工验证，不自动 finalize）
- 不 push（用户等待手工验证，不自动 finalize）

## 后续约定

- 无命令不要随便新开分支（用户指令）
- 仅在收到明确命令后才推送 origin、创建 tag、发布新版本
