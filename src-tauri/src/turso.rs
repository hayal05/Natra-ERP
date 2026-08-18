use keyring::Entry;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "NATRA-ERP-TURSO";
const ACCOUNT_URL: &str = "database_url";
const ACCOUNT_TOKEN: &str = "auth_token";

#[derive(Debug, Serialize, Deserialize)]
pub struct TursoConfig { pub database_url: String, pub configured: bool }

fn entry(account: &str) -> Result<Entry, String> { Entry::new(SERVICE, account).map_err(|e| e.to_string()) }

pub fn save(url: &str, token: &str) -> Result<(), String> {
    if !url.starts_with("libsql://") && !url.starts_with("https://") { return Err("Invalid Turso database URL".into()); }
    if token.trim().is_empty() { return Err("Turso auth token is required".into()); }
    entry(ACCOUNT_URL)?.set_password(url.trim()).map_err(|e| e.to_string())?;
    entry(ACCOUNT_TOKEN)?.set_password(token.trim()).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn status() -> Result<TursoConfig, String> {
    let url = entry(ACCOUNT_URL)?.get_password().unwrap_or_default();
    Ok(TursoConfig { configured: !url.is_empty(), database_url: url })
}

pub fn clear() -> Result<(), String> {
    let _ = entry(ACCOUNT_URL)?.delete_credential();
    let _ = entry(ACCOUNT_TOKEN)?.delete_credential();
    Ok(())
}

pub fn credentials() -> Result<(String, String), String> {
    Ok((entry(ACCOUNT_URL)?.get_password().map_err(|e| e.to_string())?, entry(ACCOUNT_TOKEN)?.get_password().map_err(|e| e.to_string())?))
}
