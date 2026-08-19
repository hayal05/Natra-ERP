use libsql::{params, Builder};
use rusqlite::{params as sqlite_params, Connection};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct SyncStatus { pub pending: i64, pub failed: i64, pub last_error: Option<String> }

const MAX_SYNC_PAYLOAD_BYTES: usize = 1_048_576;

pub fn status(path: &PathBuf) -> Result<SyncStatus, String> {
    let c = Connection::open(path).map_err(|e| e.to_string())?;
    let pending: i64 = c.query_row("SELECT COUNT(*) FROM sync_outbox", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    let failed: i64 = c.query_row("SELECT COUNT(*) FROM sync_outbox WHERE last_error IS NOT NULL", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    let last_error: Option<String> = c.query_row("SELECT last_error FROM sync_outbox WHERE last_error IS NOT NULL ORDER BY created_at DESC LIMIT 1", [], |r| r.get(0)).ok();
    Ok(SyncStatus { pending, failed, last_error })
}
fn mark_failure(path:&PathBuf,id:&str,error:&str)->Result<(),String>{let c=Connection::open(path).map_err(|e|e.to_string())?;c.execute("UPDATE sync_outbox SET attempts=attempts+1,last_error=? WHERE id=?",sqlite_params![error,id]).map_err(|e|e.to_string())?;Ok(())}
fn mark_success(path:&PathBuf,id:&str)->Result<(),String>{let c=Connection::open(path).map_err(|e|e.to_string())?;c.execute("DELETE FROM sync_outbox WHERE id=?",sqlite_params![id]).map_err(|e|e.to_string())?;Ok(())}
fn load_outbox(path:&PathBuf)->Result<Vec<(String,String,String,String)>,String>{let c=Connection::open(path).map_err(|e|e.to_string())?;let mut stmt=c.prepare("SELECT id,entity,entity_id,payload FROM sync_outbox ORDER BY created_at LIMIT 50").map_err(|e|e.to_string())?;let items=stmt.query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;Ok(items)}

fn validate_sync_payload(entity: &str, entity_id: &str, payload: &str) -> Result<serde_json::Value, String> {
    if entity_id.trim().is_empty() { return Err("Sync item has an empty entity ID".into()); }
    if payload.len() > MAX_SYNC_PAYLOAD_BYTES { return Err("Sync payload exceeds the 1 MiB safety limit".into()); }
    let v: serde_json::Value = serde_json::from_str(payload).map_err(|e| format!("Invalid sync JSON: {e}"))?;
    if !v.is_object() { return Err("Sync payload must be a JSON object".into()); }
    if let Some(id) = v.get("id").and_then(|x| x.as_str()) { if id.trim().is_empty() || id != entity_id { return Err("Sync entity ID does not match payload ID".into()); } }
    match entity {
        "employee" => {
            let status = v["status"].as_str().unwrap_or("active");
            if !matches!(status, "active" | "inactive") { return Err("Invalid employee status".into()); }
            for key in ["employee_number", "first_name", "last_name"] { if v[key].as_str().unwrap_or("").trim().is_empty() { return Err(format!("Employee {key} is required")); } }
        }
        "user" => {
            let role = v["role"].as_str().unwrap_or("");
            if !matches!(role, "hr_admin" | "employee") { return Err("Invalid user role".into()); }
            if v["username"].as_str().unwrap_or("").trim().is_empty() { return Err("Username is required".into()); }
            if v["password_hash"].as_str().unwrap_or("").trim().is_empty() { return Err("Password hash is required".into()); }
        }
        "attendance" => {
            for key in ["employee_id", "attendance_date", "check_in_at", "token_id"] { if v[key].as_str().unwrap_or("").trim().is_empty() { return Err(format!("Attendance {key} is required")); } }
            if !matches!(v["status"].as_str().unwrap_or("present"), "present" | "late" | "absent") { return Err("Invalid attendance status".into()); }
        }
        "leave_type" => {
            if v["name"].as_str().unwrap_or("").trim().is_empty() { return Err("Leave type name is required".into()); }
            if v["annual_days"].as_i64().unwrap_or(-1) < 0 { return Err("Leave annual days cannot be negative".into()); }
        }
        "leave_balance" => {
            let allocated = v["allocated_days"].as_i64().unwrap_or(-1);
            let used = v["used_days"].as_i64().unwrap_or(-1);
            if allocated < 0 || used < 0 || used > allocated { return Err("Invalid leave balance".into()); }
            if v["year"].as_i64().unwrap_or(0) < 2000 { return Err("Invalid leave balance year".into()); }
        }
        "leave" => {
            let start = v["start_date"].as_str().unwrap_or("");
            let end = v["end_date"].as_str().unwrap_or("");
            if start.is_empty() || end.is_empty() || start > end { return Err("Invalid leave date range".into()); }
            if !matches!(v["status"].as_str().unwrap_or("pending"), "pending" | "approved" | "rejected" | "cancelled") { return Err("Invalid leave status".into()); }
        }
        "salary" => {
            let base = v["base_salary"].as_f64().ok_or("Invalid base salary")?;
            let allowances = v["allowances"].as_f64().ok_or("Invalid allowances")?;
            let deductions = v["deductions"].as_f64().ok_or("Invalid deductions")?;
            let net = v["net_salary"].as_f64().ok_or("Invalid net salary")?;
            if !base.is_finite() || !allowances.is_finite() || !deductions.is_finite() || !net.is_finite() || base < 0.0 || allowances < 0.0 || deductions < 0.0 || deductions > base + allowances { return Err("Invalid payroll amounts".into()); }
            let expected = base + allowances - deductions;
            if (net - expected).abs() > 0.01 { return Err("Payroll net salary does not match gross less deductions".into()); }
            if !matches!(v["status"].as_str().unwrap_or("draft"), "draft" | "processed" | "locked") { return Err("Invalid payroll status".into()); }
        }
        "payroll_period" => {
            if v["period"].as_str().unwrap_or("").trim().is_empty() { return Err("Payroll period is required".into()); }
            if !matches!(v["status"].as_str().unwrap_or("draft"), "draft" | "processed" | "locked") { return Err("Invalid payroll period status".into()); }
        }
        _ => return Err(format!("Unsupported sync entity: {entity}")),
    }
    Ok(v)
}

async fn ensure_schema(conn:&libsql::Connection)->Result<(),String>{conn.execute_batch("CREATE TABLE IF NOT EXISTS employees(id TEXT PRIMARY KEY,employee_number TEXT NOT NULL UNIQUE,first_name TEXT NOT NULL,last_name TEXT NOT NULL,email TEXT,phone TEXT,department TEXT,position TEXT,hire_date TEXT,status TEXT NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS attendance(id TEXT PRIMARY KEY,employee_id TEXT NOT NULL,attendance_date TEXT NOT NULL,check_in_at TEXT NOT NULL,status TEXT NOT NULL DEFAULT 'present',token_id TEXT NOT NULL,created_at TEXT NOT NULL,UNIQUE(employee_id,attendance_date)); CREATE TABLE IF NOT EXISTS leave_types(id TEXT PRIMARY KEY,name TEXT NOT NULL UNIQUE,annual_days INTEGER NOT NULL,active INTEGER NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS leave_balances(id TEXT PRIMARY KEY,employee_id TEXT NOT NULL,leave_type_id TEXT NOT NULL,year INTEGER NOT NULL,allocated_days INTEGER NOT NULL,used_days INTEGER NOT NULL,UNIQUE(employee_id,leave_type_id,year)); CREATE TABLE IF NOT EXISTS leave_requests(id TEXT PRIMARY KEY,employee_id TEXT NOT NULL,leave_type TEXT NOT NULL,start_date TEXT NOT NULL,end_date TEXT NOT NULL,reason TEXT,status TEXT NOT NULL,reviewed_by TEXT,reviewed_at TEXT,created_at TEXT NOT NULL,updated_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS salary_records(id TEXT PRIMARY KEY,employee_id TEXT NOT NULL,pay_period TEXT NOT NULL,base_salary REAL NOT NULL DEFAULT 0,allowances REAL NOT NULL DEFAULT 0,deductions REAL NOT NULL DEFAULT 0,net_salary REAL NOT NULL DEFAULT 0,status TEXT NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL,UNIQUE(employee_id,pay_period)); CREATE TABLE IF NOT EXISTS users(id TEXT PRIMARY KEY,username TEXT NOT NULL UNIQUE,password_hash TEXT NOT NULL,role TEXT NOT NULL,employee_id TEXT,active INTEGER NOT NULL DEFAULT 1,must_change_password INTEGER NOT NULL DEFAULT 0,created_at TEXT NOT NULL,updated_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS payroll_periods(id TEXT PRIMARY KEY,period TEXT NOT NULL UNIQUE,status TEXT NOT NULL,processed_at TEXT,processed_by TEXT,created_at TEXT NOT NULL,updated_at TEXT NOT NULL);").await.map_err(|e|e.to_string())?;let _=conn.execute("ALTER TABLE users ADD COLUMN must_change_password INTEGER NOT NULL DEFAULT 0",()).await;Ok(())}

pub async fn sync_once(path:&PathBuf,url:String,token:String)->Result<SyncStatus,String>{
    let remote=Builder::new_remote(url,token).build().await.map_err(|e|e.to_string())?;
    let conn=remote.connect().map_err(|e|e.to_string())?;
    ensure_schema(&conn).await?;
    let items=load_outbox(path)?;
    for(outbox_id,entity,entity_id,payload)in items{
        let validated = match validate_sync_payload(&entity, &entity_id, &payload) {
            Ok(v) => v,
            Err(e) => { mark_failure(path,&outbox_id,&e)?; continue; }
        };
        let result:Result<(),String>=match entity.as_str(){
"employee"=>{let v=validated;conn.execute("INSERT INTO employees(id,employee_number,first_name,last_name,email,phone,department,position,hire_date,status,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET employee_number=excluded.employee_number,first_name=excluded.first_name,last_name=excluded.last_name,email=excluded.email,phone=excluded.phone,department=excluded.department,position=excluded.position,hire_date=excluded.hire_date,status=excluded.status,updated_at=excluded.updated_at",params![v["id"].as_str().unwrap_or(&entity_id),v["employee_number"].as_str().unwrap_or(""),v["first_name"].as_str().unwrap_or(""),v["last_name"].as_str().unwrap_or(""),v["email"].as_str(),v["phone"].as_str(),v["department"].as_str(),v["position"].as_str(),v["hire_date"].as_str(),v["status"].as_str().unwrap_or("active"),v["created_at"].as_str().unwrap_or(""),v["updated_at"].as_str().unwrap_or("")]).await.map(|_|()).map_err(|e|e.to_string())}
"user"=>{let v=validated;conn.execute("INSERT INTO users(id,username,password_hash,role,employee_id,active,must_change_password,created_at,updated_at) VALUES(?,?,?,?,?,?,?,'','') ON CONFLICT(id) DO UPDATE SET username=excluded.username,password_hash=excluded.password_hash,role=excluded.role,employee_id=excluded.employee_id,active=excluded.active,must_change_password=excluded.must_change_password,updated_at=excluded.updated_at",params![v["id"].as_str().unwrap_or(&entity_id),v["username"].as_str().unwrap_or(""),v["password_hash"].as_str().unwrap_or(""),v["role"].as_str().unwrap_or("employee"),v["employee_id"].as_str(),if v["active"].as_bool().unwrap_or(true){1}else{0},if v["must_change_password"].as_bool().unwrap_or(false){1}else{0}]).await.map(|_|()).map_err(|e|e.to_string())}
"attendance"=>{let v=validated;conn.execute("INSERT INTO attendance(id,employee_id,attendance_date,check_in_at,status,token_id,created_at) VALUES(?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET employee_id=excluded.employee_id,attendance_date=excluded.attendance_date,check_in_at=excluded.check_in_at,status=excluded.status,token_id=excluded.token_id",params![v["id"].as_str().unwrap_or(&entity_id),v["employee_id"].as_str().unwrap_or(""),v["attendance_date"].as_str().unwrap_or(""),v["check_in_at"].as_str().unwrap_or(""),v["status"].as_str().unwrap_or("present"),v["token_id"].as_str().unwrap_or(""),v["check_in_at"].as_str().unwrap_or("")]).await.map(|_|()).map_err(|e|e.to_string())}
"leave_type"=>{let v=validated;conn.execute("INSERT INTO leave_types(id,name,annual_days,active,created_at,updated_at) VALUES(?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,annual_days=excluded.annual_days,active=excluded.active,updated_at=excluded.updated_at",params![v["id"].as_str().unwrap_or(&entity_id),v["name"].as_str().unwrap_or(""),v["annual_days"].as_i64().unwrap_or(0),if v["active"].as_bool().unwrap_or(true){1}else{0},v["created_at"].as_str().unwrap_or(""),v["updated_at"].as_str().unwrap_or("")]).await.map(|_|()).map_err(|e|e.to_string())}
"leave_balance"=>{let v=validated;conn.execute("INSERT INTO leave_balances(id,employee_id,leave_type_id,year,allocated_days,used_days) VALUES(?,?,?,?,?,?) ON CONFLICT(employee_id,leave_type_id,year) DO UPDATE SET allocated_days=excluded.allocated_days,used_days=excluded.used_days",params![v["id"].as_str().unwrap_or(&entity_id),v["employee_id"].as_str().unwrap_or(""),v["leave_type_id"].as_str().unwrap_or(""),v["year"].as_i64().unwrap_or(0),v["allocated_days"].as_i64().unwrap_or(0),v["used_days"].as_i64().unwrap_or(0)]).await.map(|_|()).map_err(|e|e.to_string())}
"leave"=>{let v=validated;conn.execute("INSERT INTO leave_requests(id,employee_id,leave_type,start_date,end_date,reason,status,reviewed_by,reviewed_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET employee_id=excluded.employee_id,leave_type=excluded.leave_type,start_date=excluded.start_date,end_date=excluded.end_date,reason=excluded.reason,status=excluded.status,reviewed_by=excluded.reviewed_by,reviewed_at=excluded.reviewed_at,updated_at=excluded.updated_at",params![v["id"].as_str().unwrap_or(&entity_id),v["employee_id"].as_str().unwrap_or(""),v["leave_type"].as_str().unwrap_or(""),v["start_date"].as_str().unwrap_or(""),v["end_date"].as_str().unwrap_or(""),v["reason"].as_str(),v["status"].as_str().unwrap_or("pending"),v["reviewed_by"].as_str(),v["reviewed_at"].as_str(),"",""]).await.map(|_|()).map_err(|e|e.to_string())}
"salary"=>{let v=validated;conn.execute("INSERT INTO salary_records(id,employee_id,pay_period,base_salary,allowances,deductions,net_salary,status,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET employee_id=excluded.employee_id,pay_period=excluded.pay_period,base_salary=excluded.base_salary,allowances=excluded.allowances,deductions=excluded.deductions,net_salary=excluded.net_salary,status=excluded.status,updated_at=excluded.updated_at",params![v["id"].as_str().unwrap_or(&entity_id),v["employee_id"].as_str().unwrap_or(""),v["pay_period"].as_str().unwrap_or(""),v["base_salary"].as_f64().unwrap_or(0.0),v["allowances"].as_f64().unwrap_or(0.0),v["deductions"].as_f64().unwrap_or(0.0),v["net_salary"].as_f64().unwrap_or(0.0),v["status"].as_str().unwrap_or("draft"),"",""]).await.map(|_|()).map_err(|e|e.to_string())}
"payroll_period"=>{let v=validated;conn.execute("INSERT INTO payroll_periods(id,period,status,processed_at,processed_by,created_at,updated_at) VALUES(?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET period=excluded.period,status=excluded.status,processed_at=excluded.processed_at,processed_by=excluded.processed_by,updated_at=excluded.updated_at",params![v["id"].as_str().unwrap_or(&entity_id),v["period"].as_str().unwrap_or(""),v["status"].as_str().unwrap_or("draft"),v["processed_at"].as_str(),v["processed_by"].as_str(),"",v["updated_at"].as_str().unwrap_or("")]).await.map(|_|()).map_err(|e|e.to_string())}
_=>Err(format!("Unsupported sync entity: {entity}"))};match result{Ok(())=>mark_success(path,&outbox_id)?,Err(e)=>mark_failure(path,&outbox_id,&e)?}}
    status(path)
}