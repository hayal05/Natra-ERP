use libsql::{params, Builder};
use rusqlite::{params as sqlite_params, Connection};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct SyncStatus { pub pending: i64, pub last_error: Option<String> }

pub fn status(path: &PathBuf) -> Result<SyncStatus, String> {
    let c = Connection::open(path).map_err(|e| e.to_string())?;
    let pending: i64 = c.query_row("SELECT COUNT(*) FROM sync_outbox", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    let last_error: Option<String> = c.query_row("SELECT last_error FROM sync_outbox WHERE last_error IS NOT NULL ORDER BY created_at DESC LIMIT 1", [], |r| r.get(0)).ok();
    Ok(SyncStatus { pending, last_error })
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

pub async fn sync_once(path: &PathBuf, url: String, token: String) -> Result<SyncStatus, String> {
    let remote = Builder::new_remote(url, token).build().await.map_err(|e| e.to_string())?;
    let conn = remote.connect().map_err(|e| e.to_string())?;

    conn.execute("CREATE TABLE IF NOT EXISTS attendance (id TEXT PRIMARY KEY, employee_id TEXT NOT NULL, attendance_date TEXT NOT NULL, check_in_at TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'present', token_id TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL, UNIQUE(employee_id, attendance_date))", ()).await.map_err(|e| e.to_string())?;

    let c = Connection::open(path).map_err(|e| e.to_string())?;
    let mut stmt = c.prepare("SELECT id,entity,entity_id FROM sync_outbox ORDER BY created_at LIMIT 50").map_err(|e| e.to_string())?;
    let items: Vec<(String,String,String)> = stmt.query_map([], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e| e.to_string())?.collect::<Result<_,_>>().map_err(|e| e.to_string())?;
    drop(stmt);
    drop(c);

    for (outbox_id, entity, entity_id) in items {
        if entity != "attendance" { mark_failure(path, &outbox_id, &format!("Unsupported sync entity: {entity}"))?; continue; }
        let c = Connection::open(path).map_err(|e| e.to_string())?;
        let local: Result<(String,String,String,String,String,String), rusqlite::Error> = c.query_row("SELECT id,employee_id,attendance_date,check_in_at,status,token_id FROM attendance WHERE id=?", [&entity_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)));
        drop(c);
        let (id, employee_id, date, check_in, status, token_id) = match local { Ok(v) => v, Err(e) => { mark_failure(path,&outbox_id,&format!("Local attendance missing: {e}"))?; continue; } };
        let result = conn.execute("INSERT INTO attendance(id,employee_id,attendance_date,check_in_at,status,token_id,created_at) VALUES(?1,?2,?3,?4,?5,?6,?4) ON CONFLICT(id) DO UPDATE SET employee_id=excluded.employee_id,attendance_date=excluded.attendance_date,check_in_at=excluded.check_in_at,status=excluded.status,token_id=excluded.token_id", params![id,employee_id,date,check_in,status,token_id]).await;
        match result { Ok(_) => mark_success(path,&outbox_id)?, Err(e) => { mark_failure(path,&outbox_id,&e.to_string())?; } }
    }
    status(path)
}
