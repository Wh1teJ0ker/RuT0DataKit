use ruT0_data_kit_core::datasource::detect_format;

/// 验证真实的 MySQL 5.7 dump（person_data.sql）能被正确导入。
/// 该文件含版本注释 /*!40101 ... */、CREATE DATABASE、USE、LOCK/UNLOCK TABLES、
/// SET 会话变量、CREATE TABLE with ENGINE=/CHARACTER SET/COLLATE/AUTO_INCREMENT
/// 等 MySQL-only 语法。修复前首个 CREATE DATABASE 语句即报错。
#[test]
fn real_mysql_dump_imports() {
    // 集成测试工作目录为 crate 根 (crates/core/)，SQL 文件在 workspace 根的 tests/ 下。
    let path = "../../tests/test/1/data-cls的附件/person_data.sql";
    let reader = detect_format(path).unwrap();
    let dataset = reader.read().unwrap();
    // The table has 5 columns: 编号, 用户名, 姓名, 身份证号, 手机号码.
    assert_eq!(dataset.headers.len(), 5);
    assert!(dataset.headers.contains(&"编号".to_string()));
    assert!(dataset.headers.contains(&"用户名".to_string()));
    assert!(dataset.headers.contains(&"姓名".to_string()));
    assert!(dataset.headers.contains(&"身份证号".to_string()));
    assert!(dataset.headers.contains(&"手机号码".to_string()));
    // Data rows expected.
    assert!(!dataset.rows.is_empty(), "should have data rows");
}
