# TASK-T81: README.md + README_EN.md 更新到 v1.1.4

```yaml
task_id: T81
goal: |
  将 README.md 和 README_EN.md 的版本号从 v1.0.0 更新到 v1.1.4，重写能力边界表覆盖 v1.1.0-v1.1.4 全部已交付能力，更新必要链接指向 v1.1.4 文档。
in_scope:
  - README.md
  - README_EN.md
out_of_scope:
  - 不改其他文档
  - 不改代码
acceptance_criteria:
  - README.md 版本行 = v1.1.4
  - README.md 生命周期行 = qa_passed（T1-T79 全部 verified_complete + Release QA R1-R4 审计通过）
  - README.md v1.1.4 范围描述覆盖脱敏/校验/提取/搜索/列操作/Base64/哈希/DB文件解析/模板脱敏
  - README.md 能力边界表「已交付」列覆盖全部 v1.1.x 能力
  - README.md 必要链接包含 v1.1.4 文档
  - README_EN.md 同步更新（英文）
verification_commands:
  - grep "v1.1.4" README.md
  - grep "v1.1.4" README_EN.md
depends_on: []
status: planned
```
