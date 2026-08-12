# TASK-T86: 02-技术设计文档.md 顶部版本覆盖说明追加 R4

```yaml
task_id: T86
goal: |
  在 docs/02-技术设计文档.md 顶部版本覆盖说明（line 3）追加 R4 续轮标注（T77/T78/T79：allow_special_chars 白名单 + 手机号前缀 UX 统一 + list_rules name 排序）。
in_scope:
  - docs/02-技术设计文档.md
out_of_scope:
  - 不改正文其他章节
  - 不改代码
acceptance_criteria:
  - 顶部版本覆盖说明提及 R4 续轮 T77-T79
  - 提及 allow_special_chars 白名单、手机号前缀 UX 统一、list_rules ORDER BY name ASC
verification_commands:
  - grep "R4" docs/02-技术设计文档.md | head -3
  - grep "allow_special_chars" docs/02-技术设计文档.md | head -3
depends_on: []
status: planned
```
