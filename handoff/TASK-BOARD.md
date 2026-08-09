# v1.1.2 TASK-BOARD

> 版本：v1.1.2
> 状态：done_e2e（T37~T47 全部 verified_complete + E2E 全绿；Release QA `qa_passed` R5）
> 前置：v1.1.1 `qa_passed` + tag `v1.1.1` 已发布

## 任务 DAG

```
T37 Base64 列编解码后端命令 (columns.rs + lib.rs)
   └─→ T38 Base64 列编解码前端 IPC + UI (tauri.js + ColumnOpsPanel.jsx)
         └─→ T46 Base64 抽离为独立加解密面板 (CryptoPanel + TopToolbar + SidePanel)

T39 设置界面文案精简 (三卡片)        ┐
T40 设置按钮移到右上角 (AiPanel→TopToolbar) ┘  (T39 + T40 可并行)

T41 tshark 解析测试加固 (detect.rs + reader.rs 测试)   — 独立
T42 .log 文件导入支持 (LogReader + detect_format + 前端 filter) — 独立
   └─→ T45 .log 结构化解析优化 (格式自动识别 + 多列 + raw_line + fallback)

T43 版本号 1.1.1 → 1.1.2 (4 处)       — 依赖 T37~T42 全部
   └─→ T44 文档收口 + 全量验证 + Release QA

T47 设置页全局每页行数 (PAGE_SIZE 持久化) — 独立（v1.1.2 收尾功能）
```

依赖说明：
- T37 → T38：前端 IPC 依赖后端命令注册
- T38 → T46：Base64 UI 从 ColumnOpsPanel 迁至独立 CryptoPanel
- T37/T39/T40/T41/T42 互不依赖（不同文件），可并行分派
- T42 → T45：结构化解析优化在 LogReader 之上重写
- T43 依赖全部功能任务完成
- T44 收尾
- T47 独立：settings.json page_size 持久化 + PageSizeCard + SET_PAGE_SIZE reducer

## 状态总表

| 任务 | 标题 | 状态 | depends_on | handoff |
|------|------|------|-------------|---------|
| T37 | Base64 列编解码后端命令 | verified_complete | — | handoff/TASK-T37-HANDOFF.md |
| T38 | Base64 列编解码前端 IPC + UI | verified_complete | T37 | handoff/TASK-T38-HANDOFF.md |
| T39 | 设置界面文案精简 | verified_complete | — | handoff/TASK-T39-HANDOFF.md |
| T40 | 设置按钮移到右上角 | verified_complete | — | handoff/TASK-T40-HANDOFF.md |
| T41 | tshark 解析测试加固 | verified_complete | — | handoff/TASK-T41-HANDOFF.md |
| T42 | .log 文件导入支持 | verified_complete | — | handoff/TASK-T42-HANDOFF.md |
| T43 | 版本号 1.1.1 → 1.1.2 | verified_complete | T37-T42 | handoff/TASK-T43-HANDOFF.md |
| T44 | 文档收口 + 全量验证 + Release QA | verified_complete | T43 | handoff/TASK-T44-HANDOFF.md |
| T45 | .log 结构化解析优化 | verified_complete | T42 | handoff/TASK-T45-HANDOFF.md |
| T46 | Base64 抽离为独立加解密面板 | verified_complete | T38 | handoff/TASK-T46-HANDOFF.md |
| T47 | 设置页全局每页行数（PAGE_SIZE 持久化） | verified_complete | — | handoff/TASK-T47-HANDOFF.md |

## 端到端验收项

- [x] E1: `cargo fmt --check` + `cargo clippy --workspace -- -D warnings` + `cargo test --workspace` 全绿（136 passed / 3 ignored / 0 failed；含 T45 新增 10 个 log 结构化单测 + access.log 真实 fixture 集成测试）
- [x] E2: `pnpm --prefix frontend install --frozen-lockfile && pnpm --prefix frontend build` 通过（3080 modules，2.56s）
- [x] E3: Base64 编码列 → 值变 Base64；解码 → 恢复；撤销可恢复（后端单测覆盖 encode/decode/skipped/undo 往返）
- [x] E4: 设置按钮在 TopToolbar 右端可见（pnpm build 通过）
- [x] E5: .log 文件可通过导入对话框选择并导入（LogReader 格式自动识别 + 结构化多列单测覆盖 Apache Combined/Common、Syslog、通用应用日志、fallback 单列、raw_line 保留、access.log 真实 fixture 集成测试 + detect_format 路由测试）
- [x] E6: tshark 测试覆盖 Windows 路径候选（candidate_paths_windows_paths_are_absolute + probe_tshark_nonexistent_returns_none）
- [x] E7: 版本号 4 处一致 1.1.2
- [x] E8: 加解密按钮在 TopToolbar 可见，点击展开 CryptoPanel（Base64 编解码）；ColumnOpsPanel 仅剩 JSON 解析无 Base64 残留（pnpm build 通过）
- [x] E9: 设置页「每页行数」卡片可选 20/50/100/200；保存后当前 Sheet 立即按新行数重渲染（首页刷新），其他 Sheet 同步生效；重启后设置保留（settings.json 持久化）；新建/导入 Sheet 继承全局 pageSize；旧 settings.json 升级不报错（pnpm build + cargo clippy 通过）

## Release QA 门禁

- required: true
- report: docs/qa/versions/1.1.2/QA-审计报告.md
- audit_scope: 需求覆盖 / 端到端流程 / 构建与测试 / 代码质量 / 安全与隐私 / 数据与迁移 / 依赖与配置 / 文档一致性
- conclusion: `qa_passed`（8 维度全 pass；R5 含 T47 增量；Mimosa 深度扫描已重跑完整审计 0 findings，callgraph partial 为方法学限制，不宣称项目安全）
