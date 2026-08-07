# RuT0DataKit v1.1.0

RuT0DataKit v1.1.0 在 v1.0.0 纯框架 shell 之上，交付**数据处理原型三件套**——脱敏 / 校验 / 提取——以及**规则管理基础结构（DB 持久化）**。数据全程在本地处理，不外发。

---

## ✨ 新增

### 数据脱敏（mask）
选列 + 可填「掩码字符」（默认 `*`）→ 就地脱敏：
- **≥3 字符**：保留首尾，中间用掩码字符替换（「张三丰」→「张*丰」，「司马相如」→「司**如」）
- **2 字符**：保留首字符，末位用掩码字符替换（「张三」→「张*」）
- **1 字符**：单个掩码字符；**空串**：空串

脱敏结果回写 DB + 行高亮 `masked`（浅蓝）。掩码字符可填、可重置（恢复默认 `*`）、可保存设置（持久化到 DB）。

### 数据校验（validate）
选列 + 选规则 → 正则校验 → 不通过行高亮 `invalid`（浅红）。内置「姓名校验」规则（2-4 位中文字符）。

### 数据提取（extract）
选列 + 多选规则 → 前端正则提取 → 命中行高亮 `hit`（浅绿）+ 底部 PII 列表展示。内置 3 正则：手机号 / 邮箱 / 身份证号。

### 规则管理（DB 持久化）
- 内置**三条姓名相关规则**（脱敏 / 校验 / 提取各一），持久化到 SQLite（`rules` 表，重启后保留）
- RulesPanel 改为 Workbench 主区**两栏布局**：左侧规则列表（按类别分组）+ 右侧规则详情（名称 / 描述 / 类别 / 可填参数 / 内联测试）
- 可填参数（`pattern` / `replacement`）保存后回写 DB
- **不支持新增规则**（v1.1.0 规则集固定为内置 3 条）

### 行状态高亮
| 状态 | 颜色 | 含义 |
|---|---|---|
| `masked` | 浅蓝 `#f0f5ff` | 已脱敏 |
| `invalid` | 浅红 `#fff1f0` | 校验不通过 |
| `hit` | 浅绿 `#f6ffed` | 提取命中 |

### 自动更新
启用 `createUpdaterArtifacts`，构建自动生成 `.sig` 签名文件 + `latest.json` 清单，应用内自动更新可用。

### 其他
- 版本号 1.0.0 → 1.1.0
- 安全加固：CSP 收紧 `default-src 'self'` + capabilities 删除 `home`/`desktop` 递归写权限 + updater `dialog=true`

---

## 📋 下载

| 平台 | 文件 | 说明 |
|---|---|---|
| Windows x64 | `RuT0DataKit_1.1.0_x64-setup.exe` | NSIS 安装包（推荐） |
| Windows x64 | `RuT0DataKit_1.1.0_x64_en-US.msi` | MSI 安装包 |
| macOS arm64 | `RuT0DataKit_1.1.0_aarch64.dmg` | Apple Silicon |
| macOS arm64 | `RuT0DataKit_aarch64.app.tar.gz` | 自动更新用 |
| Linux x64 | `RuT0DataKit_1.1.0_amd64.deb` | Debian/Ubuntu |
| Linux x64 | `RuT0DataKit-1.1.0-1.x86_64.rpm` | Fedora/RHEL |
| Linux x64 | `RuT0DataKit_1.1.0_amd64.AppImage` | 便携运行 |

> 每个安装包均附带 `.sig` 签名文件，供 updater 验签。

---

## ⚠️ 已知限制

- 规则集固定为内置 3 条（脱敏 / 校验 / 提取各一），**不支持新增规则**；仅可调整可填参数并持久化
- 脱敏为就地回写（upsert），不可在应用内回滚；如需原始数据请重新导入
- 脱敏掩码字符取输入值首个字符（空/null 回退为 `*`）；姓名脱敏对 ≥3 字符保留首尾、中间以掩码字符替换
- 提取结果仅在面板内展示，不持久化

---

## 🔄 升级

v1.0.0 用户可直接升级：DB schema 从 `SCHEMA_VERSION=1` 增量迁移到 `2`（新增 `rules` 表 + 索引，`IF NOT EXISTS` 幂等，不触发备份重建），历史数据兼容。启动时若 `rules` 表为空，自动 seed 3 条内置规则。

---

## ✅ 验证

- `cargo fmt --check` / `cargo clippy --workspace -- -D warnings` / `cargo test --workspace`（40 passed）：全通过
- `pnpm build`（3078 modules）：通过，无报错

---

完整技术文档见 [`docs/versions/1.1.0/`](https://github.com/Wh1teJ0ker/RuT0DataKit/tree/main/docs/versions/1.1.0)。
