# TASK-T11-HANDOFF — commands.rs 拆分

```yaml
task_id: T11
goal: |
  把 src-tauri/src/commands.rs（374 行单文件，混杂 4 类不相关关注点：导入/提取/脱敏/校验命令、规则 CRUD、设置 IO、更新检查）按关注点拆为独立子模块，使每类命令独立可读、可测。纯结构调整，不改任何命令的外部行为、IPC 签名、错误返回结构。
in_scope:
  - src-tauri/src/commands.rs
  - src-tauri/src/commands/ 下的新建子模块文件
  - src-tauri/src/lib.rs（仅当 mod 声明 / invoke_handler 注册需要同步）
out_of_scope:
  - crates/core/（整个不动）
  - src-tauri/src/db/（不改）
  - frontend/（整个不动）
  - 任何 command 的业务逻辑 / 错误映射 / 返回 JSON 结构（保持字节级行为不变）
  - capability / permission 配置（不改）
acceptance_criteria:
  - commands.rs 不再包含任何 #[tauri::command] 函数体，只剩 mod 声明 + pub use 重导出（或完全替换为 mod.rs + 子模块）
  - 4 类关注点各归独立子模块文件（建议命名）：
      - data.rs（导入 / 提取 / 脱敏 / 校验类命令）
      - rules.rs（规则 CRUD 类命令）
      - settings.rs（设置 IO 类命令）
      - update.rs（check_update 等更新检查类命令）
  - lib.rs 的 invoke_handler! 宏注册的命令路径若因拆分变化，需同步更新且仍注册全部原有命令（命令名 / 参数 / 返回结构不变）
  - 各 command 函数签名（参数、返回类型、错误类型）保持不变
  - 错误处理保持现状：CoreError / DbError 的 JSON 映射、check_update 的 error 吞噬行为均不改（这些是已知现状，本任务只搬运不修复）
  - 现有编译通过、应用行为不变
verification_commands:
  - cargo check --workspace
  - cargo build --manifest-path src-tauri/Cargo.toml
  - cargo test --workspace
files_likely_to_change:
  - src-tauri/src/commands.rs（变薄或变 mod.rs）
  - src-tauri/src/commands/data.rs
  - src-tauri/src/commands/rules.rs
  - src-tauri/src/commands/settings.rs
  - src-tauri/src/commands/update.rs
  - src-tauri/src/lib.rs（invoke_handler 同步，仅必要时）
risks:
  - lib.rs 的 invoke_handler! 用全路径注册命令，拆模块后路径前缀变化需逐条核对，漏注册会导致前端调用失败
  - 部分命令可能共享私有 helper（如 settings 读写与 data 命令共享 db 句柄获取），helper 应留在合适模块或提取到 commands/mod.rs 的私有区，避免循环 use
  - commands.rs 当前 use 的类型（DbManager / CoreError / 各 Reader）拆分后每个子模块需各自补 use
depends_on: []
status: planned
```

## 背景说明

`src-tauri/src/commands.rs` 是 374 行的 god module，把 4 类不相关的 Tauri IPC 命令塞在一个文件：
1. 数据类（导入 / 提取 / 脱敏 / 校验）
2. 规则 CRUD
3. 设置 IO（tshark 路径 / db 路径读写）
4. 更新检查（check_update）

文件不可独立测试单类命令，新增命令时找不到落点。本任务只做**关注点搬运**：按上述 4 类拆到独立子模块，不改任何命令的字节级行为、签名、错误映射。

注意：审计报告指出的「check_update 吞噬 error」「CoreError JSON 映射不一致」「settings-IO 在 commands 层」「零 command 层测试」等都是已知现状，**本任务只搬运不修复**，修复留给后续任务或单独议题。
