# TASK-T85: 创建 docs/versions/1.1.4/规划需求.md

```yaml
task_id: T85
goal: |
  创建缺失的 docs/versions/1.1.4/规划需求.md，定义 v1.1.4 版本范围与任务列表，格式参照其他版本的规划需求.md。
in_scope:
  - docs/versions/1.1.4/规划需求.md (新建)
out_of_scope:
  - 不改其他文件
  - 不改代码
acceptance_criteria:
  - docs/versions/1.1.4/规划需求.md 文件存在
  - 包含版本概述、范围定义、任务列表（T67-T79）
  - 与更新日志.md/RELEASE-NOTES.md 内容一致
verification_commands:
  - test -f docs/versions/1.1.4/规划需求.md && echo "exists"
depends_on: []
status: planned
```
