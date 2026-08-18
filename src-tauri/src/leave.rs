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

pub fn list(c: &Connection, status: Option<&str>) -> Result<Vec<LeaveRequest>, rusqlite::Error> {
    let sql = "SELECT id,employee_id,leave_type,start_date,end_date,reason,status,reviewed_by,reviewed_at FROM leave_requests";
    let mut out = Vec::new();
    if let Some(status) = status {
        let mut stmt = c.prepare(&format!("{} WHERE status=? ORDER BY start_date", sql))?;
        let rows = stmt.query_map([status], row)?;
        for item in rows { out.push(item?); }
    } else {
        let mut stmt = c.prepare("SELECT id,employee_id,leave_type,start_date,end_date,reason,status,reviewed_by,reviewed_at FROM leave_requests ORDER BY start_date")?;
        let rows = stmt.query_map([], row)?;
        for item in rows { out.push(item?); }
    }
    Ok(out)
}

pub fn review(c: &mut Connection, id: &str, status: &str, reviewed_by: &str, now: &str) -> Result<(), rusqlite::Error> {
    if status != "approved" && status != "rejected" { return Err(rusqlite::Error::InvalidParameterName("status must be approved or rejected".into())); }
    let tx = c.transaction()?;
    let changed = tx.execute("UPDATE leave_requests SET status=?,reviewed_by=?,reviewed_at=?,updated_at=? WHERE id=? AND status='pending'", params![status,reviewed_by,now,now,id])?;
    if changed == 0 { return Err(rusqlite::Error::QueryReturnedNoRows); }
    let leave = row_from_id(&tx, id)?;
    queue(&tx, &leave, now)?;
    tx.commit()
}

fn queue(tx: &rusqlite::Transaction<'_>, leave: &LeaveRequest, now: &str) -> Result<(), rusqlite::Error> {
    let payload = serde_json::to_string(leave).unwrap_or_default();
    tx.execute("INSERT INTO sync_outbox(id,operation,entity,entity_id,payload,created_at) VALUES(?, 'upsert','leave',?,?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload,created_at=excluded.created_at,last_error=NULL", params![format!("sync-leave-{}",leave.id),leave.id,payload,now])?;
    Ok(())
}

fn row(r: &rusqlite::Row<'_>) -> Result<LeaveRequest, rusqlite::Error> {
    Ok(LeaveRequest{id:r.get(0)?,employee_id:r.get(1)?,leave_type:r.get(2)?,start_date:r.get(3)?,end_date:r.get(4)?,reason:r.get(5)?,status:r.get(6)?,reviewed_by:r.get(7)?,reviewed_at:r.get(8)?})
}

fn row_from_id(c: &Connection, id: &str) -> Result<LeaveRequest, rusqlite::Error> {
    c.query_row("SELECT id,employee_id,leave_type,start_date,end_date,reason,status,reviewed_by,reviewed_at FROM leave_requests WHERE id=?", [id], row)
}
