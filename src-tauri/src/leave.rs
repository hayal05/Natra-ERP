use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LeaveRequest {
    pub id: String,
    pub employee_id: String,
    pub leave_type: String,
    pub start_date: String,
    pub end_date: String,
    pub reason: Option<String>,
    pub status: String,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<String>,
}

pub fn migrate(c: &Connection) -> Result<(), rusqlite::Error> {
    c.execute_batch("CREATE TABLE IF NOT EXISTS leave_requests(id TEXT PRIMARY KEY,employee_id TEXT NOT NULL REFERENCES employees(id),leave_type TEXT NOT NULL,start_date TEXT NOT NULL,end_date TEXT NOT NULL,reason TEXT,status TEXT NOT NULL DEFAULT 'pending',reviewed_by TEXT,reviewed_at TEXT,created_at TEXT NOT NULL,updated_at TEXT NOT NULL); CREATE INDEX IF NOT EXISTS idx_leave_employee ON leave_requests(employee_id); CREATE INDEX IF NOT EXISTS idx_leave_status ON leave_requests(status);")
}

pub fn create(c: &mut Connection, leave: &LeaveRequest, now: &str) -> Result<(), rusqlite::Error> {
    let tx = c.transaction()?;
    tx.execute("INSERT INTO leave_requests(id,employee_id,leave_type,start_date,end_date,reason,status,reviewed_by,reviewed_at,created_at,updated_at) VALUES(?,?,?,?,?,?, 'pending',NULL,NULL,?,?)", params![leave.id,leave.employee_id,leave.leave_type,leave.start_date,leave.end_date,leave.reason,now,now])?;
    let payload = serde_json::to_string(leave).unwrap_or_default();
    tx.execute("INSERT INTO sync_outbox(id,operation,entity,entity_id,payload,created_at) VALUES(?, 'upsert','leave',?,?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload,created_at=excluded.created_at,last_error=NULL", params![format!("sync-leave-{}", leave.id),leave.id,payload,now])?;
    tx.commit()
}
