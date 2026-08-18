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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LeaveType {
    pub id: String,
    pub name: String,
    pub annual_days: i64,
    pub active: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LeaveBalance {
    pub id: String,
    pub employee_id: String,
    pub leave_type_id: String,
    pub year: i32,
    pub allocated_days: i64,
    pub used_days: i64,
    pub remaining_days: i64,
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
        CREATE TABLE IF NOT EXISTS leave_types(
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            annual_days INTEGER NOT NULL CHECK(annual_days >= 0),
            active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS leave_balances(
            id TEXT PRIMARY KEY,
            employee_id TEXT NOT NULL REFERENCES employees(id),
            leave_type_id TEXT NOT NULL REFERENCES leave_types(id),
            year INTEGER NOT NULL,
            allocated_days INTEGER NOT NULL CHECK(allocated_days >= 0),
            used_days INTEGER NOT NULL DEFAULT 0 CHECK(used_days >= 0),
            UNIQUE(employee_id,leave_type_id,year)
        );
        CREATE INDEX IF NOT EXISTS idx_leave_employee ON leave_requests(employee_id);
        CREATE INDEX IF NOT EXISTS idx_leave_status ON leave_requests(status);
        CREATE INDEX IF NOT EXISTS idx_leave_dates ON leave_requests(employee_id,start_date,end_date);
        CREATE INDEX IF NOT EXISTS idx_leave_balance_employee_year ON leave_balances(employee_id,year);")
}

fn validate_dates(start_date: &str, end_date: &str) -> Result<(), rusqlite::Error> {
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
    Ok(c.query_row("SELECT EXISTS(SELECT 1 FROM employees WHERE id=? AND lower(status)='active')", [employee_id], |r| r.get::<_, i64>(0))? != 0)
}

fn has_overlap(c: &Connection, employee_id: &str, start_date: &str, end_date: &str, exclude_id: Option<&str>) -> Result<bool, rusqlite::Error> {
    let count: i64 = if let Some(id) = exclude_id {
        c.query_row("SELECT COUNT(*) FROM leave_requests WHERE employee_id=? AND status IN ('pending','approved') AND id<>? AND start_date<=? AND end_date>=?", params![employee_id,id,end_date,start_date], |r| r.get(0))?
    } else {
        c.query_row("SELECT COUNT(*) FROM leave_requests WHERE employee_id=? AND status IN ('pending','approved') AND start_date<=? AND end_date>=?", params![employee_id,end_date,start_date], |r| r.get(0))?
    };
    Ok(count > 0)
}

fn parse_date(s: &str) -> Result<chrono::NaiveDate, rusqlite::Error> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| rusqlite::Error::InvalidParameterName("Invalid leave date.".into()))
}

fn inclusive_days(start: &str, end: &str) -> Result<i64, rusqlite::Error> {
    let a = parse_date(start)?;
    let b = parse_date(end)?;
    Ok((b - a).num_days() + 1)
}

pub fn create(c: &mut Connection, leave: &LeaveRequest, now: &str) -> Result<(), rusqlite::Error> {
    validate_dates(&leave.start_date, &leave.end_date)?;
    if leave.leave_type.trim().is_empty() { return Err(rusqlite::Error::InvalidParameterName("Leave type is required.".into())); }
    if !employee_is_active(c, &leave.employee_id)? { return Err(rusqlite::Error::InvalidParameterName("Leave can only be requested for an active employee.".into())); }
    let leave_type_id: Option<String> = c.query_row("SELECT id FROM leave_types WHERE lower(name)=lower(?) AND active=1", [leave.leave_type.trim()], |r| r.get(0)).optional()?;
    if leave_type_id.is_none() { return Err(rusqlite::Error::InvalidParameterName("The selected leave type is not active.".into())); }
    if has_overlap(c, &leave.employee_id, &leave.start_date, &leave.end_date, None)? { return Err(rusqlite::Error::InvalidParameterName("The employee already has a pending or approved leave that overlaps these dates.".into())); }
    let tx = c.transaction()?;
    let pending = LeaveRequest { id:leave.id.clone(), employee_id:leave.employee_id.clone(), leave_type:leave.leave_type.trim().to_string(), start_date:leave.start_date.clone(), end_date:leave.end_date.clone(), reason:leave.reason.clone(), status:"pending".into(), reviewed_by:None, reviewed_at:None };
    tx.execute("INSERT INTO leave_requests(id,employee_id,leave_type,start_date,end_date,reason,status,reviewed_by,reviewed_at,created_at,updated_at) VALUES(?,?,?,?,?,?, 'pending',NULL,NULL,?,?)", params![pending.id,pending.employee_id,pending.leave_type,pending.start_date,pending.end_date,pending.reason,now,now])?;
    queue(&tx,&pending,now)?;
    tx.commit()
}

pub fn list(c: &Connection, status: Option<&str>) -> Result<Vec<LeaveRequest>, rusqlite::Error> {
    let sql = "SELECT id,employee_id,leave_type,start_date,end_date,reason,status,reviewed_by,reviewed_at FROM leave_requests";
    let mut out=Vec::new();
    if let Some(status)=status { let mut stmt=c.prepare(&format!("{} WHERE status=? ORDER BY start_date,id",sql))?; for item in stmt.query_map([status],row)? { out.push(item?); } }
    else { let mut stmt=c.prepare("SELECT id,employee_id,leave_type,start_date,end_date,reason,status,reviewed_by,reviewed_at FROM leave_requests ORDER BY start_date,id")?; for item in stmt.query_map([],row)? { out.push(item?); } }
    Ok(out)
}

pub fn list_types(c: &Connection) -> Result<Vec<LeaveType>, rusqlite::Error> {
    let mut stmt=c.prepare("SELECT id,name,annual_days,active FROM leave_types ORDER BY name")?;
    let rows=stmt.query_map([],|r|Ok(LeaveType{id:r.get(0)?,name:r.get(1)?,annual_days:r.get(2)?,active:r.get::<_,i64>(3)?!=0}))?;
    rows.collect()
}

pub fn save_type(c: &mut Connection, item: &LeaveType, now: &str) -> Result<(), rusqlite::Error> {
    let name=item.name.trim();
    if name.is_empty() { return Err(rusqlite::Error::InvalidParameterName("Leave type name is required.".into())); }
    if item.annual_days < 0 { return Err(rusqlite::Error::InvalidParameterName("Annual leave days cannot be negative.".into())); }
    c.execute("INSERT INTO leave_types(id,name,annual_days,active,created_at,updated_at) VALUES(?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,annual_days=excluded.annual_days,active=excluded.active,updated_at=excluded.updated_at",params![item.id,name,item.annual_days,item.active,now,now])?;
    Ok(())
}

pub fn list_balances(c: &Connection, employee_id: Option<&str>, year: i32) -> Result<Vec<LeaveBalance>, rusqlite::Error> {
    let mut out=Vec::new();
    if let Some(employee_id)=employee_id {
        let mut stmt=c.prepare("SELECT b.id,b.employee_id,b.leave_type_id,b.year,b.allocated_days,b.used_days,b.allocated_days-b.used_days FROM leave_balances b WHERE b.employee_id=? AND b.year=? ORDER BY b.leave_type_id")?;
        for item in stmt.query_map(params![employee_id,year],balance_row)? { out.push(item?); }
    } else {
        let mut stmt=c.prepare("SELECT b.id,b.employee_id,b.leave_type_id,b.year,b.allocated_days,b.used_days,b.allocated_days-b.used_days FROM leave_balances b WHERE b.year=? ORDER BY b.employee_id,b.leave_type_id")?;
        for item in stmt.query_map([year],balance_row)? { out.push(item?); }
    }
    Ok(out)
}

pub fn set_balance(c: &mut Connection, item: &LeaveBalance) -> Result<(), rusqlite::Error> {
    if item.allocated_days < 0 || item.used_days < 0 || item.used_days > item.allocated_days { return Err(rusqlite::Error::InvalidParameterName("Leave balance values are invalid.".into())); }
    let employee_exists: bool=c.query_row("SELECT EXISTS(SELECT 1 FROM employees WHERE id=?)",[item.employee_id.as_str()],|r|r.get::<_,i64>(0))? != 0;
    if !employee_exists { return Err(rusqlite::Error::InvalidParameterName("Employee does not exist.".into())); }
    let type_exists: bool=c.query_row("SELECT EXISTS(SELECT 1 FROM leave_types WHERE id=? AND active=1)",[item.leave_type_id.as_str()],|r|r.get::<_,i64>(0))? != 0;
    if !type_exists { return Err(rusqlite::Error::InvalidParameterName("Leave type does not exist or is inactive.".into())); }
    c.execute("INSERT INTO leave_balances(id,employee_id,leave_type_id,year,allocated_days,used_days) VALUES(?,?,?,?,?,?) ON CONFLICT(employee_id,leave_type_id,year) DO UPDATE SET allocated_days=excluded.allocated_days,used_days=excluded.used_days",params![item.id,item.employee_id,item.leave_type_id,item.year,item.allocated_days,item.used_days])?;
    Ok(())
}

pub fn review(c: &mut Connection, id: &str, status: &str, reviewed_by: &str, now: &str) -> Result<(), rusqlite::Error> {
    if status != "approved" && status != "rejected" { return Err(rusqlite::Error::InvalidParameterName("Status must be approved or rejected.".into())); }
    if reviewed_by.trim().is_empty() { return Err(rusqlite::Error::InvalidParameterName("Reviewer is required.".into())); }
    let tx=c.transaction()?;
    let leave=row_from_id(&tx,id)?;
    if leave.status!="pending" { return Err(rusqlite::Error::InvalidParameterName("Only pending leave requests can be reviewed.".into())); }
    if status=="approved" {
        if !employee_is_active(&tx,&leave.employee_id)? { return Err(rusqlite::Error::InvalidParameterName("The employee is inactive and cannot have leave approved.".into())); }
        if has_overlap(&tx,&leave.employee_id,&leave.start_date,&leave.end_date,Some(id))? { return Err(rusqlite::Error::InvalidParameterName("This leave overlaps another pending or approved leave.".into())); }
        let days=inclusive_days(&leave.start_date,&leave.end_date)?;
        let year=leave.start_date[0..4].parse::<i32>().map_err(|_|rusqlite::Error::InvalidParameterName("Invalid leave year.".into()))?;
        let balance: Option<(String,i64,i64)>=tx.query_row("SELECT b.id,b.allocated_days,b.used_days FROM leave_balances b JOIN leave_types t ON t.id=b.leave_type_id WHERE b.employee_id=? AND b.year=? AND lower(t.name)=lower(?) AND t.active=1",params![leave.employee_id,year,leave.leave_type],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        let (balance_id,allocated,used)=balance.ok_or_else(||rusqlite::Error::InvalidParameterName("No leave balance is configured for this employee and leave type.".into()))?;
        if allocated-used < days { return Err(rusqlite::Error::InvalidParameterName(format!("Insufficient leave balance. Requested {} day(s), {} remaining.",days,allocated-used))); }
        tx.execute("UPDATE leave_balances SET used_days=used_days+? WHERE id=?",params![days,balance_id])?;
    }
    tx.execute("UPDATE leave_requests SET status=?,reviewed_by=?,reviewed_at=?,updated_at=? WHERE id=? AND status='pending'",params![status,reviewed_by.trim(),now,now,id])?;
    let updated=row_from_id(&tx,id)?;
    queue(&tx,&updated,now)?;
    tx.commit()
}

fn queue(tx:&rusqlite::Transaction<'_>,leave:&LeaveRequest,now:&str)->Result<(),rusqlite::Error>{let payload=serde_json::to_string(leave).unwrap_or_default();tx.execute("INSERT INTO sync_outbox(id,operation,entity,entity_id,payload,created_at) VALUES(?, 'upsert','leave',?,?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload,created_at=excluded.created_at,last_error=NULL",params![format!("sync-leave-{}",leave.id),leave.id,payload,now])?;Ok(())}
fn row(r:&rusqlite::Row<'_>)->Result<LeaveRequest,rusqlite::Error>{Ok(LeaveRequest{id:r.get(0)?,employee_id:r.get(1)?,leave_type:r.get(2)?,start_date:r.get(3)?,end_date:r.get(4)?,reason:r.get(5)?,status:r.get(6)?,reviewed_by:r.get(7)?,reviewed_at:r.get(8)?})}
fn balance_row(r:&rusqlite::Row<'_>)->Result<LeaveBalance,rusqlite::Error>{Ok(LeaveBalance{id:r.get(0)?,employee_id:r.get(1)?,leave_type_id:r.get(2)?,year:r.get(3)?,allocated_days:r.get(4)?,used_days:r.get(5)?,remaining_days:r.get(6)?})}
fn row_from_id(c:&Connection,id:&str)->Result<LeaveRequest,rusqlite::Error>{c.query_row("SELECT id,employee_id,leave_type,start_date,end_date,reason,status,reviewed_by,reviewed_at FROM leave_requests WHERE id=?",[id],row)}
