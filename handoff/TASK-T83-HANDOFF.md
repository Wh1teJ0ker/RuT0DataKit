implemented_changes:
  - file: src-tauri/src/commands/columns.rs
    changes:
      - "新增 HashCase enum（Lower/Upper），serde lowercase，放在 Base64Mode enum之前"
      - "hash_column_inner 函数签名追加 case: HashCase 参数"
      - "hash_column_inner 内三个 algorithm 分支的 format!(\"{:x}\", digest) 后追加条件 to_uppercase()：if matches!(case, HashCase::Upper) { hex.to_uppercase() } else { hex }"
      - "log_operation_with_snapshot 的 params JSON 追加 case 字段（serde lowercase 序列化）"
      - "hash_column Tauri 命令签名追加 case: HashCase 参数，传递给 hash_column_inner"
      - "新增测试 hash_column_inner_uppercase：验证 MD5/SHA1/SHA256 三种算法 case=Upper 时输出全大写 hex"
      - "更新现有测试调用：所有 hash_column_inner 调用补充 HashCase::Lower 参数（18个测试点）"
  - file: frontend/src/tauri.js
    changes:
      - "hashColumn wrapper 追加 case_ 参数，传递给 invoke 时 key 为 case"
  - file: frontend/src/components/panels/CryptoPanel.jsx
    changes:
      - "import 追加 Radio"
      - "新增 hashCase state，默认 'lower'"
      - "handleExecute 中哈希分支调用 hashColumn 时传入 hashCase"
      - "isHashOp 为 true 时显示 Radio.Group（小写/大写），Alert 之前"
verification_run:
  - command: "cargo fmt --all"
    result: "passed"
  - command: "cargo clippy --all-targets --all-features -- -D warnings"
    result: "passed"
  - command: "cargo test -p ruT0-data-kit --lib -- columns::"
    result: "19 passed, 0 failed"
  - command: "pnpm --prefix frontend build"
    result: "passed (vite build success)"
verification_results:
  - command: "cargo fmt --all"
    passed: true
  - command: "cargo clippy --all-targets --all-features -- -D warnings"
    passed: true
  - command: "cargo test -p ruT0-data-kit --lib -- columns::"
    passed: true
    output: "19 passed; 0 failed; 0 ignored"
  - command: "pnpm --prefix frontend build"
    passed: true
docs_updated: []
commit_summary: "feat(columns): add HashCase (Lower/Upper) to hash_column for optional uppercase hex output (T83)"
reported_status: "verified_complete"
scope_deviation: "none"
