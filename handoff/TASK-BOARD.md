# TASK-BOARD — v0.6.2 数据校验通用规则：正则校验（release_complete）

> 版本：v0.6.2
> 创建：2026-07-24
> 完成：2026-07-24
> 依赖版本：v0.6.1（release_complete @ 48bb419）
> 依据：用户 goal「加入数据校验通用规则，正则校验」
> **状态：release_complete（qa_passed）**

## 任务 DAG（全部 done）

```yaml
goal: 加入内置正则校验通用规则（scope="regex", tag="validate"），用户在 RulesView
  填写自定义正则 pattern + 失败消息，试运行验证，应用并跳转到数据校验视图绑定列。
  与现有脱敏模版 UX 完全对称。bump 0.6.2，全链路验收通过。
version: 0.6.2
depends_on_version: 0.6.1
tasks:
  - id: T17-1
    title: Core 添加 regex 内置校验规则（regex_validate_rule）
    status: done  # builtin.rs:199 + mod.rs:24 + 2 新测试，cargo test 493 passed
  - id: T17-2
    title: Tauri 添加 trial_validate 命令
    status: done  # validate.rs:64 + main.rs:42，Value::Null 修复，cargo build 0 error
  - id: T17-3
    title: Frontend RulesView 校验参数表单 + 试运行 + 应用
    status: done  # tauri.js trialValidate + RulesView VALIDATE_PARAM_META/getParamMeta/
                  # runValidateTrialForRow/applyValidateForRow，npm build 绿
  - id: T17-4
    title: 版本号 bump 0.6.1→0.6.2（4 manifest）+ docs 同步 + QA 报告
    status: done  # 本任务
```

任务 DAG 结构：

```
T17-1 (core builtin) ─┐
                       ├─→ T17-3 (frontend) ─→ T17-4 (version+docs+QA)
T17-2 (tauri command) ─┘
```

T17-1 和 T17-2 无文件交集，并行实施。T17-3 依赖两者。T17-4 最后。

## E2E 验收结果

| 项 | 验证命令 | 结果 |
|----|----------|------|
| 前端构建 | `npm --prefix frontend run build` | vite 3008 modules built，2.21s ✅ |
| 核心测试 | `cargo test --workspace` | 493 passed / 0 failed / 5 ignored ✅（比 v0.6.1 基线 491 +2） |
| 核心编译 | `cargo build -p ruT0-data-kit-core` | 0 error，1 warning（历史遗留 crate 名） ✅ |
| Tauri 编译 | `cargo build --manifest-path src-tauri/Cargo.toml` | 0 error ✅ |
| grep regex_validate_rule | `grep -rn regex_validate_rule crates/core/src/rules/builtin.rs mod.rs` | builtin.rs:48/199/554/567 + mod.rs:24 ✅ |
| grep trial_validate | `grep -rn trial_validate src-tauri/src/commands/validate.rs main.rs` | validate.rs:1/64 + main.rs:42 ✅ |
| grep VALIDATE_PARAM_META | `grep -n VALIDATE_PARAM_META RulesView.jsx tauri.js` | RulesView.jsx:75/85 + tauri.js:324 ✅ |
| 版本号一致 | 4 manifest grep | 全部 0.6.2 ✅ |

## Release QA 门禁

- required: true → **passed**
- report: `docs/qa/versions/0.6.2/QA-审计报告.md`（结论 qa_passed）
- 五维度全部通过：需求覆盖 / 端到端 / 构建测试 / 代码质量 / 文档一致性 / 安全隐私

## 三层改动落地证据

### T17-1 Core（crates/core）

- `crates/core/src/rules/builtin.rs:199`：`pub fn regex_validate_rule() -> FieldRule`
- `crates/core/src/rules/builtin.rs:48`：加入 `builtin_ruleset().validators` vec（7→8）
- `crates/core/src/rules/mod.rs:24`：`pub use builtin::{...}` 追加 `regex_validate_rule`
- 新增 2 测试：`regex_validate_rule_fields` + `regex_validate_rule_with_pattern_builds_and_validates`

### T17-2 Tauri（src-tauri）

- `src-tauri/src/commands/validate.rs:64`：`pub fn trial_validate(scope, params_json, sample_value) -> Result<Value, String>`
- `src-tauri/src/main.rs:42`：`generate_handler!` 注册 `commands::trial_validate`
- `Value::Null` 修复（`json!` 宏 scope 无 `null`）

### T17-3 Frontend（frontend/src）

- `frontend/src/tauri.js:324`：`export async function trialValidate(scope, paramsJson, sampleValue)`
- `frontend/src/components/RulesView.jsx:75`：`VALIDATE_PARAM_META`
- `frontend/src/components/RulesView.jsx:85`：`getParamMeta(scope)`
- `frontend/src/components/RulesView.jsx`：`runValidateTrialForRow` + `applyValidateForRow`
- `RowExpanded` 结果区 mask→masked / validate→valid（合法✓ / 非法✗+message）
- 操作列有 meta 就显示按钮（不再只看 isMaskRow）

### T17-4 版本 + docs

- 4 manifest 版本号 0.6.1→0.6.2（Cargo.toml:8 / src-tauri/Cargo.toml:3 / tauri.conf.json:4 / frontend/package.json:4）
- `docs/04-版本标准.md`：v0.6.2 里程碑行
- `docs/versions/0.6.2/更新日志.md`：回填完毕
- `docs/qa/versions/0.6.2/QA-审计报告.md`：本版本 QA 报告

## 安全约束（不变）

- 不外发数据：全本地处理；规则与样本不上传（保留 v0.1.0 §6 安全约束，v0.6.2 不变）。
- `trial_validate` 镜像 `trial_mask`：不读文件、不落盘，只对单条样例值在内存中校验。

## 收尾

- 三件套（HANDOFF/REPORT/REVIEW）全部清理，仅保留本 TASK-BOARD 作为版本归档。
- `docs/04-版本标准.md` v0.6.2 里程碑行状态 `release_complete`。
- `docs/versions/0.6.2/更新日志.md` 版本状态 `release_complete`。
- v0.6.2 发布完成。
