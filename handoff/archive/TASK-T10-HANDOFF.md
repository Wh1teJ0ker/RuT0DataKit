# TASK-T10-HANDOFF — datasource mod 拆分

```yaml
task_id: T10
goal: |
  把 crates/core/src/datasource/mod.rs（731 行单文件，含 7 个格式 Reader + SQL tokenizer + 工厂 + 测试）按格式拆为独立子模块，使每个 reader 独立可读、可测、可扩展。纯结构调整，不改任何 reader 的外部行为、公共 API 签名或测试断言。
in_scope:
  - crates/core/src/datasource/mod.rs
  - crates/core/src/datasource/ 下的新建子模块文件
  - crates/core/src/lib.rs（仅当 mod 声明需要同步）
out_of_scope:
  - crates/core/src/pcap/（不改）
  - crates/core/src/processor/（不改）
  - crates/core/src/model.rs（不改）
  - crates/core/src/error.rs（不改）
  - src-tauri/（整个不动）
  - frontend/（整个不动）
  - 任何 reader 的解析逻辑 / 错误映射 / 返回值结构（保持字节级行为不变）
  - 测试断言（保持现有 3 个 datasource 测试不变，不改 expected 值）
acceptance_criteria:
  - datasource/mod.rs 不再包含任何 Reader struct / impl / helper fn 的实现体，只剩 mod 声明 + pub use 重导出 + detect_format 工厂（工厂可留在 mod.rs 或移到 factory.rs，二者择一）
  - 每个格式 Reader（Csv/Xlsx/Json/Txt/Sql/Pcap）有独立 .rs 子模块文件，文件名小写对应格式
  - cell_to_string / value_to_string / extract_table_name 等内部 helper 归入对应 reader 模块或一个共享 util.rs，不再散落在 mod.rs
  - 现有 3 个 datasource 单测（detect_format routing / csv headers / csv flexible columns）保持原断言通过，位置可迁移到对应子模块的 #[cfg(test)] 块
  - crates/core/src/lib.rs 的 pub mod datasource 仍可见，detect_format / 各 Reader struct / Reader trait 仍可从 ruT0_data_kit_core::datasource:: 访问到（公共 API 表面不变）
  - SqlReader 的 split_statements / extract_table_name 逻辑迁移到 sql.rs 但行为不变（仍不处理块注释 / BEGIN END，这是已知现状，不在本任务修复范围）
verification_commands:
  - cargo check --workspace
  - cargo test --workspace
  - cargo test -p ruT0-data-kit-core --lib datasource
files_likely_to_change:
  - crates/core/src/datasource/mod.rs
  - crates/core/src/datasource/csv.rs
  - crates/core/src/datasource/xlsx.rs
  - crates/core/src/datasource/json.rs
  - crates/core/src/datasource/txt.rs
  - crates/core/src/datasource/sql.rs
  - crates/core/src/datasource/pcap.rs（datasource 侧的 PcapReader 适配器，非 core/src/pcap/）
  - crates/core/src/datasource/util.rs（可选：共享 helper）
  - crates/core/src/datasource/factory.rs（可选：detect_format 工厂）
risks:
  - PcapReader 适配器在 datasource/ 与 core/src/pcap/reader.rs 同名，拆分时注意模块路径不冲突（datasource::pcap::PcapReader vs pcap::reader::PcapReader）
  - 各 reader 间如有共享 helper（cell_to_string 被 XlsxReader 用、value_to_string 被 JsonReader 用），需统一放到 util.rs 或各自复制，避免循环引用
  - 测试迁移时注意 #[cfg(test)] 块的 use 路径更新
depends_on: []
status: planned
```

## 背景说明

当前 `crates/core/src/datasource/mod.rs` 是 731 行的 god module，把 7 个不相关的格式解析器 + 一个 SQL 语句分割器 + detect_format 工厂 + 3 个单元测试全塞在一个文件里。文件头部注释声称「新增数据源只需实现 Reader trait 并在 detect_format 工厂按扩展名分发」，但文件本身不可读、不可独立测试单个格式。

本任务只做**结构搬运**：把已存在的代码按格式拆到独立子模块，不改任何字节级行为。外部消费者（src-tauri commands.rs、lib.rs 公共 API）的访问路径必须保持不变。
