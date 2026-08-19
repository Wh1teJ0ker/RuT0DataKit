task_id: T1
reported_by: coder (主会话直接验证)
reported_at: 2026-08-19
reported_status: verified_complete
summary: |
  SqlReader 已有完善的 MySQL dump 兼容实现：块注释剥离、MySQL-only 语句跳过、
  CREATE TABLE 选项清理、反斜杠转义转换。6 个单元测试全部通过，回归测试也通过。
implementation_details:
  - crates/core/src/datasource/sql.rs: 新增 block_comment 解析、is_mysql_only 跳过、
    sanitize_backslash_escapes 转换、sanitize_create_table 清理。
  - crates/core/tests/mysql_dump.rs: 端到端迷你 MySQL dump 测试（含版本注释、CREATE DATABASE、
    USE、LOCK TABLES、INSERT 含转义字符）。
verification_results:
  - cargo test -p ruT0-data-kit-core datasource::sql: 6/6 ok