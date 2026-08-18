use rusqlite::{params, Connection, OptionalExtension};
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
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS leave_requests(
            id TEXT PRIMARY KEY,
            employee_id TEXT NOT NULL REFERENCES employees(id),
            leave_type TEXT NOT NULL,
            start_date TEXT NOT NULL,
            end_date TEXT NOT NULL,
            reason TEXT,
            status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','approved','rejected')),
            reviewed_by TEXT,
            reviewed_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_leave_employee ON leave_requests(employee_id);
        CREATE INDEX IF NOT EXISTS idx_leave_status ON leave_requests(status);
        CREATE INDEX IF NOT EXISTS idx_leave_dates ON leave_requests(employee_id,start_date,end_date);"
    )
}

fn validate_dates(start_date: &str, end_date: &str) -> Result<(), rusqlite::Error> {
    // Dates are stored as ISO YYYY-MM-DD strings, so lexical comparison is chronological.
    if start_date.len() != 10 || end_date.len() != 10
        || start_date.as_bytes().get(4) != Some(&b'-')
        || end_date.as_bytes().get(4) != Some(&b'-')
        || start_date.as_bytes().get(7) != Some(&b'-')
        || end_date.as_bytes().get(7) != Some(&b'-')
        || !start_date.chars().enumerate().all(|(i,c)| i == 4 || i == 7 || c.is_ascii_digit())
        || !end_date.chars().enumerate().all(|(i,c)| i == 4 || i == 7 || c.is_ascii_digit())
        || start_date > end_date
    {
        return Err(rusqlite::Error::InvalidParameterName("Leave dates must be valid ISO dates and the end date cannot be before the start date.".into()));
    }
    Ok(())
}

fn employee_is_active(c: &Connection, employee_id: &str) -> Result<bool, rusqlite::Error> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM employees WHERE id=? AND lower(status)='active')",
        [employee_id],
        |r| r.get::<_, i64>(0),
    )? != 0)
}

fn has_overlap(c: &Connection, employee_id: &str, start_date: &str, end_date: &str, exclude_id: Option<&str>) -> Result<bool, rusqlite::Error> {
    let count: i64 = if let Some(id) = exclude_id {
        c.query_row(
            "SELECT COUNT(*) FROM leave_requests WHERE employee_id=? AND status IN ('pending','approved') AND id<>? AND start_date<=? AND end_date>=?",
            params![employee_id, id, end_date, start_date],
            |r| r.get(0),
        )?
    } else {
        c.query_row(
            "SELECT COUNT(*) FROM leave_requests WHERE employee_id=? AND status IN ('pending','approved') AND start_date<=? AND end_date>=?",
            params![employee_id, end_date, start_date],
            |r| r.get(0),
        )?
    };
    Ok(count > 0)
}

pub fn create(c: &mut Connection, leave: &LeaveRequest, now: &str) -> Result<(), rusqlite::Error> {
    validate_dates(&leave.start_date, &leave.end_date)?;
    if leave.leave_type.trim().is_empty() {
        return Err(rusqlite::Error::InvalidParameterName("Leave type is required.".into()));
    }
    if !employee_is_active(c, &leave.employee_id)? {
        return Err(rusqlite::Error::InvalidParameterName("Leave can only be requested for an active employee.".into()));
    }
    if has_overlap(c, &leave.employee_id, &leave.start_date, &leave.end_date, None)? {
        return Err(rusqlite::Error::InvalidParameterName("The employee already has a pending or approved leave that overlaps these dates.".into()));
    }

    let tx = c.transaction()?;
    let pending = LeaveRequest {
        id: leave.id.clone(),
        employee_id: leave.employee_id.clone(),
        leave_type: leave.leave_type.trim().to_string(),
        start_date: leave.start_date.clone(),
        end_date: leave.end_date.clone(),
        reason: leave.reason.clone(),
        status: "pending".into(),
        reviewed_by: None,
        reviewed_at: None,
    };
    tx.execute(
        "INSERT INTO leave_requests(id,employee_id,leave_type,start_date,end_date,reason,status,reviewed_by,reviewed_at,created_at,updated_at) VALUES(?,?,?,?,?,?, 'pending',NULL,NULL,?,?)",
        params![pending.id,pending.employee_id,pending.leave_type,pending.start_date,pending.end_date,pending.reason,now,now],
    )?;
    queue(&tx, &pending, now)?;
    tx.commit()
}

pub fn list(c: &Connection, status: Option<&str>) -> Result<Vec<LeaveRequest>, rusqlite::Error> {
    let sql = "SELECT id,employee_id,leave_type,start_date,end_date,reason,status,reviewed_by,reviewed_at FROM leave_requests";
    let mut out = Vec::new();
    if let Some(status) = status {
        let mut stmt = c.prepare(&format!("{} WHERE status=? ORDER BY start_date,id", sql))?;
        let rows = stmt.query_map([status], row)?;
        for item in rows { out.push(item?); }
    } else {
        let mut stmt = c.prepare("SELECT id,employee_id,leave_type,start_date,end_date,reason,status,reviewed_by,reviewed_at FROM leave_requests ORDER BY start_date,id")?;
        let rows = stmt.query_map([], row)?;
        for item in rows { out.push(item?); }
    }
    Ok(out)
}

pub fn review(c: &mut Connection, id: &str, status: &str, reviewed_by: &str, now: &str) -> Result<(), rusqlite::Error> {
    if status != "approved" && status != "rejected" {
        return Err(rusqlite::Error::InvalidParameterName("Status must be approved or rejected.".into()));
    }
    if reviewed_by.trim().is_empty() {
        return Err(rusqlite::Error::InvalidParameterName("Reviewer is required.".into()));
    }

    let tx = c.transaction()?;
    let leave = row_from_id(&tx, id)?;
    if leave.status != "pending" {
        return Err(rusqlite::Error::InvalidParameterName("Only pending leave requests can be reviewed.".into()));
    }
    if status == "approved" {
        if !employee_is_active(&tx, &leave.employee_id)? {
            return Err(rusqlite::Error::InvalidParameterName("The employee is inactive and cannot have leave approved.".into()));
        }
        if has_overlap(&tx, &leave.employee_id, &leave.start_date, &leave.end_date, Some(id))? {
            return Err(rusqlite::Error::InvalidParameterName("This leave overlaps another pending or approved leave.".into()));
        }
    }

    tx.execute(
        "UPDATE leave_requests SET status=?,reviewed_by=?,reviewed_at=?,updated_at=? WHERE id=? AND status='pending'",
        params![status,reviewed_by.trim(),now,now,id],
    )?;
    let updated = row_from_id(&tx, id)?;
    queue(&tx, &updated, now)?;
    tx.commit()
}

fn queue(tx: &rusqlite::Transaction<'_>, leave: &LeaveRequest, now: &str) -> Result<(), rusqlite::Error> {
    let payload = serde_json::to_string(leave).unwrap_or_default();
    tx.execute(
        "INSERT INTO sync_outbox(id,operation,entity,entity_id,payload,created_at) VALUES(?, 'upsert','leave',?,?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload,created_at=excluded.created_at,last_error=NULL",
        params![format!("sync-leave-{}",leave.id),leave.id,payload,now],
    )?;
    Ok(())
}

fn row(r: &rusqlite::Row<'_>) -> Result<LeaveRequest, rusqlite::Error> {
    Ok(LeaveRequest{id:r.get(0)?,employee_id:r.get(1)?,leave_type:r.get(2)?,start_date:r.get(3)?,end_date:r.get(4)?,reason:r.get(5)?,status:r.get(6)?,reviewed_by:r.get(7)?,reviewed_at:r.get(8)?})
}

fn row_from_id(c: &Connection, id: &str) -> Result<LeaveRequest, rusqlite::Error> {
    c.query_row("SELECT id,employee_id,leave_type,start_date,end_date,reason,status,reviewed_by,reviewed_at FROM leave_requests WHERE id=?", [id], row)
}
