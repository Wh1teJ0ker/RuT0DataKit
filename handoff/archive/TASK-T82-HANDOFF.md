# TASK-T82: 04-版本标准.md v1.1.4 里程碑行补全 R3/R4

```yaml
task_id: T82
goal: |
  更新 docs/04-版本标准.md §2 里程碑索引表中 v1.1.4 行，补全 R3（hash_column + DbReader）和 R4（特殊符号白名单 + 手机号前缀UX统一 + name排序）的内容。
in_scope:
  - docs/04-版本标准.md
out_of_scope:
  - 不改其他行（v1.0.0-v1.1.3 行不变）
  - 不改代码
acceptance_criteria:
  - v1.1.4 行核心交付列包含 R3（hash_column MD5/SHA1/SHA256 + DbReader .db文件解析）和 R4（allow_special_chars 白名单 + 手机号前缀每行内嵌 + name排序）的描述
  - 状态列仍为 qa_passed
verification_commands:
  - grep "hash_column" docs/04-版本标准.md
  - grep "DbReader" docs/04-版本标准.md
  - grep "name ASC" docs/04-版本标准.md
depends_on: []
status: planned
```
