```yaml
implemented_changes:
  - src-tauri/Cargo.toml：[dependencies] 新增 `base64 = "0.22"`（STANDARD engine API）。
  - src-tauri/src/db/mod.rs：新增 `base64_transform_column_cells<F>(&self, sheet_id, col_idx, transform: F)`
    方法，镜像 `replace_cells_inner` 模式（单事务 + before/after 快照），用泛型闭包
    封装转换逻辑使 DB 层不依赖 commands 层。SQL 全部用 `?N` + `params![]` 绑定。
    返回 `(affected, skipped, before_changed, after_changed)`。
    同时把 `list_undoable_operations` 的 IN 子句加入 `base64_column` 让该操作入撤销栈。
  - src-tauri/src/commands/columns.rs：
    * 新增 `Base64Mode { Encode, Decode }` enum（serde lowercase + Deserialize，
      以满足 Tauri `#[tauri::command]` 的 `CommandArg` trait bound）。
    * 新增 `Base64Result { affected: u32, skipped: u32 }`（camelCase）。
    * 新增 `base64_column_inner(db, sheet_id, column, mode) -> Result<Base64Result, String>`：
      find_col_idx → base64_transform_column_cells（传编/解码闭包）→
      log_operation_with_snapshot("base64_column") → 返回。解码用
      `STANDARD.decode` + `String::from_utf8`，失败行返回 None 计入 skipped。
    * 新增 `#[tauri::command] base64_column` wrapper。
    * 新增 5 个单测：encode basic / decode basic / decode skip invalid /
      encode+decode roundtrip / logs before_snapshot + undo 恢复。
  - src-tauri/src/lib.rs：`generate_handler!` 注册 `commands::columns::base64_column`；
    模块 doc 注释更新（含 base64_column 说明）。
  - docs/02-技术设计文档.md：同步 4.9 列操作命令表 + 4.7 撤销/重做命令表 +
    DbManager 方法清单 + 模块依赖表 + 撤销栈注释，新增 base64_column 签名/语义说明。
verification_run:
  - cargo fmt --check
  - cargo clippy --workspace -- -D warnings
  - cargo test --workspace
verification_results:
  - cargo fmt --check：通过（无 diff）。
  - cargo clippy --workspace -- -D warnings：通过（0 警告，1 次因 Base64Mode 缺
    Deserialize 触发 E0277 → 加 `Deserialize` derive 后修复，再次通过）。
  - cargo test --workspace：全部通过。77 个 lib tests（含 5 个新 base64 测试）
    + 49 个 core tests + 0 个 doc-test。第一次运行 roundtrip 测试因把 row_idx=0
    表头行误算入 affected 期望值（4）失败 → 修正期望为 3（表头不入变换）后全绿。
  - SQL 参数化扫描：grep `format!` 在 db/mod.rs / columns.rs 内仅命中 1 处
    （regex 错误信息构造，非 SQL 拼接）；新增方法无字符串拼接 SQL。
docs_updated:
  - docs/02-技术设计文档.md（4.7 / 4.9 节 + DbManager 方法清单 + 模块依赖表
    + 撤销栈状态结构注释，反映 base64_column 入撤销栈与 base64_transform_column_cells 方法）
commit_summary:
  - none（按要求不 commit、不 push）
reported_status:
  - verified_complete
scope_deviation:
  - 出于让 base64_column 操作可撤销（HANDOFF acceptance: "undo_operation 可恢复
    原始值"），把 `list_undoable_operations` 的硬编码 IN 子句加入 `base64_column`
    kind。HANDOFF out_of_scope 未禁止改 db/mod.rs（in_scope 已含），且不改 IN
    子句则新操作不会出现在撤销列表中、4.7 文档承诺的「就地变更入栈」会失配。
    此改动落在 db/mod.rs 内（同一文件、同一方法、同种硬编码常量、无注入风险），
    属完成 acceptance criteria 的必要配套，已在 scope_deviation 显式说明。
  - HANDOFF 实现指引建议 `Base64Mode` 仅 derive `Serialize`；为满足 Tauri
    `#[tauri::command]` 对参数的 `CommandArg` trait bound（需 `Deserialize`），
    额外 derive 了 `Deserialize`。否则编译报 E0277，命令无法注册。
  - 修复 roundtrip 测试：初版误把 row_idx=0 表头行计入 affected 期望（4），
    实际表头不入变换（与 replace_cells_inner 一致），改为 3 后通过。非业务逻辑变更，
    仅修正测试断言。
  - 未碰 frontend / 其他 commands 模块 / schema / Cargo.toml 之外的依赖。
