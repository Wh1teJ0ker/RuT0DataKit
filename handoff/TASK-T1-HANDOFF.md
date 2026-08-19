task_id: T1
goal: |
  让 SqlReader 能导入常见 MySQL dump，同时保持 SQLite 兼容 SQL 的既有读取行为。
in_scope:
  - crates/core/src/datasource/sql.rs
  - crates/core/tests/mysql_dump.rs
out_of_scope:
  - 数据库 schema 版本
  - CSV/DB 数据源行为
  - CTF answer 与测试样例文件
acceptance_criteria:
  - MySQL 块/版本注释及 CREATE DATABASE、USE、SET、LOCK/UNLOCK TABLES 不会导致导入失败。
  - MySQL CREATE TABLE 选项和字符串反斜杠转义可转换为 SQLite 可执行 SQL。
  - 标准 SQLite SQL 回归测试通过。
verification_commands:
  - cargo test -p ruT0-data-kit-core datasource::sql
files_likely_to_change:
  - crates/core/src/datasource/sql.rs
  - crates/core/tests/mysql_dump.rs
risks:
  - SQL 引号和注释解析不得错误处理字符串字面量内的分号。
depends_on: []
status: planned
