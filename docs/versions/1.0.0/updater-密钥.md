# updater 签名密钥（v1.0.0）

> 本文件记录 Tauri updater 插件 Ed25519 签名密钥的生成操作、公钥、CI 注入说明。
> **私钥永不入库**。本文件只放 pubkey 与操作说明。

## 背景

RuT0DataKit v1.0.0 集成 `tauri-plugin-updater`，通过 GitHub Release 托管 `latest.json`
与安装包。客户端下载更新包后用内嵌的 pubkey 校验 Ed25519 签名，签名不匹配则拒绝安装。

- **算法**：Ed25519（`tauri signer` 使用 `minisign` 格式）
- **pubkey**：写入 `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`，随客户端分发
- **私钥**：仅本地保存与 CI Secret 注入，不入库

## pubkey（已填入 tauri.conf.json）

```
dW50cnVzdGVkIGNvbW1lbnQ6IG1lbmlzaWduIHB1YmxpYyBrZXk6IDc0QUNEODA3MDlDODEzRTIKUldUaUU4Z0pCOWlzZEc5OWpnYlI3ZVd6TUswODlERlhhcEo5UEZ6ZTVrdWxqdkR0ZEwvdHlnekEK
```

`src-tauri/tauri.conf.json` 对应配置：

```json
"plugins": {
  "updater": {
    "active": true,
    "endpoints": [
      "https://github.com/Wh1teJ0ker/RuT0DataKit/releases/latest/download/latest.json"
    ],
    "pubkey": "<上面 pubkey 字符串>",
    "dialog": false,
    "windows": { "installMode": "passive" }
  }
}
```

## 私钥保管

- 私钥保存于本地 `~/.tauri/ruT0datakit.key`，**不 `git add`**。
- `.gitignore` 已加 `*.key` 规则，确保任何 `.key` 文件不会被误提交。
- CI 通过 GitHub Secret 注入，不在仓库明文存放。

## 密钥生成操作（参考，已执行）

```sh
# 1. 生成 Ed25519 密钥对（交互式设密码；或在 CI 用空密码）
tauri signer generate -w ~/.tauri/ruT0datakit.key

# 输出示例：
# ✓ Your private key was written to /Users/<user>/.tauri/ruT0datakit.key
# ✓ Your public key was written to /Users/<user>/.tauri/ruT0datakit.key.pub
#   [base64 pubkey 字符串]

# 2. 把输出的 base64 pubkey 粘贴进 src-tauri/tauri.conf.json 的 plugins.updater.pubkey

# 3. 私钥留在 ~/.tauri/，不入库（.gitignore 已忽略 *.key）
```

## GitHub Secret 配置（CI 发布签名用）

`cargo tauri build` 在打包时若检测到 `TAURI_SIGNING_PRIVATE_KEY` 环境变量，
会自动对生成的安装包签名并写入 `latest.json`。在 GitHub 仓库
`Settings → Secrets and variables → Actions` 添加：

| Secret 名 | 值 | 说明 |
| --- | --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | `~/.tauri/ruT0datakit.key` 文件内容 | Ed25519 私钥（PEM/base64 原文） |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 生成时设定的密码 | 私钥解密口令；空密码则设为空字符串 |

CI workflow（发布任务，非本 T6 范围）在 `cargo tauri build` 步骤前 export 这两个变量即可。

## 验签流程

1. 客户端启动 / 用户点击「检查更新」→ `check_update` 命令 →
   `tauri-plugin-updater` 请求 `endpoints` 的 `latest.json`。
2. `latest.json` 含 `version` / `platforms.<target>.signature` / `notes` 等字段。
3. 客户端下载对应平台安装包，用 **内嵌 pubkey**（`tauri.conf.json` 中）校验
   `latest.json` 里的签名。验签失败则 `install_update` 抛 `Err`，不安装。
4. 验签通过 → `download_and_install` 执行安装（Windows 下按 `installMode: passive`
   静默安装；macOS 替换 app bundle 后提示重启）。

## 安全注意

- 私钥泄露 = 任何人可发布被客户端接受的「合法」更新。私钥仅可信环境（CI、维护者本机）使用。
- **不要**把私钥内容粘贴进任何仓库文件（包括本文档）。
- 若怀疑私钥泄露：重新生成密钥对 → 更新 `tauri.conf.json` 的 pubkey →
  发版后客户端会自动改用新 pubkey 校验后续版本（旧客户端需先升级到含新 pubkey 的版本）。
