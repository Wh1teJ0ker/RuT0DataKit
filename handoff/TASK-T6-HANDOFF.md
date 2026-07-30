# TASK-T6-HANDOFF — Tauri updater 插件

## task_id
T6

## goal
集成 Tauri updater 插件（带 Ed25519 签名），在 `tauri.conf.json` 配置 `plugins.updater`（pubkey + endpoints），装配 updater 插件到 AppBuilder，提供 `check_update`/`install_update` 两个 `#[command]`（委托插件），生成签名密钥并把 pubkey 记录到 `docs/versions/1.0.0/`（私钥不落盘仓库）。

## in_scope
允许新增/修改的文件：

- `src-tauri/tauri.conf.json`：配置 `plugins.updater`：
  - `active: true`
  - `endpoints: ["https://github.com/Wh1teJ0ker/RuT0DataKit/releases/latest/download/latest.json"]`（GitHub Release 托管）
  - `pubkey: "<生成的公钥>"`（从 `tauri signer generate` 输出填入）
  - `dialog: false`（前端自管 UI）
  - `windows.installMode: "passive"`（Windows 静默安装）
- `src-tauri/src/lib.rs`：**扩展** T4 的 `run()`，装配 `tauri-plugin-updater`（`.plugin(tauri_plugin_updater::Builder::new().build())`）。
- `src-tauri/src/commands.rs`（或拆 `commands/updater.rs`）：新增两个 `#[command]`：
  - `check_update(updater: tauri_plugin_updater::UpdaterExt) -> Result<UpdateStatus, String>`：调 `app.updater().check()` 获取 `Update`，若 `Ok(Some(update))` 返回 `UpdateStatus { available: true, version: Some(update.version), notes: Some(update.body) }`；`Ok(None)` 返回 `available=false`；`Err(e)` 视情况——无网络时静默降级返回 `available=false`（不 panic、不抛错给前端），网络错误但可判定为「无新版本」则降级，其它错误抛 `Err(e.to_string())`。
  - `install_update(app: AppHandle) -> Result<(), String>`：调 `app.updater().check()` 拿 `Update`，`update.download_and_install()` 委托插件（下载 + 验签 + 安装 + 重启提示）。
  - `UpdateStatus` struct：`#[derive(Serialize)]` + `#[serde(rename_all = "camelCase")]`，字段 `available: bool`/`version: Option<String>`/`notes: Option<String>`。
  - 命令需在 `lib.rs` 的 `generate_handler!` 注册（`.invoke_handler(tauri::generate_handler![check_update, install_update])`）。
- `src-tauri/capabilities/default.json`：**扩展**权限，加 `updater:default`（T1 已占位声明 updater 权限，本任务确认激活——T1 的 `updater:default` 权限已在，若 T1 未含则补）。
- `docs/versions/1.0.0/updater-密钥.md`（新增）：记录密钥生成操作步骤 + pubkey + GitHub Secret 配置说明（`TAURI_SIGNING_PRIVATE_KEY`/`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`）；**私钥不写进此文件**，仅写「私钥保存于本地 `~/.tauri/ruT0datakit.key`，CI 通过 GitHub Secret 注入」。
- **密钥生成**：coder 执行 `tauri signer generate -w ~/.tauri/ruT0datakit.key`（若环境有 `tauri-cli`），生成 Ed25519 密钥对；pubkey 填入 `tauri.conf.json`，私钥留在 `~/.tauri/`（不 `git add` 私钥）。若 coder 无 GUI 但有 `tauri-cli`，可生成；若不可，在 REPORT 注明「密钥生成待主会话环境执行」，`pubkey` 字段留空或占位 `"<待生成>"`，主会话补生成后填入。

## out_of_scope
明确不许动的：

- **不许实现 CSV/XLSX 导入**（T5）。
- **不许动 `crates/core/`**。
- **不许动 frontend/**（前端 `checkUpdate`/`installUpdate` 封装与设置页 UI 由 T8 处理；本任务只写后端命令）。若需前端调用，T8 的 `tauri.js` 封装会调 `invoke('check_update')`。
- **不许实现设置页 UI**（T8）。
- **不许把私钥写入任何仓库文件**（`.gitignore` 应忽略 `*.key`，pubkey 可入库）。
- **不许动 docs/ 下除 `docs/versions/1.0.0/updater-密钥.md` 外的文档**。
- **不许动 handoff/ 其它任务文件**。
- **不许改 DB 层**（T4 产物）。

## acceptance_criteria
1. `src-tauri/tauri.conf.json` 的 `plugins.updater` 配置 `active=true`、`endpoints` 指向 GitHub Release latest 的 `latest.json`、`pubkey` 填入（或占位 `<待生成>` 若环境无法生成）、`dialog=false`、`windows.installMode=passive`。
2. `src-tauri/src/lib.rs` 装配 `tauri-plugin-updater` 到 AppBuilder。
3. `check_update` 命令注册到 `generate_handler!`，返回 `UpdateStatus`（`available`/`version`/`notes`），`UpdateStatus` 带 `#[serde(rename_all = "camelCase")]`。
4. `check_update` 在无新版本时返回 `available=false`（不 panic）。
5. `check_update` 无网络时静默降级返回 `available=false`，不抛错给前端。
6. `install_update` 命令注册，调用插件 `download_and_install`（验签失败时插件会拒绝安装，抛 `Err`）。
7. `docs/versions/1.0.0/updater-密钥.md` 存在，记录 pubkey + 操作步骤 + GitHub Secret 配置说明，无私钥。
8. `cargo check --workspace` 通过。
9. `.gitignore` 忽略 `*.key`（私钥不入库）。

## verification_commands
```sh
# 1. 编译
cargo check --workspace

# 2. updater 配置核对
grep -n 'updater\|pubkey\|endpoints\|installMode' src-tauri/tauri.conf.json

# 3. 命令注册核对
grep -n 'check_update\|install_update\|generate_handler' src-tauri/src/lib.rs src-tauri/src/commands.rs 2>/dev/null

# 4. serde camelCase
grep -rn 'rename_all' src-tauri/src/commands.rs src-tauri/src/commands/ 2>/dev/null

# 5. 私钥未入库
grep -rn '\.key' .gitignore
git check-ignore ~/.tauri/ruT0datakit.key 2>/dev/null || echo "private key not in repo"

# 6. 密钥文档
ls docs/versions/1.0.0/updater-密钥.md
```

`check_update` 的真实网络/模拟 latest.json 核验由主会话环境补；coder 至少 `cargo check` 通过并描述降级逻辑（无网络/无新版本/有新版本/签名失败 四路径）。

## files_likely_to_change
- `src-tauri/tauri.conf.json`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands.rs`（或 `commands/updater.rs`）
- `src-tauri/capabilities/default.json`
- `docs/versions/1.0.0/updater-密钥.md`
- `.gitignore`（加 `*.key`）

## risks
- **密钥生成需用户参与**：`tauri signer generate` 会提示设密码；coder 环境若无法交互式输密码，可用 `tauri signer generate -w ~/.tauri/ruT0datakit.key --password ""` 或环境变量 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。若完全无法生成，`pubkey` 留占位，主会话补。
- **`UpdaterExt` trait**：Tauri v2 updater 用 `app.updater()`（需 `use tauri_plugin_updater::UpdaterExt;`）；命令参数用 `updater: tauri_plugin_updater::UpdaterExt` 不对，应 `app: AppHandle` 或 `updater: tauri::AppHandle` + `UpdaterExt` trait import。注意 v2 API：`app.updater()?.check()` 返回 `Result<Option<Update>>`。
- **无网络降级**：`check()` 的 `Err` 分支需判 `is_network_error` 或统一降级为 `available=false`（v1.0.0 不区分错误类型，静默降级即可）。
- **CI 签名注入**：本地 dev 不需签名密钥（`cargo tauri dev` 不验签），但 `cargo tauri build` 需 `TAURI_SIGNING_PRIVATE_KEY` 环境变量；本任务只配 dev 能跑通，CI 配置留给后续 CI 任务。

## depends_on
[T1]

## status
planned
