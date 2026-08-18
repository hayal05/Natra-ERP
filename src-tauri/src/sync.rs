use rusqlite::{params, Connection};
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

pub fn mark_success(path: &PathBuf, id: &str) -> Result<(), String> {
    let c = Connection::open(path).map_err(|e| e.to_string())?;
    c.execute("DELETE FROM sync_outbox WHERE id=?", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}
