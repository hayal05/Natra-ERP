use rusqlite::{params, Connection, OptionalExtension};

const MAX_FAILURES: i64 = 5;
const LOCK_SECONDS: i64 = 15 * 60;

fn now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn ensure_table(c: &Connection) -> Result<(), rusqlite::Error> {
    c.execute_batch("CREATE TABLE IF NOT EXISTS login_security (username TEXT PRIMARY KEY, failed_attempts INTEGER NOT NULL DEFAULT 0, locked_until INTEGER NOT NULL DEFAULT 0, last_failed_at INTEGER NOT NULL DEFAULT 0)")?;
    Ok(())
}

/// Returns Ok when the username is allowed to attempt authentication.
/// A locked account is intentionally reported without revealing whether the username exists.
pub fn check_login(path: &std::path::Path, username: &str) -> Result<(), String> {
    let c = Connection::open(path).map_err(|e| e.to_string())?;
    ensure_table(&c).map_err(|e| e.to_string())?;
    let until: i64 = c
        .query_row(
            "SELECT locked_until FROM login_security WHERE lower(username)=lower(?)",
            [username],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(0);
    let now = now();
    if until > now {
        let remaining = until - now;
        return Err(format!(
            "Account temporarily locked. Try again in {} minute(s).",
            (remaining + 59) / 60
        ));
    }
    if until != 0 {
        c.execute("UPDATE login_security SET locked_until=0,failed_attempts=0 WHERE lower(username)=lower(?)", [username]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn record_failure(path: &std::path::Path, username: &str) -> Result<(), String> {
    let c = Connection::open(path).map_err(|e| e.to_string())?;
    ensure_table(&c).map_err(|e| e.to_string())?;
    let current: i64 = c
        .query_row(
            "SELECT failed_attempts FROM login_security WHERE lower(username)=lower(?)",
            [username],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(0);
    let attempts = current + 1;
    let locked_until = if attempts >= MAX_FAILURES {
        now() + LOCK_SECONDS
    } else {
        0
    };
    c.execute("INSERT INTO login_security(username,failed_attempts,locked_until,last_failed_at) VALUES(lower(?),?,?,?) ON CONFLICT(username) DO UPDATE SET failed_attempts=excluded.failed_attempts,locked_until=excluded.locked_until,last_failed_at=excluded.last_failed_at", params![username, attempts, locked_until, now()]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn record_success(path: &std::path::Path, username: &str) -> Result<(), String> {
    let c = Connection::open(path).map_err(|e| e.to_string())?;
    ensure_table(&c).map_err(|e| e.to_string())?;
    c.execute(
        "DELETE FROM login_security WHERE lower(username)=lower(?)",
        [username],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
