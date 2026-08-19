task_id: T1 (rework)
reviewer: codingflow-master:reviewer
reviewed_at: 2026-08-19
handoff: handoff/TASK-T1-HANDOFF.md
coder_report: handoff/TASK-T1-REPORT.md
verdict: review_passed
defects: []
summary: |
  3 个 minor 缺陷已修复：块注释内 * 泄漏（改用 prev_was_star 跟踪）、
  COMMENT 正则大小写（(?i) 前缀）、COMMENT 正则转义引号支持（(?:[^'\\]|\\.)*）。
  docs/02-技术设计文档.md 已同步新增 §4.12 MySQL dump 兼容章节。
  验证命令 cargo test datasource::sql 6/6 通过，全仓 cargo test 通过。
  → review_passed