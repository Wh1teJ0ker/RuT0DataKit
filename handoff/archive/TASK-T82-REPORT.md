# TASK-T82: 04-版本标准.md v1.1.4 里程碑行补全 R3/R4

```yaml
implemented_changes:
  - 修改 docs/04-版本标准.md 第 25 行（v1.1.4 行）的「核心交付」列
  - 在原有 T70 续轮内容后追加 R3（T73-T76）描述：hash_column IPC 命令 + HashAlgorithm enum + list_undoable_operations 白名单 + CryptoPanel UI + hashColumn wrapper + DbReader 数据源（.db/.sqlite/.sqlite3 多表联合 + __table 列 + 跳过 sqlite_% + quote_identifier 转义）+ detect_format 3 match arm + md-5/sha1/sha2/hex 4 依赖
  - 追加 R4（T77-T79）描述：allow_special bool→allow_special_chars String 白名单 + 手机号前缀 UX 统一（ValidatePanel 每行内嵌 Select + ExtractPanel phone_prefixes 运行时覆盖 + extract_validate_to_new_sheet phone_prefixes 参数）+ list_rules ORDER BY id ASC→name ASC + list_rules_sorted_by_name 测试
  - 状态列保持 qa_passed 不变
verification_run:
  - grep "hash_column" docs/04-版本标准.md
  - grep "DbReader" docs/04-版本标准.md
  - grep "name ASC" docs/04-版本标准.md
  - grep "allow_special_chars" docs/04-版本标准.md
verification_results:
  - grep "hash_column": PASS（命中 v1.1.4 行）
  - grep "DbReader": PASS（命中 v1.1.4 行）
  - grep "name ASC": PASS（命中 v1.1.4 行）
  - grep "allow_special_chars": PASS（命中 v1.1.4 行）
docs_updated:
  - docs/04-版本标准.md（v1.1.4 行核心交付列追加 R3/R4 内容）
commit_summary:
  - 4f2418a docs(version-standard): 补全 v1.1.4 里程碑行 R3/R4 内容
reported_status:
  - verified_complete
scope_deviation:
  - none
```
