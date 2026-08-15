//! 会话与 sheet 创建 / 查询。
//!
//! T91 从 `db/mod.rs` 拆出；方法签名 / SQL / 测试逻辑保持不变。

use super::{
    now_rfc3339, DbManager, DbError, SessionDetail, SessionSummary, SheetSummary,
};
use rusqlite::params;

// v1.1+ IPC 将调用；单测已覆盖。
#[allow(
    dead_code,
    reason = "v1.1+ IPC 将接入（list_sessions/get_session 等）；单测已覆盖"
)]
impl DbManager {
    /// 创建导入会话，返回 `id`。
    pub fn create_session(
        &self,
        name: &str,
        source_path: Option<&str>,
        source_type: &str,
        row_count: u32,
    ) -> Result<i64, DbError> {
        let conn = self.conn();
        let now = now_rfc3339();
        conn.execute(
            "INSERT INTO sessions (name, source_path, source_type, row_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![name, source_path, source_type, row_count as i64, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 创建 sheet，返回 `id`。
    pub fn create_sheet(&self, session_id: i64, name: &str, position: i32) -> Result<i64, DbError> {
        let conn = self.conn();
        let now = now_rfc3339();
        conn.execute(
            "INSERT INTO sheets (session_id, name, position, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![session_id, name, position, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 列出全部会话摘要（按 `created_at` 降序）。
    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>, DbError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, source_type, row_count, created_at
             FROM sessions
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(SessionSummary {
                id: r.get::<_, i64>(0)?,
                name: r.get::<_, String>(1)?,
                source_type: r.get::<_, String>(2)?,
                row_count: r.get::<_, i64>(3)? as u32,
                created_at: r.get::<_, String>(4)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 取会话详情：摘要 + 关联 sheets（按 `position` 升序）。
    pub fn get_session(&self, session_id: i64) -> Result<SessionDetail, DbError> {
        let conn = self.conn();
        let session = conn.query_row(
            "SELECT id, name, source_type, row_count, created_at
             FROM sessions
             WHERE id = ?1",
            params![session_id],
            |r| {
                Ok(SessionSummary {
                    id: r.get::<_, i64>(0)?,
                    name: r.get::<_, String>(1)?,
                    source_type: r.get::<_, String>(2)?,
                    row_count: r.get::<_, i64>(3)? as u32,
                    created_at: r.get::<_, String>(4)?,
                })
            },
        )?;
        let mut stmt = conn.prepare(
            "SELECT id, session_id, name, position, created_at
             FROM sheets
             WHERE session_id = ?1
             ORDER BY position ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |r| {
            Ok(SheetSummary {
                id: r.get::<_, i64>(0)?,
                session_id: r.get::<_, i64>(1)?,
                name: r.get::<_, String>(2)?,
                position: r.get::<_, i32>(3)?,
                created_at: r.get::<_, String>(4)?,
            })
        })?;
        let mut sheets = Vec::new();
        for row in rows {
            sheets.push(row?);
        }
        Ok(SessionDetail { session, sheets })
    }
}

#[cfg(test)]
mod tests {
    use crate::db::test_support;

    #[test]
    fn list_and_get_session() {
        let (_dir, mgr) = test_support::open();
        let sid = mgr
            .create_session("s1", Some("/a/b.csv"), "csv", 100)
            .unwrap();
        let sh1 = mgr.create_sheet(sid, "Sheet1", 1).unwrap();
        let sh2 = mgr.create_sheet(sid, "Sheet2", 0).unwrap();
        // list
        let list = mgr.list_sessions().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, sid);
        assert_eq!(list[0].name, "s1");
        assert_eq!(list[0].source_type, "csv");
        assert_eq!(list[0].row_count, 100);
        // get（按 position 升序）
        let detail = mgr.get_session(sid).unwrap();
        assert_eq!(detail.session.id, sid);
        assert_eq!(detail.sheets.len(), 2);
        assert_eq!(detail.sheets[0].id, sh2); // position=0 在前
        assert_eq!(detail.sheets[1].id, sh1); // position=1 在后
    }
}
