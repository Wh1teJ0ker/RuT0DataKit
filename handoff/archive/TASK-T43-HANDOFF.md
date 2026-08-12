```yaml
task_id: T43
goal: |
  版本号 1.1.1 → 1.1.2（6 处一致升级）。
in_scope:
  - Cargo.toml（workspace.package.version）
  - src-tauri/tauri.conf.json（version）
  - frontend/package.json（version）
  - frontend/src/constants.js（APP_VERSION）
out_of_scope:
  - 不改功能代码
  - 不改文档版本号（T44 做）
  - 不改 crates/core/Cargo.toml 和 src-tauri/Cargo.toml（version.workspace = true，无需改）
acceptance_criteria:
  - grep -rn '1\.1\.2' Cargo.toml src-tauri/tauri.conf.json frontend/package.json frontend/src/constants.js 全部命中
  - grep -rn '1\.1\.1' Cargo.toml src-tauri/tauri.conf.json frontend/package.json frontend/src/constants.js 无残留
verification_commands:
  - grep -rn '1\.1\.2' Cargo.toml src-tauri/tauri.conf.json frontend/package.json frontend/src/constants.js
files_likely_to_change:
  - Cargo.toml
  - src-tauri/tauri.conf.json
  - frontend/package.json
  - frontend/src/constants.js
risks: []
depends_on: [T37, T38, T39, T40, T41, T42]
status: planned
```

## 实现指引

4 处版本号升级（crates/core 和 src-tauri 用 `version.workspace = true`，无需改）：

1. `Cargo.toml`：`version = "1.1.1"` → `version = "1.1.2"`
2. `src-tauri/tauri.conf.json`：`"version": "1.1.1"` → `"version": "1.1.2"`
3. `frontend/package.json`：`"version": "1.1.1"` → `"version": "1.1.2"`
4. `frontend/src/constants.js`：`APP_VERSION` 的值 `v1.1.1` → `v1.1.2`

注意 Cargo.toml 顶部的注释行（如 `# v1.1.1 撤销/重做...`）也更新为 v1.1.2 描述。
