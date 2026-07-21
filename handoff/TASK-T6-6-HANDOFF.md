```yaml
task_id: T6-6
goal: |
  收尾：版本号 0.4.0 → 0.4.1（4 处）+ docs 同步 + Phase 7 端到端 + Phase 8
  Release QA + Phase 9 release_complete + git tag v0.4.1 + push GitHub SSH。

in_scope:
  - Cargo.toml                                       # workspace.package.version
  - src-tauri/Cargo.toml                             # version
  - src-tauri/tauri.conf.json                        # version
  - frontend/package.json                            # version
  - docs/00-需求文档.md                              # §6 安全约束保留；v0.4.1 段补 5 项缺陷修复
  - docs/01-页面与交互说明.md                         # FileToolbar 移除 / ToolsView Select / 预处理自动跳转 / RegexTool 构造模式
  - docs/02-技术设计文档.md                          # 新增 detect_sql_blind_features 命令 + construct_regex 模块
  - docs/03-开发任务清单.md                          # 新增 v0.4.1 段 T6-1~T6-6
  - docs/04-版本标准.md                              # 里程碑索引补 0.4.1 行 + v0.4.1 验收口径段
  - docs/versions/0.4.1/规划需求.md                   # 新建
  - docs/versions/0.4.1/更新日志.md                   # 新建，T6-1~T6-6 verified_complete + release_complete
  - docs/qa/versions/0.4.1/QA-审计报告.md             # 新建，§0-§9，结论 qa_passed
  - README.md / README_EN.md                         # 版本号 + 功能列表补 5 项修复
  - handoff/                                         # 全部 trio 文件清理（HANDOFF/REPORT/REVIEW + TASK-BOARD）

out_of_scope:
  - 不改后端命令签名（T6-1/T6-4/T6-5 已完成）
  - 不外发数据：保持纯本地处理
  - 不引入新依赖（除非 T6-5 construct_regex 必需，且需安全审计）

acceptance_criteria:
  - 4 处版本号一致为 0.4.1
  - tauri build 产物 RuT0DataKit_0.4.1_aarch64.dmg 存在，Info.plist CFBundleShortVersionString=0.4.1
  - cargo test --workspace 全绿（含新增 e2e：preprocess_to_search_finds_hits / looks_like_blind_probe / construct_regex）
  - cd frontend && npm run build 通过，无 FileToolbar 残留、无 regexTemplate 残留
  - docs/qa/versions/0.4.1/QA-审计报告.md 结论 qa_passed，覆盖 8 项 audit_scope
  - docs/04 里程碑索引 0.4.1 行 release_complete
  - docs/versions/0.4.1/更新日志.md T6-1~T6-6 verified_complete + release_complete
  - git tag v0.4.1 推送到 git@github.com:Wh1teJ0ker/RuT0DataKit.git
  - handoff/ 目录在 release 后清理（TASK-BOARD + trio 全部移除）

verification_commands:
  - cargo build --workspace
  - cargo test --workspace
  - cd frontend && npm run build
  - cd src-tauri && npx @tauri-apps/cli@latest build
  - /usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" src-tauri/target/release/bundle/macos/RuT0DataKit.app/Contents/Info.plist
  - git tag --list v0.4.1
  - git push git@github.com:Wh1teJ0ker/RuT0DataKit.git v0.4.1

files_likely_to_change:
  - Cargo.toml
  - src-tauri/Cargo.toml
  - src-tauri/tauri.conf.json
  - frontend/package.json
  - docs/00-需求文档.md
  - docs/01-页面与交互说明.md
  - docs/02-技术设计文档.md
  - docs/03-开发任务清单.md
  - docs/04-版本标准.md
  - docs/versions/0.4.1/规划需求.md
  - docs/versions/0.4.1/更新日志.md
  - docs/qa/versions/0.4.1/QA-审计报告.md
  - README.md
  - README_EN.md

risks:
  - 4 处版本号必须同时改，遗漏会导致 tauri build 产物文件名/Info.plist 不一致。
  - QA 报告必须覆盖 8 项 audit_scope，缺一项不算 qa_passed。
  - git tag v0.4.1 push 失败时（网络/SSH）需重试，不阻塞本地 release_complete 标记。

depends_on: [T6-1, T6-2, T6-3, T6-4, T6-5]
status: planned
```

## 上下文

本任务是 v0.4.1 收尾，依赖 T6-1~T6-5 全部 verified_complete。流程：

1. Phase 7 端到端：`cargo test --workspace` + `npm build` + `tauri build`，收集 9 项 E2E 证据。
2. Phase 8 Release QA：写 `docs/qa/versions/0.4.1/QA-审计报告.md`，覆盖 §0-§9，结论 qa_passed。
3. Phase 9 release_complete：4 处版本号 + docs/04 里程碑 + 更新日志 + 规划需求 全部标 release_complete。
4. git commit + tag v0.4.1 + push GitHub SSH。
5. 清理 handoff/ 全部 trio 文件。

**安全约束（必须逐字保留）**：不外发数据：全本地处理；规则与样本不上传。
