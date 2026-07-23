//! E2E 集成测试共享辅助：用 `CARGO_MANIFEST_DIR` 定位 fixtures 与 rules。
//!
//! `CARGO_MANIFEST_DIR` 在集成测试中指向 `crates/core`，所以需要回退两级
//! 才能到达仓库根，再进入 `tests/fixtures/samples` 与 `rules`。

use std::path::PathBuf;

/// 仓库根下 `tests/fixtures/samples` 目录。
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/samples")
}

/// `tests/fixtures/samples/csv/sample_mask.csv`。
pub fn csv_path() -> PathBuf {
    fixtures_dir().join("csv/sample_mask.csv")
}

/// `tests/fixtures/samples/csv/sample_mask.xlsx`。
pub fn xlsx_path() -> PathBuf {
    fixtures_dir().join("csv/sample_mask.xlsx")
}

/// 仓库根下 `rules` 目录。
pub fn rules_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../rules")
}
