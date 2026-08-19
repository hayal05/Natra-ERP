use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize, Clone)]
pub struct AttendanceHistoryRow {
    pub id: String,
    pub employee_id: String,
    pub employee_name: String,
    pub department: Option<String>,
    pub attendance_date: String,
    pub check_in_at: String,
    pub status: String,
    pub token_id: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct AttendanceSummary {
    pub employee_id: Option<String>,
    pub from_date: String,
    pub to_date: String,
    pub recorded_days: i64,
    pub present_days: i64,
    pub late_days: i64,
    pub first_check_in: Option<String>,
    pub last_check_in: Option<String>,
}

fn connect(path: &Path) -> Result<Connection, String> {
    let c = Connection::open(path).map_err(|e| e.to_string())?;
    c.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| e.to_string())?;
    Ok(c)
}

fn valid_date(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    if !b
        .iter()
        .enumerate()
        .all(|(i, x)| matches!(i, 4 | 7) || x.is_ascii_digit())
    {
        return false;
    }
    let month = u32::from(b[5] - b'0') * 10 + u32::from(b[6] - b'0');
    let day = u32::from(b[8] - b'0') * 10 + u32::from(b[9] - b'0');
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

fn valid_id(value: &str) -> bool {
    let v = value.trim();
    !v.is_empty()
        && v.len() <= 128
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn validate_range(from_date: &str, to_date: &str) -> Result<(), String> {
    if !valid_date(from_date) || !valid_date(to_date) || from_date > to_date {
        return Err(
            "Invalid attendance date range. Use YYYY-MM-DD with a valid month and day.".into(),
        );
    }
    Ok(())
}

pub fn history(
    path: &Path,
    employee_id: Option<&str>,
    from_date: &str,
    to_date: &str,
) -> Result<Vec<AttendanceHistoryRow>, String> {
    validate_range(from_date, to_date)?;
    if let Some(id) = employee_id {
        if !valid_id(id) {
            return Err("Invalid employee ID.".into());
        }
    }
    let c = connect(path)?;
    let mut stmt = c.prepare(
        "SELECT a.id,a.employee_id,e.first_name||' '||e.last_name,e.department,a.attendance_date,a.check_in_at,a.status,a.token_id
         FROM attendance a JOIN employees e ON e.id=a.employee_id
         WHERE a.attendance_date BETWEEN ? AND ? AND (? IS NULL OR a.employee_id=?)
         ORDER BY a.attendance_date DESC,a.check_in_at DESC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![from_date, to_date, employee_id, employee_id], |r| {
            Ok(AttendanceHistoryRow {
                id: r.get(0)?,
                employee_id: r.get(1)?,
                employee_name: r.get(2)?,
                department: r.get(3)?,
                attendance_date: r.get(4)?,
                check_in_at: r.get(5)?,
                status: r.get(6)?,
                token_id: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn summary(
    path: &Path,
    employee_id: Option<&str>,
    from_date: &str,
    to_date: &str,
) -> Result<AttendanceSummary, String> {
    validate_range(from_date, to_date)?;
    if let Some(id) = employee_id {
        if !valid_id(id) {
            return Err("Invalid employee ID.".into());
        }
    }
    let c = connect(path)?;
    let row = c.query_row(
        "SELECT COUNT(*),COALESCE(SUM(CASE WHEN status='present' THEN 1 ELSE 0 END),0),COALESCE(SUM(CASE WHEN status='late' THEN 1 ELSE 0 END),0),MIN(check_in_at),MAX(check_in_at)
         FROM attendance WHERE attendance_date BETWEEN ? AND ? AND (? IS NULL OR employee_id=?)",
        params![from_date,to_date,employee_id,employee_id],
        |r| Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?,r.get::<_,Option<String>>(3)?,r.get::<_,Option<String>>(4)?))
    ).map_err(|e| e.to_string())?;
    Ok(AttendanceSummary {
        employee_id: employee_id.map(str::to_owned),
        from_date: from_date.into(),
        to_date: to_date.into(),
        recorded_days: row.0,
        present_days: row.1,
        late_days: row.2,
        first_check_in: row.3,
        last_check_in: row.4,
    })
}

pub fn delete(path: &Path, attendance_id: &str) -> Result<(), String> {
    if !valid_id(attendance_id) {
        return Err("Valid attendance record ID is required.".into());
    }
    let mut c = connect(path)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let exists: Option<(String, String, String)> = tx
        .query_row(
            "SELECT id,attendance_date,status FROM attendance WHERE id=?",
            [attendance_id.trim()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let (_, date, status) = exists.ok_or_else(|| "Attendance record not found.".to_string())?;
    if !valid_date(&date) {
        return Err("Attendance record has an invalid date and cannot be safely deleted.".into());
    }
    if !matches!(status.as_str(), "present" | "late" | "absent") {
        return Err("Attendance record has an invalid status and cannot be safely deleted.".into());
    }
    tx.execute("DELETE FROM attendance WHERE id=?", [attendance_id.trim()])
        .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM sync_outbox WHERE entity='attendance' AND entity_id=?",
        [attendance_id.trim()],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
