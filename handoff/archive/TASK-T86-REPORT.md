# TASK-T86 REPORT

```yaml
implemented_changes:
  - 文件：docs/02-技术设计文档.md
  - 仅改 line 3（顶部版本覆盖说明），在 R3 续轮描述末尾（"schema 不变 `SCHEMA_VERSION=5`，`params` 列复用存 Generic JSON）。"）后追加一段 R4 续轮 T77/T78/T79 的标注：
    - T77：ExtractParams::Generic 的 allow_special: bool -> allow_special_chars: String（#[serde(default)] 向后兼容），空串=不允许任何特殊字符，非空如 _-.@=仅允许这些字符（白名单语义），is_valid_generic 逐字符 allow_special_chars.contains(c) 判定
    - T78：手机号前缀 UX 统一 — ValidatePanel phone-validate 前缀从底部全局 Form.Item 改为每行内嵌 Select mode="tags"（多行合并去重）+ ExtractPanel phone-extract 新增 phone_prefixes 运行时覆盖（extract_validate_to_new_sheet 新增 phone_prefixes: Vec<String> 参数，非空时构造 ExtractParams::PhonePrefix 覆盖 DB rule.params）
    - T79：list_rules SQL ORDER BY id ASC -> ORDER BY name ASC（Unicode 码点排序，ASCII 大写排前，中文按码点序），新增 list_rules_sorted_by_name 测试，schema 不变 SCHEMA_VERSION=5，无新增依赖 crate
verification_run:
  - grep "R4" docs/02-技术设计文档.md | head -3
  - grep "allow_special_chars" docs/02-技术设计文档.md | head -3
  - grep "name ASC" docs/02-技术设计文档.md | head -3
verification_results:
  - grep "R4"：通过，line 3 追加段含 "R4 续轮 T77/T78/T79 已落地特殊符号白名单 + 手机号前缀 UX 统一 + 规则排序"
  - grep "allow_special_chars"：通过，line 3 追加段含 "allow_special: bool -> allow_special_chars: String"
  - grep "name ASC"：通过，line 3 追加段含 "ORDER BY id ASC -> ORDER BY name ASC"
docs_updated:
  - docs/02-技术设计文档.md（仅 line 3 末尾追加一段 R4 续轮标注）
commit_summary:
  - docs(design): 02-技术设计文档版本覆盖说明追加 R4 续轮 T77-T79 (b782595)
reported_status:
  - verified_complete
scope_deviation:
  - none
```
