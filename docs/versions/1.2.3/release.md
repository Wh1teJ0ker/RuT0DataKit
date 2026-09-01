# v1.2.3 发布说明

> 版本号：v1.2.3（SCHEMA_VERSION=5，无数据库迁移）
> 发布日期：2026-09-01
> 里程碑：Windows pcap 导入三层兼容修复

## What's New

### Windows pcap 导入兼容

本版本修复 Windows 上导入 pcap 文件失败的三层根因：

1. **`-q` 标志 + 错误分类**：tshark 默认把 banner / 捕获信息写到 stderr，Windows 控制台编码下经 `String::from_utf8_lossy` 解码后变成含乱码的错误串，被误判为「解析失败」。修复：命令统一加 `-q` 关闭 banner，使 stdout 纯净为字段、stderr 纯净为错误；spawn 失败 → `DependencyMissing`（提示配置路径/安装 Wireshark）；非零退出 → `Other`；字段无效 → `NotImplemented`（引导升级）。

2. **自动探测 + 缓存**：Windows 上 Wireshark 默认安装到 `C:\Program Files\Wireshark\` 而不加入 PATH，导致 `Command::new("tshark")` spawn 失败。修复：`resolve_tshark_cmd()` 在无用户覆盖时自动探测候选路径（含 Windows 绝对路径），探测结果进程生命周期内缓存。

3. **探测健康检查简化 + `CREATE_NO_WINDOW`**：设置页配置路径后仍无法使用。根因：`probe_tshark` 的 NUL 健康检查误判有效 tshark 为不兼容。修复：简化探测为仅 `--version` 退出 0 + 首行非空。同时所有 tshark 子进程统一通过 `build_tshark_command()` 构造，Windows 上设 `CREATE_NO_WINDOW` 标志，避免 GUI 应用 spawn CLI 子进程时弹控制台窗口闪烁。

三层修复叠加后：Windows 用户即使未在设置页配置 tshark 路径，导入 pcap 时也会自动探测到 `C:\Program Files\Wireshark\tshark.exe` 并成功解析。

## Downloads

通过 GitHub Releases 下载对应平台的安装包。安装后启动即完成升级，无需手动迁移数据。

| 平台 | 架构 | 格式 |
|------|------|------|
| Linux | x86_64 | `.tar.gz` |
| macOS | aarch64 | `.dmg` 或 `.zip` |
| macOS | x86_64 | `.dmg` 或 `.zip` |
| Windows | x86_64 | `.msi` 或 `.zip` |

## 升级说明

- 直接安装覆盖旧版本，应用数据自动继承
- `SCHEMA_VERSION=5` 不变，无数据库迁移
