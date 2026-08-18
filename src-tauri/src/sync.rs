use libsql::{params, Builder};
use rusqlite::{params as sqlite_params, Connection};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct SyncStatus { pub pending: i64, pub failed: i64, pub last_error: Option<String> }

pub fn status(path: &PathBuf) -> Result<SyncStatus, String> {
    let c = Connection::open(path).map_err(|e| e.to_string())?;
    let pending: i64 = c.query_row("SELECT COUNT(*) FROM sync_outbox", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    let failed: i64 = c.query_row("SELECT COUNT(*) FROM sync_outbox WHERE last_error IS NOT NULL", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    let last_error: Option<String> = c.query_row("SELECT last_error FROM sync_outbox WHERE last_error IS NOT NULL ORDER BY created_at DESC LIMIT 1", [], |r| r.get(0)).ok();
    Ok(SyncStatus { pending, failed, last_error })
}

fn mark_failure(path: &PathBuf, id: &str, error: &str) -> Result<(), String> {
    let c = Connection::open(path).map_err(|e| e.to_string())?;
    c.execute("UPDATE sync_outbox SET attempts=attempts+1,last_error=? WHERE id=?", sqlite_params![error, id]).map_err(|e| e.to_string())?;
    Ok(())
}

fn mark_success(path: &PathBuf, id: &str) -> Result<(), String> {
    let c = Connection::open(path).map_err(|e| e.to_string())?;
    c.execute("DELETE FROM sync_outbox WHERE id=?", sqlite_params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_outbox(path: &PathBuf) -> Result<Vec<(String,String,String,String)>, String> {
    let c = Connection::open(path).map_err(|e| e.to_string())?;
    let mut stmt = c.prepare("SELECT id,entity,entity_id,payload FROM sync_outbox ORDER BY created_at LIMIT 50").map_err(|e| e.to_string())?;
    let items = stmt.query_map([], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
    Ok(items)
}

async fn ensure_schema(conn: &libsql::Connection) -> Result<(), String> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS employees(id TEXT PRIMARY KEY,employee_number TEXT NOT NULL UNIQUE,first_name TEXT NOT NULL,last_name TEXT NOT NULL,email TEXT,phone TEXT,department TEXT,position TEXT,hire_date TEXT,status TEXT NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS attendance(id TEXT PRIMARY KEY,employee_id TEXT NOT NULL,attendance_date TEXT NOT NULL,check_in_at TEXT NOT NULL,status TEXT NOT NULL DEFAULT 'present',token_id TEXT NOT NULL UNIQUE,created_at TEXT NOT NULL,UNIQUE(employee_id,attendance_date)); CREATE TABLE IF NOT EXISTS leave_requests(id TEXT PRIMARY KEY,employee_id TEXT NOT NULL,leave_type TEXT NOT NULL,start_date TEXT NOT NULL,end_date TEXT NOT NULL,reason TEXT,status TEXT NOT NULL,reviewed_by TEXT,reviewed_at TEXT,created_at TEXT NOT NULL,updated_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS salary_records(id TEXT PRIMARY KEY,employee_id TEXT NOT NULL,pay_period TEXT NOT NULL,base_salary REAL NOT NULL DEFAULT 0,allowances REAL NOT NULL DEFAULT 0,deductions REAL NOT NULL DEFAULT 0,net_salary REAL NOT NULL DEFAULT 0,status TEXT NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL,UNIQUE(employee_id,pay_period)); CREATE TABLE IF NOT EXISTS users(id TEXT PRIMARY KEY,username TEXT NOT NULL UNIQUE,password_hash TEXT NOT NULL,role TEXT NOT NULL,employee_id TEXT,active INTEGER NOT NULL DEFAULT 1,must_change_password INTEGER NOT NULL DEFAULT 0,created_at TEXT NOT NULL,updated_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS payroll_periods(id TEXT PRIMARY KEY,period TEXT NOT NULL UNIQUE,status TEXT NOT NULL,processed_at TEXT,processed_by TEXT,created_at TEXT NOT NULL,updated_at TEXT NOT NULL);").await.map_err(|e| e.to_string())?;
    let _ = conn.execute("ALTER TABLE users ADD COLUMN must_change_password INTEGER NOT NULL DEFAULT 0", ()).await;
    Ok(())
}

pub async fn sync_once(path: &PathBuf, url: String, token: String) -> Result<SyncStatus, String> {
    let remote = Builder::new_remote(url, token).build().await.map_err(|e| e.to_string())?;
    let conn = remote.connect().map_err(|e| e.to_string())?;
    ensure_schema(&conn).await?;
    let items = load_outbox(path)?;
    for (outbox_id, entity, entity_id, payload) in items {
        let result: Result<(), String> = match entity.as_str() {
            "employee" => {
                let v: serde_json::Value = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
                conn.execute("INSERT INTO employees(id,employee_number,first_name,last_name,email,phone,department,position,hire_date,status,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET employee_number=excluded.employee_number,first_name=excluded.first_name,last_name=excluded.last_name,email=excluded.email,phone=excluded.phone,department=excluded.department,position=excluded.position,hire_date=excluded.hire_date,status=excluded.status,updated_at=excluded.updated_at", params![v["id"].as_str().unwrap_or(&entity_id),v["employee_number"].as_str().unwrap_or(""),v["first_name"].as_str().unwrap_or(""),v["last_name"].as_str().unwrap_or(""),v["email"].as_str(),v["phone"].as_str(),v["department"].as_str(),v["position"].as_str(),v["hire_date"].as_str(),v["status"].as_str().unwrap_or("active"),v["created_at"].as_str().unwrap_or(""),v["updated_at"].as_str().unwrap_or("")]).await.map(|_| ()).map_err(|e| e.to_string())
            }
            "user" => {
                let v: serde_json::Value = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
                conn.execute("INSERT INTO users(id,username,password_hash,role,employee_id,active,must_change_password,created_at,updated_at) VALUES(?,?,?,?,?,?,?,'','') ON CONFLICT(id) DO UPDATE SET username=excluded.username,password_hash=excluded.password_hash,role=excluded.role,employee_id=excluded.employee_id,active=excluded.active,must_change_password=excluded.must_change_password,updated_at=excluded.updated_at", params![v["id"].as_str().unwrap_or(&entity_id),v["username"].as_str().unwrap_or(""),v["password_hash"].as_str().unwrap_or(""),v["role"].as_str().unwrap_or("employee"),v["employee_id"].as_str(),if v["active"].as_bool().unwrap_or(true){1}else{0},if v["must_change_password"].as_bool().unwrap_or(false){1}else{0}]).await.map(|_| ()).map_err(|e| e.to_string())
            }
            "attendance" => {
                let v: serde_json::Value = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
                conn.execute("INSERT INTO attendance(id,employee_id,attendance_date,check_in_at,status,token_id,created_at) VALUES(?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET employee_id=excluded.employee_id,attendance_date=excluded.attendance_date,check_in_at=excluded.check_in_at,status=excluded.status,token_id=excluded.token_id", params![v["id"].as_str().unwrap_or(&entity_id),v["employee_id"].as_str().unwrap_or(""),v["attendance_date"].as_str().unwrap_or(""),v["check_in_at"].as_str().unwrap_or(""),v["status"].as_str().unwrap_or("present"),v["token_id"].as_str().unwrap_or(""),v["check_in_at"].as_str().unwrap_or("")]).await.map(|_| ()).map_err(|e| e.to_string())
            }
            "leave" => {
                let v: serde_json::Value = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
                conn.execute("INSERT INTO leave_requests(id,employee_id,leave_type,start_date,end_date,reason,status,reviewed_by,reviewed_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET employee_id=excluded.employee_id,leave_type=excluded.leave_type,start_date=excluded.start_date,end_date=excluded.end_date,reason=excluded.reason,status=excluded.status,reviewed_by=excluded.reviewed_by,reviewed_at=excluded.reviewed_at,updated_at=excluded.updated_at", params![v["id"].as_str().unwrap_or(&entity_id),v["employee_id"].as_str().unwrap_or(""),v["leave_type"].as_str().unwrap_or(""),v["start_date"].as_str().unwrap_or(""),v["end_date"].as_str().unwrap_or(""),v["reason"].as_str(),v["status"].as_str().unwrap_or("pending"),v["reviewed_by"].as_str(),v["reviewed_at"].as_str(),"",""]).await.map(|_| ()).map_err(|e| e.to_string())
            }
            "salary" => {
                let v: serde_json::Value = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
                conn.execute("INSERT INTO salary_records(id,employee_id,pay_period,base_salary,allowances,deductions,net_salary,status,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET employee_id=excluded.employee_id,pay_period=excluded.pay_period,base_salary=excluded.base_salary,allowances=excluded.allowances,deductions=excluded.deductions,net_salary=excluded.net_salary,status=excluded.status,updated_at=excluded.updated_at", params![v["id"].as_str().unwrap_or(&entity_id),v["employee_id"].as_str().unwrap_or(""),v["pay_period"].as_str().unwrap_or(""),v["base_salary"].as_f64().unwrap_or(0.0),v["allowances"].as_f64().unwrap_or(0.0),v["deductions"].as_f64().unwrap_or(0.0),v["net_salary"].as_f64().unwrap_or(0.0),v["status"].as_str().unwrap_or("draft"),"",""]).await.map(|_| ()).map_err(|e| e.to_string())
            }
            _ => Err(format!("Unsupported sync entity: {entity}"))
        };
        match result { Ok(()) => mark_success(path,&outbox_id)?, Err(e) => mark_failure(path,&outbox_id,&e)? }
    }
    status(path)
}
